use super::log_request;
use super::AppState;

use actix_web::{get, web, HttpResponse, Responder};

pub fn init (cfg: &mut web::ServiceConfig){
    cfg.service(get_semua_kategori);
}

/// Get all categories
#[utoipa::path(
    get,
    path = "/semuaKategori",
    responses(
        (status = 200, description = "List all categories successfully", body = Vec<KategoriSoal>),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/semuaKategori")]
async fn get_semua_kategori(
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("GET: /semuaKategori", &app_state.connections);

    let categories = app_state.context.kategori.get_all_category().await;   

    match categories {
        Ok(categories) => HttpResponse::Ok().json(categories),
        Err(e) => {
            println!("Error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

