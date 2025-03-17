use crate::controller::log_request;
use crate::model::license_code::{VerifyLicenseRequest, ActivateLicenseResponse, LicenseStatus, UserInfo, SubscriptionInfo, MayarLicenseVerifyRequest, MayarLicenseVerifyResponse, LicenseCodeResponse};
use crate::model::user_subscription::{CreateUserSubscriptionRequest, UserSubscription};
use crate::service::license_service::LicenseService;
use crate::utils::auth::{generate_token, TokenClaims};
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};
use utoipa::ToSchema;
use serde_json::json;
use uuid::Uuid;
use log::{info, error, warn};

use crate::model::payment_transaction::{License, LicenseVerificationRequest};
use crate::model::premium_plan::PremiumPlan;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct GeneratePaymentLinkResponse {
    pub payment_link: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    // Routes that require authentication
    cfg.service(
        web::scope("/license")
            .route("/payment-link/{plan_id}", web::get().to(generate_payment_link))
            .route("/user-licenses", web::get().to(get_user_licenses))
    );
    
    // Routes that don't require authentication
    cfg.service(
        web::scope("/license-public")
            .route("/verify", web::post().to(verify_license))
            .route("/verify-mock", web::post().to(verify_license_mock))
    );
}

/// Generate a payment link for a premium plan
pub async fn generate_payment_link(
    req: HttpRequest,
    path: web::Path<i32>,
    state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("/license/payment-link/{plan_id}", &state.connections);
    
    let plan_id = path.into_inner();
    
    // Get user_id from token
    let user_id = match crate::utils::auth::get_user_id_from_token(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(json!({ "error": "Unauthorized" })),
    };

    // Get user details
    let user = match state.context.users.get_user_by_id(&user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().json(json!({ "error": "User not found" })),
        Err(e) => {
            error!("Error getting user: {}", e);
            return HttpResponse::InternalServerError().json(json!({ "error": "Internal server error" }));
        }
    };

    // Get plan details
    let plan = match state.context.premium_plans.get_premium_plan_by_id(plan_id).await {
        Ok(Some(plan)) => plan,
        Ok(None) => return HttpResponse::NotFound().json(json!({ "error": "Plan not found" })),
        Err(e) => {
            error!("Error getting plan: {}", e);
            return HttpResponse::InternalServerError().json(json!({ "error": "Internal server error" }));
        }
    };

    // Get Mayar payment link from plan
    let mayar_link_payment = match &plan.mayar_link_payment {
        Some(link) if !link.is_empty() => link.clone(),
        _ => {
            error!("No payment link configured for plan ID: {}", plan_id);
            return HttpResponse::BadRequest().json(json!({ "error": "No payment link configured for this plan" }));
        }
    };

    // Get product ID
    let product_id = match &plan.mayar_product_id {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            error!("No product ID configured for plan ID: {}", plan_id);
            return HttpResponse::BadRequest().json(json!({ "error": "No product ID configured for this plan" }));
        }
    };

    // Encode user details for URL - store these as temporary variables to avoid borrowing issues
    let email_encoded = urlencoding::encode(&user.email).to_string();
    let name_encoded = urlencoding::encode(&user.display_name).to_string();
    
    // Handle phone number (optional)
    let phone_encoded = match &user.phone_number {
        Some(phone) => urlencoding::encode(phone).to_string(),
        None => urlencoding::encode("").to_string(), // Empty string if no phone number
    };

    // Construct payment URL
    let payment_link = format!(
        "{}?email={}&productId={}&name={}&phone={}",
        mayar_link_payment, email_encoded, product_id, name_encoded, phone_encoded
    );

    info!("Generated payment link for user ID: {}, plan ID: {}", user_id, plan_id);
    HttpResponse::Ok().json(json!({ "payment_link": payment_link }))
}

