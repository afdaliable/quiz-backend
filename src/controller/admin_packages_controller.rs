use crate::controller::log_request;
use crate::middleware::admin_middleware::AdminMiddleware;
use crate::model::paket_soal::{
    PaketSoal, AdminPaketSoal, AdminPaketSoalRequest, CreatePaketSoalRequest,
    UpdatePaketSoalRequest, PackageSearchRequest, PaginatedPackagesResponse, AddQuestionsRequest,
    PackageOperationResponse, GeneratePackageRequest, GeneratePackageResponse, DifficultyMix,
    PreviewDistributionRequest, PreviewDistributionResponse, SourceDistributionItem,
    SimulasiTemplate, SimulasiTemplateRequest, GenerateSimulasiRequest, GenerateSimulasiResponse,
    GeneratedSimulasiItem, SourceWeight,
    PackageSourceSummaryItem, PackageSourceSummaryResponse,
    SourceDistributionEntry, PackageSourceDistributionResponse,
    TrackSourcesQuery, TrackSourceEntry, TrackSourcesResponse,
};
use std::collections::HashSet;
use rand::seq::SliceRandom;
use crate::model::soal::AdminSoal;
use crate::service::redis_service::RedisService;
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest, get, post, put, delete};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use std::collections::HashMap;
use chrono::Datelike;

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
            // Static paths MUST come before /{id} to avoid capture
            .service(list_simulasi_templates)
            .service(create_simulasi_template)
            .service(update_simulasi_template)
            .service(delete_simulasi_template)
            .service(generate_simulasi_batch)
            .service(preview_distribution)
            .service(generate_package)
            // AFD-244/249: static paths before /{id}
            .service(source_distribution_summary)
            .service(track_sources)
            .service(get_package_by_id)
            .service(update_package)
            .service(delete_package)
            .service(duplicate_package)
            .service(toggle_package_published)
            .service(get_package_questions)
            .service(add_questions_to_package)
            .service(remove_question_from_package)
            .service(package_source_distribution)
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

            // Invalidate list caches karena paket soal baru ditambahkan
            if let Some(redis_pool) = &data.redis_pool {
                let mut con = redis_pool.quiz_cache().as_ref().clone();
                let _ = RedisService::invalidate_package_caches(&mut con).await;
            }

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

/// Flip a package between visible and hidden in the quiz app.
///
/// Until now visibility was decided by accident: the public listing
/// queries INNER JOIN kategori_soal, so a package with no category
/// silently vanished from app.nagih.id -- conflating "not categorised
/// yet" with "not meant to be published". `is_published` makes the
/// intent explicit, and the public queries in db_context now filter on it.
///
/// Redis caches the package listings for an hour, so the caches are
/// invalidated here; without that an admin would unpublish a package and
/// still see it live for up to an hour.
#[utoipa::path(
    post,
    path = "/admin/packages/{id}/toggle-published",
    params(("id" = i32, Path, description = "Package ID")),
    responses(
        (status = 200, description = "Publish state toggled", body = AdminPaketSoal),
        (status = 404, description = "Package not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = []))
)]
#[post("/{id}/toggle-published")]
async fn toggle_package_published(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let pool = &*data.context.soal.pool;

    let result = sqlx::query(
        "UPDATE dbquizapp.paket_soal SET is_published = NOT is_published, updated_at = NOW() WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            if let Some(redis_pool) = &data.redis_pool {
                let mut con = redis_pool.quiz_cache().as_ref().clone();
                let _ = RedisService::invalidate_package_caches(&mut con).await;
            }
            match get_package_by_id_internal(&data, id).await {
                Ok(pkg) => HttpResponse::Ok().json(pkg),
                Err(e) => {
                    println!("Error fetching package after toggle: {:?}", e);
                    HttpResponse::InternalServerError().json(ErrorResponse {
                        error: "Publish state toggled but failed to fetch package".to_string(),
                    })
                }
            }
        }
        Ok(_) => HttpResponse::NotFound().json(ErrorResponse {
            error: "Package not found".to_string(),
        }),
        Err(e) => {
            println!("Error toggling publish state: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to toggle publish state".to_string(),
            })
        }
    }
}

