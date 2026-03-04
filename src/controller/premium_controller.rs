use crate::controller::log_request;
use crate::model::premium_plan::{PremiumPlan, PremiumPlanResponse, CreatePremiumPlanRequest, UpdatePremiumPlanRequest};
use crate::model::user_subscription::{UserSubscription, UserSubscriptionResponse, CreateUserSubscriptionRequest, UpdateUserSubscriptionRequest};
use crate::model::premium_quiz_access::{PremiumQuizAccess, PremiumQuizAccessResponse, CreatePremiumQuizAccessRequest, UpdatePremiumQuizAccessRequest, QuizAccessCheckResponse};
use crate::model::payment_transaction::{PaymentTransaction, PaymentTransactionResponse, CreatePaymentRequest, PaymentStatus, MayarWebhookPayload};
use crate::AppState;
use actix_web::{get, post, put, delete, web, HttpResponse, Responder, HttpRequest};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use utoipa::ToSchema;
use crate::utils::auth::extract_user_id;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/premium")
            // Premium Plans
            .route("/plans", web::get().to(get_premium_plans))
            .route("/plans/{id}", web::get().to(get_premium_plan_by_id))
            .route("/plans", web::post().to(create_premium_plan))
            .route("/plans/{id}", web::put().to(update_premium_plan))
            .route("/plans/{id}", web::delete().to(delete_premium_plan))
            
            // User Subscriptions
            .route("/subscriptions", web::get().to(get_user_subscriptions))
            .route("/subscriptions/active", web::get().to(get_active_subscription))
            .route("/subscriptions/{id}", web::get().to(get_subscription_by_id))
            .route("/subscriptions", web::post().to(create_subscription))
            .route("/subscriptions/{id}", web::put().to(update_subscription))
            .route("/subscriptions/{id}/cancel", web::post().to(cancel_subscription))
            
            // Premium Quiz Access
            .route("/quiz-access", web::get().to(get_all_premium_quiz_access))
            .route("/quiz-access/{id}", web::get().to(get_premium_quiz_access_by_id))
            .route("/quiz-access/quiz/{paket_soal_id}", web::get().to(get_premium_quiz_access_by_paket_soal_id))
            .route("/quiz-access", web::post().to(create_premium_quiz_access))
            .route("/quiz-access/{id}", web::put().to(update_premium_quiz_access))
            .route("/quiz-access/{id}", web::delete().to(delete_premium_quiz_access))
            .route("/check-status", web::get().to(check_user_premium_status))
    );
}

// Premium Plans
async fn get_premium_plans(data: web::Data<AppState<'_>>) -> impl Responder {
    log_request("/premium/plans", &data.connections);

    match data.context.premium_plans.get_all_premium_plans().await {
        Ok(plans) => {
            let response: Vec<PremiumPlanResponse> = plans.into_iter().map(|plan| plan.into()).collect();
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get premium plans: {}", e),
            })
        }
    }
}

async fn get_premium_plan_by_id(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    log_request("/premium/plans/{id}", &data.connections);

    match data.context.premium_plans.get_premium_plan_by_id(id).await {
        Ok(Some(plan)) => {
            let response: PremiumPlanResponse = plan.into();
            HttpResponse::Ok().json(response)
        }
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Premium plan with ID {} not found", id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get premium plan: {}", e),
        }),
    }
}

async fn create_premium_plan(
    req: web::Json<CreatePremiumPlanRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("/premium/plans", &data.connections);

    // TODO: Add admin role check here

    match data.context.premium_plans.create_premium_plan(&req).await {
        Ok(id) => {
            HttpResponse::Created().json(serde_json::json!({
                "id": id,
                "message": "Premium plan created successfully"
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to create premium plan: {}", e),
        }),
    }
}

async fn update_premium_plan(
    path: web::Path<i32>,
    req: web::Json<UpdatePremiumPlanRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    log_request("/premium/plans/{id}", &data.connections);

    // TODO: Add admin role check here

    match data.context.premium_plans.update_premium_plan(id, &req).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Premium plan updated successfully"
        })),
        Ok(false) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Premium plan with ID {} not found or no changes made", id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to update premium plan: {}", e),
        }),
    }
}

async fn delete_premium_plan(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    log_request("/premium/plans/{id}", &data.connections);

    // TODO: Add admin role check here

    match data.context.premium_plans.delete_premium_plan(id).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Premium plan deleted successfully"
        })),
        Ok(false) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Premium plan with ID {} not found", id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to delete premium plan: {}", e),
        }),
    }
}

