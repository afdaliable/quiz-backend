use crate::controller::log_request;
use crate::middleware::admin_middleware::AdminMiddleware;
use crate::model::soal::{Soal, AdminSoal, UpdateSoalRequest, QuestionSearchRequest, PaginatedQuestionsResponse, BulkImportRequest, BulkImportResponse, CreateSoalRequest, CsvImportRequest, CsvImportResponse, SoalWithTaxonomy};
use crate::model::paket_soal::{SoalPackageItem, SoalPackagesResponse, SoalCoverageStats};
use crate::dao::taxonomy_dao::TaxonomyDao;
use crate::service::csv_import_service::{CsvImportService, CsvImportConfig};
use crate::service::redis_service::RedisService;
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest, get, post, put, delete};
use actix_multipart::Multipart;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::io::Write;
use tempfile::NamedTempFile;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/soal")
            .wrap(AdminMiddleware::new())
            .service(search_questions)
            .service(list_questions)
            .service(get_dropdowns)
            .service(create_question)
            .service(update_question)
            .service(delete_question)
            .service(bulk_import_questions)
            .service(upload_csv_preview)
            .service(upload_csv_import)
            .service(download_csv_template)
            // AFD-244: static paths before /{id} wildcard
            .service(coverage_stats)
            .service(get_question_by_id)
            .service(get_soal_packages)
            .service(upload_image)
    );
}

/// Turns free-typed search text into a MySQL FULLTEXT BOOLEAN MODE query:
/// each whitespace-separated token becomes `+token*` (required, prefix
/// match), stripped of anything but alphanumerics so user input can't
/// inject boolean-mode operators (+-><()~*"@). Returns None for empty/
/// punctuation-only input, which the caller treats as "no search filter".
pub(crate) fn build_fulltext_boolean_query(search: &str) -> Option<String> {
    // Split on ANY non-alphanumeric char, not just whitespace -- MySQL's
    // FULLTEXT parser treats hyphens/punctuation as word boundaries too
    // (indexes "Unsur-unsur" as two words: "unsur", "unsur"). The old
    // version only split on whitespace, then stripped punctuation WITHIN
    // each token instead of splitting on it -- "Unsur-unsur" glued into
    // "Unsurunsur", a token that was never indexed, so the query silently
    // matched nothing. Confirmed live: a real soal's exact opening phrase
    // returned zero search results because of this.
    let cleaned: Vec<String> = search
        .split(|c: char| !c.is_alphanumeric())
        .filter(|tok| !tok.is_empty())
        .map(|tok| format!("+{}*", tok))
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.join(" "))
    }
}