/// Duplicate a package, including its question mapping.
///
/// The admin UI has had a "duplicate" button wired to this path for a
/// while but no handler existed -- confirmed live, it returned 404. The
/// copy is created unpublished-by-default in the sense that it carries a
/// distinct name; question links are copied so the new package is
/// immediately usable rather than an empty shell.
#[utoipa::path(
    post,
    path = "/admin/packages/{id}/duplicate",
    params(("id" = i32, Path, description = "Package ID to duplicate")),
    responses(
        (status = 201, description = "Package duplicated", body = AdminPaketSoal),
        (status = 404, description = "Source package not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = []))
)]
#[post("/{id}/duplicate")]
async fn duplicate_package(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let source_id = path.into_inner();
    let pool = &*data.context.soal.pool;

    let source = match sqlx::query_as::<_, (String, Option<i32>, bool)>(
        "SELECT nama_paket_soal, kategori_id, is_premium FROM dbquizapp.paket_soal WHERE id = ?",
    )
    .bind(source_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse {
                error: "Package not found".to_string(),
            })
        }
        Err(e) => {
            println!("Error fetching package to duplicate: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch source package".to_string(),
            });
        }
    };
    let (name, kategori_id, is_premium) = source;

    // Name must stay unique-ish for humans scanning the list; a bare
    // "(copy)" collides the moment you duplicate twice.
    let new_name = {
        let base = format!("{} (copy)", name);
        let taken = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM dbquizapp.paket_soal WHERE nama_paket_soal = ?",
        )
        .bind(&base)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        if taken == 0 {
            base
        } else {
            format!("{} (copy {})", name, chrono::Utc::now().format("%H%M%S"))
        }
    };

    let insert = sqlx::query(
        r#"
        INSERT INTO dbquizapp.paket_soal (nama_paket_soal, kategori_id, is_premium, created_at, updated_at)
        VALUES (?, ?, ?, NOW(), NOW())
        "#,
    )
    .bind(&new_name)
    .bind(kategori_id)
    .bind(is_premium)
    .execute(pool)
    .await;

    let new_id = match insert {
        Ok(r) => r.last_insert_id() as i32,
        Err(e) => {
            println!("Error inserting duplicated package: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to create duplicate".to_string(),
            });
        }
    };

    // Copy the question mapping in one statement rather than row-by-row.
    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO dbquizapp.paket_soal_items (paket_soal_id, soal_id)
        SELECT ?, soal_id FROM dbquizapp.paket_soal_items WHERE paket_soal_id = ?
        "#,
    )
    .bind(new_id)
    .bind(source_id)
    .execute(pool)
    .await
    {
        println!("Error copying package items: {:?}", e);
        // The package itself exists; report rather than silently returning
        // a copy that is missing its questions.
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Package duplicated but copying its questions failed".to_string(),
        });
    }

    if let Some(redis_pool) = &data.redis_pool {
        let mut con = redis_pool.quiz_cache().as_ref().clone();
        let _ = RedisService::invalidate_package_caches(&mut con).await;
    }

    match get_package_by_id_internal(&data, new_id).await {
        Ok(pkg) => HttpResponse::Created().json(pkg),
        Err(e) => {
            println!("Error fetching duplicated package: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Package duplicated but failed to fetch details".to_string(),
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
                // Keep the linked simulasi name in sync (rename reflects in Simulasi Ujian list).
                let _ = sqlx::query("UPDATE exam_simulations SET nama_simulasi = ? WHERE paket_soal_id = ?")
                    .bind(&package_req.nama_paket_soal)
                    .bind(package_id)
                    .execute(&*data.context.soal.pool)
                    .await;
                // Invalidate list caches karena data paket soal berubah
                if let Some(redis_pool) = &data.redis_pool {
                    let mut con = redis_pool.quiz_cache().as_ref().clone();
                    let _ = RedisService::invalidate_package_caches(&mut con).await;
                }
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

    // Delete linked exam_simulations first (a generated simulasi paket has one),
    // so deleting the paket doesn't orphan it / break the simulasi list.
    if let Err(e) = sqlx::query("DELETE FROM exam_simulations WHERE paket_soal_id = ?")
        .bind(package_id)
        .execute(&mut *tx)
        .await
    {
        let _ = tx.rollback().await;
        println!("Error deleting linked exam_simulations: {:?}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Failed to delete linked simulasi".to_string(),
        });
    }

    // Delete package items
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
                // Invalidate list caches karena paket soal dihapus
                if let Some(redis_pool) = &data.redis_pool {
                    let mut con = redis_pool.quiz_cache().as_ref().clone();
                    let _ = RedisService::invalidate_package_caches(&mut con).await;
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

/// Generate a random package based on difficulty mix and optional taxonomy filters
#[utoipa::path(
    post,
    path = "/admin/packages/generate",
    request_body = GeneratePackageRequest,
    responses(
        (status = 201, description = "Package generated successfully", body = GeneratePackageResponse),
        (status = 400, description = "Invalid request or not enough questions available"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("/generate")]
async fn generate_package(
    req: web::Json<GeneratePackageRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/packages/generate", &data.connections);

    let total_needed = req.difficulty_mix.total();
    if total_needed == 0 {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "difficulty_mix must request at least 1 question total".to_string(),
        });
    }

    if req.nama_paket_soal.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "nama_paket_soal cannot be empty".to_string(),
        });
    }

    let pool = &*data.context.soal.pool;
    let do_balance = req.source_balance.unwrap_or(false);
    let empty_vec: Vec<String> = Vec::new();
    let tax = TaxonomyFilter {
        track_id: req.track_id.as_deref(),
        category_id: req.category_id.as_deref(),
        subcategory_id: req.subcategory_id.as_deref(),
        topic_id: req.topic_id.as_deref(),
        topic_ids: req.topic_ids.as_deref().unwrap_or(&empty_vec),
        tag_ids: req.tag_ids.as_deref().unwrap_or(&empty_vec),
        tag_labels: req.tag_labels.as_deref().unwrap_or(&empty_vec),
    };

    let mut rng = rand::thread_rng();
    let mut selected_ids: Vec<i32> = Vec::with_capacity(total_needed as usize);
    let mut total_source_dist: HashMap<String, usize> = HashMap::new();

    let difficulties: &[(&str, u32)] = &[
        ("easy",   req.difficulty_mix.easy.unwrap_or(0)),
        ("medium", req.difficulty_mix.medium.unwrap_or(0)),
        ("hard",   req.difficulty_mix.hard.unwrap_or(0)),
    ];

    // Track shortfall per requested difficulty so we can backfill difficulty-agnostically
    // (the soal pool is overwhelmingly 'medium', so easy/hard requests would otherwise fail).
    let mut shortfall: usize = 0;

    for (diff_label, count) in difficulties {
        if *count == 0 { continue; }

        let candidates = match fetch_candidates_full(
            pool,
            req.track_slug.as_deref(),
            req.category_slug.as_deref(),
            req.subcategory_slug.as_deref(),
            &tax,
            Some(diff_label),
            &selected_ids,
        ).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error fetching {} candidates: {:?}", diff_label, e);
                return HttpResponse::InternalServerError().json(ErrorResponse {
                    error: format!("Failed to query {} questions", diff_label),
                });
            }
        };

        let take_now = (*count as usize).min(candidates.len());
        if take_now < *count as usize {
            shortfall += *count as usize - take_now;
        }
        if take_now == 0 { continue; }

        let (picked, dist) = if do_balance {
            source_balanced_sample(
                candidates,
                take_now,
                req.allowed_sources.as_deref(),
                req.source_weights.as_deref(),
                &mut rng,
            )
        } else {
            let mut ids: Vec<i32> = candidates.into_iter().map(|(id, _)| id).collect();
            ids.shuffle(&mut rng);
            ids.truncate(take_now);
            (ids, HashMap::new())
        };

        for (source, n) in dist {
            *total_source_dist.entry(source).or_insert(0) += n;
        }
        selected_ids.extend(picked);
    }

    // Backfill shortfall ignoring difficulty (soft difficulty preference).
    if shortfall > 0 {
        let candidates = match fetch_candidates_full(
            pool,
            req.track_slug.as_deref(),
            req.category_slug.as_deref(),
            req.subcategory_slug.as_deref(),
            &tax,
            None, // any difficulty
            &selected_ids,
        ).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error fetching backfill candidates: {:?}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Failed to query backfill questions".to_string(),
                });
            }
        };
        let take_now = shortfall.min(candidates.len());
        if take_now > 0 {
            let (picked, dist) = if do_balance {
                source_balanced_sample(
                    candidates, take_now,
                    req.allowed_sources.as_deref(), req.source_weights.as_deref(), &mut rng,
                )
            } else {
                let mut ids: Vec<i32> = candidates.into_iter().map(|(id, _)| id).collect();
                ids.shuffle(&mut rng);
                ids.truncate(take_now);
                (ids, HashMap::new())
            };
            for (source, n) in dist { *total_source_dist.entry(source).or_insert(0) += n; }
            selected_ids.extend(picked);
        }
    }

    if selected_ids.is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "No questions matched the selected filters (track/category/subcategory/topic). Try widening the filters.".to_string(),
        });
    }

    let mut generation_rules = serde_json::json!({
        "track_slug": req.track_slug,
        "category_slug": req.category_slug,
        "subcategory_slug": req.subcategory_slug,
        "track_id": req.track_id,
        "category_id": req.category_id,
        "subcategory_id": req.subcategory_id,
        "topic_id": req.topic_id,
        "topic_ids": req.topic_ids,
        "tag_ids": req.tag_ids,
        "tag_labels": req.tag_labels,
        "source_balance": do_balance,
        "allowed_sources": req.allowed_sources,
        "source_weights": req.source_weights,
    });
    if do_balance && !total_source_dist.is_empty() {
        generation_rules["source_distribution"] =
            serde_json::to_value(&total_source_dist).unwrap_or_default();
    }

    let difficulty_mix_json = serde_json::json!({
        "easy":   req.difficulty_mix.easy.unwrap_or(0),
        "medium": req.difficulty_mix.medium.unwrap_or(0),
        "hard":   req.difficulty_mix.hard.unwrap_or(0),
    });

    // Assign a "LATIHAN TOPIK" category so generated topic pakets appear on home.
    let latihan_kategori_id: i32 = match get_or_create_kategori(pool, "LATIHAN TOPIK").await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("get_or_create_kategori (topik) error: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to resolve category".to_string() });
        }
    };

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Error starting transaction: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to start transaction".to_string(),
            });
        }
    };

    let kode_paket: Option<String> = if let Some(ref prefix) = req.kode_prefix {
        let prefix = prefix.trim().to_uppercase();
        if prefix.is_empty() {
            None
        } else {
            let year = chrono::Utc::now().year() as u16;
            match generate_kode_paket(&mut tx, &prefix, year).await {
                Ok(kode) => Some(kode),
                Err(e) => {
                    let _ = tx.rollback().await;
                    eprintln!("Error generating kode_paket: {:?}", e);
                    return HttpResponse::InternalServerError().json(ErrorResponse {
                        error: "Failed to generate package code".to_string(),
                    });
                }
            }
        }
    } else {
        None
    };

    let insert_result = sqlx::query(
        r#"
        INSERT INTO paket_soal
            (nama_paket_soal, kategori_id, is_premium, is_generated, generation_rules, difficulty_mix, kode_paket, created_at, updated_at)
        VALUES (?, ?, 0, 1, ?, ?, ?, NOW(), NOW())
        "#
    )
    .bind(&req.nama_paket_soal)
    .bind(latihan_kategori_id)
    .bind(generation_rules.to_string())
    .bind(difficulty_mix_json.to_string())
    .bind(&kode_paket)
    .execute(&mut *tx)
    .await;

    let paket_soal_id = match insert_result {
        Ok(r) => r.last_insert_id() as i32,
        Err(e) => {
            let _ = tx.rollback().await;
            eprintln!("Error inserting paket_soal: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to create package record".to_string(),
            });
        }
    };

    for (order, question_id) in selected_ids.iter().enumerate() {
        if let Err(e) = sqlx::query(
            "INSERT INTO paket_soal_items (paket_soal_id, soal_id) VALUES (?, ?)"
        )
        .bind(paket_soal_id)
        .bind(question_id)
        .execute(&mut *tx)
        .await
        {
            let _ = tx.rollback().await;
            eprintln!("Error inserting item order={} question_id={}: {:?}", order, question_id, e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to insert package questions".to_string(),
            });
        }
    }

    if let Err(e) = tx.commit().await {
        eprintln!("Error committing generate transaction: {:?}", e);
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: "Failed to commit package generation".to_string(),
        });
    }

    HttpResponse::Created().json(GeneratePackageResponse {
        paket_soal_id,
        nama_paket_soal: req.nama_paket_soal.clone(),
        kode_paket,
        total_questions: total_needed,
        difficulty_mix: DifficultyMix {
            easy: req.difficulty_mix.easy,
            medium: req.difficulty_mix.medium,
            hard: req.difficulty_mix.hard,
        },
        selected_question_ids: selected_ids,
    })
}

