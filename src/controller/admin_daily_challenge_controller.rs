use crate::middleware::admin_middleware::AdminMiddleware;
use crate::middleware::auth_middleware::AuthenticatedUser;
use crate::model::daily_challenge::{AdminChallengeResponse, SetChallengeRequest};
use crate::AppState;
use actix_web::{web, HttpResponse};
use chrono::NaiveDate;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/daily-challenge")
            .wrap(AdminMiddleware::new())
            .route("", web::post().to(set_challenge))
            .route("", web::get().to(list_challenges)),
    );
}

/// POST /admin/daily-challenge — create or update a daily challenge
async fn set_challenge(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
    req: web::Json<SetChallengeRequest>,
) -> HttpResponse {
    let challenge_date = match NaiveDate::parse_from_str(&req.challenge_date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid date format. Use YYYY-MM-DD"
            }));
        }
    };

    match state
        .context
        .daily_challenges
        .upsert_challenge(&user.user_id, req.soal_id, challenge_date)
        .await
    {
        Ok(challenge) => HttpResponse::Ok().json(AdminChallengeResponse {
            id: challenge.id,
            challenge_date: challenge.challenge_date.to_string(),
            soal_id: challenge.soal_id,
            created_by: challenge.created_by,
            created_at: challenge.created_at.to_rfc3339(),
        }),
        Err(e) => {
            eprintln!("DB error upsert_challenge: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to set daily challenge"
            }))
        }
    }
}

/// GET /admin/daily-challenge — list recent challenges
async fn list_challenges(state: web::Data<AppState<'_>>) -> HttpResponse {
    match state.context.daily_challenges.list_challenges(30).await {
        Ok(challenges) => {
            let response: Vec<AdminChallengeResponse> = challenges
                .into_iter()
                .map(|c| AdminChallengeResponse {
                    id: c.id,
                    challenge_date: c.challenge_date.to_string(),
                    soal_id: c.soal_id,
                    created_by: c.created_by,
                    created_at: c.created_at.to_rfc3339(),
                })
                .collect();
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            eprintln!("DB error list_challenges: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to list challenges"
            }))
        }
    }
}
