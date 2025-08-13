use actix_web::{dev::{Service, ServiceRequest, ServiceResponse, Transform}, Error, HttpMessage, http::StatusCode, FromRequest, HttpRequest};
use actix_web::http::header;
use futures::future::{LocalBoxFuture, Ready};
use std::task::{Context, Poll};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::pin::Pin;

// Supabase JWT Claims
#[derive(Debug, Serialize, Deserialize)]
struct SupabaseClaims {
    // Required fields from Supabase
    aud: String,
    exp: usize,
    sub: String,
    email: String,
    role: String,
    // Optional fields
    iat: Option<usize>,
    app_metadata: Option<AppMetadata>,
    user_metadata: Option<UserMetadata>,
}

// Google OAuth JWT Claims
#[derive(Debug, Serialize, Deserialize)]
struct GoogleClaims {
    sub: String,     // Subject (user ID)
    exp: usize,      // Expiration time
    iat: usize,      // Issued at
    aud: String,     // Audience
    iss: String,     // Issuer
    email: String,   // User email
    name: String,    // User name
}

#[derive(Debug, Serialize, Deserialize)]
struct AppMetadata {
    provider: String,
    providers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserMetadata {
    email: String,
    email_verified: bool,
    phone_verified: bool,
    sub: String,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub email: Option<String>,
}

impl FromRequest for AuthenticatedUser {
    type Error = Error;
    type Future = Pin<Box<dyn futures::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let user_id = req.headers()
            .get("user_id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let email = if let Some(google_claims) = req.extensions().get::<GoogleClaims>() {
            Some(google_claims.email.clone())
        } else if let Some(supabase_claims) = req.extensions().get::<SupabaseClaims>() {
            Some(supabase_claims.email.clone())
        } else {
            None
        };

        Box::pin(async move {
            match user_id {
                Some(id) => Ok(AuthenticatedUser { 
                    user_id: id, 
                    email 
                }),
                None => Err(actix_web::error::ErrorUnauthorized("Missing user authentication")),
            }
        })
    }
}

pub struct AuthMiddleware {
    jwt_secret: String,
}

impl AuthMiddleware {
    pub fn new(jwt_secret: String) -> Self {
        AuthMiddleware { jwt_secret }
    }
}

