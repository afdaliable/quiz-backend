use crate::controller::log_request;
use crate::middleware::admin_middleware::AdminMiddleware;
use crate::model::paket_soal::{PaketSoal, AdminPaketSoal, AdminPaketSoalRequest, CreatePaketSoalRequest, UpdatePaketSoalRequest, PackageSearchRequest, PaginatedPackagesResponse, AddQuestionsRequest, PackageOperationResponse};
use crate::model::soal::AdminSoal;
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest, get, post, put, delete};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    println!("DEBUG: admin_packages_controller::init called - registering routes");
    cfg.service(
        web::scope("/admin/packages")
            .wrap(AdminMiddleware::new())
            .service(get_all_packages)
            .service(create_package)
            .service(get_package_by_id)
            .service(update_package)
            .service(delete_package)
            .service(get_package_questions)
            .service(add_questions_to_package)
            .service(remove_question_from_package)
    );
}

/// Get all packages with pagination and filtering
#[utoipa::path(
    get,
    path = "/admin/packages",
    params(
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("limit" = Option<u32>, Query, description = "Items per page (default: 20)"),
        ("search" = Option<String>, Query, description = "Search in package name"),
        ("kategori_id" = Option<i32>, Query, description = "Filter by category ID"),
        ("is_premium" = Option<bool>, Query, description = "Filter by premium status")
    ),
    responses(
        (status = 200, description = "Packages retrieved successfully", body = PaginatedPackagesResponse),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("")]
async fn get_all_packages(
    query: web::Query<PackageSearchRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    println!("DEBUG: get_all_packages function called!");
    log_request("/admin/packages", &data.connections);

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;

    // Build dynamic query
    let mut where_conditions = vec!["1=1"];
    let mut bind_values: Vec<String> = vec![];

    if let Some(ref search) = query.search {
        if !search.is_empty() {
            where_conditions.push("p.nama_paket_soal LIKE ?");
            bind_values.push(format!("%{}%", search));
        }
    }

    if let Some(kategori_id) = query.kategori_id {
        where_conditions.push("p.kategori_id = ?");
        bind_values.push(kategori_id.to_string());
    }

    if let Some(is_premium) = query.is_premium {
        where_conditions.push("p.is_premium = ?");
        bind_values.push(is_premium.to_string());
    }

    let where_clause = where_conditions.join(" AND ");

    // Count total packages
    let count_query = format!(
        "SELECT COUNT(*) FROM dbquizapp.paket_soal p WHERE {}",
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
            println!("Error counting packages: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to count packages".to_string(),
            });
        }
    };

    // Get packages with category name and question count
    let packages_query = format!(
        r#"
        SELECT 
            p.id, p.nama_paket_soal, p.kategori_id, p.is_premium,
            k.nama_kategori as kategori_name,
            COALESCE(question_count.count, 0) as questions_count,
            CASE 
                WHEN p.status = 1 THEN 'published'
                WHEN p.status = 0 THEN 'draft'
                ELSE 'draft'
            END as status,
            NOW() as created_at,
            NOW() as updated_at
        FROM dbquizapp.paket_soal p
        LEFT JOIN dbquizapp.kategori_soal k ON p.kategori_id = k.id
        LEFT JOIN (
            SELECT paket_soal_id, COUNT(*) as count 
            FROM dbquizapp.paket_soal_items 
            GROUP BY paket_soal_id
        ) question_count ON p.id = question_count.paket_soal_id
        WHERE {}
        ORDER BY p.id DESC
        LIMIT ? OFFSET ?
        "#,
        where_clause
    );

    let mut packages_query_builder = sqlx::query_as::<_, AdminPaketSoal>(&packages_query);
    for value in &bind_values {
        packages_query_builder = packages_query_builder.bind(value);
    }

    let packages = match packages_query_builder
        .bind(limit)
        .bind(offset)
        .fetch_all(&*data.context.soal.pool)
        .await 
    {
        Ok(packages) => packages,
        Err(e) => {
            println!("Error fetching packages: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch packages".to_string(),
            });
        }
    };

    let total_pages = (total as f64 / limit as f64).ceil() as u32;

    HttpResponse::Ok().json(PaginatedPackagesResponse {
        packages,
        total,
        page,
        limit,
        total_pages,
    })
}