// User Subscriptions
async fn get_user_subscriptions(
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("/premium/subscriptions", &data.connections);

    // Extract user ID from token
    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    match data.context.user_subscriptions.get_user_subscriptions(&user_id).await {
        Ok(subscriptions) => {
            // Convert to response objects with plan names
            let mut response_subscriptions = Vec::new();
            
            for subscription in subscriptions {
                if let Ok(Some(plan)) = data.context.premium_plans.get_premium_plan_by_id(subscription.plan_id).await {
                    let days_remaining = if let Some(end_date) = subscription.end_date {
                        let now = Utc::now();
                        if end_date > now {
                            Some((end_date - now).num_days())
                        } else {
                            Some(0)
                        }
                    } else {
                        None
                    };
                    
                    response_subscriptions.push(UserSubscriptionResponse {
                        id: subscription.id,
                        user_id: subscription.user_id,
                        plan_id: subscription.plan_id,
                        plan_name: plan.name,
                        start_date: subscription.start_date,
                        end_date: subscription.end_date,
                        status: subscription.status.clone(),
                        is_lifetime: plan.is_lifetime,
                        days_remaining,
                    });
                }
            }
            
            HttpResponse::Ok().json(response_subscriptions)
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get user subscriptions: {}", e),
        }),
    }
}

async fn get_active_subscription(
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("/premium/subscriptions/active", &data.connections);

    // Extract user ID from token
    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    match data.context.user_subscriptions.get_active_subscription(&user_id).await {
        Ok(Some(subscription)) => {
            let days_remaining = if let Some(end_date) = subscription.end_date {
                let now = Utc::now();
                if end_date > now {
                    Some((end_date - now).num_days())
                } else {
                    Some(0)
                }
            } else {
                None
            };
            
            let response = UserSubscriptionResponse {
                id: subscription.id,
                user_id: subscription.user_id,
                plan_id: subscription.plan_id,
                plan_name: subscription.plan_name,
                start_date: subscription.start_date,
                end_date: subscription.end_date,
                status: subscription.status.clone(),
                is_lifetime: subscription.is_lifetime,
                days_remaining,
            };
            
            HttpResponse::Ok().json(response)
        }
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
            error: "No active subscription found".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get active subscription: {}", e),
        }),
    }
}

async fn get_subscription_by_id(
    data: web::Data<AppState<'_>>,
    path: web::Path<i32>,
    req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();
    log_request("/premium/subscriptions/{id}", &data.connections);

    // Extract user ID from token
    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    match data.context.user_subscriptions.get_user_subscription_by_id(id).await {
        Ok(Some(subscription)) => {
            // Verify that the subscription belongs to the user
            if subscription.user_id != user_id {
                return HttpResponse::Forbidden().json(ErrorResponse {
                    error: "You don't have permission to view this subscription".to_string(),
                });
            }
            
            if let Ok(Some(plan)) = data.context.premium_plans.get_premium_plan_by_id(subscription.plan_id).await {
                let days_remaining = if let Some(end_date) = subscription.end_date {
                    let now = Utc::now();
                    if end_date > now {
                        Some((end_date - now).num_days())
                    } else {
                        Some(0)
                    }
                } else {
                    None
                };
                
                let response = UserSubscriptionResponse {
                    id: subscription.id,
                    user_id: subscription.user_id,
                    plan_id: subscription.plan_id,
                    plan_name: plan.name,
                    start_date: subscription.start_date,
                    end_date: subscription.end_date,
                    status: subscription.status.clone(),
                    is_lifetime: plan.is_lifetime,
                    days_remaining,
                };
                
                HttpResponse::Ok().json(response)
            } else {
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Failed to get plan details".to_string(),
                })
            }
        }
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Subscription with ID {} not found", id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get subscription: {}", e),
        }),
    }
}

async fn create_subscription(
    data: web::Data<AppState<'_>>,
    subscription: web::Json<CreateUserSubscriptionRequest>,
    req: HttpRequest,
) -> impl Responder {
    log_request("/premium/subscriptions", &data.connections);

    // TODO: Add admin role check here or payment verification

    match data.context.user_subscriptions.create_subscription(&subscription).await {
        Ok(id) => {
            HttpResponse::Created().json(serde_json::json!({
                "id": id,
                "message": "Subscription created successfully"
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to create subscription: {}", e),
        }),
    }
}

async fn update_subscription(
    data: web::Data<AppState<'_>>,
    path: web::Path<i32>,
    update: web::Json<UpdateUserSubscriptionRequest>,
    req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();
    log_request("/premium/subscriptions/{id}", &data.connections);

    // TODO: Add admin role check here

    match data.context.user_subscriptions.update_subscription(id, &update).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Subscription updated successfully"
        })),
        Ok(false) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Subscription with ID {} not found or no changes made", id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to update subscription: {}", e),
        }),
    }
}