// Public routes that don't need authentication
const PUBLIC_ROUTES: [&str; 10] = [
    "/signup",
    "/auth/v1/token",
    "/swagger-ui",
    "/api-docs/openapi.json",
    "/auth/google/callback",
    "/user/check-phone",
    "/user/update-phone",
    "/api/user/update-phone",
    "/payment/webhook",
    "/license-public"
];

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        futures::future::ready(Ok(AuthMiddlewareService {
            service,
            jwt_secret: self.jwt_secret.clone(),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: S,
    jwt_secret: String,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Log the path for debugging
        println!("Request path: {}", req.path());
        
        // Check if the route is public
        let path = req.path();
        let is_public = PUBLIC_ROUTES.iter().any(|route| {
            path == *route || path.starts_with(route) && (path.len() == route.len() || path.chars().nth(route.len()) == Some('/'))
        });

        if is_public {
            return Box::pin(self.service.call(req));
        }

        let auth_header = req.headers().get(header::AUTHORIZATION);
        let jwt_secret = self.jwt_secret.clone();
        
        match auth_header {
            Some(auth_str) => {
                if let Ok(auth_str) = auth_str.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = &auth_str[7..];
                        
                        // Try to decode as a Google OAuth token first
                        let mut google_validation = Validation::new(Algorithm::HS256);
                        google_validation.validate_exp = true;
                        // Don't validate aud and iss for Google OAuth tokens
                        google_validation.validate_aud = false;
                        // Don't validate issuer for Google OAuth tokens
                        google_validation.required_spec_claims.remove("iss");
                        
                        match decode::<GoogleClaims>(
                            token,
                            &DecodingKey::from_secret(jwt_secret.as_bytes()),
                            &google_validation
                        ) {
                            Ok(token_data) => {
                                // Successfully decoded as Google OAuth token
                                // Extract user ID before moving the claims
                                let user_id = token_data.claims.sub.clone();
                                
                                // Insert claims into request extensions
                                req.extensions_mut().insert(token_data.claims);
                                
                                // Add user_id header for backward compatibility
                                let mut req_mut = req;
                                if let Ok(user_id_value) = header::HeaderValue::from_str(&user_id) {
                                    req_mut.headers_mut().insert(
                                        header::HeaderName::from_static("user_id"),
                                        user_id_value
                                    );
                                }
                                
                                return Box::pin(self.service.call(req_mut));
                            },
                            Err(e) => {
                                // Check if token is expired
                                if let jsonwebtoken::errors::ErrorKind::ExpiredSignature = e.kind() {
                                    return Box::pin(async move {
                                        let error_response = json!({
                                            "error": "session_expired",
                                            "message": "Your session has expired. Please log in again.",
                                            "status_code": 401
                                        });
                                        
                                        Err(actix_web::error::InternalError::new(
                                            error_response,
                                            StatusCode::UNAUTHORIZED,
                                        ).into())
                                    });
                                }
                                
                                // Try to decode as a Supabase token
                                let mut supabase_validation = Validation::new(Algorithm::HS256);
                                supabase_validation.validate_exp = true;
                                supabase_validation.set_audience(&["authenticated"]);
                                // Don't require iss since Supabase doesn't always include it
                                supabase_validation.required_spec_claims.remove("iss");
                                supabase_validation.required_spec_claims.remove("sub");
                                
                                match decode::<SupabaseClaims>(
                                    token,
                                    &DecodingKey::from_secret(jwt_secret.as_bytes()),
                                    &supabase_validation
                                ) {
                                    Ok(token_data) => {
                                        // Only allow authenticated users
                                        if token_data.claims.role != "authenticated" {
                                            return Box::pin(async move {
                                                let error_response = json!({
                                                    "error": "invalid_role",
                                                    "message": "Invalid role. Please log in with appropriate permissions.",
                                                    "status_code": 401
                                                });
                                                
                                                Err(actix_web::error::InternalError::new(
                                                    error_response,
                                                    StatusCode::UNAUTHORIZED,
                                                ).into())
                                            });
                                        }
                                        
                                        // Extract user ID before moving the claims
                                        let user_id = token_data.claims.sub.clone();
                                        
                                        // Insert claims into request extensions
                                        req.extensions_mut().insert(token_data.claims);
                                        
                                        // Add user_id header for backward compatibility
                                        let mut req_mut = req;
                                        if let Ok(user_id_value) = header::HeaderValue::from_str(&user_id) {
                                            req_mut.headers_mut().insert(
                                                header::HeaderName::from_static("user_id"),
                                                user_id_value
                                            );
                                        }
                                        
                                        return Box::pin(self.service.call(req_mut));
                                    },
                                    Err(e) => {
                                        eprintln!("Token validation error: {:?}", e);
                                        
                                        // Check if token is expired
                                        if let jsonwebtoken::errors::ErrorKind::ExpiredSignature = e.kind() {
                                            return Box::pin(async move {
                                                let error_response = json!({
                                                    "error": "session_expired",
                                                    "message": "Your session has expired. Please log in again.",
                                                    "status_code": 401
                                                });
                                                
                                                Err(actix_web::error::InternalError::new(
                                                    error_response,
                                                    StatusCode::UNAUTHORIZED,
                                                ).into())
                                            });
                                        }
                                        
                                        return Box::pin(async move {
                                            let error_response = json!({
                                                "error": "invalid_token",
                                                "message": "Invalid token. Please log in again.",
                                                "status_code": 401
                                            });
                                            
                                            Err(actix_web::error::InternalError::new(
                                                error_response,
                                                StatusCode::UNAUTHORIZED,
                                            ).into())
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                Box::pin(async move {
                    let error_response = json!({
                        "error": "invalid_auth_header",
                        "message": "Invalid authorization header. Please log in again.",
                        "status_code": 401
                    });
                    
                    Err(actix_web::error::InternalError::new(
                        error_response,
                        StatusCode::UNAUTHORIZED,
                    ).into())
                })
            }
            None => Box::pin(async move {
                let error_response = json!({
                    "error": "missing_auth_header",
                    "message": "Missing authorization header. Please log in to access this resource.",
                    "status_code": 401
                });
                
                Err(actix_web::error::InternalError::new(
                    error_response,
                    StatusCode::UNAUTHORIZED,
                ).into())
            })
        }
    }
}