/// Preview source distribution without generating a package
#[post("/preview-distribution")]
async fn preview_distribution(
    req: web::Json<PreviewDistributionRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/packages/preview-distribution", &data.connections);

    let pool = &*data.context.soal.pool;
    let mut rng = rand::thread_rng();
    let empty_vec: Vec<String> = Vec::new();
    let tax = TaxonomyFilter {
        track_id: req.track_id.as_deref(),
        category_id: req.category_id.as_deref(),
        subcategory_id: req.subcategory_id.as_deref(),
        topic_id: req.topic_id.as_deref(),
        topic_ids: req.topic_ids.as_deref().unwrap_or(&empty_vec),
        tag_ids: req.tag_ids.as_deref().unwrap_or(&empty_vec),
        tag_labels: req.tag_labels.as_deref().unwrap_or(&empty_vec),
    };

    let mut source_available: HashMap<String, usize> = HashMap::new();
    let mut source_pick: HashMap<String, usize> = HashMap::new();
    let mut total_available = 0usize;

    let diff_total = req.difficulty_mix.easy.unwrap_or(0)
        + req.difficulty_mix.medium.unwrap_or(0)
        + req.difficulty_mix.hard.unwrap_or(0);

    if diff_total == 0 {
        // No difficulty filter — pick total_questions proportionally from all soal
        let total_needed = req.total_questions.unwrap_or(100) as usize;
        let candidates = match fetch_candidates_full(
            pool,
            req.track_slug.as_deref(),
            req.category_slug.as_deref(),
            req.subcategory_slug.as_deref(),
            &tax,
            None, // no difficulty filter
            &[],
        ).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error fetching candidates (no-difficulty) for preview: {:?}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Failed to query questions".to_string(),
                });
            }
        };
        for (_, source) in &candidates {
            let key = source.clone().unwrap_or_else(|| "unknown".to_string());
            *source_available.entry(key).or_insert(0) += 1;
        }
        total_available = candidates.len();
        let pick_count = total_needed.min(candidates.len());
        let (_, dist) = source_balanced_sample(
            candidates,
            pick_count,
            req.allowed_sources.as_deref(),
            req.source_weights.as_deref(),
            &mut rng,
        );
        for (source, n) in dist {
            *source_pick.entry(source).or_insert(0) += n;
        }
    } else {
        let difficulties: &[(&str, u32)] = &[
            ("easy",   req.difficulty_mix.easy.unwrap_or(0)),
            ("medium", req.difficulty_mix.medium.unwrap_or(0)),
            ("hard",   req.difficulty_mix.hard.unwrap_or(0)),
        ];

        for (diff_label, count) in difficulties {
            if *count == 0 { continue; }

            let candidates = match fetch_candidates_full(
                pool,
                req.track_slug.as_deref(),
                req.category_slug.as_deref(),
                req.subcategory_slug.as_deref(),
                &tax,
                Some(diff_label),
                &[],
            ).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error fetching {} candidates for preview: {:?}", diff_label, e);
                    return HttpResponse::InternalServerError().json(ErrorResponse {
                        error: format!("Failed to query {} questions", diff_label),
                    });
                }
            };

            for (_, source) in &candidates {
                let key = source.clone().unwrap_or_else(|| "unknown".to_string());
                *source_available.entry(key).or_insert(0) += 1;
            }
            total_available += candidates.len();

            let pick_count = (*count as usize).min(candidates.len());
            let (_, dist) = source_balanced_sample(
                candidates,
                pick_count,
                req.allowed_sources.as_deref(),
                req.source_weights.as_deref(),
                &mut rng,
            );
            for (source, n) in dist {
                *source_pick.entry(source).or_insert(0) += n;
            }
        }
    }

    let kode_preview = if let Some(ref prefix) = req.kode_prefix {
        let prefix = prefix.trim().to_uppercase();
        if prefix.is_empty() {
            None
        } else {
            let year = chrono::Utc::now().year() as u16;
            peek_kode_preview(pool, &prefix, year).await.ok()
        }
    } else {
        None
    };

    let mut sources: Vec<SourceDistributionItem> = source_available
        .into_iter()
        .map(|(source, available)| {
            let would_pick = source_pick.get(&source).copied().unwrap_or(0);
            SourceDistributionItem { source, available, would_pick }
        })
        .collect();
    sources.sort_by(|a, b| b.would_pick.cmp(&a.would_pick));

    HttpResponse::Ok().json(PreviewDistributionResponse {
        total_available,
        kode_preview,
        sources,
    })
}

