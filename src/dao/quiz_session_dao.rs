use super::Table;
use crate::model::{QuizSession, CreateQuizSessionRequest, UpdateQuizSessionRequest, CompleteQuizSessionRequest, LeaderboardEntry};
use crate::model::quiz_session::{QuizHistoryEntry, QuizHistoryResponse};
use sqlx::{Error, Row};
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
        
        // Get the quiz session to retrieve paket info for scoring
        let session = self.get_quiz_session_by_id(session_id, user_id).await?;
        
        // Calculate score based on actual correct answers
        let (correct_answers, incorrect_answers, score) = self.calculate_score(
            &session.kategori_soal, 
            &session.nama_paket_soal,
            &request.answers
        ).await?;
        
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

    pub async fn get_leaderboard(
        &self,
        paket_soal_id: Option<i32>,
        limit: i32,
    ) -> Result<Vec<LeaderboardEntry>, Error> {
        let rows = if let Some(paket_id) = paket_soal_id {
            sqlx::query(
                r#"
                SELECT
                    u.id AS user_id,
                    u.display_name,
                    u.picture_url,
                    MAX(qs.score) AS best_score,
                    COUNT(qs.id) AS total_quizzes,
                    MAX(qs.correct_answers) AS correct_answers
                FROM quiz_sessions qs
                JOIN users u ON qs.user_id = u.id
                WHERE qs.is_completed = TRUE AND qs.paket_soal_id = ?
                GROUP BY u.id, u.display_name, u.picture_url
                ORDER BY best_score DESC, correct_answers DESC
                LIMIT ?
                "#,
            )
            .bind(paket_id)
            .bind(limit)
            .fetch_all(&*self.pool)
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT
                    u.id AS user_id,
                    u.display_name,
                    u.picture_url,
                    MAX(qs.score) AS best_score,
                    COUNT(qs.id) AS total_quizzes,
                    MAX(qs.correct_answers) AS correct_answers
                FROM quiz_sessions qs
                JOIN users u ON qs.user_id = u.id
                WHERE qs.is_completed = TRUE
                GROUP BY u.id, u.display_name, u.picture_url
                ORDER BY best_score DESC, correct_answers DESC
                LIMIT ?
                "#,
            )
            .bind(limit)
            .fetch_all(&*self.pool)
            .await?
        };

        let entries = rows
            .into_iter()
            .enumerate()
            .map(|(i, row)| LeaderboardEntry {
                rank: (i + 1) as i64,
                user_id: row.get("user_id"),
                display_name: row.get("display_name"),
                picture_url: row.get("picture_url"),
                best_score: row.get("best_score"),
                total_quizzes: row.get("total_quizzes"),
                correct_answers: row.get("correct_answers"),
            })
            .collect();

        Ok(entries)
    }

    pub async fn get_user_quiz_history(
        &self,
        user_id: &str,
        page: i64,
        limit: i64,
    ) -> Result<QuizHistoryResponse, Error> {
        let offset = (page - 1) * limit;

        // Get total count
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM quiz_sessions
            WHERE user_id = ? AND is_completed = TRUE
            "#,
        )
        .bind(user_id)
        .fetch_one(&*self.pool)
        .await?;

        // Get paginated rows — total_questions from paket_soal_items (not correct+incorrect)
        let rows = sqlx::query(
            r#"
            SELECT qs.id, qs.nama_paket_soal, qs.kategori_soal, qs.score,
                   qs.correct_answers, qs.incorrect_answers,
                   (SELECT COUNT(*) FROM paket_soal_items psi WHERE psi.paket_soal_id = qs.paket_soal_id) AS total_questions,
                   (qs.total_time - COALESCE(qs.time_remaining, 0)) AS duration_seconds,
                   qs.updated_at
            FROM quiz_sessions qs
            WHERE qs.user_id = ? AND qs.is_completed = TRUE
            ORDER BY qs.updated_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await?;

        let data = rows
            .into_iter()
            .map(|row| {
                let correct: i32 = row.get("correct_answers");
                let wrong: i32 = row.get("incorrect_answers");
                let total: i32 = row.get("total_questions");
                let unanswered = (total - correct - wrong).max(0);
                QuizHistoryEntry {
                    id: row.get("id"),
                    package_name: row.get("nama_paket_soal"),
                    category: row.get("kategori_soal"),
                    score: row.get("score"),
                    correct,
                    wrong,
                    unanswered,
                    total,
                    duration_seconds: row.get("duration_seconds"),
                    completed_at: row.get("updated_at"),
                }
            })
            .collect();

        Ok(QuizHistoryResponse { data, total, page, limit })
    }

    async fn calculate_score(
        &self,
        kategori_soal: &str,
        nama_paket_soal: &str,
        user_answers: &[Option<i32>],
    ) -> Result<(i32, i32, i32), Error> {
        // Fetch the correct answers from the database using the same query as get_paket_soal_response
        let paket_response = sqlx::query(
            r#"
            SELECT s.correct_answer
            FROM kategori_soal ks 
            JOIN paket_soal ps ON ks.id = ps.kategori_id 
            JOIN paket_soal_items psi ON psi.paket_soal_id = ps.id 
            JOIN soal s ON psi.soal_id = s.id 
            WHERE ks.nama_kategori = ? AND ps.nama_paket_soal = ?
            ORDER BY psi.id
            "#,
        )
        .bind(kategori_soal)
        .bind(nama_paket_soal)
        .fetch_all(&*self.pool)
        .await?;

        let mut correct_count = 0;
        let mut incorrect_count = 0;

        for (index, row) in paket_response.iter().enumerate() {
            if let Some(user_answer) = user_answers.get(index) {
                if let Some(user_answer) = user_answer {
                    // Note: 0 is a valid answer (first option), not "no answer"
                    // No need to skip 0 as it represents the first option
                    // Get correct answer from database (opt1, opt2, opt3, opt4, opt5)
                    let correct_answer: String = row.get("correct_answer");
                    
                    // Convert correct answer to index (opt1=0, opt2=1, etc. - 0-based indexing)
                    let correct_index = match correct_answer.as_str() {
                        "opt1" => 0,
                        "opt2" => 1, 
                        "opt3" => 2,
                        "opt4" => 3,
                        "opt5" => 4,
                        _ => 999, // Invalid answer (use 999 so it never matches valid user answers)
                    };

                    if *user_answer == correct_index {
                        correct_count += 1;
                    } else {
                        incorrect_count += 1;
                    }
                }
                // else: user_answer is None — question was left unanswered, not counted as wrong
            }
        }

        // Calculate percentage score
        let total_questions = paket_response.len() as i32;
        let score = if total_questions > 0 {
            (correct_count as f32 / total_questions as f32 * 100.0).round() as i32
        } else {
            0
        };

        Ok((correct_count, incorrect_count, score))
    }
}