mod middleware;

use actix_web::{web, App, HttpServer};
use actix_cors::Cors;
use quiz_backend::config::Config;
use quiz_backend::dao::Database;
use quiz_backend::{controller, AppState};
use std::sync::{Arc, Mutex};
use http::header;
use crate::middleware::auth_middleware::AuthMiddleware;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("=== Quiz Backend ===");

    let config_file: &'static str = "config.json";
    let config = Arc::new(Config::from_file(config_file));
    println!("Using configuration file from {0}", config_file);

    let db_context = Database::new(&config.get_database_url()).await;
    println!("Connected to database: {0}", config.get_database_url());

    let app_state = web::Data::new(AppState {
        connections: Mutex::new(0),
        context: Arc::new(db_context),
    });

    let app_url = config.get_app_url().to_owned();
    let api_key = config.get_api_key().to_string();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("https://kuis.canducation.com")
            .allowed_origin("http://localhost:4200")
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
            .allowed_header(header::CONTENT_TYPE)
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(AuthMiddleware::new(api_key.clone()))
            .app_data(app_state.clone())
            .configure(controller::init_soal_controller)
    })
    .bind(app_url)?
    .run()
    .await
}
