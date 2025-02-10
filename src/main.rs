mod middleware;

use actix_web::{web, App, HttpServer};
use actix_cors::Cors;
use quiz_backend::config::Config;
use quiz_backend::dao::Database;
use quiz_backend::{controller, AppState};
use supabase_auth::models::SignUpWithPasswordOptions;
use std::sync::{Arc, Mutex};
use http::header;
use crate::middleware::auth_middleware::AuthMiddleware;
use supabase_auth::models::AuthClient;

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

    let app_state = web::Data::new(AppState {
        connections: Mutex::new(0),
        context: Arc::new(db_context),
        config: config.clone(),
        auth_client,
        sign_up_with_password_options: SignUpWithPasswordOptions::default(),
    });

    let app_url = config.get_app_url();
    let jwt_secret = config.get_jwt_secret().to_string();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("https://kuis.canducation.com")
            .allowed_origin("http://localhost:4200")
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE
            ])
            .expose_headers(vec![header::AUTHORIZATION])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(AuthMiddleware::new(jwt_secret.clone()))
            .app_data(app_state.clone())
            .configure(controller::init_soal_controller)
            .configure(controller::init_auth_controller)
    })
    .bind(app_url)?
    .run()
    .await
}
