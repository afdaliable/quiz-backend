mod middleware;

use actix_web::{web, App, HttpServer};
use actix_files as afiles;
use actix_cors::Cors;
use quiz_backend::config::Config;
use quiz_backend::dao::Database;
use quiz_backend::{controller, AppState};
use quiz_backend::service::redis_service::RedisPool;
use quiz_backend::service::ai_service::AiService;
use supabase_auth::models::SignUpWithPasswordOptions;
use std::sync::{Arc, Mutex};
use http::header;
use crate::middleware::auth_middleware::AuthMiddleware;
use supabase_auth::models::AuthClient;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use quiz_backend::docs::ApiDoc;
use quiz_backend::service::difficulty_service::recalculate_difficulty;
use std::time::Duration;
use tokio::time;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    println!("=== Quiz Backend ===");

    let config_file: &'static str = "config.json";
    let config = Arc::new(Config::from_file(config_file));
    println!("Using configuration file from {0}", config_file);

    let write_url = config.get_database_url();
    let read_url = config.get_read_database_url();

    let db_context = Arc::new(Database::new(&write_url).await);
    println!("Connected to database (write): {}", write_url);

    let read_db_context: Arc<Database<'_>> = if config.has_read_replica() {
        println!("Connected to database (read replica): {}", read_url);
        Arc::new(Database::new(&read_url).await)
    } else {
        db_context.clone()
    };

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

    let ai_service = match AiService::from_config(&config) {
        Some(svc) => {
            println!("AI service initialized (model: {})", config.get_ai_config()
                .map(|c| c.model.as_str())
                .unwrap_or("unknown"));
            Some(Arc::new(svc))
        }
        None => {
            println!("AI service not configured (no 'ai' section in config.json) — AI features disabled");
            None
        }
    };

    let app_state = web::Data::new(AppState {
        connections: Mutex::new(0),
        context: db_context,
        read_context: read_db_context,
        config: config.clone(),
        auth_client,
        sign_up_with_password_options: SignUpWithPasswordOptions::default(),
        redis_pool,
        ai_service,
    });

    let app_url = config.get_app_url();
    let jwt_secret = config.get_jwt_secret().to_string();

    // Create upload directory on startup
    let soal_images_dir = format!("{}/soal-images", config.get_upload_dir());
    std::fs::create_dir_all(&soal_images_dir)
        .map_err(|e| std::io::Error::new(e.kind(), format!("Failed to create soal-images upload directory '{}': {}", soal_images_dir, e)))?;
    println!("Upload directory: {}", soal_images_dir);

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

    // Background task: recalculate difficulty weekly (AFD-206)
    let app_state_diff = app_state.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(7 * 24 * 3600));
        loop {
            interval.tick().await;
            recalculate_difficulty(app_state_diff.context.soal.pool.clone()).await;
        }
    });

    // Background task: recompute soal coverage/tag-variant analytics daily
    let app_state_analytics = app_state.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(24 * 3600));
        loop {
            interval.tick().await;
            if let Err(e) = quiz_backend::service::soal_analytics_service::compute_and_store_summary(
                &app_state_analytics.context.soal.pool,
            )
            .await
            {
                eprintln!("Daily soal analytics recompute failed: {}", e);
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
            .configure(quiz_backend::controller::search_controller::configure_routes)
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
            .configure(controller::init_admin_alerts_controller)
            .configure(controller::init_admin_passage_controller)
            .configure(controller::init_materi_generate_controller)
            .configure(controller::init_taxonomy_classify_controller)
            .configure(controller::init_soal_quality_controller)
            .configure(controller::init_soal_analytics_controller)
            .configure(controller::init_internal_soal_controller)
            .configure(controller::init_question_comment_controller)
            .configure(controller::init_subscription_controller)
            .configure(controller::init_analytics_controller)
            .configure(controller::init_ai_controller)
            // NOTE: init_admin_simulasi_controller MUST come before init_admin_hierarchy_controller
            // because admin_hierarchy_controller registers a catch-all `web::scope("/admin")`
            // (for bulk_update_questions, get_question_stats) that would swallow any /admin/* path
            // registered after it. Actix-web matches scopes in registration order, first-match-wins.
            .configure(controller::init_admin_simulasi_controller)
            .configure(quiz_backend::controller::simulasi_ujian_controller::configure_routes)
            .configure(controller::init_admin_hierarchy_controller)
            .configure(quiz_backend::controller::public_profile_controller::configure_routes)
            .configure(quiz_backend::controller::question_feedback_controller::configure_routes)
            .configure(quiz_backend::controller::leaderboard_controller::configure_routes)
            .configure(quiz_backend::controller::taxonomy_controller::configure_routes)
            .service(
                afiles::Files::new("/static/soal-images", soal_images_dir.clone())
                    .use_last_modified(true)
            )
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
    })
    .bind(app_url)?
    .run()
    .await
}
