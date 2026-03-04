use super::log_request;
use super::AppState;

// use crate::model::Soal;
use actix_web::{get, web, HttpResponse, Responder, HttpRequest};
use crate::utils::auth::extract_user_id;
use crate::service::redis_service::RedisService;
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_soal)
       .service(get_paket_soal_response)
       .service(get_paket_soal_by_category)
       .service(get_list_paket_soal)
       .service(get_list_paket_soal_lengkap)
       .service(check_quiz_access);
}

/// Get a specific soal by ID
#[utoipa::path(
    get,
    path = "/soal/{id}",
    responses(
        (status = 200, description = "Soal found successfully", body = Soal),
        (status = 404, description = "Soal not found")
    ),
    params(
        ("id" = String, Path, description = "Soal identifier")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/soal/{id}")]
async fn get_soal(
    soal_id: web::Path<String>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("GET: /soal", &app_state.connections);

    let soal = app_state.context.soal.get_soal_by_id(&soal_id).await;

    match soal {
        Err(_) => HttpResponse::NotFound().finish(),
        Ok(soal) => HttpResponse::Ok().json(soal),
    }
}

/// Get paket soal response by category and package name
#[utoipa::path(
    get,
    path = "/paket-soal-response/{nama_kategori}/{nama_paket_soal}",
    responses(
        (status = 200, description = "Paket soal found successfully", body = PaketSoalResponse),
        (status = 404, description = "Paket soal not found")
    ),
    params(
        ("nama_kategori" = String, Path, description = "Category name"),
        ("nama_paket_soal" = String, Path, description = "Package name")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/paket-soal-response/{nama_kategori}/{nama_paket_soal}")]
async fn get_paket_soal_response(
    path: web::Path<(String, String)>,
    req: HttpRequest,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    let (nama_kategori, nama_paket_soal) = path.into_inner();
    log_request("GET: /paket-soal-response", &app_state.connections);
    eprintln!("🚀 [ENDPOINT] get_paket_soal_response called with kategori='{}', paket_soal='{}'", nama_kategori, nama_paket_soal);
    
    // Create cache key
    let cache_key = format!("{}:{}", nama_kategori, nama_paket_soal);
    let cache_key_hash = format!("{:x}", md5::compute(&cache_key));
    eprintln!("🔑 [CACHE] Cache key: '{}' -> hash: '{}' -> parsed: {}", cache_key, cache_key_hash, cache_key_hash.parse::<u64>().unwrap_or(0));
    
    // Try to get from Redis cache first
    let paket_soal_response = if let Some(redis_pool) = &app_state.redis_pool {
        let mut con = redis_pool.quiz_cache().as_ref().clone();
        
        // Try to get from cache
        match RedisService::get_cached_quiz_by_key::<crate::model::PaketSoalResponse>(&mut con, &cache_key_hash).await {
            Ok(Some(cached_response)) => {
                println!("Cache hit for paket soal: {}/{}", nama_kategori, nama_paket_soal);
                cached_response
            },
            _ => {
                println!("Cache miss for paket soal: {}/{}", nama_kategori, nama_paket_soal);
                // Get from database
                match app_state.context.paket_soal_response.get_paket_soal_response(&nama_kategori, &nama_paket_soal).await {
                    Ok(response) => {
                        // Cache the response for 1 hour
                        if let Err(e) = RedisService::cache_quiz_by_key(&mut con, &cache_key_hash, &response, 3600).await {
                            eprintln!("Failed to cache paket soal response: {:?}", e);
                        }
                        response
                    },
                    Err(e) => {
                        println!("Error: {:?}", e);
                        return HttpResponse::NotFound().finish();
                    }
                }
            }
        }
    } else {
        // No Redis, get directly from database
        match app_state.context.paket_soal_response.get_paket_soal_response(&nama_kategori, &nama_paket_soal).await {
            Ok(response) => response,
            Err(e) => {
                println!("Error: {:?}", e);
                return HttpResponse::NotFound().finish();
            }
        }
    };
    
    // Check if the user can access this quiz package
    let mut can_access = true;
    let mut subscription_required = false;
    
    if paket_soal_response.is_premium {
        subscription_required = true;
        
        // Extract user ID from token
        let user_id_opt = extract_user_id(&req);
        
        if let Some(user_id) = user_id_opt {
            // Check if user has access to this premium quiz package
            match app_state.context.premium_quiz_access.check_user_access_to_quiz(&user_id, paket_soal_response.paket_soal_id).await {
                Ok(has_access) => {
                    can_access = has_access;
                },
                Err(e) => {
                    println!("Error checking user access: {:?}", e);
                    can_access = false;
                }
            }
        } else {
            // No token provided, user cannot access premium quiz
            can_access = false;
        }
    }
    
    // Get available premium plans if user cannot access
    let available_plans = if subscription_required && !can_access {
        match app_state.context.premium_plans.get_all_premium_plans().await {
            Ok(plans) => Some(plans),
            Err(_) => None,
        }
    } else {
        None
    };
    
    // Return the quiz package with access information
    eprintln!("📤 [RESPONSE] Returning {} questions in response", paket_soal_response.kumpulan_soal.len());
    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "quiz_package": paket_soal_response,
        "can_access": can_access,
        "subscription_required": subscription_required,
        "available_plans": available_plans
    }))
}