/// Create a new package
#[utoipa::path(
    post,
    path = "/admin/packages",
    request_body = CreatePaketSoalRequest,
    responses(
        (status = 201, description = "Package created successfully", body = PaketSoal),
        (status = 400, description = "Invalid package data"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("")]
async fn create_package(
    package_req: web::Json<CreatePaketSoalRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/packages", &data.connections);

    // Validate package data
    if package_req.nama_paket_soal.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Package name cannot be empty".to_string(),
        });
    }

    // Check if category exists (only if kategori_id is provided)
    if let Some(kategori_id) = package_req.kategori_id {
        let category_exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM dbquizapp.kategori_soal WHERE id = ?"
        )
        .bind(kategori_id)
        .fetch_one(&*data.context.soal.pool)
        .await;

        match category_exists {
            Ok(0) => {
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: "Category not found".to_string(),
                });
            }
            Err(e) => {
                println!("Error checking category: {:?}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Failed to check category".to_string(),
                });
            }
            _ => {}
        }
    }

    let result = sqlx::query(
        r#"
        INSERT INTO dbquizapp.paket_soal (nama_paket_soal, kategori_id, is_premium, created_at, updated_at)
        VALUES (?, ?, ?, NOW(), NOW())
        "#
    )
    .bind(&package_req.nama_paket_soal)
    .bind(package_req.kategori_id)
    .bind(package_req.is_premium)
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(result) => {
            let package_id = result.last_insert_id() as i32;
            
            // Fetch the created package
            match get_package_by_id_internal(&data, package_id).await {
                Ok(package) => HttpResponse::Created().json(package),
                Err(e) => {
                    println!("Error fetching created package: {:?}", e);
                    HttpResponse::InternalServerError().json(ErrorResponse {
                        error: "Package created but failed to fetch details".to_string(),
                    })
                }
            }
        }
        Err(e) => {
            println!("Error creating package: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to create package".to_string(),
            })
        }
    }
}

/// Get package by ID
#[utoipa::path(
    get,
    path = "/admin/packages/{id}",
    params(
        ("id" = i32, Path, description = "Package ID")
    ),
    responses(
        (status = 200, description = "Package found", body = AdminPaketSoal),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Package not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/{id}")]
async fn get_package_by_id(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/packages/{id}", &data.connections);

    let package_id = path.into_inner();
    
    match get_package_by_id_internal(&data, package_id).await {
        Ok(package) => HttpResponse::Ok().json(package),
        Err(e) => {
            println!("Error fetching package: {:?}", e);
            HttpResponse::NotFound().json(ErrorResponse {
                error: "Package not found".to_string(),
            })
        }
    }
}

/// Update package
#[utoipa::path(
    put,
    path = "/admin/packages/{id}",
    params(
        ("id" = i32, Path, description = "Package ID")
    ),
    request_body = AdminPaketSoalRequest,
    responses(
        (status = 200, description = "Package updated successfully", body = PaketSoal),
        (status = 400, description = "Invalid package data"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Package not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[put("/{id}")]
async fn update_package(
    path: web::Path<i32>,
    package_req: web::Json<AdminPaketSoalRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("PUT /admin/packages/{id}", &data.connections);

    let package_id = path.into_inner();

    // Validate package data
    if package_req.nama_paket_soal.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Package name cannot be empty".to_string(),
        });
    }

    let result = sqlx::query(
        r#"
        UPDATE dbquizapp.paket_soal 
        SET nama_paket_soal = ?, kategori_id = ?, is_premium = ?, updated_at = NOW()
        WHERE id = ?
        "#
    )
    .bind(&package_req.nama_paket_soal)
    .bind(package_req.kategori_id)
    .bind(package_req.is_premium)
    .bind(package_id)
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                match get_package_by_id_internal(&data, package_id).await {
                    Ok(package) => HttpResponse::Ok().json(package),
                    Err(e) => {
                        println!("Error fetching updated package: {:?}", e);
                        HttpResponse::InternalServerError().json(ErrorResponse {
                            error: "Package updated but failed to fetch details".to_string(),
                        })
                    }
                }
            } else {
                HttpResponse::NotFound().json(ErrorResponse {
                    error: "Package not found".to_string(),
                })
            }
        }
        Err(e) => {
            println!("Error updating package: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to update package".to_string(),
            })
        }
    }
}

/// Delete package
#[utoipa::path(
    delete,
    path = "/admin/packages/{id}",
    params(
        ("id" = i32, Path, description = "Package ID")
    ),
    responses(
        (status = 204, description = "Package deleted successfully"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Package not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[delete("/{id}")]
async fn delete_package(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("DELETE /admin/packages/{id}", &data.connections);

    let package_id = path.into_inner();

    // Start transaction to delete package and its items
    let mut tx = match data.context.soal.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            println!("Error starting transaction: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to start transaction".to_string(),
            });
        }
    };

    // Delete package items first
    if let Err(e) = sqlx::query("DELETE FROM dbquizapp.paket_soal_items WHERE paket_soal_id = ?")
        .bind(package_id)
        .execute(&mut *tx)
        .await
    {
        let _ = tx.rollback().await;
        println!("Error deleting package items: {:?}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Failed to delete package items".to_string(),
        });
    }

    // Delete the package
    let result = sqlx::query("DELETE FROM dbquizapp.paket_soal WHERE id = ?")
        .bind(package_id)
        .execute(&mut *tx)
        .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                if let Err(e) = tx.commit().await {
                    println!("Error committing transaction: {:?}", e);
                    return HttpResponse::InternalServerError().json(ErrorResponse {
                        error: "Failed to commit transaction".to_string(),
                    });
                }
                HttpResponse::NoContent().finish()
            } else {
                let _ = tx.rollback().await;
                HttpResponse::NotFound().json(ErrorResponse {
                    error: "Package not found".to_string(),
                })
            }
        }
        Err(e) => {
            let _ = tx.rollback().await;
            println!("Error deleting package: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to delete package".to_string(),
            })
        }
    }
}

