use crate::controller::log_request;
use crate::model::payment_transaction::{PaymentTransaction, PaymentStatus, PaymentTransactionResponse, CreatePaymentRequest, MayarWebhookPayload};
use crate::model::user_subscription::CreateUserSubscriptionRequest;
use crate::service::payment_service::MayarPaymentService;
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use utoipa::ToSchema;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use actix_web::http::header::HeaderValue;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

// Response for phone number check before payment
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PhoneNumberCheckResponse {
    pub has_phone: bool,
    pub phone_number: Option<String>,
    pub user_id: String,
    pub can_proceed: bool,
}

// Helper function to extract user info from JWT token
fn extract_user_info_from_token(auth_header: Option<&HeaderValue>, user_id: &str) -> (String, String, String) {
    // Default values
    let mut email = format!("user_{}@example.com", user_id);
    let mut name = user_id.to_string();
    let mut phone = "083120827585".to_string(); // Default phone number
    
    if let Some(header) = auth_header {
        if let Ok(token_str) = header.to_str() {
            if token_str.starts_with("Bearer ") {
                let token = token_str.trim_start_matches("Bearer ").trim();
                let parts: Vec<&str> = token.split('.').collect();
                
                if parts.len() >= 2 {
                    // The JWT payload is in the second part (index 1)
                    let payload_part = parts[1];
                    
                    // Base64 decode the payload
                    if let Ok(decoded) = URL_SAFE_NO_PAD.decode(payload_part) {
                        if let Ok(payload) = String::from_utf8(decoded) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload) {
                                // Extract email
                                if let Some(e) = json.get("email").and_then(|e| e.as_str()) {
                                    email = e.to_string();
                                }
                                
                                // Extract name
                                if let Some(n) = json.get("name").and_then(|n| n.as_str()) {
                                    name = n.to_string();
                                }
                                
                                // Extract phone if available
                                if let Some(p) = json.get("phone").and_then(|p| p.as_str()) {
                                    phone = p.to_string();
                                }
                                
                                // Log the extracted info for debugging
                                println!("Extracted from JWT - Email: {}, Name: {}, Phone: {}", email, name, phone);
                            }
                        }
                    }
                }
            }
        }
    }
    
    (email, name, phone)
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/payment")
            .route("/transactions", web::get().to(get_user_payment_transactions))
            .route("/transactions/{id}", web::get().to(get_payment_transaction_by_id))
            .route("/create", web::post().to(create_payment))
            .route("/webhook", web::post().to(payment_webhook))
            .route("/webhook-test", web::post().to(test_webhook))
            .route("/check/{transaction_id}", web::get().to(check_payment_status))
            .route("/check-phone/{user_id}", web::get().to(check_phone_before_payment))
    );
}

// New endpoint to check if a user has a phone number before proceeding with payment
async fn check_phone_before_payment(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    let user_id = path.into_inner();
    log_request("/payment/check-phone/{user_id}", &data.connections);

    // Get user_id from token and verify it matches the request
    let token_user_id = match req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    if token_user_id != user_id {
        return HttpResponse::Forbidden().json(ErrorResponse {
            error: "User ID in token does not match request".to_string(),
        });
    }

    // Get user from database
    match data.context.users.get_user_by_id(&user_id).await {
        Ok(Some(user)) => {
            // Check if user has a phone number
            let has_phone = user.phone_number.is_some();
            let phone = user.phone_number.clone();
            
            // Return response with phone number status
            HttpResponse::Ok().json(PhoneNumberCheckResponse {
                has_phone,
                phone_number: phone,
                user_id: user.id,
                can_proceed: has_phone,
            })
        },
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("User with ID {} not found", user_id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get user: {}", e),
        }),
    }
}

async fn get_user_payment_transactions(
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("/payment/transactions", &data.connections);

    // Get user_id from token
    let user_id = match req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    match data.context.payment_transactions.get_user_payment_transactions(user_id).await {
        Ok(transactions) => HttpResponse::Ok().json(transactions),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get payment transactions: {}", e),
        }),
    }
}