/// Search questions with pagination and filtering
#[utoipa::path(
    get,
    path = "/admin/soal/search",
    params(
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("limit" = Option<u32>, Query, description = "Items per page (default: 20)"),
        ("search" = Option<String>, Query, description = "Search in question text"),
        ("modul" = Option<String>, Query, description = "Filter by module"),
        ("pelajaran" = Option<String>, Query, description = "Filter by subject"),
        ("tag" = Option<String>, Query, description = "Filter by tag")
    ),
    responses(
        (status = 200, description = "Questions retrieved successfully", body = PaginatedQuestionsResponse),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/search")]
async fn search_questions(
    query: web::Query<QuestionSearchRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/soal/search", &data.connections);

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;

    // Build dynamic query
    let mut where_conditions = vec!["1=1"];
    let mut bind_values: Vec<String> = vec![];

    if let Some(id) = query.id {
        where_conditions.push("s.id = ?");
        bind_values.push(id.to_string());
    } else if let Some(ref search) = query.search {
        // Was `s.soal LIKE '%term%'` -- a leading wildcard defeats any
        // index, forcing a full scan of the soal table on every search
        // (confirmed live: 15s+ per request). `ft_soal_text` FULLTEXT
        // index + BOOLEAN MODE with a trailing `*` gives the same
        // prefix-as-you-type matching through the index instead.
        if let Some(boolean_query) = build_fulltext_boolean_query(search) {
            where_conditions.push("MATCH(s.soal) AGAINST (? IN BOOLEAN MODE)");
            bind_values.push(boolean_query);
        }
    }

    if let Some(ref modul) = query.modul {
        where_conditions.push("s.modul = ?");
        bind_values.push(modul.clone());
    }

    if let Some(ref pelajaran) = query.pelajaran {
        where_conditions.push("s.pelajaran = ?");
        bind_values.push(pelajaran.clone());
    }

    if let Some(ref tag) = query.tag {
        where_conditions.push("s.tag = ?");
        bind_values.push(tag.clone());
    }

    // AFD-204: taxonomy slug-based filters (JOIN approach)
    if let Some(ref track_slug) = query.track {
        where_conditions.push("EXISTS (SELECT 1 FROM exam_tracks et WHERE et.id = s.track_id AND et.slug = ?)");
        bind_values.push(track_slug.clone());
    }
    if let Some(ref cat_slug) = query.category {
        where_conditions.push("EXISTS (SELECT 1 FROM categories c WHERE c.id = s.category_id AND c.slug = ?)");
        bind_values.push(cat_slug.clone());
    }
    if let Some(ref sub_slug) = query.subcategory {
        where_conditions.push("EXISTS (SELECT 1 FROM subcategories sc WHERE sc.id = s.subcategory_id AND sc.slug = ?)");
        bind_values.push(sub_slug.clone());
    }
    if let Some(ref top_slug) = query.topic {
        // AFD-226: match direct FK OR via question_topics M2M
        where_conditions.push(
            "(EXISTS (SELECT 1 FROM topics tp WHERE tp.id = s.topic_id AND tp.slug = ?) \
             OR EXISTS (SELECT 1 FROM question_topics qt JOIN topics tp ON tp.id = qt.topic_id WHERE qt.question_id = s.id AND tp.slug = ?))"
        );
        bind_values.push(top_slug.clone());
        bind_values.push(top_slug.clone());
    }
    if let Some(ref diff) = query.difficulty {
        where_conditions.push("s.difficulty_est = ?");
        bind_values.push(diff.clone());
    }
    if let Some(ref bloom) = query.bloom_level {
        where_conditions.push("s.bloom_level = ?");
        bind_values.push(bloom.clone());
    }
    if let Some(ref fmt) = query.format {
        where_conditions.push("s.format = ?");
        bind_values.push(fmt.clone());
    }
    if let Some(ref src) = query.source {
        where_conditions.push("s.source = ?");
        bind_values.push(src.clone());
    }
    if let Some(ref status) = query.status {
        where_conditions.push("s.status = ?");
        bind_values.push(status.clone());
    }
    if let Some(has_answer) = query.has_answer {
        if has_answer {
            where_conditions.push("s.correct_answer IS NOT NULL AND s.correct_answer != ''");
        } else {
            where_conditions.push("(s.correct_answer IS NULL OR s.correct_answer = '')");
        }
    }
    if let Some(has_solution) = query.has_solution {
        if has_solution {
            where_conditions.push("s.solution IS NOT NULL AND s.solution != ''");
        } else {
            where_conditions.push("(s.solution IS NULL OR s.solution = '')");
        }
    }
    if let Some(has_tag) = query.has_tag {
        if has_tag {
            where_conditions.push("s.tag IS NOT NULL AND s.tag != ''");
        } else {
            where_conditions.push("(s.tag IS NULL OR s.tag = '')");
        }
    }

    let where_clause = where_conditions.join(" AND ");

    // Count total questions
    let count_query = format!(
        "SELECT COUNT(*) FROM dbquizapp.soal s WHERE {}",
        where_clause
    );

    let mut count_query_builder = sqlx::query_scalar::<_, i64>(&count_query);
    for value in &bind_values {
        count_query_builder = count_query_builder.bind(value);
    }

    let total: i64 = match count_query_builder
        .fetch_one(&*data.context.soal.pool)
        .await 
    {
        Ok(count) => count,
        Err(e) => {
            println!("Error counting questions: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to count questions".to_string(),
            });
        }
    };

    // Get questions (AFD-226: include topic_ids_csv via correlated subquery)
    let questions_query = format!(
        r#"
        SELECT
            s.*,
            COALESCE(s.created_at, NOW()) as created_at,
            COALESCE(s.updated_at, NOW()) as updated_at,
            0 as usage_count,
            (SELECT GROUP_CONCAT(DISTINCT qt.topic_id ORDER BY qt.topic_id SEPARATOR ',')
             FROM question_topics qt WHERE qt.question_id = s.id) AS topic_ids_csv
        FROM dbquizapp.soal s
        WHERE {}
        ORDER BY s.id DESC
        LIMIT ? OFFSET ?
        "#,
        where_clause
    );

    let mut questions_query_builder = sqlx::query_as::<_, AdminSoal>(&questions_query);
    for value in &bind_values {
        questions_query_builder = questions_query_builder.bind(value);
    }

    let questions = match questions_query_builder
        .bind(limit)
        .bind(offset)
        .fetch_all(&*data.context.soal.pool)
        .await 
    {
        Ok(questions) => questions,
        Err(e) => {
            println!("Error fetching questions: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch questions".to_string(),
            });
        }
    };

    let total_pages = (total as f64 / limit as f64).ceil() as u32;

    HttpResponse::Ok().json(PaginatedQuestionsResponse {
        questions,
        total,
        page,
        limit,
        total_pages,
    })
}

