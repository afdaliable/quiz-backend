use actix_web::{web, HttpResponse, Responder, get, post, put, delete, patch};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::AppState;
use crate::middleware::admin_middleware::AdminMiddleware;
use crate::dao::admin_hierarchy_dao::AdminHierarchyDao;
use crate::dao::taxonomy_dao::TaxonomyDao;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/hierarchy")
            .wrap(AdminMiddleware::new())
            // Tracks
            .service(list_tracks)
            .service(create_track)
            .service(update_track)
            .service(delete_track)
            .service(reorder_tracks)
            // Categories
            .service(list_categories)
            .service(create_category)
            .service(update_category)
            .service(delete_category)
            .service(reorder_categories)
            // Subcategories
            .service(list_subcategories)
            .service(create_subcategory)
            .service(update_subcategory)
            .service(delete_subcategory)
            // Topics
            .service(list_topics)
            .service(create_topic)
            .service(update_topic)
            .service(delete_topic)
            // Tags
            .service(list_tags)
            .service(create_tag)
            .service(update_tag)
            .service(delete_tag)
            .service(merge_tags)
            // Full tree
            .service(get_hierarchy_tree)
    )
    .service(
        web::scope("/admin")
            .wrap(AdminMiddleware::new())
            .service(bulk_update_questions)
            .service(get_question_stats)
    );
}

// ── Request/Response types ───────────────────────────────────────────────────

