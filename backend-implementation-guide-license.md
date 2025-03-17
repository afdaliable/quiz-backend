# Backend Implementation Guide for License Controller

This guide explains the implementation details of the license controller for the Mayar payment integration.

## Overview

The license controller handles the following functionality:
1. Generating payment links for premium plans
2. Verifying license codes from Mayar
3. Creating user subscriptions based on verified licenses
4. Managing user licenses

## Database Schema

The license system uses the following tables:

### 1. `licenses` Table

```sql
CREATE TABLE licenses (
    id INT AUTO_INCREMENT PRIMARY KEY,
    license_code VARCHAR(255) NOT NULL,
    user_id VARCHAR(255) NOT NULL,
    plan_id INT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'Active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expired_at TIMESTAMP NULL,
    is_lifetime BOOLEAN DEFAULT FALSE,
    product_id VARCHAR(255) NOT NULL,
    UNIQUE KEY (license_code),
    INDEX (user_id),
    INDEX (plan_id)
);
```

### 2. `user_subscriptions` Table (Existing)

```sql
CREATE TABLE user_subscriptions (
    id INT AUTO_INCREMENT PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    plan_id INT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'Active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expired_at TIMESTAMP NULL,
    is_lifetime BOOLEAN DEFAULT FALSE,
    INDEX (user_id),
    INDEX (plan_id)
);
```

## Implementation Details

### 1. License Controller

The license controller (`src/controller/license_controller.rs`) implements the following endpoints:

#### a. Generate Payment Link

```rust
pub async fn generate_payment_link(
    req: HttpRequest,
    path: web::Path<i32>,
    state: web::Data<AppState>,
) -> impl Responder {
    let plan_id = path.into_inner();
    let user_id = match get_user_id_from_token(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(json!({ "error": "Unauthorized" })),
    };

    // Get user details
    let user = match user_dao::get_user_by_id(&state.db_context, &user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().json(json!({ "error": "User not found" })),
        Err(e) => {
            log::error!("Error getting user: {}", e);
            return HttpResponse::InternalServerError().json(json!({ "error": "Internal server error" }));
        }
    };

    // Get plan details
    let plan = match premium_plan_dao::get_premium_plan_by_id(&state.db_context, plan_id).await {
        Ok(Some(plan)) => plan,
        Ok(None) => return HttpResponse::NotFound().json(json!({ "error": "Plan not found" })),
        Err(e) => {
            log::error!("Error getting plan: {}", e);
            return HttpResponse::InternalServerError().json(json!({ "error": "Internal server error" }));
        }
    };

    // Generate payment link
    let config = state.db_context.get_config();
    let mayar_base_url = config.payment.get_mayar_base_url();
    let product_id = match plan_id {
        1 => config.payment.get_mayar_product_id_basic(),
        2 => config.payment.get_mayar_product_id_premium(),
        3 => config.payment.get_mayar_product_id_lifetime(),
        _ => return HttpResponse::BadRequest().json(json!({ "error": "Invalid plan ID" })),
    };

    // Encode user details for URL
    let email = urlencoding::encode(&user.email);
    let name = urlencoding::encode(&user.display_name);
    let phone = urlencoding::encode(&user.phone.unwrap_or_default());

    // Construct payment URL
    let payment_link = format!(
        "{}/m/special-plan?email={}&productId={}&name={}&phone={}&user_id={}",
        mayar_base_url, email, product_id, name, phone, user_id
    );

    HttpResponse::Ok().json(json!({ "payment_link": payment_link }))
}
```

#### b. Verify License

