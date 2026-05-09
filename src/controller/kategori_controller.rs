use super::log_request;
use super::AppState;

use actix_web::{get, web, HttpResponse, Responder};
use crate::service::redis_service::{RedisService, CKEY_SEMUA_KATEGORI, CACHE_TTL_STATIC};
use crate::model::KategoriSoal;

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

    if let Some(redis_pool) = &app_state.redis_pool {
        let mut con = redis_pool.quiz_cache().as_ref().clone();
        if let Ok(Some(cached)) = RedisService::get_cached_quiz_by_key::<Vec<KategoriSoal>>(&mut con, CKEY_SEMUA_KATEGORI).await {
            println!("[CACHE HIT] /semuaKategori");
            return HttpResponse::Ok().json(cached);
        }
        println!("[CACHE MISS] /semuaKategori — fetch from DB");
        match app_state.context.kategori.get_all_category().await {
            Ok(categories) => {
                let _ = RedisService::cache_quiz_by_key(&mut con, CKEY_SEMUA_KATEGORI, &categories, CACHE_TTL_STATIC).await;
                HttpResponse::Ok().json(categories)
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
                HttpResponse::InternalServerError().finish()
            }
        }
    } else {
        match app_state.context.kategori.get_all_category().await {
            Ok(categories) => HttpResponse::Ok().json(categories),
            Err(e) => {
                eprintln!("Error: {:?}", e);
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}

