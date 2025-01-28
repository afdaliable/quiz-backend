use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use crate::AppState;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct SignUpRequest {
    email: String,
    password: String,
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct AuthResponse {
    access_token: String,
    token_type: String,
    expires_in: i32,
    refresh_token: String,
    user: SupabaseUser,
}

#[derive(Debug, Serialize)]
struct SupabaseUser {
    id: String,
    email: String,
    display_name: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(signup)
       .service(login);
}

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

    match app_state.auth_client.sign_up_with_email_and_password(
        &signup_req.email,
        &signup_req.password,
        Some(options)
    ).await {
        Ok(session) => {
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
            eprintln!("Signup error: {:?}", e);
            HttpResponse::BadRequest().body(format!("Signup failed: {}", e))
        }
    }
}

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
            // For login, we'll get display_name from user_metadata if it exists
            let display_name = serde_json::to_value(session.user.user_metadata)
                .unwrap_or(serde_json::Value::Null)
                .as_object()
                .and_then(|obj| obj.get("display_name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

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
            eprintln!("Login error: {:?}", e);
            HttpResponse::BadRequest().body(format!("Login failed: {}", e))
        }
    }
}