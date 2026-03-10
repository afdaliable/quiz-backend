use actix_web::{dev::{Service, ServiceRequest, ServiceResponse, Transform}, Error, HttpMessage, http::{StatusCode, header}};
use futures::future::{LocalBoxFuture, Ready};
use std::task::{Context, Poll};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::OnceLock;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::AppState;

// Shared HTTP client for Authentik userinfo calls
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to build HTTP client")
    })
}

// Claims from Authentik userinfo endpoint
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthentikClaims {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub groups: Option<Vec<String>>,
}

// Minimal JWT claims struct for legacy HMAC token fallback
#[derive(Debug, Serialize, Deserialize)]
struct LegacyClaims {
    sub: String,
    exp: usize,
}

// Validates Bearer token against Authentik's userinfo endpoint.
// Returns claims if token is valid, None otherwise.
async fn validate_authentik_token(token: &str) -> Option<AuthentikClaims> {
    // Userinfo URL is separate from issuer — Authentik uses a common userinfo endpoint
    let userinfo_url = std::env::var("AUTHENTIK_USERINFO_URL")
        .unwrap_or_else(|_| "https://auth.canducation.com/application/o/userinfo/".to_string());

    let response = http_client()
        .get(&userinfo_url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .ok()?;

    if response.status().is_success() {
        response.json::<AuthentikClaims>().await.ok()
    } else {
        None
    }
}

pub struct AdminMiddleware;

impl AdminMiddleware {
    pub fn new() -> Self {
        AdminMiddleware
    }
}

impl<S, B> Transform<S, ServiceRequest> for AdminMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AdminMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        futures::future::ready(Ok(AdminMiddlewareService {
            service: Rc::new(RefCell::new(service)),
        }))
    }
}

pub struct AdminMiddlewareService<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> Service<ServiceRequest> for AdminMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.borrow_mut().poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        println!("Admin middleware: checking access for {}", req.path());

        let service = self.service.clone();

        Box::pin(async move {
            // Extract Bearer token from Authorization header
            let token = req
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
                .map(|s| s.to_string());

            let token = match token {
                Some(t) => t,
                None => {
                    return Err(actix_web::error::InternalError::new(
                        json!({"error": "unauthenticated", "message": "Authorization header required for admin access.", "status_code": 401}),
                        StatusCode::UNAUTHORIZED,
                    ).into())
                }
            };

            // ── 1. Try Authentik token validation ────────────────────────────
            if let Some(claims) = validate_authentik_token(&token).await {
                let admin_group = std::env::var("AUTHENTIK_ADMIN_GROUP")
                    .unwrap_or_else(|_| "quiz-admins".to_string());

                let is_admin = claims
                    .groups
                    .as_ref()
                    .map(|groups| groups.iter().any(|g| g == &admin_group))
                    .unwrap_or(false);

                if !is_admin {
                    println!(
                        "Admin middleware: Authentik user {} lacks group '{}'",
                        claims.email.as_deref().unwrap_or(&claims.sub),
                        admin_group
                    );
                    return Err(actix_web::error::InternalError::new(
                        json!({"error": "insufficient_permissions", "message": "Admin group membership required.", "status_code": 403}),
                        StatusCode::FORBIDDEN,
                    ).into());
                }

                println!(
                    "Admin middleware: Authentik admin access granted for {}",
                    claims.email.as_deref().unwrap_or(&claims.sub)
                );

                let user_id = claims.sub.clone();
                req.extensions_mut().insert(claims);

                let mut req = req;
                if let Ok(v) = header::HeaderValue::from_str(&user_id) {
                    req.headers_mut().insert(
                        header::HeaderName::from_static("user_id"),
                        v,
                    );
                }
                return service.borrow_mut().call(req).await;
            }

            // ── 2. Fallback: legacy HMAC JWT + DB role check ──────────────────
            if let Some(app_data) = req.app_data::<actix_web::web::Data<AppState>>() {
                let jwt_secret = app_data.config.get_jwt_secret().to_string();

                let mut validation = Validation::new(Algorithm::HS256);
                validation.validate_exp = true;
                validation.validate_aud = false;
                validation.required_spec_claims.remove("iss");

                if let Ok(token_data) = decode::<LegacyClaims>(
                    &token,
                    &DecodingKey::from_secret(jwt_secret.as_bytes()),
                    &validation,
                ) {
                    let user_id = token_data.claims.sub.clone();

                    match check_admin_role(app_data, &user_id).await {
                        Ok(true) => {
                            println!("Admin middleware: legacy admin access granted for {}", user_id);
                            let mut req = req;
                            if let Ok(v) = header::HeaderValue::from_str(&user_id) {
                                req.headers_mut().insert(
                                    header::HeaderName::from_static("user_id"),
                                    v,
                                );
                            }
                            return service.borrow_mut().call(req).await;
                        }
                        Ok(false) => {
                            return Err(actix_web::error::InternalError::new(
                                json!({"error": "insufficient_permissions", "message": "You don't have admin permissions.", "status_code": 403}),
                                StatusCode::FORBIDDEN,
                            ).into());
                        }
                        Err(e) => {
                            println!("Admin middleware: DB error checking role: {:?}", e);
                        }
                    }
                }
            }

            // ── 3. All validation paths failed ────────────────────────────────
            Err(actix_web::error::InternalError::new(
                json!({"error": "invalid_token", "message": "Invalid or expired token.", "status_code": 401}),
                StatusCode::UNAUTHORIZED,
            ).into())
        })
    }
}

async fn check_admin_role(
    app_state: &actix_web::web::Data<AppState<'_>>,
    user_id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query_scalar::<_, Option<String>>(
        "SELECT role FROM dbquizapp.users WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(user_id)
    .fetch_optional(&*app_state.context.users.pool)
    .await?;

    match result {
        Some(Some(role)) => Ok(role == "admin" || role == "superadmin"),
        _ => Ok(false),
    }
}
