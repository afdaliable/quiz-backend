use actix_web::{web, HttpResponse, HttpRequest};
use chrono::Utc;
use crate::model::{
    QuizSession, QuizSessionResponse, CreateQuizSessionRequest,
    UpdateQuizSessionRequest, CompleteQuizSessionRequest, StartRandomSessionRequest,
};
use crate::model::xp::{XpBreakdownResponse, XpAwardResultResponse, CompleteSessionWithXpResponse};
use crate::service::xp_service::{compute_quiz_xp, award_quiz_xp};
use crate::service::difficulty_service::upsert_question_stats;
use crate::service::redis_service::RedisService;
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
            // Cache in Redis for cross-node consistency
            if let Some(redis_pool) = &state.redis_pool {
                let mut con = redis_pool.progress().as_ref().clone();
                let _ = RedisService::cache_active_session(&mut con, &session.id, &session).await;
            }
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
        Ok(response) => {
            // Cache session in Redis — reconstruct QuizSession from response fields
            if let Some(redis_pool) = &state.redis_pool {
                let question_ids_json = serde_json::to_string(
                    &response.questions.iter().map(|q| q.id).collect::<Vec<i32>>()
                ).unwrap_or_else(|_| "[]".to_string());
                let now = Utc::now();
                let session_to_cache = QuizSession {
                    id: response.session_id.clone(),
                    user_id: user_id.to_string(),
                    paket_soal_id: None,
                    kategori_soal: response.kategori_soal.clone(),
                    nama_paket_soal: response.nama_paket_soal.clone(),
                    session_type: response.session_type.clone(),
                    simulasi_id: None,
                    question_ids: Some(question_ids_json),
                    current_question: 0,
                    answers: None,
                    marked_questions: None,
                    time_remaining: Some(response.total_time),
                    total_time: Some(response.total_time),
                    is_completed: false,
                    score: 0,
                    correct_answers: 0,
                    incorrect_answers: 0,
                    pomodoro_enabled: false,
                    pomodoro_sessions: 0,
                    pomodoro_focus_minutes: 0,
                    pomodoro_questions_answered: 0,
                    created_at: now,
                    updated_at: now,
                };
                let mut con = redis_pool.progress().as_ref().clone();
                let _ = RedisService::cache_active_session(&mut con, &session_to_cache.id, &session_to_cache).await;
            }
            HttpResponse::Ok().json(response)
        }
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

    // Check Redis first (cross-node session consistency)
    if let Some(redis_pool) = &state.redis_pool {
        let mut con = redis_pool.progress().as_ref().clone();
        if let Ok(Some(session)) = RedisService::get_active_session(&mut con, &session_id).await {
            if session.user_id == *user_id {
                let response: QuizSessionResponse = session.into();
                return HttpResponse::Ok().json(response);
            }
        }
    }

    // Fallback to MySQL
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

    // Redis-primary path: update Redis immediately, fire-and-forget MySQL
    if let Some(redis_pool) = &state.redis_pool {
        let mut con = redis_pool.progress().as_ref().clone();
        if let Ok(Some(mut session)) = RedisService::get_active_session(&mut con, &session_id).await {
            if session.user_id == *user_id && !session.is_completed {
                // Apply request fields to cached session
                let now = Utc::now();
                if let Some(cq) = req.current_question {
                    session.current_question = cq;
                }
                if let Some(ref answers) = req.answers {
                    session.answers = serde_json::to_string(answers).ok();
                }
                if let Some(ref marked) = req.marked_questions {
                    session.marked_questions = serde_json::to_string(marked).ok();
                }
                if let Some(tr) = req.time_remaining {
                    session.time_remaining = Some(tr);
                }
                session.updated_at = now;

                // Save updated session back to Redis
                let _ = RedisService::cache_active_session(&mut con, &session_id, &session).await;

                // Fire-and-forget: persist to MySQL in background
                {
                    let pool = state.context.quiz_sessions.pool.clone();
                    let sid = session_id.clone();
                    let uid = user_id.to_string();
                    let req_inner = req.into_inner();
                    tokio::spawn(async move {
                        let now_db = Utc::now();
                        let mut q = "UPDATE quiz_sessions SET updated_at = ?".to_string();
                        if req_inner.current_question.is_some() { q.push_str(", current_question = ?"); }
                        if req_inner.answers.is_some() { q.push_str(", answers = ?"); }
                        if req_inner.marked_questions.is_some() { q.push_str(", marked_questions = ?"); }
                        if req_inner.time_remaining.is_some() { q.push_str(", time_remaining = ?"); }
                        q.push_str(" WHERE id = ? AND user_id = ? AND is_completed = FALSE");

                        let mut sql = sqlx::query(&q).bind(now_db);
                        if let Some(cq) = req_inner.current_question { sql = sql.bind(cq); }
                        if let Some(ref a) = req_inner.answers {
                            sql = sql.bind(serde_json::to_string(a).unwrap_or_default());
                        }
                        if let Some(ref m) = req_inner.marked_questions {
                            sql = sql.bind(serde_json::to_string(m).unwrap_or_default());
                        }
                        if let Some(tr) = req_inner.time_remaining { sql = sql.bind(tr); }
                        let _ = sql.bind(sid).bind(uid).execute(&*pool).await;
                    });
                }

                let response: QuizSessionResponse = session.into();
                return HttpResponse::Ok().json(response);
            }
        }
    }

    // Fallback: synchronous MySQL update (Redis miss or unavailable)
    match state.context.quiz_sessions.update_quiz_session(&session_id, user_id, &req).await {
        Ok(session) => {
            // Cache result in Redis for future requests
            if let Some(redis_pool) = &state.redis_pool {
                let mut con = redis_pool.progress().as_ref().clone();
                let _ = RedisService::cache_active_session(&mut con, &session_id, &session).await;
            }
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
            // Delete from Redis (fire-and-forget — stale TTL is 4h but delete proactively)
            if let Some(redis_pool) = &state.redis_pool {
                let redis_con = redis_pool.progress();
                let sid_redis = session.id.clone();
                tokio::spawn(async move {
                    let mut con = redis_con.as_ref().clone();
                    let _ = RedisService::delete_active_session(&mut con, &sid_redis).await;
                });
            }

            // Fire-and-forget: update question_attempt_stats (AFD-206)
            {
                let pool = state.context.soal.pool.clone();
                let answers = req.answers.clone();
                let time_spent = session.total_time.unwrap_or(0)
                    - session.time_remaining.unwrap_or(0);

                // Determine question_ids: explicit for random, or from paket_soal_items for standard
                let q_ids_json = session.question_ids.clone();
                let paket_soal_id = session.paket_soal_id;

                tokio::spawn(async move {
                    let question_ids: Vec<i32> = if let Some(json) = q_ids_json {
                        serde_json::from_str(&json).unwrap_or_default()
                    } else if let Some(pkg_id) = paket_soal_id {
                        sqlx::query_scalar::<_, i32>(
                            "SELECT soal_id FROM paket_soal_items WHERE paket_soal_id = ? ORDER BY id ASC"
                        )
                        .bind(pkg_id)
                        .fetch_all(&*pool)
                        .await
                        .unwrap_or_default()
                    } else {
                        vec![]
                    };

                    upsert_question_stats(pool, question_ids, answers, time_spent).await;
                });
            }

            // Simulasi attempt hook — update simulasi_user_attempts + compute passing bonus
            let (simulasi_bonus, simulasi_passed_for_xp) = if session.session_type == "simulasi" {
                let pool_sim = state.context.soal.pool.clone();
                let dao = crate::dao::exam_simulation_dao::ExamSimulationDao::new(pool_sim.clone());
                // Resolve passing_score for the linked simulasi.
                let passing = if let Some(sim_id) = session.simulasi_id {
                    match dao.get_by_id(sim_id).await {
                        Ok(sim) => sim.passing_score,
                        Err(_) => 60,
                    }
                } else { 60 };
                let passed = session.score >= passing;

                // Update the attempts row (fire-and-forget).
                {
                    let dao2 = crate::dao::exam_simulation_dao::ExamSimulationDao::new(pool_sim.clone());
                    let sid = session.id.clone();
                    let score = session.score;
                    tokio::spawn(async move {
                        let _ = dao2.complete_attempt(&sid, score, passed).await;
                    });
                }

                (if passed { /* +50% bonus, computed below */ 1 } else { 0 }, passed)
            } else {
                (0, false)
            };

            // Award XP — fire-and-forget, tidak blok response
            let is_study_mode = session.session_type == "study";
            let breakdown = compute_quiz_xp(session.score, session.correct_answers, is_study_mode);

            // For simulasi that passed, give 50% bonus on base XP total.
            let simulasi_bonus_value = if simulasi_bonus == 1 && simulasi_passed_for_xp {
                breakdown.total / 2
            } else { 0 };
            let final_total = breakdown.total + simulasi_bonus_value;

            let xp_breakdown_resp = XpBreakdownResponse {
                quiz_complete: breakdown.quiz_complete,
                correct_answers: breakdown.correct_answers,
                score_bonus: breakdown.score_bonus,
                simulasi_bonus: simulasi_bonus_value,
                total: final_total,
            };

            {
                let pool_xp = state.context.users.pool.clone();
                let uid_xp = user_id.to_string();
                let sid_xp = session.id.clone();
                // Build a breakdown with the bonus included so the awarded XP matches the response.
                let bd = crate::service::xp_service::QuizXpBreakdown {
                    quiz_complete:    breakdown.quiz_complete,
                    correct_answers:  breakdown.correct_answers,
                    score_bonus:      breakdown.score_bonus + simulasi_bonus_value,
                    total:            final_total,
                };
                tokio::spawn(async move {
                    let _ = award_quiz_xp(&pool_xp, &uid_xp, &sid_xp, &bd).await;
                });
            }

            // xp_result tidak tersedia synchronous — XP tetap dihitung di background
            let xp_result_resp: Option<XpAwardResultResponse> = None;

            let resp: QuizSessionResponse = session.into();
            let with_xp = CompleteSessionWithXpResponse {
                id: resp.id,
                user_id: resp.user_id,
                paket_soal_id: resp.paket_soal_id,
                kategori_soal: resp.kategori_soal,
                nama_paket_soal: resp.nama_paket_soal,
                session_type: resp.session_type,
                question_ids: resp.question_ids,
                current_question: resp.current_question,
                answers: resp.answers,
                marked_questions: resp.marked_questions,
                time_remaining: resp.time_remaining,
                total_time: resp.total_time,
                is_completed: resp.is_completed,
                score: resp.score,
                correct_answers: resp.correct_answers,
                incorrect_answers: resp.incorrect_answers,
                created_at: resp.created_at,
                updated_at: resp.updated_at,
                xp_breakdown: Some(xp_breakdown_resp),
                xp_result: xp_result_resp,
            };
            HttpResponse::Ok().json(with_xp)
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
