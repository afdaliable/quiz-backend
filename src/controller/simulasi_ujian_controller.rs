use actix_web::{get, post, web, HttpResponse, Responder};
use chrono::Utc;
use uuid::Uuid;
use sqlx::Row;
use serde_json::{json, Value as JsonValue};

use crate::AppState;
use crate::dao::exam_simulation_dao::ExamSimulationDao;
use crate::middleware::auth_middleware::AuthenticatedUser;
use crate::service::simulasi_generation_service::generate_questions_for_simulasi;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_simulasi)
       .service(get_simulasi_detail)
       .service(start_simulasi)
       .service(get_my_attempts);
}

#[get("/simulasi-ujian")]
async fn list_simulasi(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
) -> impl Responder {
    let dao = ExamSimulationDao::new(state.context.soal.pool.clone());
    match dao.list_for_user(&user.user_id).await {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(e) => {
            eprintln!("Error listing simulasi: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch simulasi"}))
        }
    }
}

#[get("/simulasi-ujian/{id}")]
async fn get_simulasi_detail(
    path: web::Path<i32>,
    state: web::Data<AppState<'_>>,
    _user: AuthenticatedUser,
) -> impl Responder {
    let id = path.into_inner();
    let dao = ExamSimulationDao::new(state.context.soal.pool.clone());
    match dao.get_by_id(id).await {
        Ok(sim) => HttpResponse::Ok().json(sim),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(json!({"error": "Simulasi not found"})),
        Err(e) => {
            eprintln!("Error fetching simulasi {}: {:?}", id, e);
            HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch simulasi"}))
        }
    }
}

