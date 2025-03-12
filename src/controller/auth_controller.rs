use actix_web::{post, web, HttpResponse, Responder};
use serde_json::json;
use crate::AppState;
use crate::model::{SignUpRequest, LoginRequest, AuthResponse, SupabaseUser};
use crate::utils::google_oauth::GoogleOAuthClient;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};
use chrono::{Utc, Duration};
use oauth2::TokenResponse;

/// Request payload for Google OAuth callback
#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleAuthRequest {
    pub code: String,
}

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    email: String,
    name: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(signup)
       .service(login)
       .service(google_callback);
}

/// Register a new user
#[utoipa::path(
    post,
    path = "/signup",
    request_body = SignUpRequest,
    responses(
        (status = 200, description = "User successfully registered", body = AuthResponse),
        (status = 400, description = "Invalid registration data"),
        (status = 500, description = "Internal server error")
    ),
    tag = "auth",
    security() // Empty security means no authentication required
)]
#[post("/signup")]
async fn signup(
    signup_req: web::Json<SignUpRequest>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    let mut options = app_state.sign_up_with_password_options.clone();
    let user_metadata = json!({
        "display_name": signup_req.display_name.clone(),
        "raw_user_meta_data": {
            "display_name": signup_req.display_name.clone()
        }
    });
    
    options.data = Some(user_metadata);

    // First attempt Supabase signup
    match app_state.auth_client.sign_up_with_email_and_password(
        &signup_req.email,
        &signup_req.password,
        Some(options)
    ).await {
        Ok(session) => {
            // After successful Supabase signup, store user in local database
            let user_id = session.user.id.clone();
            let result = sqlx::query(
                r#"
                INSERT INTO users (id, email, display_name)
                VALUES (?, ?, ?)
                "#
            )
            .bind(&user_id)
            .bind(&signup_req.email)
            .bind(&signup_req.display_name)
            .execute(&*app_state.context.users.pool)
            .await;

            match result {
                Ok(_) => {
                    let display_name = signup_req.display_name.clone();
                    let response = AuthResponse {
                        access_token: session.access_token,
                        token_type: "bearer".to_string(),
                        expires_in: 3600,
                        refresh_token: session.refresh_token,
                        user: SupabaseUser {
                            id: session.user.id,
                            email: session.user.email,
                            display_name,
                            picture: None,
                        }
                    };
                    HttpResponse::Ok().json(response)
                },
                Err(e) => {
                    eprintln!("Database error: {:?}", e);
                    HttpResponse::InternalServerError().body("Failed to store user data")
                }
            }
        },
        Err(e) => {
            eprintln!("Signup error: {:?}", e);
            HttpResponse::BadRequest().body(format!("Signup failed: {}", e))
        }
    }
}

/// Login with email and password
#[utoipa::path(
    post,
    path = "/auth/v1/token",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 400, description = "Invalid credentials"),
        (status = 500, description = "Internal server error")
    ),
    tag = "auth",
    security() // Empty security means no authentication required
)]
#[post("/auth/v1/token")]
async fn login(
    login_req: web::Json<LoginRequest>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    match app_state.auth_client.login_with_email(
        &login_req.email,
        &login_req.password
    ).await {
        Ok(session) => {
            // After Supabase authentication, fetch user from our database
            match app_state.context.users.get_user_by_email(&login_req.email).await {
                Ok(local_user) => {
                    let response = AuthResponse {
                        access_token: session.access_token,
                        token_type: "bearer".to_string(),
                        expires_in: 3600,
                        refresh_token: session.refresh_token,
                        user: SupabaseUser {
                            id: session.user.id,
                            email: session.user.email,
                            display_name: local_user.display_name,
                            picture: local_user.picture_url,
                        }
                    };
                    HttpResponse::Ok().json(response)
                },
                Err(e) => {
                    eprintln!("Failed to fetch user data: {:?}", e);
                    // Still return auth response but with empty display_name
                    let response = AuthResponse {
                        access_token: session.access_token,
                        token_type: "bearer".to_string(),
                        expires_in: 3600,
                        refresh_token: session.refresh_token,
                        user: SupabaseUser {
                            id: session.user.id,
                            email: session.user.email,
                            display_name: String::new(),
                            picture: None,
                        }
                    };
                    HttpResponse::Ok().json(response)
                }
            }
        },
        Err(e) => {
            eprintln!("Login error: {:?}", e);
            HttpResponse::BadRequest().body(format!("Login failed: {}", e))
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
                                        Some(picture)
                                    ).await;
                                }
                            }
                            
                            // User exists, generate JWT token
                            let expiration = Utc::now()
                                .checked_add_signed(Duration::hours(1))
                                .expect("valid timestamp")
                                .timestamp() as usize;
                            
                            let claims = Claims {
                                sub: user.id.clone(),
                                exp: expiration,
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
                                    let response = AuthResponse {
                                        access_token: jwt,
                                        token_type: "bearer".to_string(),
                                        expires_in: 3600,
                                        refresh_token: "".to_string(), // Google OAuth doesn't use refresh tokens in this flow
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
                                    // Generate JWT token for new user
                                    let expiration = Utc::now()
                                        .checked_add_signed(Duration::hours(1))
                                        .expect("valid timestamp")
                                        .timestamp() as usize;
                                    
                                    let claims = Claims {
                                        sub: user_id.clone(),
                                        exp: expiration,
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
                                            let response = AuthResponse {
                                                access_token: jwt,
                                                token_type: "bearer".to_string(),
                                                expires_in: 3600,
                                                refresh_token: "".to_string(),
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