/// Get question by ID
#[utoipa::path(
    get,
    path = "/admin/soal/{id}",
    params(
        ("id" = i32, Path, description = "Question ID")
    ),
    responses(
        (status = 200, description = "Question found", body = AdminSoal),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Question not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/{id}")]
async fn get_question_by_id(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/soal/{id}", &data.connections);

    let question_id = path.into_inner();
    
    let query = r#"
        SELECT
            s.*,
            COALESCE(s.created_at, NOW()) as created_at,
            COALESCE(s.updated_at, NOW()) as updated_at,
            0 as usage_count
        FROM dbquizapp.soal s
        WHERE s.id = ?
    "#;

    match sqlx::query_as::<_, Soal>(query)
        .bind(question_id)
        .fetch_one(&*data.context.soal.pool)
        .await
    {
        Ok(mut soal) => {
            let taxonomy_dao = TaxonomyDao::new(data.context.soal.pool.clone());
            let taxonomy = taxonomy_dao.get_taxonomy_context(&soal).await;
            // AFD-226: populate topic_ids from question_topics M2M
            let topic_ids = taxonomy_dao.get_topics_for_question(soal.id).await.unwrap_or_default();
            soal.topic_ids = if topic_ids.is_empty() { None } else { Some(topic_ids) };
            HttpResponse::Ok().json(SoalWithTaxonomy {
                base: soal,
                taxonomy: Some(taxonomy),
            })
        }
        Err(e) => {
            println!("Error fetching question: {:?}", e);
            HttpResponse::NotFound().json(ErrorResponse {
                error: "Question not found".to_string(),
            })
        }
    }
}

/// Create new question
#[utoipa::path(
    post,
    path = "/admin/soal",
    request_body = CreateSoalRequest,
    responses(
        (status = 201, description = "Question created successfully", body = Soal),
        (status = 400, description = "Invalid question data"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("")]
async fn create_question(
    question_req: web::Json<CreateSoalRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/soal", &data.connections);

    // Validate question data
    if question_req.soal.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Question text cannot be empty".to_string(),
        });
    }

    match data.context.soal.create_soal(&*question_req).await {
        Ok(question) => {
            let taxonomy_dao = TaxonomyDao::new(data.context.soal.pool.clone());
            // Manage question_tags if tag_ids provided
            if let Some(ref tag_ids) = question_req.tag_ids {
                if let Err(e) = taxonomy_dao.set_question_tags(question.id, tag_ids).await {
                    eprintln!("Error setting question tags: {:?}", e);
                }
            }
            // AFD-226: Manage question_topics if topic_ids provided
            if let Some(ref topic_ids) = question_req.topic_ids {
                if let Err(e) = taxonomy_dao.set_question_topics(question.id, topic_ids).await {
                    eprintln!("Error setting question topics: {:?}", e);
                }
            }
            HttpResponse::Created().json(question)
        }
        Err(e) => {
            println!("Error creating question: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to create question".to_string(),
            })
        }
    }
}

