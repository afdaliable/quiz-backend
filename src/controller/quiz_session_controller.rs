use actix_web::{web, HttpResponse, HttpRequest};
use crate::model::{
    QuizSession, QuizSessionResponse, CreateQuizSessionRequest,
    UpdateQuizSessionRequest, CompleteQuizSessionRequest, StartRandomSessionRequest,
};
use crate::AppState;
use crate::middleware::auth_middleware::AuthenticatedUser;

pub async fn create_quiz_session(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
    req: web::Json<CreateQuizSessionRequest>,
) -> HttpResponse {
    let user_id = &user.user_id;

    match state.context.quiz_sessions.create_quiz_session(user_id, &req).await {
        Ok(session) => {
            let response: QuizSessionResponse = session.into();
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            eprintln!("Error creating quiz session: {:?}", e);
            HttpResponse::InternalServerError().json({
                serde_json::json!({
                    "success": false,
                    "message": "Failed to create quiz session"
                })
            })
        }
    }
}

pub async fn start_random_session(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
    req: web::Json<StartRandomSessionRequest>,
) -> HttpResponse {
    let user_id = &user.user_id;

    // Validasi: count hanya 10, 20, atau 30
    if ![10u32, 20, 30].contains(&req.count) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "message": "count harus 10, 20, atau 30"
        }));
    }

    match state.context.quiz_sessions.create_random_session(user_id, &req).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(sqlx::Error::RowNotFound) => {
            HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "message": "Tidak ada soal yang tersedia untuk kriteria yang dipilih"
            }))
        }
        Err(e) => {
            eprintln!("Error starting random session: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "Gagal memulai sesi latihan random"
            }))
        }
    }
}

pub async fn get_quiz_session(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let user_id = &user.user_id;

    match state.context.quiz_sessions.get_quiz_session_by_id(&session_id, user_id).await {
        Ok(session) => {
            let response: QuizSessionResponse = session.into();
            HttpResponse::Ok().json(response)
        }
        Err(sqlx::Error::RowNotFound) => {
            HttpResponse::NotFound().json({
                serde_json::json!({
                    "success": false,
                    "message": "Quiz session not found"
                })
            })
        }
        Err(e) => {
            eprintln!("Error getting quiz session: {:?}", e);
            HttpResponse::InternalServerError().json({
                serde_json::json!({
                    "success": false,
                    "message": "Failed to get quiz session"
                })
            })
        }
    }
}

pub async fn save_quiz_progress(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
    path: web::Path<String>,
    req: web::Json<UpdateQuizSessionRequest>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let user_id = &user.user_id;

    match state.context.quiz_sessions.update_quiz_session(&session_id, user_id, &req).await {
        Ok(session) => {
            let response: QuizSessionResponse = session.into();
            HttpResponse::Ok().json(response)
        }
        Err(sqlx::Error::RowNotFound) => {
            HttpResponse::NotFound().json({
                serde_json::json!({
                    "success": false,
                    "message": "Quiz session not found or already completed"
                })
            })
        }
        Err(e) => {
            eprintln!("Error updating quiz session: {:?}", e);
            HttpResponse::InternalServerError().json({
                serde_json::json!({
                    "success": false,
                    "message": "Failed to save quiz progress"
                })
            })
        }
    }
}

pub async fn complete_quiz_session(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
    path: web::Path<String>,
    req: web::Json<CompleteQuizSessionRequest>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let user_id = &user.user_id;

    match state.context.quiz_sessions.complete_quiz_session(&session_id, user_id, &req).await {
        Ok(session) => {
            let response: QuizSessionResponse = session.into();
            HttpResponse::Ok().json(response)
        }
        Err(sqlx::Error::RowNotFound) => {
            HttpResponse::NotFound().json({
                serde_json::json!({
                    "success": false,
                    "message": "Quiz session not found or already completed"
                })
            })
        }
        Err(e) => {
            eprintln!("Error completing quiz session: {:?}", e);
            HttpResponse::InternalServerError().json({
                serde_json::json!({
                    "success": false,
                    "message": "Failed to complete quiz session"
                })
            })
        }
    }
}