async fn get_payment_transaction_by_id(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    let id = path.into_inner();
    log_request("/payment/transactions/{id}", &data.connections);

    // Get user_id from token
    let user_id = match req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    match data.context.payment_transactions.get_payment_transaction_by_id(id).await {
        Ok(Some(transaction)) => {
            // Verify that the transaction belongs to the user
            if transaction.user_id != user_id {
                return HttpResponse::Forbidden().json(ErrorResponse {
                    error: "You don't have permission to view this transaction".to_string(),
                });
            }
            
            HttpResponse::Ok().json(transaction)
        }
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Payment transaction with ID {} not found", id),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get payment transaction: {}", e),
        }),
    }
}

async fn create_payment(
    req: web::Json<CreatePaymentRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/payment/create", &data.connections);

    // Get user_id from token
    let token_user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => {
            // Try to extract user_id from JWT token
            if let Some(auth_header) = http_req.headers().get("authorization") {
                let auth_str = auth_header.to_str().unwrap_or_default();
                if auth_str.starts_with("Bearer ") {
                    let token = auth_str.trim_start_matches("Bearer ").trim();
                    // Extract user_id from token
                    if let Some(parts) = token.split('.').collect::<Vec<&str>>().get(1) {
                        if let Ok(decoded) = URL_SAFE_NO_PAD.decode(*parts) {
                            if let Ok(payload) = String::from_utf8(decoded) {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload) {
                                    if let Some(sub) = json.get("sub").and_then(|s| s.as_str()) {
                                        sub.to_string()
                                    } else {
                                        return HttpResponse::Unauthorized().json(ErrorResponse {
                                            error: "Unauthorized: Could not extract user_id from token".to_string(),
                                        });
                                    }
                                } else {
                                    return HttpResponse::Unauthorized().json(ErrorResponse {
                                        error: "Unauthorized: Invalid token payload".to_string(),
                                    });
                                }
                            } else {
                                return HttpResponse::Unauthorized().json(ErrorResponse {
                                    error: "Unauthorized: Invalid token encoding".to_string(),
                                });
                            }
                        } else {
                            return HttpResponse::Unauthorized().json(ErrorResponse {
                                error: "Unauthorized: Could not decode token".to_string(),
                            });
                        }
                    } else {
                        return HttpResponse::Unauthorized().json(ErrorResponse {
                            error: "Unauthorized: Invalid token format".to_string(),
                        });
                    }
                } else {
                    return HttpResponse::Unauthorized().json(ErrorResponse {
                        error: "Unauthorized: Invalid authorization header format".to_string(),
                    });
                }
            } else {
                return HttpResponse::Unauthorized().json(ErrorResponse {
                    error: "Unauthorized: Missing user_id header and authorization header".to_string(),
                });
            }
        }
    };

    if token_user_id != req.user_id {
        return HttpResponse::Forbidden().json(ErrorResponse {
            error: "User ID in token does not match request".to_string(),
        });
    }

    // Extract user info from JWT token
    let (user_email, user_name, mut user_phone) = extract_user_info_from_token(http_req.headers().get("authorization"), &req.user_id);

    // Try to get user's phone number from database if available
    match data.context.users.get_user_by_id(&req.user_id).await {
        Ok(Some(user)) => {
            // Use the user's email and name from the database
            let email = user.email;
            let name = user.display_name;
            
            // If user has a phone number in the database, use it
            if let Some(phone) = user.phone_number {
                user_phone = phone;
                println!("Using phone number from database: {}", user_phone);
            }
            
            // Update user info with values from database
            let (_, _, _) = (email, name, user_phone.clone());
        },
        _ => {
            println!("User not found in database or error occurred, using token info");
        }
    }

    // Get plan details
    let plan = match data.context.premium_plans.get_premium_plan_by_id(req.plan_id).await {
        Ok(Some(plan)) => plan,
        Ok(None) => return HttpResponse::NotFound().json(ErrorResponse {
            error: "Premium plan not found".to_string(),
        }),
        Err(e) => return HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get premium plan details: {}", e),
        }),
    };

    // Create payment service
    let payment_service = MayarPaymentService::new(
        data.config.get_mayar_api_key().to_string(),
        "https://api.mayar.id/hl/v1".to_string(),
    );

    // Create payment request
    let mayar_response = match payment_service.create_payment(
        &user_name,
        &user_email,
        plan.price,
        &user_phone,
        &req.redirect_url,
        &format!("Payment for {} plan", plan.name),
    ).await {
        Ok(response) => response,
        Err(e) => return HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to create payment: {}", e),
        }),
    };

    // Save both IDs from Mayar response
    // This is the key change - we need to store both the transaction_id and the id
    match data.context.payment_transactions.create_payment_transaction_with_mayar_id(
        &req.user_id,
        req.plan_id,
        plan.price,
        &mayar_response.data.transaction_id,
        &mayar_response.data.link,
        &mayar_response.data.id, // Store the Mayar ID as well
    ).await {
        Ok(id) => {
            HttpResponse::Created().json(serde_json::json!({
                "id": id,
                "transaction_id": mayar_response.data.transaction_id,
                "mayar_id": mayar_response.data.id, // Include the Mayar ID in the response
                "payment_link": mayar_response.data.link,
                "message": "Payment created successfully"
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to save payment transaction: {}", e),
        }),
    }
}

async fn payment_webhook(
    payload_json: web::Json<serde_json::Value>,
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("/payment/webhook", &data.connections);

    // Log the entire webhook payload for debugging
    println!("=============== WEBHOOK RECEIVED ===============");
    println!("Headers: {:?}", req.headers());
    println!("Raw payload: {}", serde_json::to_string_pretty(&payload_json).unwrap_or_default());
    println!("=================================================");

    // Try to parse as MayarWebhookPayload
    let payload_result = serde_json::from_value::<MayarWebhookPayload>(payload_json.clone());
    
    if let Err(e) = &payload_result {
        println!("Error parsing webhook payload: {}", e);
        println!("Will try to extract fields directly from JSON");
    }
    
    // Extract fields from the payload
    let event = payload_json.get("event")
        .and_then(|e| e.as_str())
        .unwrap_or("unknown");
    
    println!("Event: {}", event);
    
    // Accept testing events for development purposes
    if event == "testing" {
        println!("Received testing webhook event");
        return HttpResponse::Ok().json(serde_json::json!({
            "message": "Testing webhook received successfully"
        }));
    }
    
    // Verify webhook event
    if event != "payment.received" && event != "payment.reminder" {
        println!("Unsupported webhook event: {}", event);
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: format!("Unsupported webhook event: {}", event),
        });
    }
    
    // Extract transaction ID and status with more flexibility
    let transaction_id = payload_json.get("data")
        .and_then(|d| d.get("transactionId"))
        .and_then(|t| t.as_str())
        .or_else(|| {
            payload_json.get("data")
                .and_then(|d| d.get("transaction_id"))
                .and_then(|t| t.as_str())
        })
        .unwrap_or("unknown");
    
    let mayar_id = payload_json.get("data")
        .and_then(|d| d.get("id"))
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");
    
    println!("Transaction ID: {}", transaction_id);
    println!("Mayar ID: {}", mayar_id);
    
    // Verify webhook signature if provided by Mayar
    if let Some(signature) = req.headers().get("X-Mayar-Signature") {
        // Get webhook secret from config
        let webhook_secret = data.config.get_mayar_webhook_secret().to_string();
        
        // Skip verification if webhook secret is not configured
        if !webhook_secret.is_empty() {
            let signature_str = signature.to_str().unwrap_or_default();
            
            // Verify signature (implementation depends on Mayar's signature method)
            // This is a placeholder - implement according to Mayar's documentation
            println!("Webhook signature received: {}", signature_str);
            
            // For now, we'll just log it and proceed
        }
    } else {
        println!("No webhook signature found in request");
    }

    // Try to find the transaction in the database with more flexibility
    let mut transaction_result = data.context.payment_transactions.get_payment_transaction_by_transaction_id(transaction_id).await;
    
    // If not found by transaction_id, try by mayar_id
    if transaction_result.is_err() || (transaction_result.is_ok() && transaction_result.as_ref().unwrap().is_none()) {
        println!("Transaction not found by transaction_id, trying mayar_id");
        transaction_result = data.context.payment_transactions.get_payment_transaction_by_mayar_id(mayar_id).await;
    }
    
    if let Err(e) = &transaction_result {
        println!("Error retrieving transaction: {}", e);
        
        // Try to find any pending transactions for this user
        if let Some(customer_mobile) = payload_json.get("data")
            .and_then(|d| d.get("customerMobile"))
            .and_then(|m| m.as_str()) {
            
            println!("Trying to find user by phone number: {}", customer_mobile);
            
            // Format phone number (remove leading zeros or country code if needed)
            let formatted_phone = if customer_mobile.starts_with("0") {
                customer_mobile[1..].to_string()
            } else if customer_mobile.starts_with("+62") {
                customer_mobile[3..].to_string()
            } else if customer_mobile.starts_with("62") {
                customer_mobile[2..].to_string()
            } else {
                customer_mobile.to_string()
            };
            
            println!("Formatted phone number: {}", formatted_phone);
            
            // Try to find user by phone number
            match data.context.users.get_user_by_phone(&formatted_phone).await {
                Ok(Some(user)) => {
                    println!("Found user with ID: {} for phone: {}", user.id, formatted_phone);
                    
                    // Get pending transactions for this user
                    // Note: You'll need to add this method to your payment_transaction_dao.rs
                    // This is just a placeholder for the concept
                    println!("Looking for pending transactions for user: {}", user.id);
                    
                    // For now, we'll just acknowledge the webhook
                    return HttpResponse::Ok().json(serde_json::json!({
                        "message": "Webhook acknowledged, but transaction not found. User identified by phone.",
                        "user_id": user.id,
                        "phone": formatted_phone
                    }));
                },
                Ok(None) => {
                    println!("No user found with phone number: {}", formatted_phone);
                },
                Err(e) => {
                    println!("Error finding user by phone: {}", e);
                }
            }
        }
        
        return HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Transaction with ID {} not found and could not find user by phone", transaction_id),
        });
    }
    
    let transaction = match transaction_result {
        Ok(Some(transaction)) => transaction,
        Ok(None) => {
            println!("Transaction with ID {} not found", transaction_id);
            
            // Log all transaction IDs in the database for debugging
            match data.context.payment_transactions.get_all_transaction_ids().await {
                Ok(ids) => {
                    println!("Available transaction IDs in database:");
                    for id in ids {
                        println!("  - {}", id);
                    }
                },
                Err(e) => {
                    println!("Error retrieving transaction IDs: {}", e);
                }
            }
            
            return HttpResponse::NotFound().json(ErrorResponse {
                error: format!("Transaction with ID {} not found", transaction_id),
            });
        },
        Err(e) => {
            println!("Failed to get transaction: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to get transaction: {}", e),
            });
        },
    };
    // Determine payment status
    let status_str = payload_json.get("data")
        .and_then(|d| d.get("status"))
        .and_then(|s| s.as_str())
        .unwrap_or("PENDING");
        
    let status = match status_str.to_uppercase().as_str() {
        "SUCCESS" => PaymentStatus::Completed,
        "EXPIRED" => PaymentStatus::Expired,
        "FAILED" => PaymentStatus::Failed,
        _ => PaymentStatus::Pending,
    };

    println!("Updating payment status for transaction {} to {:?}", transaction_id, status);

    // Extract payment method if available
    let payment_method = payload_json.get("data")
        .and_then(|d| d.get("paymentMethod"))
        .and_then(|p| p.as_str());

    // Update payment status
    let webhook_data = serde_json::to_string(&payload_json).unwrap_or_default();
    match data.context.payment_transactions.update_payment_status(
        transaction_id,
        status.clone(),
        payment_method,
        Some(&webhook_data),
    ).await {
        Ok(_) => {
            // If payment is completed, create subscription
            if let PaymentStatus::Completed = status {
                println!("Payment completed, creating subscription for user {}", transaction.user_id);
                let subscription_request = CreateUserSubscriptionRequest {
                    user_id: transaction.user_id.clone(),
                    plan_id: transaction.plan_id,
                };
                
                match data.context.user_subscriptions.create_subscription(&subscription_request).await {
                    Ok(subscription_id) => {
                        println!("Subscription created successfully with ID: {}", subscription_id);
                    },
                    Err(e) => {
                        println!("Failed to create subscription: {}", e);
                        return HttpResponse::InternalServerError().json(ErrorResponse {
                            error: format!("Failed to create subscription: {}", e),
                        });
                    }
                }
            }
            
            println!("Webhook processed successfully");
            HttpResponse::Ok().json(serde_json::json!({
                "message": "Webhook processed successfully",
                "transaction_id": transaction_id,
                "status": format!("{:?}", status)
            }))
        }
        Err(e) => {
            println!("Failed to update payment status: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to update payment status: {}", e),
            })
        },
    }
}