/// Update question
#[utoipa::path(
    put,
    path = "/admin/soal/{id}",
    params(
        ("id" = i32, Path, description = "Question ID")
    ),
    request_body = UpdateSoalRequest,
    responses(
        (status = 200, description = "Question updated successfully", body = Soal),
        (status = 400, description = "Invalid question data"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Question not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[put("/{id}")]
async fn update_question(
    path: web::Path<i32>,
    question_req: web::Json<UpdateSoalRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("PUT /admin/soal/{id}", &data.connections);

    let question_id = path.into_inner();

    // Validate question text only if provided
    if let Some(ref soal) = question_req.soal {
        if soal.trim().is_empty() {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Question text cannot be empty".to_string(),
            });
        }
    }

    // COALESCE keeps existing DB value when the field is not sent (None/null).
    let result = sqlx::query(
        r#"
        UPDATE dbquizapp.soal
        SET passage_id = ?,
            soal = COALESCE(?, soal),
            question_type = COALESCE(?, question_type),
            opt1 = COALESCE(?, opt1),
            opt2 = COALESCE(?, opt2),
            opt3 = COALESCE(?, opt3),
            opt4 = COALESCE(?, opt4),
            opt5 = COALESCE(?, opt5),
            correct_answer = COALESCE(?, correct_answer),
            solution = COALESCE(?, solution),
            sumberfile = COALESCE(?, sumberfile),
            modul = COALESCE(?, modul),
            pelajaran = COALESCE(?, pelajaran),
            tag = COALESCE(?, tag),
            track_id = COALESCE(?, track_id),
            category_id = COALESCE(?, category_id),
            subcategory_id = COALESCE(?, subcategory_id),
            topic_id = COALESCE(?, topic_id),
            difficulty_est = COALESCE(?, difficulty_est),
            difficulty_calc = COALESCE(?, difficulty_calc),
            bloom_level = COALESCE(?, bloom_level),
            format = COALESCE(?, format),
            source = COALESCE(?, source),
            status = COALESCE(?, status),
            updated_at = NOW()
        WHERE id = ?
        "#
    )
    .bind(question_req.passage_id)
    .bind(&question_req.soal)
    .bind(&question_req.question_type)
    .bind(&question_req.opt1)
    .bind(&question_req.opt2)
    .bind(&question_req.opt3)
    .bind(&question_req.opt4)
    .bind(&question_req.opt5)
    .bind(&question_req.correct_answer)
    .bind(&question_req.solution)
    .bind(&question_req.sumberfile)
    .bind(&question_req.modul)
    .bind(&question_req.pelajaran)
    .bind(&question_req.tag)
    .bind(&question_req.track_id)
    .bind(&question_req.category_id)
    .bind(&question_req.subcategory_id)
    .bind(&question_req.topic_id)
    .bind(&question_req.difficulty_est)
    .bind(&question_req.difficulty_calc)
    .bind(&question_req.bloom_level)
    .bind(&question_req.format)
    .bind(&question_req.source)
    .bind(&question_req.status)
    .bind(question_id)
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                let taxonomy_dao = TaxonomyDao::new(data.context.soal.pool.clone());
                // Manage question_tags if tag_ids provided
                if let Some(ref tag_ids) = question_req.tag_ids {
                    if let Err(e) = taxonomy_dao.set_question_tags(question_id, tag_ids).await {
                        eprintln!("Error setting question tags: {:?}", e);
                    }
                }
                // AFD-226: Manage question_topics if topic_ids provided
                if let Some(ref topic_ids) = question_req.topic_ids {
                    if let Err(e) = taxonomy_dao.set_question_topics(question_id, topic_ids).await {
                        eprintln!("Error setting question topics: {:?}", e);
                    }
                }
                // Content changed -- drop any cached /paket-soal-response for
                // packages containing this question, or students keep seeing
                // the pre-edit version until the 1h cache TTL expires.
                if let Some(redis_pool) = &data.redis_pool {
                    RedisService::invalidate_paket_soal_response_for_questions(
                        &data.context.soal.pool,
                        redis_pool,
                        &[question_id],
                    ).await;
                }
                // Fetch the updated question
                match data.context.soal.get_soal_by_id(&question_id.to_string()).await {
                    Ok(mut question) => {
                        // AFD-226: populate topic_ids from question_topics M2M
                        let topic_ids = taxonomy_dao.get_topics_for_question(question.id).await.unwrap_or_default();
                        question.topic_ids = if topic_ids.is_empty() { None } else { Some(topic_ids) };
                        HttpResponse::Ok().json(question)
                    }
                    Err(e) => {
                        println!("Error fetching updated question: {:?}", e);
                        HttpResponse::InternalServerError().json(ErrorResponse {
                            error: "Question updated but failed to fetch details".to_string(),
                        })
                    }
                }
            } else {
                HttpResponse::NotFound().json(ErrorResponse {
                    error: "Question not found".to_string(),
                })
            }
        }
        Err(e) => {
            println!("Error updating question: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to update question".to_string(),
            })
        }
    }
}

/// Delete question
#[utoipa::path(
    delete,
    path = "/admin/soal/{id}",
    params(
        ("id" = i32, Path, description = "Question ID")
    ),
    responses(
        (status = 204, description = "Question deleted successfully"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Question not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[delete("/{id}")]
async fn delete_question(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("DELETE /admin/soal/{id}", &data.connections);

    let question_id = path.into_inner();

    let result = sqlx::query(
        "DELETE FROM dbquizapp.soal WHERE id = ?"
    )
    .bind(question_id)
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                HttpResponse::NoContent().finish()
            } else {
                HttpResponse::NotFound().json(ErrorResponse {
                    error: "Question not found".to_string(),
                })
            }
        }
        Err(e) => {
            println!("Error deleting question: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to delete question".to_string(),
            })
        }
    }
}

/// Bulk import questions
#[utoipa::path(
    post,
    path = "/admin/soal/bulk-import",
    request_body = BulkImportRequest,
    responses(
        (status = 200, description = "Bulk import completed", body = BulkImportResponse),
        (status = 400, description = "Invalid bulk import data"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("/bulk-import")]
async fn bulk_import_questions(
    import_req: web::Json<BulkImportRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/soal/bulk-import", &data.connections);

    if import_req.questions.is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "No questions provided for import".to_string(),
        });
    }

    let mut success_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    for (index, question) in import_req.questions.iter().enumerate() {
        // Validate question data
        if question.soal.trim().is_empty() {
            failed_count += 1;
            errors.push(format!("Question {}: Question text cannot be empty", index + 1));
            continue;
        }

        let result = data.context.soal.create_soal(question).await;

        match result {
            Ok(_) => success_count += 1,
            Err(e) => {
                failed_count += 1;
                errors.push(format!("Question {}: {}", index + 1, e.to_string()));
            }
        }
    }

    HttpResponse::Ok().json(BulkImportResponse {
        success_count,
        failed_count,
        errors,
    })
}