/// Verify a license code from Mayar
pub async fn verify_license(
    license_data: web::Json<VerifyLicenseRequest>,
    state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("/license/verify", &state.connections);
    info!("Starting license verification process");
    
    let data = license_data.into_inner();
    info!("License data: code={}, product_id={}, email={}", data.license_code, data.product_id, data.email);
    
    // Validate license code format
    if data.license_code.len() < 8 || data.license_code.len() > 64 {
        error!("Invalid license code format: {}", data.license_code);
        return HttpResponse::BadRequest().json(json!({ 
            "success": false, 
            "error": "Invalid license code format" 
        }));
    }

    // Check if license already exists in our database
    info!("Checking if license already exists in database");
    match state.context.license_codes.get_license_by_code(&data.license_code).await {
        Ok(Some(_)) => {
            error!("License code already used: {}", data.license_code);
            return HttpResponse::BadRequest().json(json!({ 
                "success": false, 
                "error": "License code already used" 
            }));
        }
        Err(e) => {
            error!("Error checking license: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": format!("Error checking license: {}", e) 
            }));
        }
        _ => {
            info!("License code not found in database, proceeding with verification");
        }
    }

    // Verify license with Mayar
    info!("Verifying license with Mayar API");
    let license_service = LicenseService::new(
        state.config.get_mayar_api_key().to_string(),
        state.config.get_mayar_saas_api_url().to_string()
    );

    let mayar_response = match license_service.verify_license(&data.license_code, &data.product_id).await {
        Ok(response) => {
            info!("Mayar API response received successfully");
            if !response.isLicenseActive {
                error!("License is not active according to Mayar");
                return HttpResponse::BadRequest().json(json!({ 
                    "success": false, 
                    "error": "License is not active" 
                }));
            }
            response
        },
        Err(e) => {
            error!("Error verifying license with Mayar: {}", e);
            return HttpResponse::BadRequest().json(json!({ 
                "success": false, 
                "error": format!("License verification failed: {}", e) 
            }));
        }
    };

    // Get plan ID from product ID
    info!("Getting plan ID from product ID: {}", data.product_id);
    let plan_id = match state.context.premium_plans.get_premium_plan_by_mayar_product_id(&data.product_id).await {
        Ok(Some(plan)) => {
            info!("Found plan with ID: {} for product ID: {}", plan.id, data.product_id);
            plan.id
        },
        Ok(None) => {
            // If plan not found by product ID, try to get plan with ID 5 (Special Plan)
            info!("Plan not found for product ID: {}, trying to use plan ID 5", data.product_id);
            5 // Use plan ID 5 (Special Plan) as fallback
        },
        Err(e) => {
            error!("Error getting plan: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": format!("Error getting plan: {}", e) 
            }));
        }
    };

    // Get or create user by email
    info!("Getting or creating user by email: {}", data.email);
    let user = match state.context.users.get_user_by_email(&data.email).await {
        Ok(user) => {
            info!("Found existing user with ID: {}", user.id);
            user
        },
        Err(e) => {
            // If user not found, create a new one
            if let sqlx::Error::RowNotFound = e {
                info!("User not found, creating new user with email: {}", data.email);
                let user_id = Uuid::new_v4().to_string();
                
                match state.context.users.create_user_with_picture_and_phone(
                    &user_id,
                    &data.email,
                    &data.name,
                    None,
                    data.phone.as_deref(),
                ).await {
                    Ok(_) => {
                        info!("User created successfully with ID: {}", user_id);
                        match state.context.users.get_user_by_id(&user_id).await {
                            Ok(Some(user)) => {
                                info!("Retrieved newly created user");
                                user
                            },
                            _ => {
                                error!("Failed to retrieve newly created user");
                                return HttpResponse::InternalServerError().json(json!({ 
                                    "success": false, 
                                    "error": "Failed to create user" 
                                }));
                            }
                        }
                    },
                    Err(e) => {
                        error!("Error creating user: {}", e);
                        return HttpResponse::InternalServerError().json(json!({ 
                            "success": false, 
                            "error": format!("Failed to create user account: {}", e) 
                        }));
                    }
                }
            } else {
                error!("Error getting user: {}", e);
                return HttpResponse::InternalServerError().json(json!({ 
                    "success": false, 
                    "error": format!("Error getting user: {}", e) 
                }));
            }
        }
    };

    // Get plan details
    info!("Getting plan details for plan ID: {}", plan_id);
    let plan = match state.context.premium_plans.get_premium_plan_by_id(plan_id).await {
        Ok(Some(plan)) => {
            info!("Found plan: {}", plan.name);
            plan
        },
        Ok(None) => {
            error!("Plan not found for ID: {}", plan_id);
            // Create a default plan object
            PremiumPlan {
                id: plan_id,
                name: "Special Plan".to_string(),
                description: "Default plan".to_string(),
                price: 1000.0,
                duration_days: 60,
                is_lifetime: false,
                features: "[]".to_string(),
                mayar_product_id: Some(data.product_id.clone()),
                mayar_link_payment: None,
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
            }
        }
        Err(e) => {
            error!("Error getting plan: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": format!("Error getting plan: {}", e) 
            }));
        }
    };

    // Parse expiration date from Mayar response
    info!("Parsing expiration date from Mayar response");
    let expired_at = match &mayar_response.licenseCode.expiredAt {
        Some(date_str) => {
            info!("Expiration date from Mayar: {}", date_str);
            match chrono::DateTime::parse_from_rfc3339(date_str) {
                Ok(date) => {
                    info!("Successfully parsed expiration date");
                    Some(date.with_timezone(&Utc))
                },
                Err(e) => {
                    error!("Failed to parse expiration date: {}", e);
                    // If we can't parse the date, use plan duration
                    if plan.is_lifetime {
                        info!("Using lifetime plan (no expiration)");
                        None
                    } else {
                        let exp_date = Utc::now() + chrono::Duration::days(plan.duration_days as i64);
                        info!("Using plan duration: {} days, expiration: {}", plan.duration_days, exp_date);
                        Some(exp_date)
                    }
                }
            }
        },
        None => {
            info!("No expiration date from Mayar, using plan settings");
            if plan.is_lifetime {
                info!("Using lifetime plan (no expiration)");
                None
            } else {
                let exp_date = Utc::now() + chrono::Duration::days(plan.duration_days as i64);
                info!("Using plan duration: {} days, expiration: {}", plan.duration_days, exp_date);
                Some(exp_date)
            }
        }
    };

    // Create license in our database
    info!("Creating license in database");
    let license_data_json = match serde_json::to_string(&mayar_response) {
        Ok(json) => Some(json),
        Err(e) => {
            error!("Failed to serialize Mayar response: {}", e);
            None
        }
    };
    
    let license_id = match state.context.license_codes.create_license(
        &data.license_code,
        Some(&user.id),
        plan_id,
        LicenseStatus::Active,
        Some(&mayar_response.licenseCode.transactionId),
        &data.product_id,
        mayar_response.licenseCode.customerId.as_deref(),
        Some(&user.display_name),
        Some(&user.email),
        expired_at,
        mayar_response.licenseCode.activationLimit.as_deref(),
        mayar_response.licenseCode.useCount,
        license_data_json.as_deref(),
    ).await {
        Ok(id) => {
            info!("License created successfully with ID: {}", id);
            id
        },
        Err(e) => {
            error!("Error creating license: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": format!("Failed to save license: {}", e) 
            }));
        }
    };

    // Create a subscription for the user
    info!("Creating subscription for user: {}", user.id);
    let subscription_request = CreateUserSubscriptionRequest {
        user_id: user.id.clone(),
        plan_id,
    };
    
    // Log the subscription request details for debugging
    info!("Creating subscription with user_id: {}, plan_id: {}", user.id, plan_id);
    
    let subscription_id = match state.context.user_subscriptions.create_subscription(&subscription_request).await {
        Ok(id) => {
            info!("Subscription created successfully with ID: {}", id);
            id
        },
        Err(e) => {
            error!("Error creating subscription: {}", e);
            
            // More detailed error message that includes the error details
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": format!("Failed to create subscription: {}", e)
            }));
        }
    };

    // Get the subscription details
    info!("Getting subscription details for ID: {}", subscription_id);
    let subscription = match state.context.user_subscriptions.get_user_subscription_by_id(subscription_id).await {
        Ok(Some(subscription)) => {
            info!("Retrieved subscription details successfully");
            subscription
        },
        Ok(None) => {
            error!("Subscription not found after creation, ID: {}", subscription_id);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": "Failed to get subscription details" 
            }));
        },
        Err(e) => {
            error!("Error getting subscription details: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": format!("Failed to get subscription details: {}", e) 
            }));
        }
    };

    // Generate a JWT token for the user
    info!("Generating JWT token for user: {}", user.id);
    
    let token = match generate_token(
        &user.id,
        &user.email,
        &user.display_name,
        state.config.get_jwt_secret(),
        &state.config.get_app_url()
    ) {
        Ok(t) => {
            info!("JWT token generated successfully");
            t
        },
        Err(e) => {
            error!("Failed to generate token: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": format!("Failed to generate token: {}", e) 
            }));
        }
    };

    // Create the response
    info!("License verification completed successfully, preparing response");
    
    // Format the end_date separately to avoid serialization issues
    let formatted_end_date = match subscription.end_date {
        Some(date) => {
            match serde_json::to_value(date) {
                Ok(date_value) => date_value,
                Err(e) => {
                    error!("Failed to serialize end_date: {}", e);
                    serde_json::Value::Null
                }
            }
        },
        None => serde_json::Value::Null
    };
    
    // Create the response JSON
    let response = json!({
        "success": true,
        "message": "License activated successfully",
        "token": token,
        "user": {
            "id": user.id,
            "email": user.email,
            "display_name": user.display_name
        },
        "subscription": {
            "id": subscription.id,
            "plan_id": subscription.plan_id,
            "plan_name": plan.name,
            "expired_at": formatted_end_date,
            "is_lifetime": plan.is_lifetime
        }
    });
    
    info!("Sending successful response to client");
    HttpResponse::Ok().json(response)
}

