use crate::controller::log_request;
use crate::model::users::{CheckPhoneNumberRequest, CheckPhoneNumberResponse, UpdatePhoneNumberRequest, UpdatePhoneNumberResponse, OnboardingRequest, OnboardingResponse, RecommendationsResponse};
use crate::model::xp::{UserXpResponse, XpHistoryEntry, XpHistoryResponse};
use crate::model::user_preferences::UpdatePreferencesRequest;
use crate::model::{QuizHistoryQuery};
use crate::levels::compute_level_info;
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/user")
            .route("/check-phone", web::post().to(check_phone_number))
            .route("/update-phone", web::post().to(update_phone_number))
            .route("/quiz-history", web::get().to(get_quiz_history))
            .route("/profile", web::get().to(get_user_profile))
            .route("/stats", web::get().to(get_user_stats))
            .route("/onboarding", web::patch().to(update_onboarding))
            .route("/recommendations", web::get().to(get_recommendations))
            .route("/me/xp", web::get().to(get_user_xp))
            .route("/me/xp/history", web::get().to(get_user_xp_history))
            .route("/preferences", web::get().to(get_preferences))
            .route("/preferences", web::put().to(update_preferences))
    );
}

/// Check if a user has a phone number
async fn check_phone_number(
    req: web::Json<CheckPhoneNumberRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/user/check-phone", &data.connections);

    // Get user_id from token and verify it matches the request
    let token_user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    if token_user_id != req.user_id {
        return HttpResponse::Forbidden().json(ErrorResponse {
            error: "User ID in token does not match request".to_string(),
        });
    }

    // Get user from database
    match data.context.users.get_user_by_id(&req.user_id).await {
        Ok(Some(user)) => {
            // Return response with phone number status
            HttpResponse::Ok().json(CheckPhoneNumberResponse {
                has_phone: user.phone_number.is_some(),
                phone_number: user.phone_number,
                user_id: user.id,
            })
        },
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("User with ID {} not found", req.user_id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get user: {}", e),
        }),
    }
}

/// Get quiz history for the authenticated user
async fn get_quiz_history(
    query: web::Query<QuizHistoryQuery>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/user/quiz-history", &data.connections);

    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100).max(1);

    match data.context.quiz_sessions.get_user_quiz_history(&user_id, page, limit).await {
        Ok(history) => HttpResponse::Ok().json(history),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to fetch quiz history: {}", e),
        }),
    }
}

/// Update a user's phone number
async fn update_phone_number(
    req: web::Json<UpdatePhoneNumberRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/user/update-phone", &data.connections);

    // Get user_id from token and verify it matches the request
    let token_user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    if token_user_id != req.user_id {
        return HttpResponse::Forbidden().json(ErrorResponse {
            error: "User ID in token does not match request".to_string(),
        });
    }

    // Validate phone number format (basic validation)
    if req.phone_number.len() < 10 || req.phone_number.len() > 15 {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Invalid phone number format. Phone number should be between 10 and 15 digits.".to_string(),
        });
    }

    // Get user from database
    match data.context.users.get_user_by_id(&req.user_id).await {
        Ok(Some(user)) => {
            // Update user's phone number
            match data.context.users.update_user_profile(
                &user.id,
                &user.display_name,
                user.picture_url.as_deref(),
                Some(&req.phone_number),
            ).await {
                Ok(_) => HttpResponse::Ok().json(UpdatePhoneNumberResponse {
                    success: true,
                    message: "Phone number updated successfully".to_string(),
                    user_id: user.id,
                    phone_number: req.phone_number.clone(),
                }),
                Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
                    error: format!("Failed to update phone number: {}", e),
                }),
            }
        },
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("User with ID {} not found", req.user_id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get user: {}", e),
        }),
    }
}

/// Get profile of the authenticated user
async fn get_user_profile(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/user/profile", &data.connections);

    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    match data.context.users.get_user_profile_with_subscription(&user_id).await {
        Ok(Some(profile)) => HttpResponse::Ok().json(profile),
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("User with ID {} not found", user_id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get user profile: {}", e),
        }),
    }
}

/// Save onboarding data for the authenticated user
async fn update_onboarding(
    req: web::Json<OnboardingRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/user/onboarding", &data.connections);

    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    let goals_json = match serde_json::to_string(&req.goals) {
        Ok(s) => s,
        Err(e) => return HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to serialize goals: {}", e),
        }),
    };

    match data.context.users.save_onboarding_data(
        &user_id,
        &goals_json,
        req.timeframe.as_deref(),
        req.exam_date,
        req.onboarding_completed,
    ).await {
        Ok(_) => HttpResponse::Ok().json(OnboardingResponse { success: true }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to save onboarding data: {}", e),
        }),
    }
}