/// List all questions (comprehensive endpoint similar to get_list_paket_soal_lengkap)
#[utoipa::path(
    get,
    path = "/admin/soal/questions",
    responses(
        (status = 200, description = "Questions retrieved successfully", body = [AdminSoal]),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/questions")]
async fn list_questions(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/soal/questions", &data.connections);
    
    let questions = data.context.soal.get_all_soal().await;

    match questions {
        Err(e) => {
            eprintln!("Error fetching questions: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch questions".to_string(),
            })
        },
        Ok(questions) => HttpResponse::Ok().json(questions),
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DropdownData {
    pub modules: Vec<String>,
    pub subjects: Vec<String>,
    pub tags: Vec<String>,
}

/// Get dropdown data for questions form
#[utoipa::path(
    get,
    path = "/admin/soal/dropdowns",
    responses(
        (status = 200, description = "Dropdown data retrieved successfully", body = DropdownData),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/dropdowns")]
async fn get_dropdowns(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/soal/dropdowns", &data.connections);

    // Get distinct modules
    let modules_query = r#"
        SELECT DISTINCT modul 
        FROM dbquizapp.soal 
        WHERE modul IS NOT NULL AND modul != '' 
        ORDER BY modul ASC
    "#;

    let modules = match sqlx::query_scalar::<_, String>(modules_query)
        .fetch_all(&*data.context.soal.pool)
        .await 
    {
        Ok(modules) => modules,
        Err(e) => {
            println!("Error fetching modules: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch modules".to_string(),
            });
        }
    };

    // Get distinct subjects
    let subjects_query = r#"
        SELECT DISTINCT pelajaran 
        FROM dbquizapp.soal 
        WHERE pelajaran IS NOT NULL AND pelajaran != '' 
        ORDER BY pelajaran ASC
    "#;

    let subjects = match sqlx::query_scalar::<_, String>(subjects_query)
        .fetch_all(&*data.context.soal.pool)
        .await 
    {
        Ok(subjects) => subjects,
        Err(e) => {
            println!("Error fetching subjects: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch subjects".to_string(),
            });
        }
    };

    // Get distinct tags
    let tags_query = r#"
        SELECT DISTINCT tag 
        FROM dbquizapp.soal 
        WHERE tag IS NOT NULL AND tag != '' 
        ORDER BY tag ASC
    "#;

    let tags = match sqlx::query_scalar::<_, String>(tags_query)
        .fetch_all(&*data.context.soal.pool)
        .await 
    {
        Ok(tags) => tags,
        Err(e) => {
            println!("Error fetching tags: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch tags".to_string(),
            });
        }
    };

    HttpResponse::Ok().json(DropdownData {
        modules,
        subjects,
        tags,
    })
}

/// Upload CSV file for preview and validation
#[utoipa::path(
    post,
    path = "/admin/soal/upload-csv-preview",
    request_body(
        content = String,
        description = "CSV file content",
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, description = "CSV preview generated successfully"),
        (status = 400, description = "Invalid CSV format or content"),
        (status = 403, description = "Admin access required"),
        (status = 413, description = "File too large"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("/upload-csv-preview")]
async fn upload_csv_preview(
    mut payload: Multipart,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/soal/upload-csv-preview", &data.connections);

    let csv_service = CsvImportService::default();
    
    // Process multipart form data
    while let Some(mut field) = payload.try_next().await.unwrap_or(None) {
        let content_disposition = field.content_disposition();
        
        if let Some(name) = content_disposition.get_name() {
            if name == "file" {
                // Read file content
                let mut file_content = Vec::new();
                while let Some(chunk) = field.try_next().await.unwrap_or(None) {
                    file_content.extend_from_slice(&chunk);
                }

                // Validate file
                if let Err(e) = csv_service.validate_file(&file_content) {
                    return HttpResponse::BadRequest().json(ErrorResponse {
                        error: e,
                    });
                }

                // Convert to string
                let csv_content = match std::str::from_utf8(&file_content) {
                    Ok(content) => content,
                    Err(_) => {
                        return HttpResponse::BadRequest().json(ErrorResponse {
                            error: "File must be valid UTF-8 text".to_string(),
                        });
                    }
                };

                // Parse and generate preview
                match csv_service.parse_csv_preview(csv_content).await {
                    Ok(preview) => {
                        return HttpResponse::Ok().json(preview);
                    }
                    Err(e) => {
                        return HttpResponse::BadRequest().json(ErrorResponse {
                            error: e,
                        });
                    }
                }
            }
        }
    }

    HttpResponse::BadRequest().json(ErrorResponse {
        error: "No file provided in multipart form data".to_string(),
    })
}

