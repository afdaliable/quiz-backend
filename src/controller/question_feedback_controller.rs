use actix_web::{web, HttpRequest, HttpResponse};
use crate::AppState;
use crate::dao::question_feedback_dao::QuestionFeedbackDao;
use crate::model::question_feedback::{SubmitRatingRequest, SubmitReportRequest, UpdateReportStatusRequest};
use crate::middleware::admin_middleware::AdminMiddleware;

fn extract_user_id(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

// ── User endpoints ────────────────────────────────────────────────────────────

/// POST /questions/{id}/rate
/// Body: `{ "rating": "helpful" | "confusing" }` — upsert
/// Body ausente / null → hapus rating (toggle off)
async fn rate_question(
    req: HttpRequest,
    path: web::Path<i32>,
    body: web::Bytes,
    data: web::Data<AppState<'_>>,
) -> HttpResponse {
    let question_id = path.into_inner();
    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None     => return HttpResponse::Unauthorized().json("Unauthorized"),
    };

    let dao = QuestionFeedbackDao::new(data.context.soal.pool.clone());

    if body.is_empty() || body.as_ref() == b"null" {
        let _ = dao.delete_rating(question_id, &user_id).await;
    } else {
        let rating_req: SubmitRatingRequest = match serde_json::from_slice(&body) {
            Ok(r)  => r,
            Err(_) => return HttpResponse::BadRequest().json("Body tidak valid"),
        };
        if let Err(e) = dao.upsert_rating(question_id, &user_id, &rating_req.rating).await {
            return HttpResponse::InternalServerError().json(e.to_string());
        }
    }

    match dao.get_rating_summary(question_id, &user_id).await {
        Ok(summary) => HttpResponse::Ok().json(summary),
        Err(e)      => HttpResponse::InternalServerError().json(e.to_string()),
    }
}

/// GET /questions/{id}/ratings
async fn get_ratings(
    req: HttpRequest,
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
) -> HttpResponse {
    let question_id = path.into_inner();
    let user_id = extract_user_id(&req).unwrap_or_default();

    let dao = QuestionFeedbackDao::new(data.context.soal.pool.clone());

    match dao.get_rating_summary(question_id, &user_id).await {
        Ok(summary) => HttpResponse::Ok().json(summary),
        Err(e)      => HttpResponse::InternalServerError().json(e.to_string()),
    }
}

/// POST /questions/{id}/report
async fn report_question(
    req: HttpRequest,
    path: web::Path<i32>,
    body: web::Json<SubmitReportRequest>,
    data: web::Data<AppState<'_>>,
) -> HttpResponse {
    let question_id = path.into_inner();
    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None     => return HttpResponse::Unauthorized().json("Unauthorized"),
    };

    if let Some(ref d) = body.detail {
        if d.len() > 500 {
            return HttpResponse::BadRequest()
                .json("Detail terlalu panjang (maks 500 karakter)");
        }
    }

    let dao = QuestionFeedbackDao::new(data.context.soal.pool.clone());

    match dao.create_report(question_id, &user_id, &body).await {
        Ok(id) => HttpResponse::Ok().json(serde_json::json!({
            "id":      id,
            "message": "Laporan terkirim"
        })),
        Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
    }
}

// ── Admin endpoints ───────────────────────────────────────────────────────────

/// GET /admin/questions/reports?status=pending&page=1&per_page=20
async fn admin_list_reports(
    query: web::Query<std::collections::HashMap<String, String>>,
    data:  web::Data<AppState<'_>>,
) -> HttpResponse {
    let status   = query.get("status").map(|s| s.as_str());
    let page     = query.get("page").and_then(|p| p.parse::<u32>().ok()).unwrap_or(1);
    let per_page = query.get("per_page")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(20)
        .min(100);

    let dao = QuestionFeedbackDao::new(data.context.soal.pool.clone());

    match dao.list_reports_admin(status, page, per_page).await {
        Ok((reports, total)) => HttpResponse::Ok().json(serde_json::json!({
            "data":     reports,
            "total":    total,
            "page":     page,
            "per_page": per_page,
        })),
        Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
    }
}

/// GET /admin/questions/most-reported?limit=20
async fn admin_most_reported(
    query: web::Query<std::collections::HashMap<String, String>>,
    data:  web::Data<AppState<'_>>,
) -> HttpResponse {
    let limit = query.get("limit")
        .and_then(|l| l.parse::<u32>().ok())
        .unwrap_or(20)
        .min(100);

    let dao = QuestionFeedbackDao::new(data.context.soal.pool.clone());

    match dao.get_most_reported_questions(limit).await {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e)   => HttpResponse::InternalServerError().json(e.to_string()),
    }
}

/// PATCH /admin/questions/reports/{id}
async fn admin_update_report(
    path: web::Path<String>,
    body: web::Json<UpdateReportStatusRequest>,
    data: web::Data<AppState<'_>>,
) -> HttpResponse {
    let report_id = path.into_inner();
    let dao = QuestionFeedbackDao::new(data.context.soal.pool.clone());

    match dao.update_report_status(&report_id, &body).await {
        Ok(_)  => HttpResponse::Ok().json(serde_json::json!({ "message": "Status diperbarui" })),
        Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
    }
}

// ── Route registration ────────────────────────────────────────────────────────

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // User-facing routes (auth required — enforced by global AuthMiddleware)
    cfg.service(
        web::scope("/questions")
            .route("/{id}/rate",    web::post().to(rate_question))
            .route("/{id}/ratings", web::get().to(get_ratings))
            .route("/{id}/report",  web::post().to(report_question)),
    );

    // Admin routes (additionally wrapped with AdminMiddleware)
    cfg.service(
        web::scope("/admin/questions")
            .wrap(AdminMiddleware::new())
            .route("/reports",        web::get().to(admin_list_reports))
            .route("/most-reported",  web::get().to(admin_most_reported))
            .route("/reports/{id}",   web::patch().to(admin_update_report)),
    );
}