/// Get paket soal recommendations based on user's onboarding goals
async fn get_recommendations(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/user/recommendations", &data.connections);

    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    // Fetch user to read stored onboarding goals
    let user = match data.context.users.get_user_by_id(&user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().json(ErrorResponse {
            error: "User not found".to_string(),
        }),
        Err(e) => return HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to fetch user: {}", e),
        }),
    };

    // Parse goals JSON string → Vec<String>
    let goals: Vec<String> = user.onboarding_goals
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    // Map goals → kategori_soal names (in-process, no DB round-trip)
    let categories: Vec<&str> = goals.iter()
        .filter_map(|g| match g.as_str() {
            "cpns"                  => Some("CPNS"),
            "pppk"                  => Some("PPPK"),
            "snbt" | "masuk_ptn"    => Some("UTBK"),
            "beasiswa"              => Some("BEASISWA"),
            "kedinasan"             => Some("KEDINASAN"),
            "upkp"                  => Some("UPKP"),
            _                       => None, // ppg, nakes, bumn → fallback
        })
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Goals that had a valid mapping (for the response field)
    let matched_goals: Vec<String> = goals.iter()
        .filter(|g| matches!(g.as_str(), "cpns" | "pppk" | "snbt" | "masuk_ptn" | "beasiswa" | "kedinasan" | "upkp"))
        .cloned()
        .collect();

    match data.context.paket_soal_response.get_recommended_packages(&categories).await {
        Ok(packages) => HttpResponse::Ok().json(RecommendationsResponse {
            data: packages,
            based_on_goals: matched_goals,
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to fetch recommendations: {}", e),
        }),
    }
}

/// Get learning statistics of the authenticated user
async fn get_user_stats(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/user/stats", &data.connections);

    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    match data.context.users.get_user_learning_stats(&user_id).await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get user stats: {}", e),
        }),
    }
}

/// Get XP status of the authenticated user
async fn get_user_xp(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    let pool = &*data.context.users.pool;

    let row: Option<(i64, i32)> = sqlx::query_as(
        "SELECT total_xp, current_level FROM users WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&user_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let (total_xp, _) = match row {
        Some(r) => r,
        None => return HttpResponse::NotFound().json(ErrorResponse {
            error: "User not found".to_string(),
        }),
    };

    let (level_cfg, next_cfg, progress) = compute_level_info(total_xp);
    let xp_in_level = total_xp - level_cfg.total_xp_required;
    let xp_to_next_level = next_cfg.map(|n| n.total_xp_required - total_xp);

    let global_rank: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) + 1 FROM users WHERE total_xp > ? AND deleted_at IS NULL",
    )
    .bind(total_xp)
    .fetch_one(pool)
    .await
    .unwrap_or(1);

    HttpResponse::Ok().json(UserXpResponse {
        total_xp,
        current_level: level_cfg.level,
        level_name: level_cfg.name.to_string(),
        level_icon: level_cfg.icon.to_string(),
        progress,
        xp_in_level,
        xp_to_next_level,
        global_rank,
    })
}

/// Get preferences of the authenticated user
async fn get_preferences(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    match data.context.users.get_user_preferences(&user_id).await {
        Ok(prefs) => HttpResponse::Ok().json(prefs),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get preferences: {}", e),
        }),
    }
}

/// Update preferences of the authenticated user
async fn update_preferences(
    req: web::Json<UpdatePreferencesRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    match data.context.users.update_user_preferences(&user_id, &req).await {
        Ok(prefs) => HttpResponse::Ok().json(prefs),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to update preferences: {}", e),
        }),
    }
}

/// Get paginated XP transaction history of the authenticated user
async fn get_user_xp_history(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    let page: u32 = query.get("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1)
        .max(1);
    let limit: u32 = query.get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(20)
        .min(100)
        .max(1);
    let offset = (page - 1) * limit;

    let pool = &*data.context.users.pool;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM xp_transactions WHERE user_id = ?",
    )
    .bind(&user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let rows: Vec<(String, i32, String, Option<String>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, amount, source, description, created_at FROM xp_transactions WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(&user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let entries: Vec<XpHistoryEntry> = rows
        .into_iter()
        .map(|(id, amount, source, description, created_at)| XpHistoryEntry {
            id,
            amount,
            source,
            description,
            created_at,
        })
        .collect();

    let total_pages = if total == 0 { 1 } else { ((total as u32) + limit - 1) / limit };

    HttpResponse::Ok().json(XpHistoryResponse {
        entries,
        total,
        page,
        limit,
        total_pages,
    })
}