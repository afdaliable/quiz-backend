use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;

use crate::AppState;
use crate::middleware::admin_middleware::AdminMiddleware;
use crate::dao::exam_simulation_dao::ExamSimulationDao;
use crate::model::exam_simulation::{CreateExamSimulationRequest, UpdateExamSimulationRequest};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/simulasi-ujian")
            .wrap(AdminMiddleware::new())
            // /stats must come before /{id} to avoid being captured as id
            .route("/stats", web::get().to(admin_simulasi_stats))
            .route("/{id}/attempts", web::get().to(admin_get_attempts))
            .route("/{id}", web::get().to(admin_get_simulasi))
            .route("/{id}", web::put().to(admin_update_simulasi))
            .route("/{id}", web::delete().to(admin_delete_simulasi))
            .route("", web::get().to(admin_list_simulasi))
            .route("", web::post().to(admin_create_simulasi))
    );
}

#[derive(Debug, Deserialize)]
pub struct AdminListQuery {
    pub is_active: Option<bool>,
}

async fn admin_list_simulasi(
    state: web::Data<AppState<'_>>,
    q: web::Query<AdminListQuery>,
) -> impl Responder {
    let dao = ExamSimulationDao::new(state.context.soal.pool.clone());
    match dao.list_admin(q.is_active).await {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(e) => {
            eprintln!("Error listing simulasi (admin): {:?}", e);
            HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch simulasi"}))
        }
    }
}

async fn admin_create_simulasi(
    state: web::Data<AppState<'_>>,
    req: web::Json<CreateExamSimulationRequest>,
) -> impl Responder {
    // Basic validation.
    if req.duration_minutes <= 0 {
        return HttpResponse::BadRequest().json(json!({"error": "duration_minutes must be > 0"}));
    }
    if req.total_questions <= 0 {
        return HttpResponse::BadRequest().json(json!({"error": "total_questions must be > 0"}));
    }
    if !(0..=100).contains(&req.passing_score) {
        return HttpResponse::BadRequest().json(json!({"error": "passing_score must be 0-100"}));
    }

    let dao = ExamSimulationDao::new(state.context.soal.pool.clone());
    match dao.create(&req).await {
        Ok(id) => match dao.get_by_id(id).await {
            Ok(sim) => HttpResponse::Created().json(sim),
            Err(e) => {
                eprintln!("Error fetching created simulasi: {:?}", e);
                HttpResponse::Ok().json(json!({"id": id}))
            }
        }
        Err(e) => {
            eprintln!("Error creating simulasi: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"error": "Failed to create simulasi"}))
        }
    }
}

async fn admin_simulasi_stats(state: web::Data<AppState<'_>>) -> impl Responder {
    let dao = ExamSimulationDao::new(state.context.soal.pool.clone());
    match dao.admin_stats().await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => {
            eprintln!("Error fetching simulasi stats: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch stats"}))
        }
    }
}

async fn admin_get_simulasi(
    path: web::Path<i32>,
    state: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = ExamSimulationDao::new(state.context.soal.pool.clone());
    match dao.get_by_id(id).await {
        Ok(sim) => HttpResponse::Ok().json(sim),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(json!({"error": "Simulasi not found"})),
        Err(e) => {
            eprintln!("Error fetching simulasi: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch simulasi"}))
        }
    }
}

async fn admin_update_simulasi(
    path: web::Path<i32>,
    state: web::Data<AppState<'_>>,
    req: web::Json<UpdateExamSimulationRequest>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = ExamSimulationDao::new(state.context.soal.pool.clone());
    if let Err(e) = dao.update(id, &req).await {
        eprintln!("Error updating simulasi: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "Failed to update simulasi"}));
    }
    match dao.get_by_id(id).await {
        Ok(sim) => HttpResponse::Ok().json(sim),
        Err(e) => {
            eprintln!("Error re-fetching simulasi: {:?}", e);
            HttpResponse::Ok().json(json!({"id": id}))
        }
    }
}

async fn admin_delete_simulasi(
    path: web::Path<i32>,
    state: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = ExamSimulationDao::new(state.context.soal.pool.clone());
    match dao.soft_delete(id).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            eprintln!("Error deleting simulasi: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"error": "Failed to delete simulasi"}))
        }
    }
}

async fn admin_get_attempts(
    path: web::Path<i32>,
    state: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    let dao = ExamSimulationDao::new(state.context.soal.pool.clone());
    match dao.list_attempts_by_simulasi(id).await {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(e) => {
            eprintln!("Error listing attempts: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch attempts"}))
        }
    }
}
