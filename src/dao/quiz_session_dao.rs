use super::Table;
use crate::model::{QuizSession, CreateQuizSessionRequest, UpdateQuizSessionRequest, CompleteQuizSessionRequest};
use sqlx::Error;
use chrono::Utc;
use uuid::Uuid;

impl<'c> Table<'c, QuizSession> {
    pub async fn create_quiz_session(
        &self,
        user_id: &str,
        request: &CreateQuizSessionRequest,
    ) -> Result<QuizSession, Error> {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        sqlx::query(
            r#"
            INSERT INTO quiz_sessions (
                id, user_id, paket_soal_id, kategori_soal, nama_paket_soal, 
                total_time, current_question, is_completed, score, 
                correct_answers, incorrect_answers, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, 0, FALSE, 0, 0, 0, ?, ?)
            "#,
        )
        .bind(&session_id)
        .bind(user_id)
        .bind(request.paket_soal_id)
        .bind(&request.kategori_soal)
        .bind(&request.nama_paket_soal)
        .bind(request.total_time)
        .bind(now)
        .bind(now)
        .execute(&*self.pool)
        .await?;
        
        self.get_quiz_session_by_id(&session_id, user_id).await
    }

    pub async fn get_quiz_session_by_id(
        &self,
        session_id: &str,
        user_id: &str,
    ) -> Result<QuizSession, Error> {
        sqlx::query_as::<_, QuizSession>(
            r#"
            SELECT * FROM quiz_sessions 
            WHERE id = ? AND user_id = ?
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_one(&*self.pool)
        .await
    }

    pub async fn update_quiz_session(
        &self,
        session_id: &str,
        user_id: &str,
        request: &UpdateQuizSessionRequest,
    ) -> Result<QuizSession, Error> {
        let now = Utc::now();
        
        // Build the update query dynamically based on provided fields
        let mut query = "UPDATE quiz_sessions SET updated_at = ?".to_string();
        let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::MySql> + Send + 'static>> = vec![
            Box::new(now)
        ];

        if let Some(current_question) = request.current_question {
            query.push_str(", current_question = ?");
            params.push(Box::new(current_question));
        }

        if let Some(ref answers) = request.answers {
            let answers_json = serde_json::to_string(answers).unwrap_or_default();
            query.push_str(", answers = ?");
            params.push(Box::new(answers_json));
        }

        if let Some(ref marked_questions) = request.marked_questions {
            let marked_json = serde_json::to_string(marked_questions).unwrap_or_default();
            query.push_str(", marked_questions = ?");
            params.push(Box::new(marked_json));
        }

        if let Some(time_remaining) = request.time_remaining {
            query.push_str(", time_remaining = ?");
            params.push(Box::new(time_remaining));
        }

        query.push_str(" WHERE id = ? AND user_id = ? AND is_completed = FALSE");
        
        let mut sql_query = sqlx::query(&query);
        
        // Add all parameters
        sql_query = sql_query.bind(now);
        
        if let Some(current_question) = request.current_question {
            sql_query = sql_query.bind(current_question);
        }

        if let Some(ref answers) = request.answers {
            let answers_json = serde_json::to_string(answers).unwrap_or_default();
            sql_query = sql_query.bind(answers_json);
        }

        if let Some(ref marked_questions) = request.marked_questions {
            let marked_json = serde_json::to_string(marked_questions).unwrap_or_default();
            sql_query = sql_query.bind(marked_json);
        }

        if let Some(time_remaining) = request.time_remaining {
            sql_query = sql_query.bind(time_remaining);
        }

        sql_query = sql_query.bind(session_id).bind(user_id);
        
        sql_query.execute(&*self.pool).await?;
        
        self.get_quiz_session_by_id(session_id, user_id).await
    }

    pub async fn complete_quiz_session(
        &self,
        session_id: &str,
        user_id: &str,
        request: &CompleteQuizSessionRequest,
    ) -> Result<QuizSession, Error> {
        let now = Utc::now();
        let answers_json = serde_json::to_string(&request.answers).unwrap_or_default();
        
        // Calculate score based on answers
        // This is a placeholder - you'll need to implement actual scoring logic
        // by fetching questions and comparing answers
        let correct_answers = 0; // TODO: Calculate based on correct answers
        let incorrect_answers = request.answers.len() as i32 - correct_answers;
        let score = (correct_answers as f32 / request.answers.len() as f32 * 100.0) as i32;
        
        sqlx::query(
            r#"
            UPDATE quiz_sessions 
            SET answers = ?, time_remaining = ?, is_completed = TRUE, 
                score = ?, correct_answers = ?, incorrect_answers = ?, updated_at = ?
            WHERE id = ? AND user_id = ? AND is_completed = FALSE
            "#,
        )
        .bind(answers_json)
        .bind(request.time_remaining)
        .bind(score)
        .bind(correct_answers)
        .bind(incorrect_answers)
        .bind(now)
        .bind(session_id)
        .bind(user_id)
        .execute(&*self.pool)
        .await?;
        
        self.get_quiz_session_by_id(session_id, user_id).await
    }

    pub async fn get_user_active_sessions(
        &self,
        user_id: &str,
    ) -> Result<Vec<QuizSession>, Error> {
        sqlx::query_as::<_, QuizSession>(
            r#"
            SELECT * FROM quiz_sessions 
            WHERE user_id = ? AND is_completed = FALSE
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_user_completed_sessions(
        &self,
        user_id: &str,
        limit: Option<i32>,
    ) -> Result<Vec<QuizSession>, Error> {
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
        let query = format!(
            r#"
            SELECT * FROM quiz_sessions 
            WHERE user_id = ? AND is_completed = TRUE
            ORDER BY created_at DESC
            {}
            "#,
            limit_clause
        );

        sqlx::query_as::<_, QuizSession>(&query)
            .bind(user_id)
            .fetch_all(&*self.pool)
            .await
    }

    pub async fn delete_quiz_session(
        &self,
        session_id: &str,
        user_id: &str,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
            DELETE FROM quiz_sessions 
            WHERE id = ? AND user_id = ?
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .execute(&*self.pool)
        .await
        .map(|_| ())
    }

    pub async fn cleanup_expired_sessions(&self, hours: i32) -> Result<(), Error> {
        sqlx::query(
            r#"
            DELETE FROM quiz_sessions 
            WHERE is_completed = FALSE 
            AND updated_at < DATE_SUB(NOW(), INTERVAL ? HOUR)
            "#,
        )
        .bind(hours)
        .execute(&*self.pool)
        .await
        .map(|_| ())
    }

    pub async fn get_session_exists(
        &self,
        user_id: &str,
        paket_soal_id: i32,
        kategori_soal: &str,
        nama_paket_soal: &str,
    ) -> Result<Option<QuizSession>, Error> {
        let result = sqlx::query_as::<_, QuizSession>(
            r#"
            SELECT * FROM quiz_sessions 
            WHERE user_id = ? AND paket_soal_id = ? 
            AND kategori_soal = ? AND nama_paket_soal = ?
            AND is_completed = FALSE
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(paket_soal_id)
        .bind(kategori_soal)
        .bind(nama_paket_soal)
        .fetch_optional(&*self.pool)
        .await?;

        Ok(result)
    }
}