/// Get List of all Paket Soal
#[utoipa::path(
    get,
    path = "/listpaketsoal",
    responses(
        (status = 200, description = "List of paket soal retrieved successfully", body = ListPaketSoal),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/listpaketsoal")]
async fn get_list_paket_soal(
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("GET: /listpaketsoal", &app_state.connections);
    
    let list_paket_soal = app_state.context.paket_soal_response.get_list_paket_soal().await;

    match list_paket_soal {
        Err(e) => {
            println!("Error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        },
        Ok(list_paket_soal) => HttpResponse::Ok().json(list_paket_soal),
    }
}

/// Get paket soal responses by category
#[utoipa::path(
    get,
    path = "/paket-soal-response/{nama_kategori}",
    responses(
        (status = 200, description = "List of paket soal found successfully", body = Vec<PaketSoalResponse>),
        (status = 404, description = "Category not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("nama_kategori" = String, Path, description = "Category name")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/paket-soal-response/{nama_kategori}")]
async fn get_paket_soal_by_category(
    nama_kategori: web::Path<String>,
    req: HttpRequest,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("GET: /paket-soal-response/{nama_kategori}", &app_state.connections);
    
    // Get all paket soal responses for the category
    let paket_soal_responses = match app_state.context.paket_soal_response
        .get_paket_soal_by_category(&nama_kategori)
        .await {
            Ok(responses) => responses,
            Err(e) => {
                println!("Error: {:?}", e);
                return HttpResponse::NotFound().finish();
            }
        };
    
    // Extract user ID from token
    let user_id_opt = extract_user_id(&req);
    
    // Check access for each quiz package
    let mut quiz_packages_with_access = Vec::new();
    
    for response in paket_soal_responses {
        let mut can_access = true;
        
        if response.is_premium {
            if let Some(ref user_id) = user_id_opt {
                // Check if user has access to this premium quiz package
                match app_state.context.premium_quiz_access.check_user_access_to_quiz(user_id, response.paket_soal_id).await {
                    Ok(has_access) => {
                        can_access = has_access;
                    },
                    Err(e) => {
                        println!("Error checking user access: {:?}", e);
                        can_access = false;
                    }
                }
            } else {
                // No token provided, user cannot access premium quiz
                can_access = false;
            }
        }
        
        quiz_packages_with_access.push(serde_json::json!({
            "quiz_package": response,
            "can_access": can_access,
            "subscription_required": response.is_premium
        }));
    }
    
    // Get available premium plans if user is not premium
    let available_plans = if user_id_opt.is_some() {
        let user_id = user_id_opt.unwrap();
        
        // Check if user has any active premium subscription
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) 
            FROM dbquizapp.user_subscriptions 
            WHERE user_id = ? 
            AND status = 'active' 
            AND (end_date IS NULL OR end_date > NOW())
            "#
        )
        .bind(&user_id)
        .fetch_one(&*app_state.context.user_subscriptions.pool)
        .await;
        
        match result {
            Ok(count) => {
                if count == 0 {
                    // User is not premium, get available plans
                    match app_state.context.premium_plans.get_all_premium_plans().await {
                        Ok(plans) => Some(plans),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            },
            Err(_) => None,
        }
    } else {
        // No user ID, get available plans
        match app_state.context.premium_plans.get_all_premium_plans().await {
            Ok(plans) => Some(plans),
            Err(_) => None,
        }
    };
    
    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "quiz_packages": quiz_packages_with_access,
        "available_plans": available_plans
    }))
}

