use crate::controller::log_request;
use crate::model::score_history::ScoreHistoryQuery;
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/analytics")
            .route("/score-history", web::get().to(get_score_history)),
    );
}

/// GET /analytics/score-history — skor per sesi untuk chart perkembangan
async fn get_score_history(
    query: web::Query<ScoreHistoryQuery>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/analytics/score-history", &data.connections);

    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    match data.context.quiz_sessions.get_user_score_history(
        &user_id,
        query.package_id,
        query.category.as_deref(),
        query.days,
    ).await {
        Ok(resp) => HttpResponse::Ok().json(resp),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to fetch score history: {}", e),
        }),
    }
}