/// Upload and import CSV file
#[utoipa::path(
    post,
    path = "/admin/soal/upload-csv-import",
    request_body(
        content = String,
        description = "CSV file content with import options",
        content_type = "multipart/form-data"
    ),
    responses(
        (status = 200, description = "CSV imported successfully"),
        (status = 400, description = "Invalid CSV format or import failed"),
        (status = 403, description = "Admin access required"),
        (status = 413, description = "File too large"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("/upload-csv-import")]
async fn upload_csv_import(
    mut payload: Multipart,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/soal/upload-csv-import", &data.connections);

    let csv_service = CsvImportService::default();
    let mut csv_content: Option<String> = None;
    let mut import_options = CsvImportRequest {
        skip_invalid_rows: true,
        max_errors: Some(100),
        validate_only: false,
    };
    
    // Process multipart form data
    while let Some(mut field) = payload.try_next().await.unwrap_or(None) {
        let content_disposition = field.content_disposition();
        
        if let Some(name) = content_disposition.get_name() {
            match name {
                "file" => {
                    // Read file content
                    let mut file_content = Vec::new();
                    while let Some(chunk) = field.try_next().await.unwrap_or(None) {
                        file_content.extend_from_slice(&chunk);
                    }

                    // Validate file
                    if let Err(e) = csv_service.validate_file(&file_content) {
                        return HttpResponse::BadRequest().json(ErrorResponse {
                            error: e,
                        });
                    }

                    // Convert to string
                    match std::str::from_utf8(&file_content) {
                        Ok(content) => csv_content = Some(content.to_string()),
                        Err(_) => {
                            return HttpResponse::BadRequest().json(ErrorResponse {
                                error: "File must be valid UTF-8 text".to_string(),
                            });
                        }
                    }
                }
                "options" => {
                    // Read import options JSON
                    let mut options_content = Vec::new();
                    while let Some(chunk) = field.try_next().await.unwrap_or(None) {
                        options_content.extend_from_slice(&chunk);
                    }
                    
                    if let Ok(options_str) = std::str::from_utf8(&options_content) {
                        if let Ok(parsed_options) = serde_json::from_str::<CsvImportRequest>(options_str) {
                            import_options = parsed_options;
                        }
                    }
                }
                _ => {} // Ignore other fields
            }
        }
    }

    let csv_content = match csv_content {
        Some(content) => content,
        None => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "No CSV file provided".to_string(),
            });
        }
    };

    // Parse CSV and get preview
    let preview = match csv_service.parse_csv_preview(&csv_content).await {
        Ok(preview) => preview,
        Err(e) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: e,
            });
        }
    };

    // If validate_only is true, return preview
    if import_options.validate_only {
        return HttpResponse::Ok().json(preview);
    }

    // Extract questions to import
    let questions_to_import = if import_options.skip_invalid_rows {
        csv_service.extract_questions_from_preview(&preview)
    } else {
        if !preview.invalid_questions.is_empty() {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("CSV contains {} invalid rows. Fix errors or set skip_invalid_rows=true", preview.invalid_questions.len()),
            });
        }
        csv_service.extract_questions_from_preview(&preview)
    };

    if questions_to_import.is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "No valid questions found in CSV file".to_string(),
        });
    }

    // Import questions using existing bulk import logic
    let mut success_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    for (index, question) in questions_to_import.iter().enumerate() {
        // Validate question data
        if question.soal.trim().is_empty() {
            failed_count += 1;
            errors.push(format!("Question {}: Question text cannot be empty", index + 1));
            continue;
        }

        let result = data.context.soal.create_soal(question).await;
        match result {
            Ok(_) => success_count += 1,
            Err(e) => {
                failed_count += 1;
                errors.push(format!("Question {}: {}", index + 1, e));
                
                // Check max errors limit
                if let Some(max_errors) = import_options.max_errors {
                    if errors.len() >= max_errors {
                        break;
                    }
                }
            }
        }
    }

    let response = CsvImportResponse {
        success_count,
        failed_count,
        skipped_count: preview.invalid_questions.len() as i32,
        total_processed: (success_count + failed_count) as i32,
        errors: errors.into_iter().map(|msg| crate::model::soal::CsvImportError {
            row_number: 0, // Would need more detailed tracking
            field: "general".to_string(),
            error_type: "import_error".to_string(),
            message: msg,
            suggested_fix: None,
            raw_value: None,
        }).collect(),
        warnings: vec![], // Could add warnings from preview
        import_id: None,   // Could generate unique import ID
        estimated_time_seconds: 0, // Already completed
    };

    HttpResponse::Ok().json(response)
}

