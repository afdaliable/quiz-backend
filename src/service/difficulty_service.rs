//! AFD-206: Difficulty auto-calculation service.
//!
//! Two responsibilities:
//! 1. `upsert_question_stats` — called fire-and-forget after each quiz completion.
//!    UPSERTs into `question_attempt_stats` (total_attempts, correct_count, total_time_sec).
//! 2. `recalculate_difficulty` — weekly cron job.
//!    Reads stats, computes p_value, updates soal, creates admin_alerts for mismatches.

use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;

/// Map correct_answer string to 0-based option index.
fn correct_answer_index(s: &str) -> Option<i32> {
    match s {
        "opt1" => Some(0),
        "opt2" => Some(1),
        "opt3" => Some(2),
        "opt4" => Some(3),
        "opt5" => Some(4),
        _ => None,
    }
}

/// Map p_value to difficulty label.
fn p_value_to_difficulty(p: f64) -> &'static str {
    if p >= 0.70 { "easy" } else if p >= 0.40 { "medium" } else { "hard" }
}

/// Convert difficulty label to numeric level for comparison.
fn difficulty_level(d: &str) -> i32 {
    match d { "easy" => 2, "medium" => 1, _ => 0 }
}

/// UPSERT attempt stats for a completed quiz session.
///
/// `question_ids`: ordered list of question IDs that were in the session.
/// `user_answers`: Vec<Option<i32>> — 0-based option index chosen, None = skipped.
/// `time_spent_sec`: total seconds the user spent (total_time − time_remaining).
pub async fn upsert_question_stats(
    pool: Arc<MySqlPool>,
    question_ids: Vec<i32>,
    user_answers: Vec<Option<i32>>,
    time_spent_sec: i32,
) {
    if question_ids.is_empty() {
        return;
    }

    // Fetch correct_answer for each question
    let placeholders = question_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let query_str = format!(
        "SELECT id, correct_answer FROM soal WHERE id IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&query_str);
    for id in &question_ids {
        q = q.bind(*id);
    }

    let rows = match q.fetch_all(&*pool).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[difficulty_service] fetch correct_answer failed: {:?}", e);
            return;
        }
    };

    let correct_map: HashMap<i32, String> = rows
        .into_iter()
        .filter_map(|row| {
            use sqlx::Row;
            let id: i32 = row.get("id");
            let ca: Option<String> = row.get("correct_answer");
            ca.map(|c| (id, c))
        })
        .collect();

    let n_answered = user_answers.iter().filter(|a| a.is_some()).count().max(1) as i32;
    let time_per_q = time_spent_sec / n_answered; // distribute evenly

    // Build single bulk INSERT instead of N separate queries
    let mut placeholders: Vec<&str> = Vec::with_capacity(question_ids.len());
    let mut params: Vec<(i32, i32, i64)> = Vec::with_capacity(question_ids.len());

    for (idx, &qid) in question_ids.iter().enumerate() {
        let user_answer = user_answers.get(idx).and_then(|a| *a);
        let is_correct: i32 = if let (Some(ua), Some(ca_str)) = (user_answer, correct_map.get(&qid)) {
            if Some(ua) == correct_answer_index(ca_str) { 1 } else { 0 }
        } else {
            0
        };
        let time_contrib = if user_answer.is_some() { time_per_q } else { 0 };
        placeholders.push("(?, 1, ?, ?)");
        params.push((qid, is_correct, time_contrib as i64));
    }

    let query_str = format!(
        r#"
        INSERT INTO question_attempt_stats (question_id, total_attempts, correct_count, total_time_sec)
        VALUES {}
        ON DUPLICATE KEY UPDATE
            total_attempts = total_attempts + 1,
            correct_count  = correct_count  + VALUES(correct_count),
            total_time_sec = total_time_sec + VALUES(total_time_sec)
        "#,
        placeholders.join(", ")
    );

    let mut q = sqlx::query(&query_str);
    for (qid, is_correct, time_contrib) in params {
        q = q.bind(qid).bind(is_correct).bind(time_contrib);
    }

    if let Err(e) = q.execute(&*pool).await {
        eprintln!("[difficulty_service] bulk UPSERT qas err: {:?}", e);
    }
}

/// Weekly cron: recalculate p_value and difficulty_calc for all questions
/// with at least 50 attempts. Inserts admin_alerts for mismatches.
pub async fn recalculate_difficulty(pool: Arc<MySqlPool>) {
    eprintln!("[difficulty_service] recalculate_difficulty started");

    let rows = match sqlx::query(
        r#"
        SELECT qas.question_id, qas.total_attempts, qas.correct_count, qas.total_time_sec,
               s.difficulty_est
        FROM question_attempt_stats qas
        JOIN soal s ON s.id = qas.question_id
        WHERE qas.total_attempts >= 50
        "#,
    )
    .fetch_all(&*pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[difficulty_service] fetch stats failed: {:?}", e);
            return;
        }
    };

    let mut updated = 0u32;
    let mut alerted = 0u32;

    for row in rows {
        use sqlx::Row;
        let question_id: i32 = row.get("question_id");
        let total_attempts: i32 = row.get("total_attempts");
        let correct_count: i32 = row.get("correct_count");
        let total_time_sec: i64 = row.get("total_time_sec");
        let difficulty_est: String = row.try_get("difficulty_est")
            .unwrap_or_else(|_| "medium".to_string());

        let p_value = correct_count as f64 / total_attempts as f64;
        let avg_time = total_time_sec as f64 / total_attempts as f64;
        let difficulty_calc = p_value_to_difficulty(p_value);

        // Update soal
        if let Err(e) = sqlx::query(
            r#"
            UPDATE soal
            SET p_value = ?, avg_time_sec = ?, attempt_count = ?, difficulty_calc = ?
            WHERE id = ?
            "#,
        )
        .bind(p_value)
        .bind(avg_time)
        .bind(total_attempts)
        .bind(difficulty_calc)
        .bind(question_id)
        .execute(&*pool)
        .await
        {
            eprintln!("[difficulty_service] update soal q={} err: {:?}", question_id, e);
            continue;
        }
        updated += 1;

        // Detect mismatch: difficulty levels differ
        if difficulty_level(difficulty_calc) != difficulty_level(&difficulty_est) {
            // Skip if an unread mismatch alert already exists for this question
            let existing: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM admin_alerts
                WHERE type = 'difficulty_mismatch'
                  AND entity_type = 'question'
                  AND entity_id = ?
                  AND is_read = FALSE
                "#,
            )
            .bind(question_id.to_string())
            .fetch_one(&*pool)
            .await
            .unwrap_or(0);

            if existing == 0 {
                let detail = serde_json::json!({
                    "question_id": question_id,
                    "difficulty_est": difficulty_est,
                    "difficulty_calc": difficulty_calc,
                    "p_value": (p_value * 1000.0).round() / 1000.0,
                    "total_attempts": total_attempts,
                });
                if let Err(e) = sqlx::query(
                    r#"
                    INSERT INTO admin_alerts (type, entity_type, entity_id, detail)
                    VALUES ('difficulty_mismatch', 'question', ?, ?)
                    "#,
                )
                .bind(question_id.to_string())
                .bind(detail.to_string())
                .execute(&*pool)
                .await
                {
                    eprintln!("[difficulty_service] insert alert q={} err: {:?}", question_id, e);
                } else {
                    alerted += 1;
                }
            }
        }
    }

    eprintln!(
        "[difficulty_service] recalculate_difficulty done: {} questions updated, {} alerts created",
        updated, alerted
    );
}