/// Get List of all Paket Soal with pricing information
#[utoipa::path(
    get,
    path = "/listpaketsoallengkap",
    responses(
        (status = 200, description = "List of paket soal with pricing retrieved successfully", body = Vec<ListPaketSoalLengkap>),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/listpaketsoallengkap")]
async fn get_list_paket_soal_lengkap(
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("GET: /listpaketsoallengkap", &app_state.connections);
    
    let list_paket_soal = app_state.context.paket_soal_response.get_list_paket_soal_lengkap().await;

    match list_paket_soal {
        Err(e) => {
            println!("Error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        },
        Ok(list_paket_soal) => HttpResponse::Ok().json(list_paket_soal),
    }
}

/// Check if a user can access a specific quiz package
#[utoipa::path(
    get,
    path = "/check-quiz-access/{paket_soal_id}",
    responses(
        (status = 200, description = "Access check completed successfully"),
        (status = 401, description = "Unauthorized - No valid token provided"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("paket_soal_id" = i32, Path, description = "Quiz package ID to check access for")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/check-quiz-access/{paket_soal_id}")]
async fn check_quiz_access(
    paket_soal_id: web::Path<i32>,
    req: HttpRequest,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("GET: /check-quiz-access", &app_state.connections);
    
    let paket_soal_id = paket_soal_id.into_inner();
    
    // First, check if the quiz package exists and if it's premium
    let is_premium_result = sqlx::query_scalar::<_, bool>(
        "SELECT is_premium FROM dbquizapp.paket_soal WHERE id = ?"
    )
    .bind(paket_soal_id)
    .fetch_optional(&*app_state.context.soal.pool)
    .await;
    
    match is_premium_result {
        Ok(Some(is_premium)) => {
            // If the quiz is not premium, everyone can access it
            if !is_premium {
                return HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "can_access": true,
                    "is_premium": false,
                    "message": "This quiz package is available to all users"
                }));
            }
            
            // If the quiz is premium, check if the user has access
            // Extract user ID from token
            let user_id_opt = extract_user_id(&req);
            
            if let Some(user_id) = user_id_opt {
                // Check if user has access to this premium quiz package
                match app_state.context.premium_quiz_access.check_user_access_to_quiz(&user_id, paket_soal_id).await {
                    Ok(true) => {
                        // User has access
                        HttpResponse::Ok().json(serde_json::json!({
                            "success": true,
                            "can_access": true,
                            "is_premium": true,
                            "message": "You have access to this premium quiz package"
                        }))
                    },
                    Ok(false) => {
                        // User does not have access
                        // Get available premium plans
                        let available_plans = match app_state.context.premium_plans.get_all_premium_plans().await {
                            Ok(plans) => plans,
                            Err(_) => Vec::new(),
                        };
                        
                        HttpResponse::Ok().json(serde_json::json!({
                            "success": true,
                            "can_access": false,
                            "is_premium": true,
                            "message": "This is a premium quiz package. Please subscribe to access it.",
                            "available_plans": available_plans
                        }))
                    },
                    Err(e) => {
                        println!("Error checking user access: {:?}", e);
                        HttpResponse::InternalServerError().json(serde_json::json!({
                            "success": false,
                            "error": "Failed to check user access"
                        }))
                    }
                }
            } else {
                // No token provided
                // Get available premium plans
                let available_plans = match app_state.context.premium_plans.get_all_premium_plans().await {
                    Ok(plans) => plans,
                    Err(_) => Vec::new(),
                };
                
                HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "can_access": false,
                    "is_premium": true,
                    "message": "This is a premium quiz package. Please subscribe to access it.",
                    "available_plans": available_plans
                }))
            }
        },
        Ok(None) => {
            // Quiz package not found
            HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": "Quiz package not found"
            }))
        },
        Err(e) => {
            println!("Error checking quiz premium status: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to check quiz package status"
            }))
        }
    }
}

// Kode yang dikomentari tetap tidak berubah