/// Download CSV template for bulk import
#[utoipa::path(
    get,
    path = "/admin/soal/download-csv-template",
    responses(
        (status = 200, description = "CSV template downloaded successfully", content_type = "text/csv"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/download-csv-template")]
async fn download_csv_template(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/soal/download-csv-template", &data.connections);

    let template_content = r#"question_text,option_1,option_2,option_3,option_4,option_5,correct_answer,solution,module,subject,tag,source_file
"What is 2+2?","2","3","4","5","","3","2+2 equals 4 because it's basic arithmetic","Basic Math","Mathematics","arithmetic","math-basics.pdf"
"Which planet is closest to the Sun?","Venus","Mercury","Earth","Mars","","2","Mercury is the closest planet to the Sun","Solar System","Science","astronomy,planets","science-101.pdf"
"True or False: Paris is the capital of France","True","False","","","","1","Paris is indeed the capital and largest city of France","Geography","Geography","capitals,europe","geography.pdf"
"Select the largest ocean","Atlantic","Pacific","Indian","Arctic","","2","The Pacific Ocean is the largest ocean on Earth","Oceans","Geography","ocean,geography","earth-science.pdf"
"What is the result of 5 × 6?","25","30","35","40","","2","5 × 6 = 30. Multiplication of 5 and 6.","Multiplication","Mathematics","multiplication,basic","math-fundamentals.pdf""#;

    HttpResponse::Ok()
        .content_type("text/csv")
        .insert_header(("Content-Disposition", "attachment; filename=questions_template.csv"))
        .body(template_content)
}

/// Upload an image or audio file for use in question fields (e.g. Listening
/// Comprehension audio, shared across many soal via a passage).
/// Accepts: image/jpeg, image/png, image/gif, image/webp (max 5MB);
/// audio/mpeg (max 100MB).
/// Returns: { "url": "/static/soal-images/{uuid}.{ext}" }
#[post("/upload-image")]
async fn upload_image(
    mut payload: Multipart,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("POST /admin/soal/upload-image", &data.connections);

    let upload_dir = data.config.get_upload_dir();
    let image_dir = format!("{}/soal-images", upload_dir);

    const IMAGE_MAX_BYTES: usize = 5 * 1024 * 1024;
    const AUDIO_MAX_BYTES: usize = 100 * 1024 * 1024;

    let mut file_bytes: Vec<u8> = Vec::new();
    let mut file_ext = String::new();

    while let Some(mut field) = payload.try_next().await.unwrap_or(None) {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name().unwrap_or("").to_string();

        if field_name != "file" {
            continue;
        }

        // Validate MIME type
        let mime = field.content_type().cloned();
        // NOTE: MIME type comes from the multipart Content-Type header and can be
        // spoofed by the client. This is acceptable because the endpoint is
        // admin-only (protected by AdminMiddleware).
        let (ext, max_bytes) = match mime.as_ref().map(|m| m.essence_str()) {
            Some("image/jpeg") => ("jpg", IMAGE_MAX_BYTES),
            Some("image/png") => ("png", IMAGE_MAX_BYTES),
            Some("image/gif") => ("gif", IMAGE_MAX_BYTES),
            Some("image/webp") => ("webp", IMAGE_MAX_BYTES),
            Some("audio/mpeg") | Some("audio/mp3") => ("mp3", AUDIO_MAX_BYTES),
            _ => {
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: "Tipe file tidak didukung. Gunakan JPEG, PNG, GIF, WebP, atau MP3.".to_string(),
                });
            }
        };
        file_ext = ext.to_string();

        while let Some(chunk) = field.try_next().await.unwrap_or(None) {
            file_bytes.extend_from_slice(&chunk);
            if file_bytes.len() > max_bytes {
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: format!("File terlalu besar. Maksimal {}MB.", max_bytes / (1024 * 1024)),
                });
            }
        }
        break; // only process first 'file' field
    }

    if file_bytes.is_empty() || file_ext.is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "File tidak ditemukan dalam request.".to_string(),
        });
    }

    // Generate unique filename and save
    let filename = format!("{}.{}", Uuid::new_v4(), file_ext);
    let file_path = format!("{}/{}", image_dir, filename);

    // Defensive: ensure directory exists even if it was deleted at runtime
    if let Err(e) = tokio::fs::create_dir_all(&image_dir).await {
        eprintln!("Failed to create image dir '{}': {:?}", image_dir, e);
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Gagal menyiapkan direktori upload.".to_string(),
        });
    }

    if let Err(e) = tokio::fs::write(&file_path, &file_bytes).await {
        eprintln!("Failed to write image '{}': {:?}", file_path, e);
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Gagal menyimpan gambar.".to_string(),
        });
    }

    HttpResponse::Ok().json(serde_json::json!({
        "url": format!("/static/soal-images/{}", filename)
    }))
}

// ── AFD-244 handlers ──