#[derive(Deserialize, ToSchema)]
pub struct CreateTrackRequest {
    pub name: String,
    pub slug: Option<String>,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
    pub status: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateTrackRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
    pub status: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ReorderRequest {
    pub ids: Vec<String>,
    pub track_id: Option<String>, // required for reorder_categories
}

#[derive(Deserialize, ToSchema)]
pub struct CreateCategoryRequest {
    pub track_id: String,
    pub name: String,
    pub slug: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateCategoryRequest {
    pub track_id: Option<String>,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateSubcategoryRequest {
    pub category_id: String,
    pub name: String,
    pub slug: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateSubcategoryRequest {
    pub category_id: Option<String>,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateTopicRequest {
    pub subcategory_id: String,
    pub name: String,
    pub slug: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateTopicRequest {
    pub subcategory_id: Option<String>,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateTagRequest {
    pub name: String,
    pub slug: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct MergeTagsRequest {
    pub source_tag_ids: Vec<String>,
    pub target_tag_id: String,
}

#[derive(Deserialize, ToSchema)]
pub struct DeleteQuery {
    pub cascade: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct ListCategoriesQuery {
    pub track_id: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ListSubcategoriesQuery {
    pub category_id: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ListTopicsQuery {
    pub subcategory_id: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct TagSearchQuery {
    pub search: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct BulkUpdateQuestionsRequest {
    pub question_ids: Vec<i32>,
    pub track_id: Option<String>,
    pub category_id: Option<String>,
    pub subcategory_id: Option<String>,
    pub topic_id: Option<String>,
    pub difficulty_est: Option<String>,
    pub status: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct BulkUpdateResponse {
    pub updated_count: u64,
}

// ── Tracks ───────────────────────────────────────────────────────────────────

#[get("/tracks")]
async fn list_tracks(data: web::Data<AppState<'_>>) -> impl Responder {
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    match dao.get_tracks().await {
        Ok(tracks) => HttpResponse::Ok().json(tracks),
        Err(e) => {
            eprintln!("Error listing tracks: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to list tracks"}))
        }
    }
}

#[post("/tracks")]
async fn create_track(
    body: web::Json<CreateTrackRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    let status = body.status.as_deref().unwrap_or("upcoming");
    let sort_order = body.sort_order.unwrap_or(0);

    match dao.create_track(
        &body.name,
        body.slug.as_deref(),
        body.icon.as_deref(),
        sort_order,
        status,
    ).await {
        Ok(track) => HttpResponse::Created().json(track),
        Err(e) => {
            eprintln!("Error creating track: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

#[put("/tracks/{id}")]
async fn update_track(
    path: web::Path<String>,
    body: web::Json<UpdateTrackRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());

    match dao.update_track(
        &id,
        body.name.as_deref(),
        body.slug.as_deref(),
        body.icon.as_deref(),
        body.sort_order,
        body.status.as_deref(),
    ).await {
        Ok(Some(track)) => HttpResponse::Ok().json(track),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Track not found"})),
        Err(e) => {
            eprintln!("Error updating track {}: {:?}", id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

#[delete("/tracks/{id}")]
async fn delete_track(
    path: web::Path<String>,
    query: web::Query<DeleteQuery>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    let cascade = query.cascade.unwrap_or(false);

    match dao.delete_track(&id, cascade).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({"deleted": true})),
        Ok(false) => HttpResponse::NotFound().json(serde_json::json!({"error": "Track not found"})),
        Err(msg) => HttpResponse::Conflict().json(serde_json::json!({"error": msg})),
    }
}

#[post("/tracks/reorder")]
async fn reorder_tracks(
    body: web::Json<ReorderRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    match dao.reorder_tracks(&body.ids).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({"reordered": true})),
        Err(e) => {
            eprintln!("Error reordering tracks: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

// ── Categories ───────────────────────────────────────────────────────────────

#[get("/categories")]
async fn list_categories(
    query: web::Query<ListCategoriesQuery>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    match dao.get_categories(query.track_id.as_deref()).await {
        Ok(cats) => HttpResponse::Ok().json(cats),
        Err(e) => {
            eprintln!("Error listing categories: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to list categories"}))
        }
    }
}

#[post("/categories")]
async fn create_category(
    body: web::Json<CreateCategoryRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    let sort_order = body.sort_order.unwrap_or(0);

    match dao.create_category(&body.track_id, &body.name, body.slug.as_deref(), sort_order).await {
        Ok(cat) => HttpResponse::Created().json(cat),
        Err(e) => {
            eprintln!("Error creating category: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

#[put("/categories/{id}")]
async fn update_category(
    path: web::Path<String>,
    body: web::Json<UpdateCategoryRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());

    match dao.update_category(&id, body.track_id.as_deref(), body.name.as_deref(), body.slug.as_deref(), body.sort_order).await {
        Ok(Some(cat)) => HttpResponse::Ok().json(cat),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Category not found"})),
        Err(e) => {
            eprintln!("Error updating category {}: {:?}", id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

#[delete("/categories/{id}")]
async fn delete_category(
    path: web::Path<String>,
    query: web::Query<DeleteQuery>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    let cascade = query.cascade.unwrap_or(false);

    match dao.delete_category(&id, cascade).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({"deleted": true})),
        Ok(false) => HttpResponse::NotFound().json(serde_json::json!({"error": "Category not found"})),
        Err(msg) => HttpResponse::Conflict().json(serde_json::json!({"error": msg})),
    }
}

#[post("/categories/reorder")]
async fn reorder_categories(
    body: web::Json<ReorderRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let track_id = match &body.track_id {
        Some(id) => id.clone(),
        None => return HttpResponse::BadRequest().json(serde_json::json!({"error": "track_id required"})),
    };
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    match dao.reorder_categories(&track_id, &body.ids).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({"reordered": true})),
        Err(e) => {
            eprintln!("Error reordering categories: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

// ── Subcategories ─────────────────────────────────────────────────────────────

#[get("/subcategories")]
async fn list_subcategories(
    query: web::Query<ListSubcategoriesQuery>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    match dao.get_subcategories(query.category_id.as_deref()).await {
        Ok(subs) => HttpResponse::Ok().json(subs),
        Err(e) => {
            eprintln!("Error listing subcategories: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to list subcategories"}))
        }
    }
}

#[post("/subcategories")]
async fn create_subcategory(
    body: web::Json<CreateSubcategoryRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    let sort_order = body.sort_order.unwrap_or(0);

    match dao.create_subcategory(&body.category_id, &body.name, body.slug.as_deref(), sort_order).await {
        Ok(sub) => HttpResponse::Created().json(sub),
        Err(e) => {
            eprintln!("Error creating subcategory: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

#[put("/subcategories/{id}")]
async fn update_subcategory(
    path: web::Path<String>,
    body: web::Json<UpdateSubcategoryRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());

    match dao.update_subcategory(&id, body.category_id.as_deref(), body.name.as_deref(), body.slug.as_deref(), body.sort_order).await {
        Ok(Some(sub)) => HttpResponse::Ok().json(sub),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Subcategory not found"})),
        Err(e) => {
            eprintln!("Error updating subcategory {}: {:?}", id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

#[delete("/subcategories/{id}")]
async fn delete_subcategory(
    path: web::Path<String>,
    query: web::Query<DeleteQuery>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    let cascade = query.cascade.unwrap_or(false);

    match dao.delete_subcategory(&id, cascade).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({"deleted": true})),
        Ok(false) => HttpResponse::NotFound().json(serde_json::json!({"error": "Subcategory not found"})),
        Err(msg) => HttpResponse::Conflict().json(serde_json::json!({"error": msg})),
    }
}

// ── Topics ────────────────────────────────────────────────────────────────────

#[get("/topics")]
async fn list_topics(
    query: web::Query<ListTopicsQuery>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    match dao.get_topics(query.subcategory_id.as_deref()).await {
        Ok(topics) => HttpResponse::Ok().json(topics),
        Err(e) => {
            eprintln!("Error listing topics: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to list topics"}))
        }
    }
}

#[post("/topics")]
async fn create_topic(
    body: web::Json<CreateTopicRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    let sort_order = body.sort_order.unwrap_or(0);

    match dao.create_topic(&body.subcategory_id, &body.name, body.slug.as_deref(), sort_order).await {
        Ok(topic) => HttpResponse::Created().json(topic),
        Err(e) => {
            eprintln!("Error creating topic: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

#[put("/topics/{id}")]
async fn update_topic(
    path: web::Path<String>,
    body: web::Json<UpdateTopicRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());

    match dao.update_topic(&id, body.subcategory_id.as_deref(), body.name.as_deref(), body.slug.as_deref(), body.sort_order).await {
        Ok(Some(topic)) => HttpResponse::Ok().json(topic),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Topic not found"})),
        Err(e) => {
            eprintln!("Error updating topic {}: {:?}", id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

#[delete("/topics/{id}")]
async fn delete_topic(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());

    match dao.delete_topic(&id).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({"deleted": true})),
        Ok(false) => HttpResponse::NotFound().json(serde_json::json!({"error": "Topic not found"})),
        Err(msg) => HttpResponse::Conflict().json(serde_json::json!({"error": msg})),
    }
}

// ── Tags ──────────────────────────────────────────────────────────────────────

#[get("/tags")]
async fn list_tags(
    query: web::Query<TagSearchQuery>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let search = query.search.as_deref().unwrap_or("");
    let dao = TaxonomyDao::new(data.context.soal.pool.clone());
    match dao.search_tags(search).await {
        Ok(tags) => HttpResponse::Ok().json(tags),
        Err(e) => {
            eprintln!("Error listing tags: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to list tags"}))
        }
    }
}

#[post("/tags")]
async fn create_tag(
    body: web::Json<CreateTagRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    match dao.create_tag(&body.name, body.slug.as_deref()).await {
        Ok(tag) => HttpResponse::Created().json(tag),
        Err(e) => {
            eprintln!("Error creating tag: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

#[put("/tags/{id}")]
async fn update_tag(
    path: web::Path<String>,
    body: web::Json<UpdateTagRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());

    match dao.update_tag(&id, body.name.as_deref(), body.slug.as_deref()).await {
        Ok(Some(tag)) => HttpResponse::Ok().json(tag),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Tag not found"})),
        Err(e) => {
            eprintln!("Error updating tag {}: {:?}", id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

#[delete("/tags/{id}")]
async fn delete_tag(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());

    match dao.delete_tag(&id).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({"deleted": true})),
        Ok(false) => HttpResponse::NotFound().json(serde_json::json!({"error": "Tag not found"})),
        Err(e) => {
            eprintln!("Error deleting tag {}: {:?}", id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

#[post("/tags/merge")]
async fn merge_tags(
    body: web::Json<MergeTagsRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    if body.source_tag_ids.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "source_tag_ids cannot be empty"}));
    }

    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    match dao.merge_tags(&body.source_tag_ids, &body.target_tag_id).await {
        Ok(Some(tag)) => HttpResponse::Ok().json(tag),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"error": "Target tag not found"})),
        Err(e) => {
            eprintln!("Error merging tags: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

// ── Full tree ─────────────────────────────────────────────────────────────────

#[get("/tree")]
async fn get_hierarchy_tree(data: web::Data<AppState<'_>>) -> impl Responder {
    let dao = TaxonomyDao::new(data.context.soal.pool.clone());
    match dao.get_taxonomy_tree().await {
        Ok(tree) => HttpResponse::Ok().json(tree),
        Err(e) => {
            eprintln!("Error building hierarchy tree: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to build hierarchy tree"}))
        }
    }
}

// ── Bulk update questions ─────────────────────────────────────────────────────

#[patch("/questions/bulk")]
async fn bulk_update_questions(
    body: web::Json<BulkUpdateQuestionsRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    if body.question_ids.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "question_ids cannot be empty"}));
    }

    if body.question_ids.len() > 200 {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Maximum 200 questions per bulk update"}));
    }

    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    match dao.bulk_update_questions(
        &body.question_ids,
        body.track_id.as_deref(),
        body.category_id.as_deref(),
        body.subcategory_id.as_deref(),
        body.topic_id.as_deref(),
        body.difficulty_est.as_deref(),
        body.status.as_deref(),
    ).await {
        Ok(count) => HttpResponse::Ok().json(BulkUpdateResponse { updated_count: count }),
        Err(e) => {
            eprintln!("Error bulk updating questions: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

// ── Stats ─────────────────────────────────────────────────────────────────────

#[get("/stats/questions")]
async fn get_question_stats(data: web::Data<AppState<'_>>) -> impl Responder {
    let dao = AdminHierarchyDao::new(data.context.soal.pool.clone());
    match dao.get_question_stats().await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => {
            eprintln!("Error getting question stats: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to get stats"}))
        }
    }
}