/// Mock version of the license verification endpoint for testing
pub async fn verify_license_mock(
    license_data: web::Json<VerifyLicenseRequest>,
    state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("/license-public/verify-mock", &state.connections);
    info!("Starting mock license verification process");
    
    let data = license_data.into_inner();
    info!("License data: code={}, product_id={}, email={}", data.license_code, data.product_id, data.email);
    
    // Create a mock user or get existing user
    let user = match state.context.users.get_user_by_email(&data.email).await {
        Ok(user) => {
            info!("Found existing user with ID: {}", user.id);
            user
        },
        Err(_) => {
            // Create a new user
            info!("User not found, creating new user with email: {}", data.email);
            let user_id = Uuid::new_v4().to_string();
            
            match state.context.users.create_user_with_picture_and_phone(
                &user_id,
                &data.email,
                &data.name,
                None,
                data.phone.as_deref(),
            ).await {
                Ok(_) => {
                    info!("User created successfully with ID: {}", user_id);
                    match state.context.users.get_user_by_id(&user_id).await {
                        Ok(Some(user)) => {
                            info!("Retrieved newly created user");
                            user
                        },
                        _ => {
                            error!("Failed to retrieve newly created user");
                            return HttpResponse::InternalServerError().json(json!({ 
                                "success": false, 
                                "error": "Failed to create user" 
                            }));
                        }
                    }
                },
                Err(e) => {
                    error!("Error creating user: {}", e);
                    return HttpResponse::InternalServerError().json(json!({ 
                        "success": false, 
                        "error": format!("Failed to create user account: {}", e) 
                    }));
                }
            }
        }
    };
    
    // Use plan ID 5 (Special Plan) for testing
    let plan_id = 5;
    
    // Get plan details
    let plan = match state.context.premium_plans.get_premium_plan_by_id(plan_id).await {
        Ok(Some(plan)) => {
            info!("Found plan: {}", plan.name);
            plan
        },
        Ok(None) => {
            // Create a default plan object
            PremiumPlan {
                id: plan_id,
                name: "Special Plan".to_string(),
                description: "Default plan".to_string(),
                price: 1000.0,
                duration_days: 60,
                is_lifetime: false,
                features: "[]".to_string(),
                mayar_product_id: Some(data.product_id.clone()),
                mayar_link_payment: None,
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
            }
        }
        Err(e) => {
            error!("Error getting plan: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": format!("Error getting plan: {}", e) 
            }));
        }
    };
    
    // Create a subscription for the user
    let subscription_request = CreateUserSubscriptionRequest {
        user_id: user.id.clone(),
        plan_id,
    };
    
    let subscription_id = match state.context.user_subscriptions.create_subscription(&subscription_request).await {
        Ok(id) => {
            info!("Subscription created successfully with ID: {}", id);
            id
        },
        Err(e) => {
            error!("Error creating subscription: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": format!("Failed to create subscription: {}", e)
            }));
        }
    };
    
    // Get the subscription details
    let subscription = match state.context.user_subscriptions.get_user_subscription_by_id(subscription_id).await {
        Ok(Some(subscription)) => {
            info!("Retrieved subscription details successfully");
            subscription
        },
        Ok(None) => {
            error!("Subscription not found after creation, ID: {}", subscription_id);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": "Failed to get subscription details" 
            }));
        },
        Err(e) => {
            error!("Error getting subscription details: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": format!("Failed to get subscription details: {}", e) 
            }));
        }
    };
    
    // Generate a JWT token for the user
    let token = match generate_token(
        &user.id,
        &user.email,
        &user.display_name,
        state.config.get_jwt_secret(),
        &state.config.get_app_url()
    ) {
        Ok(t) => {
            info!("JWT token generated successfully");
            t
        },
        Err(e) => {
            error!("Failed to generate token: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": format!("Failed to generate token: {}", e) 
            }));
        }
    };
    
    // Format the end_date separately to avoid serialization issues
    let formatted_end_date = match subscription.end_date {
        Some(date) => {
            match serde_json::to_value(date) {
                Ok(date_value) => date_value,
                Err(e) => {
                    error!("Failed to serialize end_date: {}", e);
                    serde_json::Value::Null
                }
            }
        },
        None => serde_json::Value::Null
    };
    
    // Create the response JSON
    let response = json!({
        "success": true,
        "message": "License activated successfully (MOCK)",
        "token": token,
        "user": {
            "id": user.id,
            "email": user.email,
            "display_name": user.display_name
        },
        "subscription": {
            "id": subscription.id,
            "plan_id": subscription.plan_id,
            "plan_name": plan.name,
            "expired_at": formatted_end_date,
            "is_lifetime": plan.is_lifetime
        }
    });
    
    info!("Sending successful mock response to client");
    HttpResponse::Ok().json(response)
}

