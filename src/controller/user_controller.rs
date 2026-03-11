use crate::controller::log_request;
use crate::model::users::{CheckPhoneNumberRequest, CheckPhoneNumberResponse, UpdatePhoneNumberRequest, UpdatePhoneNumberResponse};
use crate::model::{QuizHistoryQuery};
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
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