#[post("/simulasi-ujian/{id}/start")]
async fn start_simulasi(
    path: web::Path<i32>,
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
) -> impl Responder {
    let simulasi_id = path.into_inner();
    let pool = state.context.soal.pool.clone();
    let dao = ExamSimulationDao::new(pool.clone());

    let sim = match dao.get_by_id(simulasi_id).await {
        Ok(s) => s,
        Err(sqlx::Error::RowNotFound) => {
            return HttpResponse::NotFound().json(json!({"error": "Simulasi not found"}));
        }
        Err(e) => {
            eprintln!("Error fetching simulasi: {:?}", e);
            return HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch simulasi"}));
        }
    };

    if !sim.is_active {
        return HttpResponse::BadRequest().json(json!({"error": "Simulasi tidak aktif"}));
    }

    // Attempt-limit check.
    let used = match dao.count_user_attempts(simulasi_id, &user.user_id).await {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Error counting attempts: {:?}", e);
            return HttpResponse::InternalServerError().json(json!({"error": "Failed to check attempts"}));
        }
    };
    if sim.max_attempts > 0 && used as i32 >= sim.max_attempts {
        return HttpResponse::BadRequest().json(json!({
            "error": "Batas attempt sudah tercapai",
            "max_attempts": sim.max_attempts,
            "used": used,
        }));
    }

    // Generate question IDs — simulasi_template fetches in insertion order to preserve section order.
    let question_ids: Vec<i32> = if sim.generation_mode == "simulasi_template" {
        let paket_id = match sim.paket_soal_id {
            Some(id) => id,
            None => return HttpResponse::BadRequest().json(json!({"error": "paket_soal_id required for simulasi_template"})),
        };
        match sqlx::query_scalar::<_, i32>(
            "SELECT soal_id FROM paket_soal_items WHERE paket_soal_id = ? ORDER BY id"
        )
        .bind(paket_id)
        .fetch_all(&*pool).await {
            Ok(ids) => ids,
            Err(e) => {
                eprintln!("Error fetching ordered questions: {:?}", e);
                return HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch questions"}));
            }
        }
    } else {
        match generate_questions_for_simulasi(&*pool, &sim).await {
            Ok(ids) => ids,
            Err(msg) => {
                eprintln!("Generation failed: {}", msg);
                return HttpResponse::BadRequest().json(json!({"error": msg}));
            }
        }
    };

    // Build subtest_index boundaries from sections_json
    let sections: Vec<JsonValue> = sim.sections_json
        .as_ref()
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut subtest_boundaries: Vec<(usize, usize, usize)> = Vec::new(); // (start, end_exclusive, section_idx)
    let mut boundary_pos = 0usize;
    for (i, sec) in sections.iter().enumerate() {
        let count = sec.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        subtest_boundaries.push((boundary_pos, boundary_pos + count, i));
        boundary_pos += count;
    }

    let get_subtest_index = |qpos: usize| -> usize {
        for &(start, end, idx) in &subtest_boundaries {
            if qpos >= start && qpos < end { return idx; }
        }
        0
    };

    // Fetch question payload (mirror RandomSessionSoal shape, withholding correct_answer).
    let placeholders = question_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let rows_sql = format!(
        "SELECT id, soal, question_type, opt1, opt2, opt3, opt4, opt5, solution, modul, pelajaran, tag, option_scores \
         FROM soal WHERE id IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&rows_sql);
    for id in &question_ids { q = q.bind(*id); }
    let rows = match q.fetch_all(&*pool).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error fetching question content: {:?}", e);
            return HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch questions"}));
        }
    };

    let mut soal_map: std::collections::HashMap<i32, JsonValue> = std::collections::HashMap::new();
    for row in rows {
        let id: i32 = row.get("id");
        soal_map.insert(id, json!({
            "id":            id,
            "soal":          row.try_get::<String, _>("soal").ok(),
            "question_type": row.try_get::<String, _>("question_type").ok().unwrap_or_else(|| "multiple_choice".to_string()),
            "opt1":          row.try_get::<Option<String>, _>("opt1").ok().flatten(),
            "opt2":          row.try_get::<Option<String>, _>("opt2").ok().flatten(),
            "opt3":          row.try_get::<Option<String>, _>("opt3").ok().flatten(),
            "opt4":          row.try_get::<Option<String>, _>("opt4").ok().flatten(),
            "opt5":          row.try_get::<Option<String>, _>("opt5").ok().flatten(),
            "solution":      row.try_get::<Option<String>, _>("solution").ok().flatten(),
            "modul":         row.try_get::<Option<String>, _>("modul").ok().flatten(),
            "pelajaran":     row.try_get::<Option<String>, _>("pelajaran").ok().flatten(),
            "tag":           row.try_get::<Option<String>, _>("tag").ok().flatten(),
            "option_scores": row
                .try_get::<Option<String>, _>("option_scores")
                .ok()
                .flatten()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
        }));
    }
    let questions: Vec<JsonValue> = question_ids.iter().enumerate()
        .filter_map(|(pos, id)| {
            soal_map.get(id).cloned().map(|mut q| {
                if let Some(obj) = q.as_object_mut() {
                    obj.insert("subtest_index".to_string(), json!(get_subtest_index(pos)));
                }
                q
            })
        })
        .collect();

    // Create the quiz_session row for this attempt.
    let session_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let question_ids_json = serde_json::to_string(&question_ids).unwrap_or_else(|_| "[]".to_string());
    let total_time = sim.duration_minutes * 60;

    let insert_res = sqlx::query(r#"
        INSERT INTO quiz_sessions (
            id, user_id, paket_soal_id, kategori_soal, nama_paket_soal,
            session_type, simulasi_id, question_ids,
            total_time, current_question, is_completed, score,
            correct_answers, incorrect_answers, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, 'simulasi', ?, ?, ?, 0, FALSE, 0, 0, 0, ?, ?)
    "#)
    .bind(&session_id)
    .bind(&user.user_id)
    .bind(sim.paket_soal_id)
    .bind(sim.paket_soal_nama.clone().unwrap_or_else(|| "Simulasi".to_string()))
    .bind(&sim.nama_simulasi)
    .bind(sim.id)
    .bind(&question_ids_json)
    .bind(total_time)
    .bind(now)
    .bind(now)
    .execute(&*pool).await;

    if let Err(e) = insert_res {
        eprintln!("Error creating quiz_session: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "Failed to create session"}));
    }

    // Insert simulasi_user_attempts row.
    let attempt_number = (used as i32) + 1;
    if let Err(e) = dao.start_attempt(simulasi_id, &user.user_id, &session_id, attempt_number).await {
        eprintln!("Error inserting attempt row: {:?}", e);
        // Don't fail the request — the quiz_session is already created.
    }

    HttpResponse::Ok().json(json!({
        "session_id":       session_id,
        "simulasi_id":      simulasi_id,
        "attempt_number":   attempt_number,
        "duration_minutes": sim.duration_minutes,
        "total_questions":  sim.total_questions,
        "passing_score":    sim.passing_score,
        "navigation_mode":  sim.navigation_mode,
        "sections":         sections,
        "questions":        questions,
    }))
}

#[get("/simulasi-ujian/{id}/my-attempts")]
async fn get_my_attempts(
    path: web::Path<i32>,
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
) -> impl Responder {
    let simulasi_id = path.into_inner();
    let dao = ExamSimulationDao::new(state.context.soal.pool.clone());
    match dao.list_attempts_for_user(simulasi_id, &user.user_id).await {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(e) => {
            eprintln!("Error listing my attempts: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"error": "Failed to fetch attempts"}))
        }
    }
}