```rust
pub async fn verify_license(
    license_data: web::Json<LicenseVerificationRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let data = license_data.into_inner();
    
    // Validate license code format
    if data.license_code.len() < 8 || data.license_code.len() > 64 {
        return HttpResponse::BadRequest().json(json!({ 
            "success": false, 
            "error": "Invalid license code format" 
        }));
    }

    // Check if license already exists
    match license_dao::get_license_by_code(&state.db_context, &data.license_code).await {
        Ok(Some(_)) => {
            return HttpResponse::BadRequest().json(json!({ 
                "success": false, 
                "error": "License code already used" 
            }));
        }
        Err(e) => {
            log::error!("Error checking license: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": "Internal server error" 
            }));
        }
        _ => {}
    }

    // Get plan ID from product ID
    let config = state.db_context.get_config();
    let plan_id = if data.product_id == config.payment.get_mayar_product_id_basic() {
        1
    } else if data.product_id == config.payment.get_mayar_product_id_premium() {
        2
    } else if data.product_id == config.payment.get_mayar_product_id_lifetime() {
        3
    } else {
        return HttpResponse::BadRequest().json(json!({ 
            "success": false, 
            "error": "Invalid product ID" 
        }));
    };

    // Get or create user
    let user = match user_dao::get_user_by_email(&state.db_context, &data.email).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            // Create new user
            let new_user = User {
                id: Uuid::new_v4().to_string(),
                email: data.email.clone(),
                display_name: data.name.clone(),
                phone: Some(data.phone.clone()),
                password_hash: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                is_active: true,
                is_verified: true,
            };

            match user_dao::create_user(&state.db_context, &new_user).await {
                Ok(_) => new_user,
                Err(e) => {
                    log::error!("Error creating user: {}", e);
                    return HttpResponse::InternalServerError().json(json!({ 
                        "success": false, 
                        "error": "Failed to create user account" 
                    }));
                }
            }
        }
        Err(e) => {
            log::error!("Error getting user: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": "Internal server error" 
            }));
        }
    };

    // Get plan details
    let plan = match premium_plan_dao::get_premium_plan_by_id(&state.db_context, plan_id).await {
        Ok(Some(plan)) => plan,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({ 
                "success": false, 
                "error": "Plan not found" 
            }));
        }
        Err(e) => {
            log::error!("Error getting plan: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": "Internal server error" 
            }));
        }
    };

    // Calculate expiration date
    let is_lifetime = plan_id == 3;
    let expired_at = if is_lifetime {
        None
    } else {
        Some(chrono::Utc::now() + chrono::Duration::days(plan.duration_days as i64))
    };

    // Create license
    let license = License {
        id: 0, // Auto-incremented by DB
        license_code: data.license_code.clone(),
        user_id: user.id.clone(),
        plan_id,
        status: "Active".to_string(),
        created_at: chrono::Utc::now(),
        expired_at,
        is_lifetime,
        product_id: data.product_id.clone(),
    };

    match license_dao::create_license(&state.db_context, &license).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("Error creating license: {}", e);
            return HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": "Failed to create license" 
            }));
        }
    }

    // Create subscription
    let subscription = UserSubscription {
        id: 0, // Auto-incremented by DB
        user_id: user.id.clone(),
        plan_id,
        status: "Active".to_string(),
        created_at: chrono::Utc::now(),
        expired_at,
        is_lifetime,
    };

    match user_subscription_dao::create_user_subscription(&state.db_context, &subscription).await {
        Ok(subscription_id) => {
            // Generate JWT token
            let token = match generate_token(&user.id, &user.email) {
                Ok(token) => token,
                Err(e) => {
                    log::error!("Error generating token: {}", e);
                    return HttpResponse::InternalServerError().json(json!({ 
                        "success": false, 
                        "error": "Failed to generate authentication token" 
                    }));
                }
            };

            // Return success response with token and user info
            HttpResponse::Ok().json(json!({
                "success": true,
                "message": "License activated successfully",
                "token": token,
                "user": {
                    "id": user.id,
                    "email": user.email,
                    "display_name": user.display_name
                },
                "subscription": {
                    "id": subscription_id,
                    "plan_id": plan_id,
                    "plan_name": plan.name,
                    "expired_at": expired_at,
                    "is_lifetime": is_lifetime
                }
            }))
        }
        Err(e) => {
            log::error!("Error creating subscription: {}", e);
            HttpResponse::InternalServerError().json(json!({ 
                "success": false, 
                "error": "Failed to create subscription" 
            }))
        }
    }
}
```

#### c. Get User Licenses

