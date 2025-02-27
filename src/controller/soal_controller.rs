use super::log_request;
use super::AppState;

// use crate::model::Soal;
use actix_web::{get, post, web, HttpResponse, Responder};
use crate::model::CreateSoalRequest;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_soal)
       .service(get_paket_soal_response)
       .service(get_paket_soal_by_category)
       .service(get_list_paket_soal)
        .service(get_all_soal)
        .service(create_soal);
}

/// Get a specific soal by ID
#[utoipa::path(
    get,
    path = "/soal/{id}",
    responses(
        (status = 200, description = "Soal found successfully", body = Soal),
        (status = 404, description = "Soal not found")
    ),
    params(
        ("id" = String, Path, description = "Soal identifier")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
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

/// Get paket soal response by category and package name
#[utoipa::path(
    get,
    path = "/paket-soal-response/{nama_kategori}/{nama_paket_soal}",
    responses(
        (status = 200, description = "Paket soal found successfully", body = PaketSoalResponse),
        (status = 404, description = "Paket soal not found")
    ),
    params(
        ("nama_kategori" = String, Path, description = "Category name"),
        ("nama_paket_soal" = String, Path, description = "Package name")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
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

/// Get List of all Paket Soal
#[utoipa::path(
    get,
    path = "/listpaketsoal",
    responses(
        (status = 200, description = "List of paket soal retrieved successfully", body = ListPaketSoal),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
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

/// Get all soal (questions)
#[utoipa::path(
    get,
    path = "/kumpulan-soal",
    responses(
        (status = 200, description = "List all soal successfully", body = Vec<Soal>),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/kumpulan-soal")]
async fn get_all_soal(
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("GET: /kumpulan-soal", &app_state.connections);

    let soal = app_state.context.soal.get_all_soal().await;

    match soal {
        Err(e) =>{
            println!("Error: {:?}", e);
            HttpResponse::InternalServerError().finish()
        },
        Ok(soal) => HttpResponse::Ok().json(soal),
    }
    
}

/// Create a new soal
#[utoipa::path(
    post,
    path = "/soal",
    request_body = CreateSoalRequest,
    responses(
        (status = 201, description = "Soal created successfully", body = Soal),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[post("/soal")]
async fn create_soal(
    app_state: web::Data<AppState<'_>>,
    payload: web::Json<CreateSoalRequest>,
) -> impl Responder {
    println!("Received POST request to /soal");  // Add this line
    log_request("POST: /soal", &app_state.connections);

    let result = app_state.context.soal.create_soal(&payload).await;

    match result {
        Ok(soal) => HttpResponse::Created().json(soal),
        Err(e) => {
            println!("Error creating soal : {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Get paket soal responses by category
#[utoipa::path(
    get,
    path = "/paket-soal-response/{nama_kategori}",
    responses(
        (status = 200, description = "List of paket soal found successfully", body = Vec<PaketSoalResponse>),
        (status = 404, description = "Category not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("nama_kategori" = String, Path, description = "Category name")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[get("/paket-soal-response/{nama_kategori}")]
async fn get_paket_soal_by_category(
    nama_kategori: web::Path<String>,
    app_state: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("GET: /paket-soal-response/{nama_kategori}", &app_state.connections);
    
    let paket_soal_responses = app_state.context.paket_soal_response
        .get_paket_soal_by_category(&nama_kategori)
        .await;

    match paket_soal_responses {
        Ok(responses) => HttpResponse::Ok().json(responses),
        Err(e) => {
            println!("Error: {:?}", e);
            HttpResponse::NotFound().finish()
        }
    }
}

// Kode yang dikomentari tetap tidak berubah
