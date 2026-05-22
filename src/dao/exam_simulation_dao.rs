use sqlx::MySqlPool;
use std::sync::Arc;
use serde_json::Value as JsonValue;
use crate::model::exam_simulation::{
    ExamSimulation, CreateExamSimulationRequest, UpdateExamSimulationRequest,
    SimulasiUserAttempt,
};

pub struct ExamSimulationDao {
    pool: Arc<MySqlPool>,
}

impl ExamSimulationDao {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    /// List for admin — with aggregate stats per simulasi.
    pub async fn list_admin(&self, is_active: Option<bool>) -> Result<Vec<ExamSimulation>, sqlx::Error> {
        let mut sql = String::from(r#"
            SELECT
                s.id, s.nama_simulasi, s.deskripsi, s.paket_soal_id, ps.nama_paket_soal AS paket_soal_nama,
                s.generation_mode, s.generation_config,
                s.duration_minutes, s.total_questions, s.passing_score,
                s.is_premium, s.max_attempts, s.is_active,
                s.created_at, s.updated_at,
                COALESCE(stats.total_attempts, 0) AS total_attempts,
                stats.avg_score AS avg_score,
                stats.pass_rate AS pass_rate
            FROM exam_simulations s
            LEFT JOIN paket_soal ps ON ps.id = s.paket_soal_id
            LEFT JOIN (
                SELECT simulasi_id,
                       COUNT(*) AS total_attempts,
                       AVG(score) AS avg_score,
                       AVG(CASE WHEN is_passed THEN 100.0 ELSE 0.0 END) AS pass_rate
                FROM simulasi_user_attempts
                WHERE completed_at IS NOT NULL
                GROUP BY simulasi_id
            ) stats ON stats.simulasi_id = s.id
        "#);
        if is_active.is_some() {
            sql.push_str(" WHERE s.is_active = ?");
        }
        sql.push_str(" ORDER BY s.created_at DESC");

        let mut q = sqlx::query_as::<_, ExamSimulation>(&sql);
        if let Some(active) = is_active {
            q = q.bind(active);
        }
        q.fetch_all(&*self.pool).await
    }

    /// List for users — only active, with user-specific attempt count + best score.
    pub async fn list_for_user(&self, user_id: &str) -> Result<Vec<ExamSimulation>, sqlx::Error> {
        sqlx::query_as::<_, ExamSimulation>(r#"
            SELECT
                s.id, s.nama_simulasi, s.deskripsi, s.paket_soal_id, ps.nama_paket_soal AS paket_soal_nama,
                s.generation_mode, s.generation_config,
                s.duration_minutes, s.total_questions, s.passing_score,
                s.is_premium, s.max_attempts, s.is_active,
                s.created_at, s.updated_at,
                COALESCE(ua.user_attempts, 0) AS user_attempts,
                ua.best_score AS best_score,
                CASE WHEN s.max_attempts = 0 OR COALESCE(ua.user_attempts, 0) < s.max_attempts
                     THEN TRUE ELSE FALSE END AS can_attempt
            FROM exam_simulations s
            LEFT JOIN paket_soal ps ON ps.id = s.paket_soal_id
            LEFT JOIN (
                SELECT simulasi_id, COUNT(*) AS user_attempts, MAX(score) AS best_score
                FROM simulasi_user_attempts
                WHERE user_id = ?
                GROUP BY simulasi_id
            ) ua ON ua.simulasi_id = s.id
            WHERE s.is_active = TRUE
            ORDER BY s.created_at DESC
        "#)
        .bind(user_id)
        .fetch_all(&*self.pool).await
    }

    pub async fn get_by_id(&self, id: i32) -> Result<ExamSimulation, sqlx::Error> {
        sqlx::query_as::<_, ExamSimulation>(r#"
            SELECT s.id, s.nama_simulasi, s.deskripsi, s.paket_soal_id, ps.nama_paket_soal AS paket_soal_nama,
                   s.generation_mode, s.generation_config,
                   s.duration_minutes, s.total_questions, s.passing_score,
                   s.is_premium, s.max_attempts, s.is_active,
                   s.created_at, s.updated_at
            FROM exam_simulations s
            LEFT JOIN paket_soal ps ON ps.id = s.paket_soal_id
            WHERE s.id = ?
        "#)
        .bind(id)
        .fetch_one(&*self.pool).await
    }

    pub async fn create(&self, req: &CreateExamSimulationRequest) -> Result<i32, sqlx::Error> {
        let result = sqlx::query(r#"
            INSERT INTO exam_simulations
                (nama_simulasi, deskripsi, paket_soal_id, generation_mode, generation_config,
                 duration_minutes, total_questions, passing_score, is_premium, max_attempts, is_active)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, TRUE)
        "#)
        .bind(&req.nama_simulasi)
        .bind(&req.deskripsi)
        .bind(req.paket_soal_id)
        .bind(&req.generation_mode)
        .bind(&req.generation_config)
        .bind(req.duration_minutes)
        .bind(req.total_questions)
        .bind(req.passing_score)
        .bind(req.is_premium)
        .bind(req.max_attempts)
        .execute(&*self.pool).await?;
        Ok(result.last_insert_id() as i32)
    }

    pub async fn update(&self, id: i32, req: &UpdateExamSimulationRequest) -> Result<(), sqlx::Error> {
        // Build dynamic SET clause to allow partial updates.
        let mut sets: Vec<&'static str> = Vec::new();
        if req.nama_simulasi.is_some()     { sets.push("nama_simulasi = ?"); }
        if req.deskripsi.is_some()         { sets.push("deskripsi = ?"); }
        if req.paket_soal_id.is_some()     { sets.push("paket_soal_id = ?"); }
        if req.generation_mode.is_some()   { sets.push("generation_mode = ?"); }
        if req.generation_config.is_some() { sets.push("generation_config = ?"); }
        if req.duration_minutes.is_some()  { sets.push("duration_minutes = ?"); }
        if req.total_questions.is_some()   { sets.push("total_questions = ?"); }
        if req.passing_score.is_some()     { sets.push("passing_score = ?"); }
        if req.is_premium.is_some()        { sets.push("is_premium = ?"); }
        if req.max_attempts.is_some()      { sets.push("max_attempts = ?"); }
        if req.is_active.is_some()         { sets.push("is_active = ?"); }
        if sets.is_empty() { return Ok(()); }

        let sql = format!("UPDATE exam_simulations SET {} WHERE id = ?", sets.join(", "));
        let mut q = sqlx::query(&sql);
        if let Some(v) = &req.nama_simulasi     { q = q.bind(v); }
        if let Some(v) = &req.deskripsi         { q = q.bind(v); }
        if let Some(v) = &req.paket_soal_id     { q = q.bind(v); }
        if let Some(v) = &req.generation_mode   { q = q.bind(v); }
        if let Some(v) = &req.generation_config { q = q.bind(v); }
        if let Some(v) = &req.duration_minutes  { q = q.bind(v); }
        if let Some(v) = &req.total_questions   { q = q.bind(v); }
        if let Some(v) = &req.passing_score     { q = q.bind(v); }
        if let Some(v) = &req.is_premium        { q = q.bind(v); }
        if let Some(v) = &req.max_attempts      { q = q.bind(v); }
        if let Some(v) = &req.is_active         { q = q.bind(v); }
        q = q.bind(id);
        q.execute(&*self.pool).await?;
        Ok(())
    }

    /// Soft-delete: set is_active = false.
    pub async fn soft_delete(&self, id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE exam_simulations SET is_active = FALSE WHERE id = ?")
            .bind(id)
            .execute(&*self.pool).await?;
        Ok(())
    }

    pub async fn count_user_attempts(&self, simulasi_id: i32, user_id: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM simulasi_user_attempts WHERE simulasi_id = ? AND user_id = ?"
        )
        .bind(simulasi_id)
        .bind(user_id)
        .fetch_one(&*self.pool).await?;
        Ok(row.0)
    }

    pub async fn start_attempt(
        &self,
        simulasi_id: i32,
        user_id: &str,
        quiz_session_id: &str,
        attempt_number: i32,
    ) -> Result<i32, sqlx::Error> {
        let result = sqlx::query(r#"
            INSERT INTO simulasi_user_attempts
                (simulasi_id, user_id, quiz_session_id, attempt_number)
            VALUES (?, ?, ?, ?)
        "#)
        .bind(simulasi_id)
        .bind(user_id)
        .bind(quiz_session_id)
        .bind(attempt_number)
        .execute(&*self.pool).await?;
        Ok(result.last_insert_id() as i32)
    }

    /// Update the attempt row when the underlying quiz_session completes.
    pub async fn complete_attempt(
        &self,
        quiz_session_id: &str,
        score: i32,
        is_passed: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(r#"
            UPDATE simulasi_user_attempts
            SET score = ?, is_passed = ?, completed_at = CURRENT_TIMESTAMP
            WHERE quiz_session_id = ?
        "#)
        .bind(score)
        .bind(is_passed)
        .bind(quiz_session_id)
        .execute(&*self.pool).await?;
        Ok(())
    }

    pub async fn list_attempts_by_simulasi(&self, simulasi_id: i32) -> Result<Vec<SimulasiUserAttempt>, sqlx::Error> {
        sqlx::query_as::<_, SimulasiUserAttempt>(r#"
            SELECT a.id, a.simulasi_id, a.user_id, a.quiz_session_id, a.attempt_number,
                   a.score, a.is_passed, a.completed_at, a.created_at,
                   u.email AS user_email, u.display_name AS user_display_name
            FROM simulasi_user_attempts a
            LEFT JOIN users u ON u.id = a.user_id
            WHERE a.simulasi_id = ?
            ORDER BY a.created_at DESC
        "#)
        .bind(simulasi_id)
        .fetch_all(&*self.pool).await
    }

    pub async fn list_attempts_for_user(&self, simulasi_id: i32, user_id: &str) -> Result<Vec<SimulasiUserAttempt>, sqlx::Error> {
        sqlx::query_as::<_, SimulasiUserAttempt>(r#"
            SELECT id, simulasi_id, user_id, quiz_session_id, attempt_number,
                   score, is_passed, completed_at, created_at,
                   NULL AS user_email, NULL AS user_display_name
            FROM simulasi_user_attempts
            WHERE simulasi_id = ? AND user_id = ?
            ORDER BY attempt_number DESC
        "#)
        .bind(simulasi_id)
        .bind(user_id)
        .fetch_all(&*self.pool).await
    }

    pub async fn admin_stats(&self) -> Result<JsonValue, sqlx::Error> {
        // CAST avg_pass_rate to SIGNED so sqlx can decode it as i64 (MySQL's AVG returns DECIMAL
        // which sqlx 0.7 cannot decode into f64 directly).
        let row: (i64, i64, i64, i64) = sqlx::query_as(r#"
            SELECT
                (SELECT COUNT(*) FROM exam_simulations WHERE is_active = TRUE) AS total_active,
                (SELECT COUNT(*) FROM simulasi_user_attempts
                    WHERE completed_at IS NOT NULL
                      AND completed_at >= CURRENT_DATE) AS attempts_today,
                (SELECT COUNT(*) FROM simulasi_user_attempts
                    WHERE completed_at IS NOT NULL
                      AND completed_at >= DATE_SUB(CURRENT_DATE, INTERVAL 7 DAY)) AS attempts_week,
                CAST(COALESCE(
                    (SELECT AVG(CASE WHEN is_passed THEN 100 ELSE 0 END)
                     FROM simulasi_user_attempts WHERE completed_at IS NOT NULL),
                    0
                ) AS SIGNED) AS avg_pass_rate
        "#).fetch_one(&*self.pool).await?;

        Ok(serde_json::json!({
            "total_active":     row.0,
            "attempts_today":   row.1,
            "attempts_week":    row.2,
            "avg_pass_rate":    row.3,
        }))
    }
}