async fn cancel_subscription(
    data: web::Data<AppState<'_>>,
    path: web::Path<i32>,
    req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();
    log_request("/premium/subscriptions/{id}/cancel", &data.connections);

    // Extract user ID from token
    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    // Verify that the subscription belongs to the user
    match data.context.user_subscriptions.get_user_subscription_by_id(id).await {
        Ok(Some(subscription)) => {
            if subscription.user_id != user_id {
                return HttpResponse::Forbidden().json(ErrorResponse {
                    error: "You don't have permission to cancel this subscription".to_string(),
                });
            }
        }
        Ok(None) => {
            return HttpResponse::NotFound().json(ErrorResponse {
                error: format!("Subscription with ID {} not found", id),
            });
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to verify subscription: {}", e),
            });
        }
    }

    match data.context.user_subscriptions.cancel_subscription(id).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Subscription cancelled successfully"
        })),
        Ok(false) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Subscription with ID {} not found", id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to cancel subscription: {}", e),
        }),
    }
}

// Premium Quiz Access
async fn get_all_premium_quiz_access(data: web::Data<AppState<'_>>) -> impl Responder {
    log_request("/premium/quiz-access", &data.connections);

    match data.context.premium_quiz_access.get_all_premium_quiz_access().await {
        Ok(access_list) => HttpResponse::Ok().json(access_list),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get premium quiz access list: {}", e),
        }),
    }
}

async fn get_premium_quiz_access_by_id(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    log_request("/premium/quiz-access/{id}", &data.connections);

    match data.context.premium_quiz_access.get_premium_quiz_access_by_id(id).await {
        Ok(Some(access)) => HttpResponse::Ok().json(access),
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Premium quiz access with ID {} not found", id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get premium quiz access: {}", e),
        }),
    }
}

async fn get_premium_quiz_access_by_paket_soal_id(
    data: web::Data<AppState<'_>>,
    path: web::Path<i32>,
) -> impl Responder {
    let paket_soal_id = path.into_inner();
    log_request("/premium/quiz-access/quiz/{paket_soal_id}", &data.connections);

    match data.context.premium_quiz_access.get_premium_quiz_access_by_paket_soal_id(paket_soal_id).await {
        Ok(Some(access)) => HttpResponse::Ok().json(access),
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Premium quiz access for paket soal ID {} not found", paket_soal_id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get premium quiz access: {}", e),
        }),
    }
}

async fn create_premium_quiz_access(
    req: web::Json<CreatePremiumQuizAccessRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("/premium/quiz-access", &data.connections);

    // TODO: Add admin role check here

    match data.context.premium_quiz_access.create_premium_quiz_access(&req).await {
        Ok(id) => {
            HttpResponse::Created().json(serde_json::json!({
                "id": id,
                "message": "Premium quiz access created successfully"
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to create premium quiz access: {}", e),
        }),
    }
}

async fn update_premium_quiz_access(
    path: web::Path<i32>,
    req: web::Json<UpdatePremiumQuizAccessRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    log_request("/premium/quiz-access/{id}", &data.connections);

    // TODO: Add admin role check here

    match data.context.premium_quiz_access.update_premium_quiz_access(id, &req).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Premium quiz access updated successfully"
        })),
        Ok(false) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Premium quiz access with ID {} not found or no changes made", id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to update premium quiz access: {}", e),
        }),
    }
}

async fn delete_premium_quiz_access(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    log_request("/premium/quiz-access/{id}", &data.connections);

    // TODO: Add admin role check here

    match data.context.premium_quiz_access.delete_premium_quiz_access(id).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Premium quiz access deleted successfully"
        })),
        Ok(false) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Premium quiz access with ID {} not found", id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to delete premium quiz access: {}", e),
        }),
    }
}

/// Check if a user has premium status
#[utoipa::path(
    get,
    path = "/premium/check-status",
    responses(
        (status = 200, description = "User premium status check completed successfully"),
        (status = 401, description = "Unauthorized - No valid token provided"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn check_user_premium_status(
    req: HttpRequest,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("GET: /premium/check-status", &app_state.connections);
    
    // Extract user ID from token
    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "No valid token provided"
        })),
    };
    
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
            let has_premium = count > 0;
            
            // If user has premium, get subscription details
            let subscription_details = if has_premium {
                match app_state.context.user_subscriptions.get_user_subscriptions_by_user_id(&user_id).await {
                    Ok(subscriptions) => {
                        // Filter active subscriptions
                        let active_subscriptions: Vec<_> = subscriptions.into_iter()
                            .filter(|sub| sub.status == "active")
                            .collect();
                        
                        if !active_subscriptions.is_empty() {
                            Some(active_subscriptions)
                        } else {
                            None
                        }
                    },
                    Err(_) => None,
                }
            } else {
                None
            };
            
            // Get available premium plans for non-premium users
            let available_plans = if !has_premium {
                match app_state.context.premium_plans.get_all_premium_plans().await {
                    Ok(plans) => Some(plans),
                    Err(_) => None,
                }
            } else {
                None
            };
            
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "is_premium": has_premium,
                "subscription_details": subscription_details,
                "available_plans": available_plans
            }))
        },
        Err(e) => {
            println!("Error checking premium status: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to check premium status"
            }))
        }
    }
}

