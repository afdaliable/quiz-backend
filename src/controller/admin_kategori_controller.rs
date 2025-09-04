use crate::controller::log_request;
use crate::model::kategori_soal::{KategoriSoal, AdminKategoriSoal, AdminKategoriRequest};
use crate::middleware::admin_middleware::AdminMiddleware;
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest, get, post, put, delete};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    println!("DEBUG: admin_kategori_controller::init called - registering routes");
    cfg.service(
        web::scope("/admin/categories")
            .wrap(AdminMiddleware::new())
            .service(get_all_categories)
            .service(create_category)
            .service(get_category_by_id)
            .service(update_category)
            .service(delete_category)
    );
}

/// Get all categories with additional admin info
#[utoipa::path(
    get,
    path = "/admin/categories",
    responses(
        (status = 200, description = "Categories retrieved successfully", body = Vec<AdminKategoriSoal>),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("")]
async fn get_all_categories(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/categories", &data.connections);

    let query = r#"
        SELECT 
            k.id,
            k.nama_kategori,
            COALESCE(package_count.count, 0) as quiz_packages_count,
            COALESCE(question_count.count, 0) as questions_count
        FROM dbquizapp.kategori_soal k
        LEFT JOIN (
            SELECT kategori_id, COUNT(*) as count 
            FROM dbquizapp.paket_soal 
            GROUP BY kategori_id
        ) package_count ON k.id = package_count.kategori_id
        LEFT JOIN (
            SELECT ps.kategori_id, COUNT(psi.soal_id) as count
            FROM dbquizapp.paket_soal ps
            JOIN dbquizapp.paket_soal_items psi ON ps.id = psi.paket_soal_id
            GROUP BY ps.kategori_id
        ) question_count ON k.id = question_count.kategori_id
        ORDER BY k.nama_kategori ASC
    "#;

    let result = sqlx::query_as::<_, AdminKategoriSoal>(query)
        .fetch_all(&*data.context.kategori.pool)
        .await;

    match result {
        Ok(categories) => HttpResponse::Ok().json(categories),
        Err(e) => {
            println!("Error fetching categories: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch categories".to_string(),
            })
        }
    }
}

/// Create a new category
#[utoipa::path(
    post,
    path = "/admin/categories",
    request_body = AdminKategoriRequest,
    responses(
        (status = 201, description = "Category created successfully", body = KategoriSoal),
        (status = 400, description = "Invalid category data"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("")]
async fn create_category(
    category_req: web::Json<AdminKategoriRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/categories", &data.connections);

    // Validate category name
    if category_req.nama_kategori.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Category name cannot be empty".to_string(),
        });
    }

    // Check if category already exists
    let existing_check = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dbquizapp.kategori_soal WHERE nama_kategori = ?"
    )
    .bind(&category_req.nama_kategori)
    .fetch_one(&*data.context.kategori.pool)
    .await;

    match existing_check {
        Ok(count) if count > 0 => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Category with this name already exists".to_string(),
            });
        }
        Err(e) => {
            println!("Error checking existing category: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to check existing categories".to_string(),
            });
        }
        _ => {}
    }

    let result = sqlx::query(
        "INSERT INTO dbquizapp.kategori_soal (nama_kategori) VALUES (?)"
    )
    .bind(&category_req.nama_kategori)
    .execute(&*data.context.kategori.pool)
    .await;

    match result {
        Ok(result) => {
            let category_id = result.last_insert_id() as i32;
            
            // Fetch the created category
            match get_category_by_id_internal(&data, category_id).await {
                Ok(category) => HttpResponse::Created().json(category),
                Err(e) => {
                    println!("Error fetching created category: {:?}", e);
                    HttpResponse::InternalServerError().json(ErrorResponse {
                        error: "Category created but failed to fetch details".to_string(),
                    })
                }
            }
        }
        Err(e) => {
            println!("Error creating category: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to create category".to_string(),
            })
        }
    }
}