// ── Helper: fetch candidate (id, source) pairs matching taxonomy + difficulty ──
//
// - difficulty: optional — pass None to skip difficulty filter (used by simulasi sections)
// - exclude_ids: soal to skip (anti-duplicate across batch packages)

/// Optional taxonomy filters by UUID (FK columns on `soal`). All None/empty = no filter.
#[derive(Default, Clone)]
struct TaxonomyFilter<'a> {
    track_id: Option<&'a str>,
    category_id: Option<&'a str>,
    subcategory_id: Option<&'a str>,
    /// Single topic (legacy). Combined with `topic_ids` via OR.
    topic_id: Option<&'a str>,
    /// Multiple topic UUIDs — soal matching ANY are included.
    topic_ids: &'a [String],
    /// Tag UUIDs — match against question_tags m2m OR the freetext soal.tag column.
    tag_ids: &'a [String],
    /// Tag labels (for matching the freetext soal.tag column).
    tag_labels: &'a [String],
}

async fn fetch_candidates_with_source(
    pool: &sqlx::MySqlPool,
    track_slug: Option<&str>,
    category_slug: Option<&str>,
    subcategory_slug: Option<&str>,
    difficulty: Option<&str>,
    exclude_ids: &[i32],
) -> Result<Vec<(i32, Option<String>)>, sqlx::Error> {
    fetch_candidates_full(
        pool, track_slug, category_slug, subcategory_slug,
        &TaxonomyFilter::default(), difficulty, exclude_ids,
    ).await
}

#[allow(clippy::too_many_arguments)]
async fn fetch_candidates_full(
    pool: &sqlx::MySqlPool,
    track_slug: Option<&str>,
    category_slug: Option<&str>,
    subcategory_slug: Option<&str>,
    tax: &TaxonomyFilter<'_>,
    difficulty: Option<&str>,
    exclude_ids: &[i32],
) -> Result<Vec<(i32, Option<String>)>, sqlx::Error> {
    let mut conditions: Vec<String> = vec!["s.status = 'active'".to_string()];
    let mut binds: Vec<String> = vec![];

    if let Some(d) = difficulty {
        conditions.push("s.difficulty_est = ?".to_string());
        binds.push(d.to_string());
    }
    // Slug-based filters (legacy callers)
    if let Some(ts) = track_slug {
        conditions.push("EXISTS (SELECT 1 FROM exam_tracks et WHERE et.id = s.track_id AND et.slug = ?)".to_string());
        binds.push(ts.to_string());
    }
    if let Some(cs) = category_slug {
        conditions.push("EXISTS (SELECT 1 FROM categories c WHERE c.id = s.category_id AND c.slug = ?)".to_string());
        binds.push(cs.to_string());
    }
    if let Some(ss) = subcategory_slug {
        conditions.push("EXISTS (SELECT 1 FROM subcategories sc WHERE sc.id = s.subcategory_id AND sc.slug = ?)".to_string());
        binds.push(ss.to_string());
    }
    // UUID-based filters (FK columns, direct match)
    if let Some(tid) = tax.track_id {
        conditions.push("s.track_id = ?".to_string());
        binds.push(tid.to_string());
    }
    if let Some(cid) = tax.category_id {
        conditions.push("s.category_id = ?".to_string());
        binds.push(cid.to_string());
    }
    if let Some(scid) = tax.subcategory_id {
        conditions.push("s.subcategory_id = ?".to_string());
        binds.push(scid.to_string());
    }
    // Topics: union of single `topic_id` + multi `topic_ids`. A soal matches if its
    // direct FK is any selected topic OR it's linked via question_topics m2m.
    let mut all_topics: Vec<String> = Vec::new();
    if let Some(topid) = tax.topic_id { all_topics.push(topid.to_string()); }
    for t in tax.topic_ids { all_topics.push(t.clone()); }
    all_topics.sort();
    all_topics.dedup();
    if !all_topics.is_empty() {
        let ph = all_topics.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        conditions.push(format!(
            "(s.topic_id IN ({ph}) OR EXISTS (SELECT 1 FROM question_topics qt WHERE qt.question_id = s.id AND qt.topic_id IN ({ph})))"
        ));
        for t in &all_topics { binds.push(t.clone()); }
        for t in &all_topics { binds.push(t.clone()); }
    }

    // Tags: match via question_tags m2m (by id) OR the freetext soal.tag column
    // (by label, since the m2m table is largely empty in current data).
    if !tax.tag_ids.is_empty() || !tax.tag_labels.is_empty() {
        let mut tag_conds: Vec<String> = Vec::new();
        if !tax.tag_ids.is_empty() {
            let ph = tax.tag_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            tag_conds.push(format!(
                "EXISTS (SELECT 1 FROM question_tags qtg WHERE qtg.question_id = s.id AND qtg.tag_id IN ({ph}))"
            ));
            for t in tax.tag_ids { binds.push(t.clone()); }
        }
        // freetext soal.tag LIKE each label (comma-separated values)
        for label in tax.tag_labels {
            tag_conds.push("s.tag LIKE ?".to_string());
            binds.push(format!("%{}%", label));
        }
        if !tag_conds.is_empty() {
            conditions.push(format!("({})", tag_conds.join(" OR ")));
        }
    }

    if !exclude_ids.is_empty() {
        let placeholders = exclude_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        conditions.push(format!("s.id NOT IN ({})", placeholders));
    }

    let query = format!(
        "SELECT s.id, s.source FROM soal s WHERE {}",
        conditions.join(" AND ")
    );

    let mut q = sqlx::query_as::<_, (i32, Option<String>)>(&query);
    for b in &binds {
        q = q.bind(b);
    }
    for id in exclude_ids {
        q = q.bind(id);
    }
    q.fetch_all(pool).await
}