/// Get questions in package
#[utoipa::path(
    get,
    path = "/admin/packages/{id}/questions",
    params(
        ("id" = i32, Path, description = "Package ID")
    ),
    responses(
        (status = 200, description = "Package questions retrieved successfully", body = Vec<AdminSoal>),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Package not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/{id}/questions")]
async fn get_package_questions(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/packages/{id}/questions", &data.connections);

    let package_id = path.into_inner();

    let query = r#"
        SELECT 
            s.*,
            COALESCE(s.created_at, NOW()) as created_at,
            COALESCE(s.updated_at, NOW()) as updated_at,
            0 as usage_count
        FROM dbquizapp.soal s
        JOIN dbquizapp.paket_soal_items psi ON s.id = psi.soal_id
        WHERE psi.paket_soal_id = ?
        ORDER BY psi.id ASC
    "#;

    match sqlx::query_as::<_, AdminSoal>(query)
        .bind(package_id)
        .fetch_all(&*data.context.soal.pool)
        .await 
    {
        Ok(questions) => HttpResponse::Ok().json(questions),
        Err(e) => {
            println!("Error fetching package questions: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch package questions".to_string(),
            })
        }
    }
}

/// Add questions to package
#[utoipa::path(
    post,
    path = "/admin/packages/{id}/questions",
    params(
        ("id" = i32, Path, description = "Package ID")
    ),
    request_body = AddQuestionsRequest,
    responses(
        (status = 200, description = "Questions added successfully", body = PackageOperationResponse),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Package not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("/{id}/questions")]
async fn add_questions_to_package(
    path: web::Path<i32>,
    request: web::Json<AddQuestionsRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/packages/{id}/questions", &data.connections);

    let package_id = path.into_inner();

    if request.question_ids.is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "No questions provided".to_string(),
        });
    }

    let mut added_count = 0;
    for question_id in &request.question_ids {
        // Check if question already exists in package
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM dbquizapp.paket_soal_items WHERE paket_soal_id = ? AND soal_id = ?"
        )
        .bind(package_id)
        .bind(question_id)
        .fetch_one(&*data.context.soal.pool)
        .await;

        match exists {
            Ok(0) => {
                // Add question to package
                match sqlx::query(
                    "INSERT INTO dbquizapp.paket_soal_items (paket_soal_id, soal_id) VALUES (?, ?)"
                )
                .bind(package_id)
                .bind(question_id)
                .execute(&*data.context.soal.pool)
                .await
                {
                    Ok(_) => added_count += 1,
                    Err(e) => println!("Error adding question {} to package: {:?}", question_id, e),
                }
            }
            Ok(_) => {
                println!("Question {} already exists in package {}", question_id, package_id);
            }
            Err(e) => {
                println!("Error checking question {} existence: {:?}", question_id, e);
            }
        }
    }

    HttpResponse::Ok().json(PackageOperationResponse {
        success: true,
        message: format!("Successfully added {} questions to package", added_count),
        affected_count: Some(added_count),
    })
}

/// Remove question from package
#[utoipa::path(
    delete,
    path = "/admin/packages/{id}/questions/{question_id}",
    params(
        ("id" = i32, Path, description = "Package ID"),
        ("question_id" = i32, Path, description = "Question ID")
    ),
    responses(
        (status = 204, description = "Question removed successfully"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Question not found in package"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[delete("/{id}/questions/{question_id}")]
async fn remove_question_from_package(
    path: web::Path<(i32, i32)>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("DELETE /admin/packages/{id}/questions/{question_id}", &data.connections);

    let (package_id, question_id) = path.into_inner();

    let result = sqlx::query(
        "DELETE FROM dbquizapp.paket_soal_items WHERE paket_soal_id = ? AND soal_id = ?"
    )
    .bind(package_id)
    .bind(question_id)
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() > 0 {
                HttpResponse::NoContent().finish()
            } else {
                HttpResponse::NotFound().json(ErrorResponse {
                    error: "Question not found in package".to_string(),
                })
            }
        }
        Err(e) => {
            println!("Error removing question from package: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to remove question from package".to_string(),
            })
        }
    }
}

// Helper function to get package by ID
async fn get_package_by_id_internal(
    data: &web::Data<AppState<'_>>,
    package_id: i32,
) -> Result<PaketSoal, sqlx::Error> {
    sqlx::query_as::<_, PaketSoal>(
        r#"
        SELECT 
            ps.id, 
            ps.nama_paket_soal, 
            ps.kategori_id, 
            ps.is_premium,
            ks.nama_kategori as kategori_nama,
            COUNT(psi.soal_id) as jumlah_soal
        FROM dbquizapp.paket_soal ps
        LEFT JOIN dbquizapp.kategori_soal ks ON ps.kategori_id = ks.id
        LEFT JOIN dbquizapp.paket_soal_items psi ON ps.id = psi.paket_soal_id
        WHERE ps.id = ?
        GROUP BY ps.id, ps.nama_paket_soal, ps.kategori_id, ps.is_premium, ks.nama_kategori
        "#
    )
    .bind(package_id)
    .fetch_one(&*data.context.soal.pool)
    .await
}