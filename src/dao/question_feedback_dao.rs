use sqlx::MySqlPool;
use std::sync::Arc;
use crate::model::question_feedback::{
    RatingKind, RatingSummary,
    SubmitReportRequest, ReportWithQuestion, QuestionReportStats, UpdateReportStatusRequest,
};

pub struct QuestionFeedbackDao {
    pool: Arc<MySqlPool>,
}

impl QuestionFeedbackDao {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ── Rating ───────────────────────────────────────────────────────────────

    pub async fn upsert_rating(
        &self,
        question_id: i32,
        user_id: &str,
        rating: &RatingKind,
    ) -> Result<(), sqlx::Error> {
        let r = match rating {
            RatingKind::Helpful   => "helpful",
            RatingKind::Confusing => "confusing",
        };
        sqlx::query(
            r#"INSERT INTO question_ratings (question_id, user_id, rating)
               VALUES (?, ?, ?)
               ON DUPLICATE KEY UPDATE rating = VALUES(rating), updated_at = NOW()"#,
        )
        .bind(question_id)
        .bind(user_id)
        .bind(r)
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_rating(
        &self,
        question_id: i32,
        user_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM question_ratings WHERE question_id = ? AND user_id = ?",
        )
        .bind(question_id)
        .bind(user_id)
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_rating_summary(
        &self,
        question_id: i32,
        user_id: &str,
    ) -> Result<RatingSummary, sqlx::Error> {
        let helpful: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM question_ratings WHERE question_id = ? AND rating = 'helpful'",
        )
        .bind(question_id)
        .fetch_one(&*self.pool)
        .await?;

        let confusing: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM question_ratings WHERE question_id = ? AND rating = 'confusing'",
        )
        .bind(question_id)
        .fetch_one(&*self.pool)
        .await?;

        let user_rating_str: Option<String> = sqlx::query_scalar(
            "SELECT rating FROM question_ratings WHERE question_id = ? AND user_id = ?",
        )
        .bind(question_id)
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await?;

        let user_rating = user_rating_str.map(|r| match r.as_str() {
            "helpful" => RatingKind::Helpful,
            _         => RatingKind::Confusing,
        });

