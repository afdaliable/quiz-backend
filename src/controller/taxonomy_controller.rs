use actix_web::{get, web, HttpResponse, Responder};
use serde::Deserialize;
use crate::AppState;
use crate::dao::taxonomy_dao::TaxonomyDao;
use crate::service::redis_service::RedisService;

#[derive(Deserialize)]
pub struct TagSearchQuery {
    pub search: Option<String>,
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_all_tracks)
       .service(get_categories_by_track)
       .service(get_subcategories_by_category)
       .service(get_topics_by_subcategory)
       .service(get_tags_by_topic)
       .service(get_taxonomy_tree)
       .service(search_tags);
}

/// List all exam tracks
#[get("/tracks")]
async fn get_all_tracks(data: web::Data<AppState<'_>>) -> impl Responder {
    let dao = TaxonomyDao::new(data.context.soal.pool.clone());
    match dao.get_all_tracks().await {
        Ok(tracks) => HttpResponse::Ok().json(tracks),
        Err(e) => {
            eprintln!("Error fetching tracks: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to fetch tracks"}))
        }
    }
}

/// List categories for a track slug
#[get("/tracks/{slug}/categories")]
async fn get_categories_by_track(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let slug = path.into_inner();
    let dao = TaxonomyDao::new(data.context.soal.pool.clone());
    match dao.get_categories_by_track_slug(&slug).await {
        Ok(cats) => HttpResponse::Ok().json(cats),
        Err(e) => {
            eprintln!("Error fetching categories for track {}: {:?}", slug, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to fetch categories"}))
        }
    }
}

/// List subcategories for a category slug
#[get("/categories/{slug}/subcategories")]
async fn get_subcategories_by_category(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let slug = path.into_inner();
    let dao = TaxonomyDao::new(data.context.soal.pool.clone());
    match dao.get_subcategories_by_category_slug(&slug).await {
        Ok(subs) => HttpResponse::Ok().json(subs),
        Err(e) => {
            eprintln!("Error fetching subcategories for category {}: {:?}", slug, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to fetch subcategories"}))
        }
    }
}

/// List topics for a subcategory slug
#[get("/subcategories/{slug}/topics")]
async fn get_topics_by_subcategory(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let slug = path.into_inner();
    let dao = TaxonomyDao::new(data.context.soal.pool.clone());
    match dao.get_topics_by_subcategory_slug(&slug).await {
        Ok(topics) => HttpResponse::Ok().json(topics),
        Err(e) => {
            eprintln!("Error fetching topics for subcategory {}: {:?}", slug, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to fetch topics"}))
        }
    }
}

/// List tags for a topic slug
#[get("/topics/{slug}/tags")]
async fn get_tags_by_topic(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let slug = path.into_inner();
    let dao = TaxonomyDao::new(data.context.soal.pool.clone());
    match dao.get_tags_by_topic_slug(&slug).await {
        Ok(tags) => HttpResponse::Ok().json(tags),
        Err(e) => {
            eprintln!("Error fetching tags for topic {}: {:?}", slug, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to fetch tags"}))
        }
    }
}

/// Full taxonomy tree — Redis cached for 1 hour
#[get("/taxonomy/tree")]
async fn get_taxonomy_tree(data: web::Data<AppState<'_>>) -> impl Responder {
    const CACHE_KEY: &str = "taxonomy:tree:v1";
    const TTL: usize = 3600;

    // Try Redis cache first
    if let Some(redis_pool) = &data.redis_pool {
        let mut con = redis_pool.quiz_cache().as_ref().clone();
        if let Ok(Some(cached)) = RedisService::get_cached_quiz_by_key::<crate::model::taxonomy::TaxonomyTree>(&mut con, CACHE_KEY).await {
            return HttpResponse::Ok().json(cached);
        }

        let dao = TaxonomyDao::new(data.context.soal.pool.clone());
        match dao.get_taxonomy_tree().await {
            Ok(tree) => {
                let _ = RedisService::cache_quiz_by_key(&mut con, CACHE_KEY, &tree, TTL).await;
                HttpResponse::Ok().json(tree)
            }
            Err(e) => {
                eprintln!("Error building taxonomy tree: {:?}", e);
                HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to build taxonomy tree"}))
            }
        }
    } else {
        let dao = TaxonomyDao::new(data.context.soal.pool.clone());
        match dao.get_taxonomy_tree().await {
            Ok(tree) => HttpResponse::Ok().json(tree),
            Err(e) => {
                eprintln!("Error building taxonomy tree: {:?}", e);
                HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to build taxonomy tree"}))
            }
        }
    }
}

/// Search tags — autocomplete
#[get("/tags")]
async fn search_tags(
    query: web::Query<TagSearchQuery>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let search = query.search.as_deref().unwrap_or("");
    let dao = TaxonomyDao::new(data.context.soal.pool.clone());
    match dao.search_tags(search).await {
        Ok(tags) => HttpResponse::Ok().json(tags),
        Err(e) => {
            eprintln!("Error searching tags: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to search tags"}))
        }
    }
}