pub async fn get_active_quiz_sessions(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
) -> HttpResponse {
    let user_id = &user.user_id;

    match state.context.quiz_sessions.get_user_active_sessions(user_id).await {
        Ok(sessions) => {
            let responses: Vec<QuizSessionResponse> = sessions
                .into_iter()
                .map(|s| s.into())
                .collect();
            HttpResponse::Ok().json(responses)
        }
        Err(e) => {
            eprintln!("Error getting active sessions: {:?}", e);
            HttpResponse::InternalServerError().json({
                serde_json::json!({
                    "success": false,
                    "message": "Failed to get active sessions"
                })
            })
        }
    }
}

pub async fn get_completed_quiz_sessions(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let user_id = &user.user_id;
    let limit = query.get("limit")
        .and_then(|l| l.parse::<i32>().ok())
        .or(Some(10));

    match state.context.quiz_sessions.get_user_completed_sessions(user_id, limit).await {
        Ok(sessions) => {
            let responses: Vec<QuizSessionResponse> = sessions
                .into_iter()
                .map(|s| s.into())
                .collect();
            HttpResponse::Ok().json(responses)
        }
        Err(e) => {
            eprintln!("Error getting completed sessions: {:?}", e);
            HttpResponse::InternalServerError().json({
                serde_json::json!({
                    "success": false,
                    "message": "Failed to get completed sessions"
                })
            })
        }
    }
}

pub async fn delete_quiz_session(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
    path: web::Path<String>,
) -> HttpResponse {
    let session_id = path.into_inner();
    let user_id = &user.user_id;

    match state.context.quiz_sessions.delete_quiz_session(&session_id, user_id).await {
        Ok(()) => {
            HttpResponse::Ok().json({
                serde_json::json!({
                    "success": true,
                    "message": "Quiz session deleted successfully"
                })
            })
        }
        Err(e) => {
            eprintln!("Error deleting quiz session: {:?}", e);
            HttpResponse::InternalServerError().json({
                serde_json::json!({
                    "success": false,
                    "message": "Failed to delete quiz session"
                })
            })
        }
    }
}

pub async fn check_existing_session(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
    req: web::Json<CreateQuizSessionRequest>,
) -> HttpResponse {
    let user_id = &user.user_id;

    match state.context.quiz_sessions.get_session_exists(
        user_id,
        req.paket_soal_id,
        &req.kategori_soal,
        &req.nama_paket_soal
    ).await {
        Ok(Some(session)) => {
            let response: QuizSessionResponse = session.into();
            HttpResponse::Ok().json({
                serde_json::json!({
                    "exists": true,
                    "session": response
                })
            })
        }
        Ok(None) => {
            HttpResponse::Ok().json({
                serde_json::json!({
                    "exists": false,
                    "session": null
                })
            })
        }
        Err(e) => {
            eprintln!("Error checking existing session: {:?}", e);
            HttpResponse::InternalServerError().json({
                serde_json::json!({
                    "success": false,
                    "message": "Failed to check existing session"
                })
            })
        }
    }
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/quiz-session")
            .route("/start", web::post().to(create_quiz_session))
            .route("/start-random", web::post().to(start_random_session))
            .route("/check", web::post().to(check_existing_session))
            .route("/active", web::get().to(get_active_quiz_sessions))
            .route("/completed", web::get().to(get_completed_quiz_sessions))
            .route("/{id}", web::get().to(get_quiz_session))
            .route("/{id}/save", web::put().to(save_quiz_progress))
            .route("/{id}/complete", web::put().to(complete_quiz_session))
            .route("/{id}", web::delete().to(delete_quiz_session))
    );
}
