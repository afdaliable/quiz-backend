use crate::model::daily_challenge::{
    SubmitChallengeRequest, SubmitChallengeResponse, TodayChallengeResponse,
    UserAttemptSummary, DailyChallengeLeaderboardResponse,
};
use crate::middleware::auth_middleware::AuthenticatedUser;
use crate::AppState;
use actix_web::{web, HttpResponse};

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/daily-challenge")
            .route("/today", web::get().to(get_today_challenge))
            .route("/submit", web::post().to(submit_challenge))
            .route("/leaderboard", web::get().to(get_leaderboard))
            .route("/stats", web::get().to(get_user_stats)),
    );
}

/// GET /daily-challenge/today
async fn get_today_challenge(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
) -> HttpResponse {
    let dc = &state.context.daily_challenges;

    let challenge = match dc.get_today().await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "No daily challenge set for today"
            }));
        }
        Err(e) => {
            eprintln!("DB error get_today: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch daily challenge"
            }));
        }
    };

    let soal = match dc.get_soal_for_challenge(challenge.soal_id).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("DB error get_soal_for_challenge: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch question"
            }));
        }
    };

    let existing_attempt = match dc.get_user_attempt_today(&user.user_id).await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("DB error get_user_attempt_today: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch user attempt"
            }));
        }
    };

    let already_answered = existing_attempt.is_some();
    let correct_answer = if already_answered {
        dc.get_correct_answer(challenge.soal_id).await.ok()
    } else {
        None
    };

    let user_attempt = existing_attempt.map(|a| UserAttemptSummary {
        selected_answer: a.selected_answer,
        is_correct: a.is_correct,
        score: a.score,
        time_taken_ms: a.time_taken_ms,
        answered_at: a.answered_at.to_rfc3339(),
    });

    HttpResponse::Ok().json(TodayChallengeResponse {
        challenge_date: challenge.challenge_date.to_string(),
        soal,
        already_answered,
        user_attempt,
        correct_answer,
    })
}

/// POST /daily-challenge/submit
async fn submit_challenge(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
    req: web::Json<SubmitChallengeRequest>,
) -> HttpResponse {
    let dc = &state.context.daily_challenges;

    let challenge = match dc.get_today().await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "No daily challenge set for today"
            }));
        }
        Err(e) => {
            eprintln!("DB error get_today: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }));
        }
    };

    if req.selected_answer < 1 || req.selected_answer > 5 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "selected_answer must be between 1 and 5"
        }));
    }

    if req.time_taken_ms < 0 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "time_taken_ms must be non-negative"
        }));
    }

    let (attempt, _is_new) = match dc
        .submit_attempt(
            &user.user_id,
            &challenge,
            req.selected_answer,
            req.time_taken_ms,
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            eprintln!("DB error submit_attempt: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to submit attempt"
            }));
        }
    };

    let correct_answer = match dc.get_correct_answer(challenge.soal_id).await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("DB error get_correct_answer: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch correct answer"
            }));
        }
    };

    let rank = dc
        .get_user_rank_today(&user.user_id, attempt.score, attempt.time_taken_ms)
        .await
        .unwrap_or(0);

    let total_participants = dc.count_participants_today().await.unwrap_or(0);

    HttpResponse::Ok().json(SubmitChallengeResponse {
        is_correct: attempt.is_correct,
        correct_answer,
        score: attempt.score,
        rank,
        total_participants,
    })
}

/// GET /daily-challenge/leaderboard
async fn get_leaderboard(state: web::Data<AppState<'_>>) -> HttpResponse {
    let dc = &state.context.daily_challenges;

    let challenge = match dc.get_today().await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "No daily challenge set for today"
            }));
        }
        Err(e) => {
            eprintln!("DB error get_today: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }));
        }
    };

    let entries = match dc.get_leaderboard_today().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("DB error get_leaderboard_today: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch leaderboard"
            }));
        }
    };

    let total_participants = dc.count_participants_today().await.unwrap_or(0);

    HttpResponse::Ok().json(DailyChallengeLeaderboardResponse {
        challenge_date: challenge.challenge_date.to_string(),
        total_participants,
        entries,
    })
}

/// GET /daily-challenge/stats
async fn get_user_stats(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
) -> HttpResponse {
    match state.context.daily_challenges.get_user_stats(&user.user_id).await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => {
            eprintln!("DB error get_user_stats: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch stats"
            }))
        }
    }
}