// ── Helper: proportional source sampling ──

fn source_balanced_sample(
    candidates: Vec<(i32, Option<String>)>,
    count: usize,
    allowed_sources: Option<&[String]>,
    source_weights: Option<&[SourceWeight]>,
    rng: &mut rand::rngs::ThreadRng,
) -> (Vec<i32>, HashMap<String, usize>) {
    // Filter by allowed_sources if specified (weights also imply a source list)
    let effective_allowed: Option<Vec<String>> = source_weights
        .map(|ws| ws.iter().map(|w| w.source.clone()).collect())
        .or_else(|| allowed_sources.map(|a| a.to_vec()));

    let filtered: Vec<(i32, String)> = candidates
        .into_iter()
        .filter(|(_, source)| {
            match (&effective_allowed, source) {
                (Some(allowed), Some(s)) => allowed.iter().any(|a| a == s),
                (Some(_), None) => false,
                (None, _) => true,
            }
        })
        .map(|(id, source)| (id, source.unwrap_or_else(|| "unknown".to_string())))
        .collect();

    let mut by_source: HashMap<String, Vec<i32>> = HashMap::new();
    for (id, source) in filtered {
        by_source.entry(source).or_default().push(id);
    }

    if by_source.is_empty() {
        return (vec![], HashMap::new());
    }

    for ids in by_source.values_mut() {
        ids.shuffle(rng);
    }

    let total_available: usize = by_source.values().map(|v| v.len()).sum();

    // Compute a target "ideal" pick count per source (f32) from either custom
    // weights or — by default — the source's share of the available pool.
    // Then convert to integer quotas with the largest-remainder method so the
    // quotas sum to exactly `count` and match the targets within ±1 each.
    let ideal: Vec<(String, f32)> = if let Some(weights) =
        source_weights.filter(|ws| !ws.is_empty())
    {
        let total_weight: f32 = weights.iter()
            .filter(|w| by_source.contains_key(&w.source))
            .map(|w| w.weight.max(0.0))
            .sum::<f32>()
            .max(0.0001);
        weights.iter()
            .filter(|w| by_source.contains_key(&w.source) && w.weight > 0.0)
            .map(|w| (w.source.clone(), (w.weight.max(0.0) / total_weight) * count as f32))
            .collect()
    } else {
        // Proportional to pool availability (the default "Source Balance").
        by_source.iter()
            .map(|(src, ids)| (src.clone(), (ids.len() as f32 / total_available as f32) * count as f32))
            .collect()
    };

    // Largest-remainder apportionment, capped by each source's availability.
    let mut quotas: HashMap<String, usize> = HashMap::new();
    let mut remainders: Vec<(String, f32)> = Vec::with_capacity(ideal.len());
    let mut assigned = 0usize;
    for (src, target) in &ideal {
        let avail = by_source.get(src).map(|v| v.len()).unwrap_or(0);
        let floor = (target.floor() as usize).min(avail);
        quotas.insert(src.clone(), floor);
        assigned += floor;
        // Only sources with spare capacity compete for the remainder seats.
        if floor < avail {
            remainders.push((src.clone(), target - target.floor()));
        }
    }
    // Distribute leftover seats by largest fractional remainder, respecting capacity.
    let mut leftover = count.saturating_sub(assigned).min(total_available.saturating_sub(assigned));
    remainders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    while leftover > 0 {
        let mut progressed = false;
        for (src, _) in &remainders {
            if leftover == 0 { break; }
            let avail = by_source.get(src).map(|v| v.len()).unwrap_or(0);
            let cur = quotas.get(src).copied().unwrap_or(0);
            if cur < avail {
                *quotas.entry(src.clone()).or_insert(0) += 1;
                leftover -= 1;
                progressed = true;
            }
        }
        if !progressed { break; } // all sources at capacity
    }

    let mut selected: Vec<i32> = Vec::with_capacity(count);
    let mut distribution: HashMap<String, usize> = HashMap::new();
    for (src, ids) in &by_source {
        let take = quotas.get(src).copied().unwrap_or(0).min(ids.len());
        if take > 0 {
            selected.extend_from_slice(&ids[..take]);
            distribution.insert(src.clone(), take);
        }
    }

    selected.shuffle(rng);
    (selected, distribution)
}

// ── Helper: get-or-create a kategori_soal by name, returns its id ──
async fn get_or_create_kategori(
    pool: &sqlx::MySqlPool,
    nama: &str,
) -> Result<i32, sqlx::Error> {
    if let Some(id) = sqlx::query_scalar::<_, i32>(
        "SELECT id FROM kategori_soal WHERE nama_kategori = ? LIMIT 1"
    )
    .bind(nama)
    .fetch_optional(pool)
    .await?
    {
        return Ok(id);
    }
    let res = sqlx::query(
        "INSERT INTO kategori_soal (nama_kategori, created_at, updated_at) VALUES (?, NOW(), NOW())"
    )
    .bind(nama)
    .execute(pool)
    .await?;
    Ok(res.last_insert_id() as i32)
}

// ── Helper: generate kode_paket inside a transaction (race-condition safe) ──