```rust
pub async fn get_user_licenses(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    let user_id = match get_user_id_from_token(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(json!({ "error": "Unauthorized" })),
    };

    match license_dao::get_licenses_by_user_id(&state.db_context, &user_id).await {
        Ok(licenses) => {
            let now = chrono::Utc::now();
            let licenses_with_days = licenses.into_iter().map(|license| {
                let days_remaining = match license.expired_at {
                    Some(expired_at) => {
                        let duration = expired_at.signed_duration_since(now);
                        duration.num_days().max(0) as i32
                    }
                    None => -1, // Lifetime license
                };

                json!({
                    "id": license.id,
                    "license_code": license.license_code,
                    "user_id": license.user_id,
                    "plan_id": license.plan_id,
                    "status": license.status,
                    "expired_at": license.expired_at,
                    "days_remaining": days_remaining,
                    "is_lifetime": license.is_lifetime,
                    "product_id": license.product_id
                })
            }).collect::<Vec<_>>();

            HttpResponse::Ok().json(licenses_with_days)
        }
        Err(e) => {
            log::error!("Error getting licenses: {}", e);
            HttpResponse::InternalServerError().json(json!({ "error": "Internal server error" }))
        }
    }
}
```

### 2. License DAO

The license DAO (`src/dao/license_dao.rs`) implements the following functions:

```rust
use sqlx::MySqlPool;
use crate::model::payment_transaction::License;

pub async fn create_license(db_context: &MySqlPool, license: &License) -> Result<i32, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        INSERT INTO licenses (
            license_code, user_id, plan_id, status, expired_at, is_lifetime, product_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
        license.license_code,
        license.user_id,
        license.plan_id,
        license.status,
        license.expired_at,
        license.is_lifetime,
        license.product_id
    )
    .execute(db_context)
    .await?;

    Ok(result.last_insert_id() as i32)
}

pub async fn get_license_by_code(db_context: &MySqlPool, license_code: &str) -> Result<Option<License>, sqlx::Error> {
    sqlx::query_as!(
        License,
        r#"
        SELECT id, license_code, user_id, plan_id, status, created_at, expired_at, is_lifetime, product_id
        FROM licenses
        WHERE license_code = ?
        "#,
        license_code
    )
    .fetch_optional(db_context)
    .await
}

pub async fn get_licenses_by_user_id(db_context: &MySqlPool, user_id: &str) -> Result<Vec<License>, sqlx::Error> {
    sqlx::query_as!(
        License,
        r#"
        SELECT id, license_code, user_id, plan_id, status, created_at, expired_at, is_lifetime, product_id
        FROM licenses
        WHERE user_id = ?
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(db_context)
    .await
}

pub async fn update_license_status(db_context: &MySqlPool, license_id: i32, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE licenses
        SET status = ?
        WHERE id = ?
        "#,
        status,
        license_id
    )
    .execute(db_context)
    .await?;

    Ok(())
}
```

### 3. License Model