/// Get all licenses for the current user
pub async fn get_user_licenses(
    req: HttpRequest,
    state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("/license/user-licenses", &state.connections);
    
    // Get user_id from token
    let user_id = match crate::utils::auth::get_user_id_from_token(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(json!({ "error": "Unauthorized" })),
    };
    
    // Get all licenses for the user
    match state.context.license_codes.get_user_licenses(&user_id).await {
        Ok(licenses) => {
            let mut response_licenses = Vec::new();
            
            for license in licenses {
                // Get plan details
                let plan_name = match state.context.premium_plans.get_premium_plan_by_id(license.plan_id).await {
                    Ok(Some(plan)) => plan.name,
                    _ => "Unknown Plan".to_string(),
                };
                
                // Calculate days remaining
                let days_remaining = match license.expired_at {
                    Some(expired_at) => {
                        let now = Utc::now();
                        if expired_at > now {
                            Some((expired_at - now).num_days())
                        } else {
                            Some(0)
                        }
                    },
                    None => None, // Lifetime license
                };
                
                // Convert status to string
                let status = match license.status {
                    LicenseStatus::Active => "active",
                    LicenseStatus::Expired => "expired",
                    LicenseStatus::Cancelled => "cancelled",
                };
                
                response_licenses.push(LicenseCodeResponse {
                    id: license.id,
                    license_code: license.license_code,
                    user_id: license.user_id,
                    plan_id: license.plan_id,
                    plan_name,
                    status: status.to_string(),
                    expired_at: license.expired_at,
                    days_remaining,
                });
            }
            
            HttpResponse::Ok().json(response_licenses)
        },
        Err(e) => {
            error!("Error getting licenses: {}", e);
            HttpResponse::InternalServerError().json(json!({ 
                "error": format!("Failed to get licenses: {}", e) 
            }))
        }
    }
} 