async fn generate_kode_paket(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    prefix: &str,
    year: u16,
) -> Result<String, sqlx::Error> {
    sqlx::query(
        "INSERT INTO paket_kode_sequences (prefix, tahun, last_number) \
         VALUES (?, ?, 1) ON DUPLICATE KEY UPDATE last_number = last_number + 1"
    )
    .bind(prefix)
    .bind(year)
    .execute(&mut **tx)
    .await?;

    let num: i32 = sqlx::query_scalar(
        "SELECT last_number FROM paket_kode_sequences WHERE prefix = ? AND tahun = ?"
    )
    .bind(prefix)
    .bind(year)
    .fetch_one(&mut **tx)
    .await?;

    let kode = if num > 999 {
        format!("{}-{}-{:04}", prefix, year, num)
    } else {
        format!("{}-{}-{:03}", prefix, year, num)
    };
    Ok(kode)
}

// ── Helper: peek next kode without incrementing ──

async fn peek_kode_preview(
    pool: &sqlx::MySqlPool,
    prefix: &str,
    year: u16,
) -> Result<String, sqlx::Error> {
    let num: Option<i32> = sqlx::query_scalar(
        "SELECT last_number FROM paket_kode_sequences WHERE prefix = ? AND tahun = ?"
    )
    .bind(prefix)
    .bind(year)
    .fetch_optional(pool)
    .await?;

    let next = num.unwrap_or(0) + 1;
    let kode = if next > 999 {
        format!("{}-{}-{:04}", prefix, year, next)
    } else {
        format!("{}-{}-{:03}", prefix, year, next)
    };
    Ok(kode)
}

// ── Simulasi Templates CRUD ──

#[get("/simulasi-templates")]
async fn list_simulasi_templates(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/packages/simulasi-templates", &data.connections);
    match sqlx::query_as::<_, SimulasiTemplate>(
        "SELECT id, exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, is_active, navigation_mode FROM simulasi_templates ORDER BY id"
    )
    .fetch_all(&*data.context.soal.pool)
    .await
    {
        Ok(templates) => HttpResponse::Ok().json(templates),
        Err(e) => {
            eprintln!("list_simulasi_templates error: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to fetch templates".to_string() })
        }
    }
}

#[post("/simulasi-templates")]
async fn create_simulasi_template(
    req: web::Json<SimulasiTemplateRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/packages/simulasi-templates", &data.connections);
    if req.exam_type.trim().is_empty() || req.kode_prefix.trim().is_empty() || req.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse { error: "exam_type, kode_prefix, and name are required".to_string() });
    }
    let sections_json = match serde_json::to_string(&req.sections) {
        Ok(j) => j,
        Err(_) => return HttpResponse::BadRequest().json(ErrorResponse { error: "Invalid sections".to_string() }),
    };
    let result = sqlx::query(
        "INSERT INTO simulasi_templates (exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, navigation_mode) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(req.exam_type.trim())
    .bind(req.kode_prefix.trim().to_uppercase())
    .bind(req.name.trim())
    .bind(&req.description)
    .bind(&sections_json)
    .bind(req.duration_minutes)
    .bind(req.passing_score.unwrap_or(60))
    .bind(req.navigation_mode.as_deref().unwrap_or("free"))
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(r) => {
            let id = r.last_insert_id() as i32;
            match sqlx::query_as::<_, SimulasiTemplate>(
                "SELECT id, exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, is_active, navigation_mode FROM simulasi_templates WHERE id = ?"
            )
            .bind(id)
            .fetch_one(&*data.context.soal.pool)
            .await
            {
                Ok(t) => HttpResponse::Created().json(t),
                Err(_) => HttpResponse::Created().json(serde_json::json!({"id": id})),
            }
        }
        Err(e) => {
            eprintln!("create_simulasi_template error: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to create template".to_string() })
        }
    }
}

#[put("/simulasi-templates/{id}")]
async fn update_simulasi_template(
    path: web::Path<i32>,
    req: web::Json<SimulasiTemplateRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("PUT /admin/packages/simulasi-templates/{id}", &data.connections);
    let template_id = path.into_inner();
    let sections_json = match serde_json::to_string(&req.sections) {
        Ok(j) => j,
        Err(_) => return HttpResponse::BadRequest().json(ErrorResponse { error: "Invalid sections".to_string() }),
    };
    let result = sqlx::query(
        "UPDATE simulasi_templates SET exam_type=?, kode_prefix=?, name=?, description=?, sections=?, duration_minutes=?, passing_score=?, navigation_mode=? WHERE id=?"
    )
    .bind(req.exam_type.trim())
    .bind(req.kode_prefix.trim().to_uppercase())
    .bind(req.name.trim())
    .bind(&req.description)
    .bind(&sections_json)
    .bind(req.duration_minutes)
    .bind(req.passing_score.unwrap_or(60))
    .bind(req.navigation_mode.as_deref().unwrap_or("free"))
    .bind(template_id)
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(_) => HttpResponse::NotFound().json(ErrorResponse { error: "Template not found".to_string() }),
        Err(e) => {
            eprintln!("update_simulasi_template error: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to update template".to_string() })
        }
    }
}

#[delete("/simulasi-templates/{id}")]
async fn delete_simulasi_template(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("DELETE /admin/packages/simulasi-templates/{id}", &data.connections);
    let template_id = path.into_inner();
    let result = sqlx::query("DELETE FROM simulasi_templates WHERE id = ?")
        .bind(template_id)
        .execute(&*data.context.soal.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(_) => HttpResponse::NotFound().json(ErrorResponse { error: "Template not found".to_string() }),
        Err(e) => {
            eprintln!("delete_simulasi_template error: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to delete template".to_string() })
        }
    }
}

// ── Generate Simulasi Batch ──

