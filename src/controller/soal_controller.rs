use super::log_request;
use super::AppState;

// use crate::model::Soal;
use actix_web::{get, web, HttpResponse, Responder};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_soal)
       .service(get_paket_soal_response)
       .service(get_list_paket_soal);
}

#[get("/soal/{id}")]
async fn get_soal(
    soal_id: web::Path<String>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("GET: /soal", &app_state.connections);

    let soal = app_state.context.soal.get_soal_by_id(&soal_id).await;

    match soal {
        Err(_) => HttpResponse::NotFound().finish(),
        Ok(soal) => HttpResponse::Ok().json(soal),
    }
}

#[get("/paket-soal-response/{nama_kategori}/{nama_paket_soal}")]
async fn get_paket_soal_response(
    path: web::Path<(String, String)>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    let (nama_kategori, nama_paket_soal) = path.into_inner();
    log_request("GET: /paket-soal-response", &app_state.connections);
    
    let paket_soal_response = app_state.context.paket_soal_response.get_paket_soal_response(&nama_kategori, &nama_paket_soal).await;

    match paket_soal_response {
        Err(e) => {
            println!("Error: {:?}", e);
            HttpResponse::NotFound().finish()
        },
        Ok(paket_soal_response) => HttpResponse::Ok().json(paket_soal_response),
    }
}

#[get("/listpaketsoal")]
async fn get_list_paket_soal(
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("GET: /listpaketsoal", &app_state.connections);
    
    let list_paket_soal = app_state.context.paket_soal_response.get_list_paket_soal().await;

    match list_paket_soal {
        Err(e) => {
            println!("Error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        },
        Ok(list_paket_soal) => HttpResponse::Ok().json(list_paket_soal),
    }
}


// Kode yang dikomentari tetap tidak berubah