async fn check_payment_status(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    let transaction_id = path.into_inner();
    log_request("/payment/check/{transaction_id}", &data.connections);

    // Get user_id from token
    let user_id = match req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    // Get transaction from database
    let transaction = match data.context.payment_transactions.get_payment_transaction_by_transaction_id(&transaction_id).await {
        Ok(Some(transaction)) => {
            // Verify that the transaction belongs to the user
            if transaction.user_id != user_id {
                return HttpResponse::Forbidden().json(ErrorResponse {
                    error: "You don't have permission to check this transaction".to_string(),
                });
            }
            
            transaction
        }
        Ok(None) => return HttpResponse::NotFound().json(ErrorResponse {
            error: format!("Transaction with ID {} not found", transaction_id),
        }),
        Err(e) => return HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to get transaction: {}", e),
        }),
    };

    // Create payment service
    let payment_service = MayarPaymentService::new(
        data.config.get_mayar_api_key().to_string(),
        "https://api.mayar.id/hl/v1".to_string(),
    );

    // Check payment status
    match payment_service.check_payment_status(&transaction_id).await {
        Ok(status) => {
            // Update payment status in database if it has changed
            if status != transaction.status {
                if let Err(e) = data.context.payment_transactions.update_payment_status(
                    &transaction_id,
                    status.clone(),
                    None,
                    None,
                ).await {
                    return HttpResponse::InternalServerError().json(ErrorResponse {
                        error: format!("Failed to update payment status: {}", e),
                    });
                }
                
                // If payment is completed, create subscription
                if let PaymentStatus::Completed = status {
                    let subscription_request = CreateUserSubscriptionRequest {
                        user_id: user_id.to_string(),
                        plan_id: transaction.plan_id,
                    };
                    
                    if let Err(e) = data.context.user_subscriptions.create_subscription(&subscription_request).await {
                        return HttpResponse::InternalServerError().json(ErrorResponse {
                            error: format!("Failed to create subscription: {}", e),
                        });
                    }
                }
            }
            
            HttpResponse::Ok().json(serde_json::json!({
                "transaction_id": transaction_id,
                "status": format!("{:?}", status),
                "message": "Payment status checked successfully"
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to check payment status: {}", e),
        }),
    }
}

// Flexible webhook handler for testing
async fn test_webhook(
    payload: web::Json<serde_json::Value>,
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("/payment/webhook-test", &data.connections);

    // Log the entire webhook payload for debugging
    println!("=============== TEST WEBHOOK RECEIVED ===============");
    println!("Headers: {:?}", req.headers());
    println!("Raw payload: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());
    println!("=================================================");

    // Extract event and transaction ID if available
    let event = payload.get("event").and_then(|e| e.as_str()).unwrap_or("unknown");
    let transaction_id = payload
        .get("data")
        .and_then(|d| d.get("transactionId"))
        .and_then(|t| t.as_str())
        .or_else(|| 
            payload
                .get("data")
                .and_then(|d| d.get("transaction_id"))
                .and_then(|t| t.as_str())
        )
        .unwrap_or("unknown");
    
    let status = payload
        .get("data")
        .and_then(|d| d.get("status"))
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");

    println!("Test webhook event: {}, transaction_id: {}, status: {}", event, transaction_id, status);

    // Always return success for test webhooks
    HttpResponse::Ok().json(serde_json::json!({
        "message": "Test webhook received successfully",
        "event": event,
        "transaction_id": transaction_id,
        "status": status
    }))
} 