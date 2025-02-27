use actix_web::{post, web, HttpResponse, Responder};
use serde_json::json;
use crate::AppState;
use crate::model::{SignUpRequest, LoginRequest, AuthResponse, SupabaseUser};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(signup)
       .service(login);
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