The license model (`src/model/payment_transaction.rs`) includes the following structs:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct License {
    pub id: i32,
    pub license_code: String,
    pub user_id: String,
    pub plan_id: i32,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expired_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_lifetime: bool,
    pub product_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LicenseVerificationRequest {
    pub license_code: String,
    pub product_id: String,
    pub email: String,
    pub name: String,
    pub phone: String,
}
```

### 4. Configuration

The configuration (`src/config.rs`) includes the following additions:

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct PaymentConfig {
    pub mayar_base_url: String,
    pub mayar_product_id_basic: String,
    pub mayar_product_id_premium: String,
    pub mayar_product_id_lifetime: String,
    pub mayar_webhook_url: String,
    pub mayar_webhook_secret: String,
}

impl PaymentConfig {
    pub fn get_mayar_base_url(&self) -> &str {
        &self.mayar_base_url
    }

    pub fn get_mayar_product_id_basic(&self) -> &str {
        &self.mayar_product_id_basic
    }

    pub fn get_mayar_product_id_premium(&self) -> &str {
        &self.mayar_product_id_premium
    }

    pub fn get_mayar_product_id_lifetime(&self) -> &str {
        &self.mayar_product_id_lifetime
    }

    pub fn get_mayar_webhook_url(&self) -> &str {
        &self.mayar_webhook_url
    }

    pub fn get_mayar_webhook_secret(&self) -> &str {
        &self.mayar_webhook_secret
    }
}
```

### 5. Route Configuration

The route configuration (`src/controller/mod.rs`) includes the following additions:

```rust
pub fn init_license_controller(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/license")
            .route("/payment-link/{plan_id}", web::get().to(license_controller::generate_payment_link))
            .route("/verify", web::post().to(license_controller::verify_license))
            .route("/user-licenses", web::get().to(license_controller::get_user_licenses))
    );
}
```

## Testing

### 1. Unit Tests

Create unit tests for the license controller in `src/controller/license_controller_test.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::payment_transaction::LicenseVerificationRequest;
    use actix_web::{http::header::HeaderValue, test, web, App};
    use sqlx::MySqlPool;
    use std::sync::Arc;

    #[actix_rt::test]
    async fn test_verify_license() {
        // Setup test database and app state
        let db_context = MySqlPool::connect("mysql://test:test@localhost/test_db").await.unwrap();
        let app_state = web::Data::new(AppState {
            db_context: db_context.clone(),
            // ... other state fields
        });

        // Create test app
        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .route("/license/verify", web::post().to(verify_license)),
        )
        .await;

        // Create test request
        let req = test::TestRequest::post()
            .uri("/license/verify")
            .set_json(&LicenseVerificationRequest {
                license_code: "TEST123456".to_string(),
                product_id: "test-product-id".to_string(),
                email: "test@example.com".to_string(),
                name: "Test User".to_string(),
                phone: "1234567890".to_string(),
            })
            .to_request();

        // Send request and check response
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // Clean up test data
        sqlx::query!("DELETE FROM licenses WHERE license_code = ?", "TEST123456")
            .execute(&db_context)
            .await
            .unwrap();
    }
}
```

### 2. Integration Tests

Create integration tests in `tests/license_api_test.rs`:

```rust
#[cfg(test)]
mod tests {
    use actix_web::{test, web, App};
    use quiz_backend::controller;
    use quiz_backend::model::payment_transaction::LicenseVerificationRequest;
    use serde_json::json;
    use sqlx::MySqlPool;

    #[actix_rt::test]
    async fn test_license_flow() {
        // Setup test database and app
        let db_context = MySqlPool::connect("mysql://test:test@localhost/test_db").await.unwrap();
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    db_context: db_context.clone(),
                    // ... other state fields
                }))
                .configure(controller::init_license_controller),
        )
        .await;

        // Test license verification
        let verify_req = test::TestRequest::post()
            .uri("/license/verify")
            .set_json(&LicenseVerificationRequest {
                license_code: "INTEGRATION_TEST_123".to_string(),
                product_id: "test-product-id".to_string(),
                email: "integration@example.com".to_string(),
                name: "Integration Test".to_string(),
                phone: "9876543210".to_string(),
            })
            .to_request();

        let verify_resp = test::call_service(&app, verify_req).await;
        assert!(verify_resp.status().is_success());

        let verify_body: serde_json::Value = test::read_body_json(verify_resp).await;
        assert_eq!(verify_body["success"], json!(true));
        
        let token = verify_body["token"].as_str().unwrap();

        // Test get user licenses
        let licenses_req = test::TestRequest::get()
            .uri("/license/user-licenses")
            .header("Authorization", format!("Bearer {}", token))
            .to_request();

        let licenses_resp = test::call_service(&app, licenses_req).await;
        assert!(licenses_resp.status().is_success());

        let licenses_body: Vec<serde_json::Value> = test::read_body_json(licenses_resp).await;
        assert!(!licenses_body.is_empty());
        assert_eq!(licenses_body[0]["license_code"], json!("INTEGRATION_TEST_123"));

        // Clean up test data
        sqlx::query!("DELETE FROM licenses WHERE license_code = ?", "INTEGRATION_TEST_123")
            .execute(&db_context)
            .await
            .unwrap();
    }
}
```

## Deployment Considerations

1. **Database Migration**: Create a migration script to add the `licenses` table to the database.

2. **Configuration**: Update the configuration file (`config.json`) to include the Mayar webhook secret:

```json
{
  "payment": {
    "mayar_base_url": "https://canducation.myr.id",
    "mayar_product_id_basic": "product-id-for-basic-plan",
    "mayar_product_id_premium": "product-id-for-premium-plan",
    "mayar_product_id_lifetime": "product-id-for-lifetime-plan",
    "mayar_webhook_url": "https://your-api.com/payment/webhook",
    "mayar_webhook_secret": "your-webhook-secret"
  }
}
```

3. **Webhook Security**: Ensure that the webhook endpoint validates the signature from Mayar using the webhook secret.

4. **Error Handling**: Implement comprehensive error handling and logging for all license-related operations.

5. **Monitoring**: Set up monitoring for license activations and webhook calls to detect any issues.

## Conclusion

This guide provides a comprehensive overview of the license controller implementation for the Mayar payment integration. By following these steps, you can ensure a secure and reliable license verification system for your application. 