        Ok(RatingSummary {
            helpful_count: helpful,
            confusing_count: confusing,
            user_rating,
        })
    }

    // ── Report ───────────────────────────────────────────────────────────────

    /// Submit laporan. Jika sudah ada laporan pending dari user yang sama, update isinya.
    pub async fn create_report(
        &self,
        question_id: i32,
        user_id: &str,
        req: &SubmitReportRequest,
    ) -> Result<String, sqlx::Error> {
        let reason = reason_to_str(&req.reason);

        let existing: Option<String> = sqlx::query_scalar(
            "SELECT id FROM question_reports WHERE question_id = ? AND user_id = ? AND status = 'pending'",
        )
        .bind(question_id)
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await?;

        if let Some(id) = existing {
            sqlx::query(
                "UPDATE question_reports SET reason = ?, detail = ?, updated_at = NOW() WHERE id = ?",
            )
            .bind(reason)
            .bind(&req.detail)
            .bind(&id)
            .execute(&*self.pool)
            .await?;
            return Ok(id);
        }

        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            r#"INSERT INTO question_reports (id, question_id, user_id, reason, detail)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(question_id)
        .bind(user_id)
        .bind(reason)
        .bind(&req.detail)
        .execute(&*self.pool)
        .await?;

        Ok(id)
    }

    /// Admin: list laporan dengan JOIN ke soal dan users (paginated).
    pub async fn list_reports_admin(
        &self,
        status: Option<&str>,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<ReportWithQuestion>, i64), sqlx::Error> {
        let offset = (page.saturating_sub(1)) * per_page;

        let total: i64 = if let Some(s) = status {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM question_reports WHERE status = ?",
            )
            .bind(s)
            .fetch_one(&*self.pool)
            .await?
        } else {
            sqlx::query_scalar("SELECT COUNT(*) FROM question_reports")
                .fetch_one(&*self.pool)
                .await?
        };

        #[derive(sqlx::FromRow)]
        struct ReportRow {
            id:            String,
            question_id:   i32,
            question_text: String,
            reporter_name: String,
            reason:        String,
            detail:        Option<String>,
            status:        String,
            admin_note:    Option<String>,
            created_at:    chrono::DateTime<chrono::Utc>,
        }

        let rows: Vec<ReportRow> = if let Some(s) = status {
            sqlx::query_as::<_, ReportRow>(
                r#"SELECT qr.id, qr.question_id,
                          LEFT(s.soal, 200) AS question_text,
                          u.display_name    AS reporter_name,
                          qr.reason, qr.detail, qr.status, qr.admin_note, qr.created_at
                   FROM question_reports qr
                   JOIN soal s  ON s.id  = qr.question_id
                   JOIN users u ON u.id  = qr.user_id
                   WHERE qr.status = ?
                   ORDER BY qr.created_at DESC
                   LIMIT ? OFFSET ?"#,
            )
            .bind(s)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&*self.pool)
            .await?
        } else {
            sqlx::query_as::<_, ReportRow>(
                r#"SELECT qr.id, qr.question_id,
                          LEFT(s.soal, 200) AS question_text,
                          u.display_name    AS reporter_name,
                          qr.reason, qr.detail, qr.status, qr.admin_note, qr.created_at
                   FROM question_reports qr
                   JOIN soal s  ON s.id  = qr.question_id
                   JOIN users u ON u.id  = qr.user_id
                   ORDER BY qr.created_at DESC
                   LIMIT ? OFFSET ?"#,
            )
            .bind(per_page)
            .bind(offset)
            .fetch_all(&*self.pool)
            .await?
        };

        let reports = rows
            .into_iter()
            .map(|r| ReportWithQuestion {
                id:            r.id,
                question_id:   r.question_id,
                question_text: r.question_text,
                reporter_name: r.reporter_name,
                reason:        reason_from_str(&r.reason),
                detail:        r.detail,
                status:        status_from_str(&r.status),
                admin_note:    r.admin_note,
                created_at:    r.created_at,
            })
            .collect();

        Ok((reports, total))
    }

    /// Admin: soal yang paling banyak dilaporkan.
    pub async fn get_most_reported_questions(
        &self,
        limit: u32,
    ) -> Result<Vec<QuestionReportStats>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct StatsRow {
            question_id:      i32,
            question_text:    String,
            pending_count:    i64,
            total_count:      i64,
            last_reported_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        let rows = sqlx::query_as::<_, StatsRow>(
            r#"SELECT qr.question_id,
                      LEFT(s.soal, 100) AS question_text,
                      SUM(CASE WHEN qr.status = 'pending' THEN 1 ELSE 0 END) AS pending_count,
                      COUNT(*) AS total_count,
                      MAX(qr.created_at) AS last_reported_at
               FROM question_reports qr
               JOIN soal s ON s.id = qr.question_id
               GROUP BY qr.question_id, s.soal
               ORDER BY pending_count DESC, last_reported_at DESC
               LIMIT ?"#,
        )
        .bind(limit)
        .fetch_all(&*self.pool)
        .await?;

        let stats = rows
            .into_iter()
            .map(|r| QuestionReportStats {
                question_id:      r.question_id,
                question_text:    r.question_text,
                pending_count:    r.pending_count,
                total_count:      r.total_count,
                last_reported_at: r.last_reported_at,
            })
            .collect();

        Ok(stats)
    }

    /// Admin: update status laporan.
    pub async fn update_report_status(
        &self,
        report_id: &str,
        req: &UpdateReportStatusRequest,
    ) -> Result<(), sqlx::Error> {
        let s = status_to_str(&req.status);
        sqlx::query(
            "UPDATE question_reports SET status = ?, admin_note = ?, updated_at = NOW() WHERE id = ?",
        )
        .bind(s)
        .bind(&req.admin_note)
        .bind(report_id)
        .execute(&*self.pool)
        .await?;
        Ok(())
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn reason_to_str(r: &crate::model::question_feedback::ReportReason) -> &'static str {
    use crate::model::question_feedback::ReportReason::*;
    match r {
        WrongAnswer        => "wrong_answer",
        UnclearExplanation => "unclear_explanation",
        NotRelevant        => "not_relevant",
        Duplicate          => "duplicate",
        Other              => "other",
    }
}

fn reason_from_str(s: &str) -> crate::model::question_feedback::ReportReason {
    use crate::model::question_feedback::ReportReason::*;
    match s {
        "wrong_answer"        => WrongAnswer,
        "unclear_explanation" => UnclearExplanation,
        "not_relevant"        => NotRelevant,
        "duplicate"           => Duplicate,
        _                     => Other,
    }
}

fn status_to_str(s: &crate::model::question_feedback::ReportStatus) -> &'static str {
    use crate::model::question_feedback::ReportStatus::*;
    match s {
        Pending  => "pending",
        Reviewed => "reviewed",
        Resolved => "resolved",
    }
}

fn status_from_str(s: &str) -> crate::model::question_feedback::ReportStatus {
    use crate::model::question_feedback::ReportStatus::*;
    match s {
        "reviewed" => Reviewed,
        "resolved" => Resolved,
        _          => Pending,
    }
}