/// Get category by ID
#[utoipa::path(
    get,
    path = "/admin/categories/{id}",
    params(
        ("id" = i32, Path, description = "Category ID")
    ),
    responses(
        (status = 200, description = "Category found", body = KategoriSoal),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Category not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/{id}")]
async fn get_category_by_id(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/categories/{id}", &data.connections);

    let category_id = path.into_inner();
    
    match get_category_by_id_internal(&data, category_id).await {
        Ok(category) => HttpResponse::Ok().json(category),
        Err(e) => {
            println!("Error fetching category: {:?}", e);
            HttpResponse::NotFound().json(ErrorResponse {
                error: "Category not found".to_string(),
            })
        }
    }
}

/// Update category
#[utoipa::path(
    put,
    path = "/admin/categories/{id}",
    params(
        ("id" = i32, Path, description = "Category ID")
    ),
    request_body = AdminKategoriRequest,
    responses(
        (status = 200, description = "Category updated successfully", body = KategoriSoal),
        (status = 400, description = "Invalid category data"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Category not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[put("/{id}")]
async fn update_category(
    path: web::Path<i32>,
    category_req: web::Json<AdminKategoriRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("PUT /admin/categories/{id}", &data.connections);

    let category_id = path.into_inner();

    // Validate category name
    if category_req.nama_kategori.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Category name cannot be empty".to_string(),
        });
    }

    // Check if another category with the same name exists (excluding current one)
    let existing_check = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dbquizapp.kategori_soal WHERE nama_kategori = ? AND id != ?"
    )
    .bind(&category_req.nama_kategori)
    .bind(category_id)
    .fetch_one(&*data.context.kategori.pool)
    .await;

    match existing_check {
        Ok(count) if count > 0 => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: "Category with this name already exists".to_string(),
            });
        }
        Err(e) => {
            println!("Error checking existing category: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to check existing categories".to_string(),
            });
        }
        _ => {}
    }

    let result = sqlx::query(
        "UPDATE dbquizapp.kategori_soal SET nama_kategori = ?, updated_at = NOW() WHERE id = ?"
    )
    .bind(&category_req.nama_kategori)
    .bind(category_id)
    .execute(&*data.context.kategori.pool)
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                match get_category_by_id_internal(&data, category_id).await {
                    Ok(category) => HttpResponse::Ok().json(category),
                    Err(e) => {
                        println!("Error fetching updated category: {:?}", e);
                        HttpResponse::InternalServerError().json(ErrorResponse {
                            error: "Category updated but failed to fetch details".to_string(),
                        })
                    }
                }
            } else {
                HttpResponse::NotFound().json(ErrorResponse {
                    error: "Category not found".to_string(),
                })
            }
        }
        Err(e) => {
            println!("Error updating category: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to update category".to_string(),
            })
        }
    }
}

/// Delete category
#[utoipa::path(
    delete,
    path = "/admin/categories/{id}",
    params(
        ("id" = i32, Path, description = "Category ID")
    ),
    responses(
        (status = 204, description = "Category deleted successfully"),
        (status = 400, description = "Category has associated quiz packages"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Category not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[delete("/{id}")]
async fn delete_category(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("DELETE /admin/categories/{id}", &data.connections);

    let category_id = path.into_inner();

    // Check if category has associated quiz packages
    let package_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dbquizapp.paket_soal WHERE kategori_id = ?"
    )
    .bind(category_id)
    .fetch_one(&*data.context.kategori.pool)
    .await;

    match package_count {
        Ok(count) if count > 0 => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("Cannot delete category. It has {} associated quiz packages.", count),
            });
        }
        Err(e) => {
            println!("Error checking category dependencies: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to check category dependencies".to_string(),
            });
        }
        _ => {}
    }

    let result = sqlx::query(
        "DELETE FROM dbquizapp.kategori_soal WHERE id = ?"
    )
    .bind(category_id)
    .execute(&*data.context.kategori.pool)
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                HttpResponse::NoContent().finish()
            } else {
                HttpResponse::NotFound().json(ErrorResponse {
                    error: "Category not found".to_string(),
                })
            }
        }
        Err(e) => {
            println!("Error deleting category: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to delete category".to_string(),
            })
        }
    }
}

// Helper function to get category by ID
async fn get_category_by_id_internal(
    data: &web::Data<AppState<'_>>,
    category_id: i32,
) -> Result<KategoriSoal, sqlx::Error> {
    sqlx::query_as::<_, KategoriSoal>(
        "SELECT id, nama_kategori FROM dbquizapp.kategori_soal WHERE id = ?"
    )
    .bind(category_id)
    .fetch_one(&*data.context.kategori.pool)
    .await
}