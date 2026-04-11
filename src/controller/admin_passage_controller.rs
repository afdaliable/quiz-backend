use crate::middleware::admin_middleware::AdminMiddleware;
use crate::model::passage::{CreatePassageRequest, PaginatedPassagesResponse, Passage, PassageDetail};
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest, get, post, put, delete};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/passages")
            .wrap(AdminMiddleware::new())
            .service(list_passages)
            .service(create_passage)
            .service(get_passage_by_id)
            .service(update_passage)
            .service(delete_passage)
    );
}

/// List all passages (paginated)
#[utoipa::path(
    get,
    path = "/admin/passages",
    params(
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("limit" = Option<u32>, Query, description = "Items per page (default: 20)")
    ),
    responses(
        (status = 200, description = "Passages retrieved successfully", body = PaginatedPassagesResponse),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = []))
)]
#[get("")]
async fn list_passages(
    query: web::Query<PaginationQuery>,
    data: web::Data<AppState<'_>>,
    _http_req: HttpRequest,
) -> impl Responder {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);

    match data.context.passages.list_passages(page, limit).await {
        Ok((passages, total)) => {
            let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;
            HttpResponse::Ok().json(PaginatedPassagesResponse {
                passages,
                total,
                page,
                limit,
                total_pages,
            })
        }
        Err(e) => {
            eprintln!("Error listing passages: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch passages".to_string(),
            })
        }
    }
}

/// Create a new passage
#[utoipa::path(
    post,
    path = "/admin/passages",
    request_body = CreatePassageRequest,
    responses(
        (status = 201, description = "Passage created successfully", body = Passage),
        (status = 400, description = "Invalid data"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = []))
)]
#[post("")]
async fn create_passage(
    req: web::Json<CreatePassageRequest>,
    data: web::Data<AppState<'_>>,
    _http_req: HttpRequest,
) -> impl Responder {
    if req.content.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Passage content cannot be empty".to_string(),
        });
    }

    match data.context.passages.create_passage(&req).await {
        Ok(passage) => HttpResponse::Created().json(passage),
        Err(e) => {
            eprintln!("Error creating passage: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to create passage".to_string(),
            })
        }
    }
}

/// Get passage detail by ID (includes list of soal that use it)
#[utoipa::path(
    get,
    path = "/admin/passages/{id}",
    params(("id" = i32, Path, description = "Passage ID")),
    responses(
        (status = 200, description = "Passage found", body = PassageDetail),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Passage not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = []))
)]
#[get("/{id}")]
async fn get_passage_by_id(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    _http_req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();

    match data.context.passages.get_passage_detail(id).await {
        Ok(detail) => HttpResponse::Ok().json(detail),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(ErrorResponse {
            error: "Passage not found".to_string(),
        }),
        Err(e) => {
            eprintln!("Error fetching passage {}: {:?}", id, e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch passage".to_string(),
            })
        }
    }
}

/// Update a passage
#[utoipa::path(
    put,
    path = "/admin/passages/{id}",
    params(("id" = i32, Path, description = "Passage ID")),
    request_body = CreatePassageRequest,
    responses(
        (status = 200, description = "Passage updated successfully", body = Passage),
        (status = 400, description = "Invalid data"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Passage not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = []))
)]
#[put("/{id}")]
async fn update_passage(
    path: web::Path<i32>,
    req: web::Json<CreatePassageRequest>,
    data: web::Data<AppState<'_>>,
    _http_req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();

    if req.content.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Passage content cannot be empty".to_string(),
        });
    }

    match data.context.passages.update_passage(id, &req).await {
        Ok(passage) => HttpResponse::Ok().json(passage),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(ErrorResponse {
            error: "Passage not found".to_string(),
        }),
        Err(e) => {
            eprintln!("Error updating passage {}: {:?}", id, e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to update passage".to_string(),
            })
        }
    }
}

/// Delete a passage (soal.passage_id set to NULL automatically via FK ON DELETE SET NULL)
#[utoipa::path(
    delete,
    path = "/admin/passages/{id}",
    params(("id" = i32, Path, description = "Passage ID")),
    responses(
        (status = 204, description = "Passage deleted successfully"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Passage not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = []))
)]
#[delete("/{id}")]
async fn delete_passage(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    _http_req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();

    match data.context.passages.delete_passage(id).await {
        Ok(rows) if rows > 0 => HttpResponse::NoContent().finish(),
        Ok(_) => HttpResponse::NotFound().json(ErrorResponse {
            error: "Passage not found".to_string(),
        }),
        Err(e) => {
            eprintln!("Error deleting passage {}: {:?}", id, e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to delete passage".to_string(),
            })
        }
    }
}
