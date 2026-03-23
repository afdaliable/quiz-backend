use actix_web::{post, web, HttpResponse, Responder, HttpRequest, get};
use serde_json::json;
use crate::AppState;
use crate::model::{LoginRequest, AuthResponse, SupabaseUser};
use crate::model::session::SessionResponse;
use crate::utils::google_oauth::GoogleOAuthClient;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};
use chrono::{Utc, Duration};
use oauth2::TokenResponse;

/// Derive a candidate username from a display name (slugify)
fn slugify_display_name(name: &str) -> String {
    let base: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    let trimmed = base.trim_matches('_');
    if trimmed.len() > 25 {
        trimmed[..25].to_string()
    } else if trimmed.is_empty() {
        "user".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Try to assign a unique username derived from the display_name.
/// Silently skips if all attempts fail (username stays NULL).
async fn assign_auto_username(
    users_dao: &crate::dao::Table<'_, crate::model::User>,
    user_id: &str,
    display_name: &str,
) {
    let base = slugify_display_name(display_name);
    // Try base, then base_1 … base_999
    let candidates = std::iter::once(base.clone())
        .chain((1u32..=999).map(|n| format!("{}_{}", base, n)));
    for candidate in candidates {
        match users_dao.username_exists(&candidate).await {
            Ok(false) => {
                let _ = users_dao.set_username_if_null(user_id, &candidate).await;
                return;
            }
            _ => continue,
        }
    }
}

/// Request payload for Google OAuth callback
#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleAuthRequest {
    pub code: String,
}

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,     // Subject (user ID)
    exp: usize,      // Expiration time
    iat: usize,      // Issued at
    aud: String,     // Audience
    iss: String,     // Issuer
    email: String,   // User email
    name: String,    // User name
}

/// Request payload for session validation
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateSessionRequest {
    pub token: String,
}

/// Response for session validation
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateSessionResponse {
    pub valid: bool,
    pub message: String,
    pub user_id: Option<String>,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(admin_login)
       .service(google_callback)
       .service(logout)
       .service(get_sessions)
       .service(validate_session);
}