#[post("/generate-simulasi")]
async fn generate_simulasi_batch(
    req: web::Json<GenerateSimulasiRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("POST /admin/packages/generate-simulasi", &data.connections);

    if req.jumlah_paket == 0 || req.jumlah_paket > 20 {
        return HttpResponse::BadRequest().json(ErrorResponse { error: "jumlah_paket must be 1–20".to_string() });
    }
    if req.nama_prefix.trim().is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse { error: "nama_prefix cannot be empty".to_string() });
    }

    let pool = &*data.context.soal.pool;

    // Load template
    let template = match sqlx::query_as::<_, SimulasiTemplate>(
        "SELECT id, exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, is_active, navigation_mode FROM simulasi_templates WHERE exam_type = ? AND is_active = true"
    )
    .bind(req.exam_type.trim())
    .fetch_optional(pool)
    .await
    {
        Ok(Some(t)) => t,
        Ok(None) => return HttpResponse::BadRequest().json(ErrorResponse { error: format!("Template '{}' not found or inactive", req.exam_type) }),
        Err(e) => {
            eprintln!("load template error: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to load template".to_string() });
        }
    };

    if template.sections.is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse { error: "Template has no sections".to_string() });
    }

    let do_balance = req.source_balance.unwrap_or(false);
    let do_create_simulasi = req.create_exam_simulasi.unwrap_or(true);
    let is_premium = req.is_premium.unwrap_or(false);
    let total_per_paket: u32 = template.sections.iter().map(|s| s.count).sum();
    let year = chrono::Utc::now().year() as u16;

    // Resolve (or create) the "SIMULASI UJIAN" category so generated pakets
    // appear on the user home (which inner-joins paket_soal → kategori_soal).
    let simulasi_kategori_id: i32 = match get_or_create_kategori(pool, "SIMULASI UJIAN").await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("get_or_create_kategori error: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to resolve simulasi category".to_string() });
        }
    };

    let mut rng = rand::thread_rng();
    let mut used_ids: HashSet<i32> = HashSet::new();
    let mut packages: Vec<GeneratedSimulasiItem> = Vec::with_capacity(req.jumlah_paket as usize);
    let mut total_source_dist: HashMap<String, usize> = HashMap::new();

    for i in 0..req.jumlah_paket {
        let nama = format!("{} Paket {}", req.nama_prefix.trim(), i + 1);
        let exclude: Vec<i32> = used_ids.iter().copied().collect();

        let mut paket_ids: Vec<i32> = Vec::with_capacity(total_per_paket as usize);

        // Collect soal per section
        let mut section_ok = true;
        for section in &template.sections {
            let candidates = match fetch_candidates_with_source(
                pool, None, None, Some(&section.subcategory_slug),
                None, &exclude,
            ).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("fetch section '{}' error: {:?}", section.name, e);
                    section_ok = false;
                    break;
                }
            };

            if candidates.len() < section.count as usize {
                return HttpResponse::BadRequest().json(ErrorResponse {
                    error: format!(
                        "Paket #{}: not enough soal for section '{}'. Need {}, available {} (after excluding used).",
                        i + 1, section.name, section.count, candidates.len()
                    ),
                });
            }

            let (picked, dist) = if do_balance {
                source_balanced_sample(candidates, section.count as usize, req.allowed_sources.as_deref(), req.source_weights.as_deref(), &mut rng)
            } else {
                let mut ids: Vec<i32> = candidates.into_iter().map(|(id, _)| id).collect();
                ids.shuffle(&mut rng);
                ids.truncate(section.count as usize);
                (ids, HashMap::new())
            };

            for (src, n) in dist {
                *total_source_dist.entry(src).or_insert(0) += n;
            }
            paket_ids.extend(&picked);
            used_ids.extend(&picked);
        }

        if !section_ok {
            return HttpResponse::InternalServerError().json(ErrorResponse { error: format!("Failed to fetch soal for paket #{}", i + 1) });
        }

        // Transaction: paket_soal + items + exam_simulations
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                eprintln!("begin tx error: {:?}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to start transaction".to_string() });
            }
        };

        let kode = match generate_kode_paket(&mut tx, &template.kode_prefix, year).await {
            Ok(k) => k,
            Err(e) => {
                let _ = tx.rollback().await;
                eprintln!("kode_paket error: {:?}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to generate package code".to_string() });
            }
        };

        let generation_rules = serde_json::json!({
            "exam_type": req.exam_type,
            "source_balance": do_balance,
            "template_sections": template.sections.iter().map(|s| &s.name).collect::<Vec<_>>(),
        });

        let paket_id: i32 = match sqlx::query(
            "INSERT INTO paket_soal (nama_paket_soal, kategori_id, is_premium, is_generated, generation_rules, kode_paket, created_at, updated_at) VALUES (?, ?, ?, 1, ?, ?, NOW(), NOW())"
        )
        .bind(&nama)
        .bind(simulasi_kategori_id)
        .bind(is_premium)
        .bind(generation_rules.to_string())
        .bind(&kode)
        .execute(&mut *tx)
        .await
        {
            Ok(r) => r.last_insert_id() as i32,
            Err(e) => {
                let _ = tx.rollback().await;
                eprintln!("insert paket_soal error: {:?}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to create package".to_string() });
            }
        };

        for question_id in &paket_ids {
            if let Err(e) = sqlx::query("INSERT INTO paket_soal_items (paket_soal_id, soal_id) VALUES (?, ?)")
                .bind(paket_id)
                .bind(question_id)
                .execute(&mut *tx)
                .await
            {
                let _ = tx.rollback().await;
                eprintln!("insert item error: {:?}", e);
                return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to insert package questions".to_string() });
            }
        }

        let simulasi_id: Option<i32> = if do_create_simulasi {
            let sections_json_str = serde_json::to_string(&template.sections).unwrap_or_else(|_| "[]".to_string());
            match sqlx::query(
                "INSERT INTO exam_simulations (nama_simulasi, deskripsi, paket_soal_id, generation_mode, exam_type, navigation_mode, sections_json, duration_minutes, total_questions, passing_score, is_premium, max_attempts, is_active) VALUES (?, ?, ?, 'simulasi_template', ?, ?, ?, ?, ?, ?, ?, 0, 1)"
            )
            .bind(&nama)
            .bind(&template.description)
            .bind(paket_id)
            .bind(&template.exam_type)
            .bind(&template.navigation_mode)
            .bind(&sections_json_str)
            .bind(template.duration_minutes)
            .bind(total_per_paket as i32)
            .bind(template.passing_score)
            .bind(is_premium)
            .execute(&mut *tx)
            .await
            {
                Ok(r) => Some(r.last_insert_id() as i32),
                Err(e) => {
                    let _ = tx.rollback().await;
                    eprintln!("insert exam_simulations error: {:?}", e);
                    return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to create exam simulasi record".to_string() });
                }
            }
        } else {
            None
        };

        if let Err(e) = tx.commit().await {
            eprintln!("commit error: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse { error: "Failed to commit transaction".to_string() });
        }

        packages.push(GeneratedSimulasiItem {
            paket_soal_id: paket_id,
            exam_simulasi_id: simulasi_id,
            kode_paket: kode,
            nama,
            total_soal: total_per_paket,
        });
    }

    // Build source distribution summary as percentages
    let grand_total: usize = total_source_dist.values().sum();
    let src_dist_summary: HashMap<String, String> = if grand_total > 0 {
        total_source_dist.iter()
            .map(|(src, n)| (src.clone(), format!("~{}%", (n * 100) / grand_total)))
            .collect()
    } else {
        HashMap::new()
    };

    HttpResponse::Created().json(GenerateSimulasiResponse {
        generated: packages.len() as u32,
        packages,
        source_distribution_summary: src_dist_summary,
    })
}

