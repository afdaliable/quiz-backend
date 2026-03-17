use super::Table;
use crate::model::{QuizSession, CreateQuizSessionRequest, UpdateQuizSessionRequest, CompleteQuizSessionRequest, LeaderboardEntry};
use crate::model::quiz_session::{QuizHistoryEntry, QuizHistoryResponse, StartRandomSessionRequest, StartRandomSessionResponse, RandomSessionSoal};
use sqlx::{Error, Row};
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;
use rand::seq::SliceRandom;

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
                session_type, question_ids,
                total_time, current_question, is_completed, score,
                correct_answers, incorrect_answers, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, 'standard', NULL, ?, 0, FALSE, 0, 0, 0, ?, ?)
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

    /// Buat sesi random: ambil soal acak yang bisa diakses user, simpan question_ids di session.
    pub async fn create_random_session(
        &self,
        user_id: &str,
        request: &StartRandomSessionRequest,
    ) -> Result<StartRandomSessionResponse, Error> {
        let count = request.count as usize;
        let kategori = request.category.as_deref().unwrap_or("Semua Kategori");
        let total_time = (count as i32) * 60;

        // Step 1: ambil semua soal ID yang bisa diakses user (memperhatikan premium access)
        let accessible_ids: Vec<i32> = match &request.category {
            Some(cat) => {
                sqlx::query_scalar(
                    r#"
                    SELECT DISTINCT s.id FROM soal s
                    INNER JOIN paket_soal_items psi ON psi.soal_id = s.id
                    INNER JOIN paket_soal p ON p.id = psi.paket_soal_id
                    INNER JOIN kategori_soal k ON k.id = p.kategori_id
                    LEFT JOIN premium_quiz_access pqa ON pqa.paket_soal_id = p.id
                    WHERE (
                        p.is_premium = false
                        OR EXISTS (
                            SELECT 1 FROM user_subscriptions us
                            WHERE us.user_id = ?
                              AND us.status = 'active'
                              AND (us.end_date IS NULL OR us.end_date > NOW())
                              AND us.plan_id >= pqa.min_plan_id
                        )
                    )
                    AND k.nama_kategori = ?
                    "#,
                )
                .bind(user_id)
                .bind(cat)
                .fetch_all(&*self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    SELECT DISTINCT s.id FROM soal s
                    INNER JOIN paket_soal_items psi ON psi.soal_id = s.id
                    INNER JOIN paket_soal p ON p.id = psi.paket_soal_id
                    LEFT JOIN premium_quiz_access pqa ON pqa.paket_soal_id = p.id
                    WHERE (
                        p.is_premium = false
                        OR EXISTS (
                            SELECT 1 FROM user_subscriptions us
                            WHERE us.user_id = ?
                              AND us.status = 'active'
                              AND (us.end_date IS NULL OR us.end_date > NOW())
                              AND us.plan_id >= pqa.min_plan_id
                        )
                    )
                    "#,
                )
                .bind(user_id)
                .fetch_all(&*self.pool)
                .await?
            }
        };

        if accessible_ids.is_empty() {
            return Err(Error::RowNotFound);
        }

        // Step 2: Fisher-Yates shuffle via rand crate, ambil sejumlah count
        let mut ids = accessible_ids;
        let mut rng = rand::thread_rng();
        ids.shuffle(&mut rng);
        let selected_ids: Vec<i32> = ids.into_iter().take(count).collect();

        // Step 3: fetch question content untuk selected_ids
        let placeholders = selected_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query_str = format!(
            "SELECT id, soal, question_type, opt1, opt2, opt3, opt4, opt5, solution, modul, pelajaran, tag FROM soal WHERE id IN ({})",
            placeholders
        );
        let mut q = sqlx::query(&query_str);
        for id in &selected_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&*self.pool).await?;

        // Build map id → soal untuk preserve shuffled order
        let soal_map: HashMap<i32, RandomSessionSoal> = rows
            .into_iter()
            .map(|row| {
                let id: i32 = row.get("id");
                (id, RandomSessionSoal {
                    id,
                    soal: row.get("soal"),
                    question_type: row.try_get("question_type").unwrap_or_else(|_| "multiple_choice".to_string()),
                    opt1: row.get("opt1"),
                    opt2: row.get("opt2"),
                    opt3: row.get("opt3"),
                    opt4: row.get("opt4"),
                    opt5: row.get("opt5"),
                    solution: row.get("solution"),
                    modul: row.get("modul"),
                    pelajaran: row.get("pelajaran"),
                    tag: row.get("tag"),
                })
            })
            .collect();

        // Susun kembali sesuai urutan shuffle
        let questions: Vec<RandomSessionSoal> = selected_ids
            .iter()
            .filter_map(|id| soal_map.get(id).cloned())
            .collect();

        let actual_count = questions.len();
        let question_ids_json = serde_json::to_string(&selected_ids).unwrap_or_else(|_| "[]".to_string());

        // Step 4: simpan session ke DB
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO quiz_sessions (
                id, user_id, paket_soal_id, kategori_soal, nama_paket_soal,
                session_type, question_ids,
                total_time, current_question, is_completed, score,
                correct_answers, incorrect_answers, created_at, updated_at
            )
            VALUES (?, ?, NULL, ?, 'Latihan Random', 'random', ?, ?, 0, FALSE, 0, 0, 0, ?, ?)
            "#,
        )
        .bind(&session_id)
        .bind(user_id)
        .bind(kategori)
        .bind(&question_ids_json)
        .bind(total_time)
        .bind(now)
        .bind(now)
        .execute(&*self.pool)
        .await?;

        Ok(StartRandomSessionResponse {
            session_id,
            session_type: "random".to_string(),
            nama_paket_soal: "Latihan Random".to_string(),
            kategori_soal: kategori.to_string(),
            total_time,
            total_questions: actual_count,
            questions,
        })
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

        let session = self.get_quiz_session_by_id(session_id, user_id).await?;

        // Branch: random session gunakan question_ids, standard gunakan kategori+paket
        let (correct_answers, incorrect_answers, score) = if session.session_type == "random" {
            let question_ids: Vec<i32> = session.question_ids
                .as_ref()
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default();
            self.calculate_score_by_ids(&question_ids, &request.answers).await?
        } else {
            self.calculate_score(
                &session.kategori_soal,
                &session.nama_paket_soal,
                &request.answers,
            ).await?
        };

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

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM quiz_sessions
            WHERE user_id = ? AND is_completed = TRUE
            "#,
        )
        .bind(user_id)
        .fetch_one(&*self.pool)
        .await?;

        // total_questions: untuk standard pakai paket_soal_items, untuk random pakai JSON_LENGTH
        let rows = sqlx::query(
            r#"
            SELECT qs.id, qs.nama_paket_soal, qs.kategori_soal, qs.session_type, qs.score,
                   qs.correct_answers, qs.incorrect_answers,
                   COALESCE(
                       (SELECT COUNT(*) FROM paket_soal_items psi WHERE psi.paket_soal_id = qs.paket_soal_id),
                       JSON_LENGTH(qs.question_ids),
                       0
                   ) AS total_questions,
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
                    session_type: row.try_get("session_type").unwrap_or_else(|_| "standard".to_string()),
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

    /// Hitung skor berdasarkan question_ids eksplisit (untuk random session).
    async fn calculate_score_by_ids(
        &self,
        question_ids: &[i32],
        user_answers: &[Option<i32>],
    ) -> Result<(i32, i32, i32), Error> {
        if question_ids.is_empty() {
            return Ok((0, 0, 0));
        }

        let placeholders = question_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query_str = format!(
            "SELECT id, correct_answer FROM soal WHERE id IN ({})",
            placeholders
        );
        let mut q = sqlx::query(&query_str);
        for id in question_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&*self.pool).await?;

        // Build map id → correct_answer
        let correct_map: HashMap<i32, String> = rows
            .into_iter()
            .filter_map(|row| {
                let id: i32 = row.get("id");
                let ca: Option<String> = row.get("correct_answer");
                ca.map(|c| (id, c))
            })
            .collect();

        let mut correct_count = 0i32;
        let mut incorrect_count = 0i32;

        // Iterate question_ids in shuffled order — matches user_answers index
        for (index, qid) in question_ids.iter().enumerate() {
            if let Some(user_answer) = user_answers.get(index).and_then(|a| *a) {
                if let Some(correct_answer) = correct_map.get(qid) {
                    let correct_index = match correct_answer.as_str() {
                        "opt1" => 0,
                        "opt2" => 1,
                        "opt3" => 2,
                        "opt4" => 3,
                        "opt5" => 4,
                        _ => 999,
                    };
                    if user_answer == correct_index {
                        correct_count += 1;
                    } else {
                        incorrect_count += 1;
                    }
                }
            }
            // None = tidak dijawab, tidak dihitung salah
        }

        let total = question_ids.len() as i32;
        let score = if total > 0 {
            (correct_count as f32 / total as f32 * 100.0).round() as i32
        } else {
            0
        };

        Ok((correct_count, incorrect_count, score))
    }

    async fn calculate_score(
        &self,
        kategori_soal: &str,
        nama_paket_soal: &str,
        user_answers: &[Option<i32>],
    ) -> Result<(i32, i32, i32), Error> {
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
                    let correct_answer: String = row.get("correct_answer");

                    let correct_index = match correct_answer.as_str() {
                        "opt1" => 0,
                        "opt2" => 1,
                        "opt3" => 2,
                        "opt4" => 3,
                        "opt5" => 4,
                        _ => 999,
                    };

                    if *user_answer == correct_index {
                        correct_count += 1;
                    } else {
                        incorrect_count += 1;
                    }
                }
            }
        }

        let total_questions = paket_response.len() as i32;
        let score = if total_questions > 0 {
            (correct_count as f32 / total_questions as f32 * 100.0).round() as i32
        } else {
            0
        };

        Ok((correct_count, incorrect_count, score))
    }
}