/// Simple admin login (bypasses Supabase, requires admin role in database)
#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Admin login successful", body = AuthResponse),
        (status = 400, description = "Invalid credentials"),
        (status = 401, description = "Unauthorized - not an admin"),
        (status = 500, description = "Internal server error")
    ),
    tag = "auth",
    security() // Empty security means no authentication required
)]
#[post("/auth/login")]
async fn admin_login(
    login_req: web::Json<LoginRequest>,
    app_state: web::Data<AppState<'_>>,
    http_request: HttpRequest,
) -> impl Responder {
    // For testing purposes, let's create a simple local authentication
    // This bypasses Supabase and works directly with the local database
    
    // Check if user exists in our database and has admin role
    match app_state.context.users.get_user_by_email(&login_req.email).await {
        Ok(user) => {
            // Check if user has admin role
            let role = user.role.as_deref().unwrap_or("user");
            if role != "admin" && role != "superadmin" {
                return HttpResponse::Unauthorized().json(json!({
                    "error": "unauthorized",
                    "message": "User is not an admin",
                    "status_code": 401
                }));
            }

            // For testing, we'll accept any password for admin users
            // In production, you should verify against a hash
            
            // Generate JWT token
            let now = Utc::now();
            let expiration = now
                .checked_add_signed(Duration::hours(1))
                .expect("valid timestamp")
                .timestamp() as usize;
            
            let claims = Claims {
                sub: user.id.clone(),
                exp: expiration,
                iat: now.timestamp() as usize,
                aud: app_state.config.get_app_url().to_string(),
                iss: app_state.config.get_app_url().to_string(),
                email: user.email.clone(),
                name: user.display_name.clone(),
            };
            
            let token = encode(
                &Header::new(Algorithm::HS256),
                &claims,
                &EncodingKey::from_secret(app_state.config.get_jwt_secret().as_bytes()),
            );
            
            match token {
                Ok(jwt) => {
                    // Update last login time
                    let _ = sqlx::query(
                        "UPDATE dbquizapp.users SET last_login = NOW() WHERE id = ?"
                    )
                    .bind(&user.id)
                    .execute(&*app_state.context.users.pool)
                    .await;

                    // Delete any existing sessions for this user
                    if let Err(e) = app_state.context.sessions.delete_all_user_sessions(&user.id).await {
                        eprintln!("Failed to delete existing sessions: {:?}", e);
                    }
                    
                    // Create a new session
                    let ip_address = http_request.connection_info().realip_remote_addr().map(|s| s.to_string());
                    let user_agent = http_request.headers().get("User-Agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());
                    
                    match app_state.context.sessions.create_session(
                        &user.id,
                        ip_address.as_deref(),
                        user_agent.as_deref()
                    ).await {
                        Ok(session) => {
                            let response = json!({
                                "token": jwt,
                                "user": {
                                    "id": user.id,
                                    "email": user.email,
                                    "display_name": user.display_name,
                                    "role": user.role,
                                },
                                "message": "Login successful"
                            });
                            HttpResponse::Ok().json(response)
                        },
                        Err(e) => {
                            eprintln!("Session creation error: {:?}", e);
                            HttpResponse::InternalServerError().json(json!({
                                "error": "session_creation_failed",
                                "message": "Failed to create session",
                                "status_code": 500
                            }))
                        }
                    }
                },
                Err(e) => {
                    eprintln!("JWT encoding error: {:?}", e);
                    HttpResponse::InternalServerError().json(json!({
                        "error": "token_generation_failed",
                        "message": "Failed to generate token",
                        "status_code": 500
                    }))
                }
            }
        },
        Err(e) => {
            eprintln!("User lookup error: {:?}", e);
            HttpResponse::BadRequest().json(json!({
                "error": "invalid_credentials",
                "message": "Invalid email or password",
                "status_code": 400
            }))
        }
    }
}

/// Handle Google OAuth callback
#[utoipa::path(
    post,
    path = "/auth/google/callback",
    request_body = GoogleAuthRequest,
    responses(
        (status = 200, description = "Google authentication successful", body = AuthResponse),
        (status = 400, description = "Invalid authentication data"),
        (status = 500, description = "Internal server error")
    ),
    tag = "auth",
    security() // Empty security means no authentication required
)]
#[post("/auth/google/callback")]
async fn google_callback(
    request: web::Json<GoogleAuthRequest>,
    app_state: web::Data<AppState<'_>>,
    http_request: HttpRequest,
) -> impl Responder {
    // Create Google OAuth client
    let google_client = GoogleOAuthClient::new(
        app_state.config.get_google_client_id(),
        app_state.config.get_google_client_secret(),
        app_state.config.get_google_redirect_uri(),
    );

    // Exchange authorization code for token
    let token_result = google_client.exchange_code_for_token(&request.code).await;
    
    match token_result {
        Ok(token) => {
            // Get user info using the access token
            let access_token = token.access_token().secret();
            let user_info_result = google_client.get_user_info(access_token).await;
            
            match user_info_result {
                Ok(user_info) => {
                    // Check if user exists in database
                    let user_result = app_state.context.users.get_user_by_email(&user_info.email).await;
                    
                    match user_result {
                        Ok(user) => {
                            // User exists, check if we need to update the profile picture
                            if let Some(picture) = &user_info.picture {
                                if user.picture_url.as_ref() != Some(picture) {
                                    // Update the user's profile picture
                                    let _ = app_state.context.users.update_user_profile(
                                        &user.id,
                                        &user.display_name,
                                        Some(picture),
                                        None // No phone number from Google OAuth
                                    ).await;
                                }
                            }
                            
                            // User exists, generate JWT token
                            let now = Utc::now();
                            let expiration = now
                                .checked_add_signed(Duration::hours(1))
                                .expect("valid timestamp")
                                .timestamp() as usize;
                            
                            let claims = Claims {
                                sub: user.id.clone(),
                                exp: expiration,
                                iat: now.timestamp() as usize,
                                aud: app_state.config.get_app_url().to_string(),
                                iss: app_state.config.get_app_url().to_string(),
                                email: user.email.clone(),
                                name: user.display_name.clone(),
                            };
                            
                            let token = encode(
                                &Header::new(Algorithm::HS256),
                                &claims,
                                &EncodingKey::from_secret(app_state.config.get_jwt_secret().as_bytes()),
                            );
                            
                            match token {
                                Ok(jwt) => {
                                    // Delete any existing sessions for this user
                                    if let Err(e) = app_state.context.sessions.delete_all_user_sessions(&user.id).await {
                                        eprintln!("Failed to delete existing sessions: {:?}", e);
                                        // Continue anyway, as this is not critical
                                    }
                                    
                                    // Create a new session
                                    let ip_address = http_request.connection_info().realip_remote_addr().map(|s| s.to_string());
                                    let user_agent = http_request.headers().get("User-Agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());
                                    
                                    let session_result = app_state.context.sessions.create_session(
                                        &user.id,
                                        ip_address.as_deref(),
                                        user_agent.as_deref()
                                    ).await;
                                    
                                    match session_result {
                                        Ok(session) => {
                                            let response = AuthResponse {
                                                access_token: jwt,
                                                token_type: "bearer".to_string(),
                                                expires_in: 3600,
                                                refresh_token: session.token,
                                                user: SupabaseUser {
                                                    id: user.id,
                                                    email: user.email,
                                                    display_name: user.display_name,
                                                    picture: user.picture_url,
                                                },
                                            };
                                            
                                            HttpResponse::Ok().json(response)
                                        },
                                        Err(e) => {
                                            eprintln!("Session creation error: {:?}", e);
                                            HttpResponse::InternalServerError().body("Failed to create session")
                                        }
                                    }
                                },
                                Err(e) => {
                                    eprintln!("JWT encoding error: {:?}", e);
                                    HttpResponse::InternalServerError().body("Failed to generate token")
                                }
                            }
                        },
                        Err(_) => {
                            // User doesn't exist, create new user
                            let user_id = uuid::Uuid::new_v4().to_string();
                            let display_name = user_info.name.clone();
                            let picture_url = user_info.picture.as_deref();
                            
                            // Use the new method to create user with picture
                            let result = app_state.context.users.create_user_with_picture(
                                &user_id,
                                &user_info.email,
                                &display_name,
                                picture_url
                            ).await;
                            
                            match result {
                                Ok(_) => {
                                    // Auto-assign username derived from display_name
                                    assign_auto_username(
                                        &app_state.context.users,
                                        &user_id,
                                        &display_name,
                                    ).await;

                                    // Generate JWT token for new user
                                    let now = Utc::now();
                                    let expiration = now
                                        .checked_add_signed(Duration::hours(1))
                                        .expect("valid timestamp")
                                        .timestamp() as usize;
                                    
                                    let claims = Claims {
                                        sub: user_id.clone(),
                                        exp: expiration,
                                        iat: now.timestamp() as usize,
                                        aud: app_state.config.get_app_url().to_string(),
                                        iss: app_state.config.get_app_url().to_string(),
                                        email: user_info.email.clone(),
                                        name: display_name.clone(),
                                    };
                                    
                                    let token = encode(
                                        &Header::new(Algorithm::HS256),
                                        &claims,
                                        &EncodingKey::from_secret(app_state.config.get_jwt_secret().as_bytes()),
                                    );
                                    
                                    match token {
                                        Ok(jwt) => {
                                            // Delete any existing sessions for this user
                                            // This is a new user, but just in case there are any orphaned sessions
                                            if let Err(e) = app_state.context.sessions.delete_all_user_sessions(&user_id).await {
                                                eprintln!("Failed to delete existing sessions: {:?}", e);
                                                // Continue anyway, as this is not critical
                                            }
                                            
                                            // Create a new session
                                            let ip_address = http_request.connection_info().realip_remote_addr().map(|s| s.to_string());
                                            let user_agent = http_request.headers().get("User-Agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());
                                            
                                            let session_result = app_state.context.sessions.create_session(
                                                &user_id,
                                                ip_address.as_deref(),
                                                user_agent.as_deref()
                                            ).await;
                                            
                                            match session_result {
                                                Ok(session) => {
                                                    let response = AuthResponse {
                                                        access_token: jwt,
                                                        token_type: "bearer".to_string(),
                                                        expires_in: 3600,
                                                        refresh_token: session.token,
                                                        user: SupabaseUser {
                                                            id: user_id,
                                                            email: user_info.email,
                                                            display_name,
                                                            picture: user_info.picture,
                                                        },
                                                    };
                                                    
                                                    HttpResponse::Ok().json(response)
                                                },
                                                Err(e) => {
                                                    eprintln!("Session creation error: {:?}", e);
                                                    HttpResponse::InternalServerError().body("Failed to create session")
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            eprintln!("JWT encoding error: {:?}", e);
                                            HttpResponse::InternalServerError().body("Failed to generate token")
                                        }
                                    }
                                },
                                Err(e) => {
                                    eprintln!("Database error: {:?}", e);
                                    HttpResponse::InternalServerError().body("Failed to store user data")
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Failed to get user info: {:?}", e);
                    HttpResponse::BadRequest().body("Failed to get user information from Google")
                }
            }
        },
        Err(e) => {
            eprintln!("Token exchange error: {:?}", e);
            HttpResponse::BadRequest().body("Failed to exchange authorization code for token")
        }
    }
}

/// Request payload for logout
#[derive(Debug, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub token: String,
}

/// Logout endpoint to invalidate a session
#[post("/auth/logout")]
async fn logout(
    logout_req: web::Json<LogoutRequest>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    // Delete the session with the provided token
    match app_state.context.sessions.delete_session(&logout_req.token).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "message": "Logged out successfully" })),
        Err(e) => {
            eprintln!("Logout error: {:?}", e);
            HttpResponse::InternalServerError().body("Failed to logout")
        }
    }
}

/// Get all active sessions for a user
#[get("/auth/sessions/{user_id}")]
async fn get_sessions(
    path: web::Path<String>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    let user_id = path.into_inner();
    
    match app_state.context.sessions.get_sessions_by_user_id(&user_id).await {
        Ok(sessions) => {
            let session_responses: Vec<SessionResponse> = sessions
                .into_iter()
                .map(|s| SessionResponse {
                    token: s.token,
                    expires_at: s.expires_at,
                })
                .collect();
            
            HttpResponse::Ok().json(session_responses)
        },
        Err(e) => {
            eprintln!("Get sessions error: {:?}", e);
            HttpResponse::InternalServerError().body("Failed to get sessions")
        }
    }
}

/// Validate a session token
#[post("/auth/validate-session")]
async fn validate_session(
    validate_req: web::Json<ValidateSessionRequest>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    // Check if the session exists and is not expired
    match app_state.context.sessions.get_session_by_token(&validate_req.token).await {
        Ok(session) => {
            // Session exists and is not expired
            HttpResponse::Ok().json(ValidateSessionResponse {
                valid: true,
                message: "Session is valid".to_string(),
                user_id: Some(session.user_id),
            })
        },
        Err(_) => {
            // Session doesn't exist or is expired
            HttpResponse::Ok().json(ValidateSessionResponse {
                valid: false,
                message: "Session is invalid or expired. Please log in again.".to_string(),
                user_id: None,
            })
        }
    }
}