// ── AFD-244: Source distribution endpoints ──

/// Get source-distribution summary for all packages (dominant source per package)
#[get("/source-distribution-summary")]
async fn source_distribution_summary(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/packages/source-distribution-summary", &data.connections);

    let packages = sqlx::query_as::<_, PackageSourceSummaryItem>(
        r#"
        SELECT
            ps.id,
            ps.nama_paket_soal,
            ps.kode_paket,
            total.total_soal,
            COALESCE(dom.source, 'unknown') AS dominant_source,
            CAST(ROUND(dom.cnt * 100.0 / total.total_soal, 1) AS DOUBLE) AS dominant_persen
        FROM paket_soal ps
        JOIN (
            SELECT paket_soal_id, COUNT(*) AS total_soal
            FROM paket_soal_items
            GROUP BY paket_soal_id
        ) total ON total.paket_soal_id = ps.id
        LEFT JOIN (
            SELECT psi.paket_soal_id, s.source, COUNT(*) AS cnt,
                   ROW_NUMBER() OVER (PARTITION BY psi.paket_soal_id ORDER BY COUNT(*) DESC) AS rn
            FROM paket_soal_items psi
            JOIN soal s ON s.id = psi.soal_id
            GROUP BY psi.paket_soal_id, s.source
        ) dom ON dom.paket_soal_id = ps.id AND dom.rn = 1
        ORDER BY dominant_persen DESC
        "#,
    )
    .fetch_all(&*data.context.soal.pool)
    .await;

    match packages {
        Ok(pkgs) => {
            let total = pkgs.len();
            HttpResponse::Ok().json(PackageSourceSummaryResponse {
                packages: pkgs,
                total,
            })
        }
        Err(e) => {
            eprintln!("source_distribution_summary error: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch source distribution summary".to_string(),
            })
        }
    }
}

/// AFD-249: Discover available sources for a given track or subcategory
#[get("/track-sources")]
async fn track_sources(
    query: web::Query<TrackSourcesQuery>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("GET /admin/packages/track-sources", &data.connections);
    let pool = &*data.context.soal.pool;

    let mut conditions = vec!["s.status = 'active'".to_string()];
    let mut binds: Vec<String> = vec![];

    if let Some(ref ts) = query.track_slug {
        conditions.push(
            "EXISTS (SELECT 1 FROM exam_tracks et WHERE et.id = s.track_id AND et.slug = ?)".to_string(),
        );
        binds.push(ts.clone());
    }
    if let Some(ref ss) = query.subcategory_slug {
        conditions.push(
            "EXISTS (SELECT 1 FROM subcategories sc WHERE sc.id = s.subcategory_id AND sc.slug = ?)".to_string(),
        );
        binds.push(ss.clone());
    }

    let where_clause = conditions.join(" AND ");
    let sql = format!(
        r#"
        SELECT
            COALESCE(s.source, 'unknown') AS source,
            COUNT(*) AS count,
            CAST(COUNT(*) * 100.0 / SUM(COUNT(*)) OVER () AS DOUBLE) AS pct
        FROM soal s
        WHERE {where_clause}
        GROUP BY COALESCE(s.source, 'unknown')
        ORDER BY count DESC
        "#
    );

    let mut q = sqlx::query(&sql);
    for b in &binds {
        q = q.bind(b);
    }

    match q.fetch_all(pool).await {
        Ok(rows) => {
            use sqlx::Row;
            let total_soal: i64 = rows.iter().map(|r| r.get::<i64, _>("count")).sum();
            let sources: Vec<TrackSourceEntry> = rows
                .into_iter()
                .map(|r| TrackSourceEntry {
                    source: r.get("source"),
                    count: r.get("count"),
                    pct: r.try_get::<f64, _>("pct").unwrap_or(0.0),
                })
                .collect();
            HttpResponse::Ok().json(TrackSourcesResponse {
                track_slug: query.track_slug.clone(),
                subcategory_slug: query.subcategory_slug.clone(),
                sources,
                total_soal,
            })
        }
        Err(e) => {
            eprintln!("track_sources error: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch track sources".to_string(),
            })
        }
    }
}

/// Get per-source breakdown for a single package
#[get("/{id}/source-distribution")]
async fn package_source_distribution(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    let package_id = path.into_inner();
    log_request("GET /admin/packages/{id}/source-distribution", &data.connections);

    // Verify package exists and get name
    let header_row = sqlx::query(
        "SELECT id, nama_paket_soal FROM paket_soal WHERE id = ?",
    )
    .bind(package_id)
    .fetch_optional(&*data.context.soal.pool)
    .await;

    let header_row = match header_row {
        Ok(Some(r)) => r,
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse {
                error: "Package not found".to_string(),
            });
        }
        Err(e) => {
            eprintln!("package_source_distribution header error: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch package".to_string(),
            });
        }
    };

    use sqlx::Row;
    let nama_paket_soal: String = header_row.get("nama_paket_soal");

    // Source breakdown
    let rows = sqlx::query(
        r#"
        SELECT
            s.source,
            COUNT(*) AS cnt,
            CAST(COUNT(*) * 100.0 / SUM(COUNT(*)) OVER () AS DOUBLE) AS pct
        FROM paket_soal_items psi
        JOIN soal s ON s.id = psi.soal_id
        WHERE psi.paket_soal_id = ?
        GROUP BY s.source
        ORDER BY cnt DESC
        "#,
    )
    .bind(package_id)
    .fetch_all(&*data.context.soal.pool)
    .await;

    match rows {
        Ok(rs) => {
            let total_soal: i64 = rs.iter().map(|r| r.get::<i64, _>("cnt")).sum();
            let sources: Vec<SourceDistributionEntry> = rs
                .into_iter()
                .map(|r| SourceDistributionEntry {
                    source: r.try_get("source").ok(),
                    count: r.get("cnt"),
                    percent: r.try_get::<f64, _>("pct").unwrap_or(0.0),
                })
                .collect();
            HttpResponse::Ok().json(PackageSourceDistributionResponse {
                paket_soal_id: package_id,
                nama_paket_soal,
                total_soal,
                sources,
            })
        }
        Err(e) => {
            eprintln!("package_source_distribution rows error: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch source distribution".to_string(),
            })
        }
    }
}

// ── Helper: get package by ID ──

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