/// Get coverage statistics: how many soal appear in 0, 1, or multiple packages
#[get("/coverage-stats")]
async fn coverage_stats(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/soal/coverage-stats", &data.connections);

    let row = sqlx::query(
        r#"
        SELECT
            COUNT(*)                         AS total_soal,
            SUM(pkg_count = 0)               AS not_in_any_package,
            SUM(pkg_count = 1)               AS in_exactly_one_package,
            SUM(pkg_count >= 2)              AS in_multiple_packages,
            MAX(pkg_count)                   AS max_package_count,
            ROUND(AVG(pkg_count), 4)         AS avg_package_count
        FROM (
            SELECT s.id, COUNT(psi.paket_soal_id) AS pkg_count
            FROM soal s
            LEFT JOIN paket_soal_items psi ON psi.soal_id = s.id
            GROUP BY s.id
        ) sub
        "#,
    )
    .fetch_one(&*data.context.soal.pool)
    .await;

    match row {
        Ok(r) => {
            use sqlx::Row;
            let stats = SoalCoverageStats {
                total_soal: r.try_get::<i64, _>("total_soal").unwrap_or(0),
                not_in_any_package: r.try_get::<i64, _>("not_in_any_package").unwrap_or(0),
                in_exactly_one_package: r.try_get::<i64, _>("in_exactly_one_package").unwrap_or(0),
                in_multiple_packages: r.try_get::<i64, _>("in_multiple_packages").unwrap_or(0),
                max_package_count: r.try_get::<i64, _>("max_package_count").unwrap_or(0),
                avg_package_count: r.try_get::<f64, _>("avg_package_count").unwrap_or(0.0),
            };
            HttpResponse::Ok().json(stats)
        }
        Err(e) => {
            eprintln!("coverage_stats error: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to compute coverage stats".to_string(),
            })
        }
    }
}

/// Get all packages that contain a given soal
#[get("/{id}/packages")]
async fn get_soal_packages(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    let soal_id = path.into_inner();
    log_request("GET /admin/soal/{id}/packages", &data.connections);

    let packages = sqlx::query_as::<_, SoalPackageItem>(
        r#"
        SELECT ps.id AS paket_soal_id, ps.nama_paket_soal, ps.kode_paket, ps.is_premium
        FROM paket_soal_items psi
        JOIN paket_soal ps ON ps.id = psi.paket_soal_id
        WHERE psi.soal_id = ?
        ORDER BY ps.id DESC
        "#,
    )
    .bind(soal_id)
    .fetch_all(&*data.context.soal.pool)
    .await;

    match packages {
        Ok(pkgs) => {
            let total = pkgs.len();
            HttpResponse::Ok().json(SoalPackagesResponse {
                soal_id,
                packages: pkgs,
                total,
            })
        }
        Err(e) => {
            eprintln!("get_soal_packages error: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch packages for soal".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_fulltext_boolean_query;

    #[test]
    fn test_fulltext_query_single_token_prefix() {
        assert_eq!(build_fulltext_boolean_query("panca"), Some("+panca*".to_string()));
    }

    #[test]
    fn test_fulltext_query_multi_token_requires_all() {
        assert_eq!(
            build_fulltext_boolean_query("budi pekerti"),
            Some("+budi* +pekerti*".to_string())
        );
    }

    #[test]
    fn test_fulltext_query_strips_boolean_operators() {
        // User input shouldn't be able to inject FULLTEXT BOOLEAN MODE
        // syntax (+-><()~*"@) -- only alphanumerics survive per token.
        assert_eq!(build_fulltext_boolean_query("+panca* -foo\""), Some("+panca* +foo*".to_string()));
    }

    #[test]
    fn test_fulltext_query_empty_input() {
        assert_eq!(build_fulltext_boolean_query(""), None);
        assert_eq!(build_fulltext_boolean_query("   "), None);
        assert_eq!(build_fulltext_boolean_query("***"), None);
    }

    #[test]
    fn test_fulltext_query_splits_hyphenated_words() {
        // MySQL's FULLTEXT parser indexes "Unsur-unsur" as two separate
        // words ("unsur", "unsur"), not one glued token -- confirmed
        // live, a real soal starting with this exact phrase returned zero
        // results before this fix.
        assert_eq!(
            build_fulltext_boolean_query("Unsur-unsur penting"),
            Some("+Unsur* +unsur* +penting*".to_string())
        );
    }

    #[test]
    fn test_image_ext_mapping() {
        let cases = vec![
            ("image/jpeg", "jpg"),
            ("image/png", "png"),
            ("image/gif", "gif"),
            ("image/webp", "webp"),
        ];
        for (mime, expected_ext) in cases {
            let mapped = match mime {
                "image/jpeg" => "jpg",
                "image/png" => "png",
                "image/gif" => "gif",
                "image/webp" => "webp",
                _ => "",
            };
            assert_eq!(mapped, expected_ext, "MIME {} should map to ext {}", mime, expected_ext);
        }
    }

    #[test]
    fn test_filename_has_uuid_format() {
        let filename = format!("{}.jpg", uuid::Uuid::new_v4());
        // UUID v4 is 36 chars, dot is 1, ext is 3 → total 40
        assert_eq!(filename.len(), 40);
        assert!(filename.ends_with(".jpg"));
    }
}