mod middleware;

use actix_web::{web, App, HttpServer};
use actix_cors::Cors;
use quiz_backend::config::Config;
use quiz_backend::dao::Database;
use quiz_backend::{controller, AppState};
use quiz_backend::service::redis_service::RedisPool;
use supabase_auth::models::SignUpWithPasswordOptions;
use std::sync::{Arc, Mutex};
use http::header;
use crate::middleware::auth_middleware::AuthMiddleware;
use supabase_auth::models::AuthClient;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use quiz_backend::docs::ApiDoc;
use std::time::Duration;
use tokio::time;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    println!("=== Quiz Backend ===");

    let config_file: &'static str = "config.json";
    let config = Arc::new(Config::from_file(config_file));
    println!("Using configuration file from {0}", config_file);

    let db_context = Database::new(&config.get_database_url()).await;
    println!("Connected to database: {0}", config.get_database_url());

    let auth_client = AuthClient::new(
        config.get_auth_url(),
        config.get_anon_key(),
        config.get_jwt_secret(),
    );

    let redis_pool = match RedisPool::new(&config.get_redis_url()).await {
        Ok(pool) => {
            println!("Connected to Redis: {}:{}", config.get_redis_host(), config.get_redis_port());
            Some(Arc::new(pool))
        },
        Err(e) => {
            eprintln!("Failed to connect to Redis: {:?}", e);
            eprintln!("Continuing without Redis cache...");
            None
        }
    };

    let app_state = web::Data::new(AppState {
        connections: Mutex::new(0),
        context: Arc::new(db_context),
        config: config.clone(),
        auth_client,
        sign_up_with_password_options: SignUpWithPasswordOptions::default(),
        redis_pool,
    });

    let app_url = config.get_app_url();
    let jwt_secret = config.get_jwt_secret().to_string();

    // Background task: clean up expired sessions every hour
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            if let Err(e) = app_state_clone.context.sessions.delete_expired_sessions().await {
                eprintln!("Failed to clean up expired sessions: {:?}", e);
            }
        }
    });

    HttpServer::new(move || {
        let cors = Cors::default()
        .allow_any_origin()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::ACCEPT,
        header::ORIGIN,
        header::ACCESS_CONTROL_REQUEST_METHOD,
        header::ACCESS_CONTROL_REQUEST_HEADERS,
    ])
    .expose_headers(vec!["Authorization"])
    .max_age(3600)
    .supports_credentials();

        App::new()
            .wrap(cors)
            .wrap(AuthMiddleware::new(jwt_secret.clone()))
            .app_data(app_state.clone())
            .configure(controller::init_soal_controller)
            .configure(controller::init_auth_controller)
            .configure(controller::init_kategori_controller)
            .configure(controller::init_premium_controller)
            .configure(controller::init_payment_controller)
            .configure(controller::init_user_controller)
            .configure(controller::init_license_controller)
            .configure(controller::init_bookmark_controller)
            .configure(quiz_backend::controller::quiz_session_controller::configure_routes)
            .configure(controller::init_admin_user_controller)
            .configure(controller::init_admin_kategori_controller)
            .configure(controller::init_admin_soal_controller)
            .configure(controller::init_admin_packages_controller)
            .configure(controller::init_admin_paket_soal_items_controller)
            .configure(controller::init_admin_analytics_controller)
            .configure(controller::init_internal_soal_controller)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
    })
    .bind(app_url)?
    .run()
    .await
}
