use super::Table;
use crate::model::User;
use crate::model::users::{UserProfileResponse, UserLearningStatsResponse};
use crate::model::public_profile::{PublicStats, CategoryBestScore, PrivacySettings};
use crate::levels::compute_level_info;
use sqlx::{Error, Row};
use chrono::{NaiveDate, Utc};

impl<'c> Table<'c, User> {
    pub async fn create_user(&self, user: &User) -> Result<(), Error> {
        sqlx::query(
            r#"
            INSERT INTO users (id, email, display_name, phone_number)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.display_name)
        .bind(&user.phone_number)
        .execute(&*self.pool)
        .await
        .map(|_| ())
    }

    pub async fn create_user_with_picture_and_phone(&self, id: &str, email: &str, display_name: &str, picture_url: Option<&str>, phone_number: Option<&str>) -> Result<(), Error> {
        let query = match (picture_url, phone_number) {
            (Some(picture), Some(phone)) => {
                sqlx::query(
                    r#"
                    INSERT INTO users (id, email, display_name, picture_url, phone_number)
                    VALUES (?, ?, ?, ?, ?)
                    "#,
                )
                .bind(id)
                .bind(email)
                .bind(display_name)
                .bind(picture)
                .bind(phone)
            },
            (Some(picture), None) => {
                sqlx::query(
                    r#"
                    INSERT INTO users (id, email, display_name, picture_url)
                    VALUES (?, ?, ?, ?)
                    "#,
                )
                .bind(id)
                .bind(email)
                .bind(display_name)
                .bind(picture)
            },
            (None, Some(phone)) => {
                sqlx::query(
                    r#"
                    INSERT INTO users (id, email, display_name, phone_number)
                    VALUES (?, ?, ?, ?)
                    "#,
                )
                .bind(id)
                .bind(email)
                .bind(display_name)
                .bind(phone)
            },
            (None, None) => {
                sqlx::query(
                    r#"
                    INSERT INTO users (id, email, display_name)
                    VALUES (?, ?, ?)
                    "#,
                )
                .bind(id)
                .bind(email)
                .bind(display_name)
            }
        };

        query.execute(&*self.pool).await.map(|_| ())
    }

    pub async fn create_user_with_picture(&self, id: &str, email: &str, display_name: &str, picture_url: Option<&str>) -> Result<(), Error> {
        self.create_user_with_picture_and_phone(id, email, display_name, picture_url, None).await
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<User, Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users WHERE email = ? AND deleted_at IS NULL
            "#,
        )
        .bind(email)
        .fetch_one(&*self.pool)
        .await
    }

    pub async fn get_user_by_id(&self, id: &str) -> Result<Option<User>, Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users WHERE id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn get_user_by_phone(&self, phone_number: &str) -> Result<Option<User>, Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM dbquizapp.users WHERE phone_number = ?"
        )
        .bind(phone_number)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn update_user_profile(&self, user_id: &str, display_name: &str, picture_url: Option<&str>, phone_number: Option<&str>) -> Result<(), Error> {
        let query = match (picture_url, phone_number) {
            (Some(picture), Some(phone)) => {
                sqlx::query(
                    r#"
                    UPDATE users 
                    SET display_name = ?, picture_url = ?, phone_number = ?, updated_at = NOW()
                    WHERE id = ? AND deleted_at IS NULL
                    "#,
                )
                .bind(display_name)
                .bind(picture)
                .bind(phone)
                .bind(user_id)
            },
            (Some(picture), None) => {
                sqlx::query(
                    r#"
                    UPDATE users 
                    SET display_name = ?, picture_url = ?, updated_at = NOW()
                    WHERE id = ? AND deleted_at IS NULL
                    "#,
                )
                .bind(display_name)
                .bind(picture)
                .bind(user_id)
            },
            (None, Some(phone)) => {
                sqlx::query(
                    r#"
                    UPDATE users 
                    SET display_name = ?, phone_number = ?, updated_at = NOW()
                    WHERE id = ? AND deleted_at IS NULL
                    "#,
                )
                .bind(display_name)
                .bind(phone)
                .bind(user_id)
            },
            (None, None) => {
                sqlx::query(
                    r#"
                    UPDATE users 
                    SET display_name = ?, updated_at = NOW()
                    WHERE id = ? AND deleted_at IS NULL
                    "#,
                )
                .bind(display_name)
                .bind(user_id)
            }
        };

        query.execute(&*self.pool).await.map(|_| ())
    }

    pub async fn update_user_profile_legacy(&self, user_id: &str, display_name: &str, picture_url: Option<&str>) -> Result<(), Error> {
        self.update_user_profile(user_id, display_name, picture_url, None).await
    }

    pub async fn get_user_profile_with_subscription(&self, user_id: &str) -> Result<Option<UserProfileResponse>, Error> {
        let user = match self.get_user_by_id(user_id).await? {
            Some(u) => u,
            None => return Ok(None),
        };

        let sub_row = sqlx::query(
            r#"
            SELECT end_date FROM dbquizapp.user_subscriptions
            WHERE user_id = ? AND status = 'active' AND (end_date IS NULL OR end_date > NOW())
            ORDER BY created_at DESC LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await?;

        let (account_status, premium_expires_at) = match sub_row {
            Some(row) => {
                let end_date = row.try_get("end_date").ok();
                ("Premium".to_string(), end_date)
            }
            None => ("Free".to_string(), None),
        };

        Ok(Some(UserProfileResponse {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            picture_url: user.picture_url,
            joined_at: user.created_at,
            account_status,
            premium_expires_at,
            onboarding_completed: user.onboarding_completed,
        }))
    }

    pub async fn save_onboarding_data(
        &self,
        user_id: &str,
        goals_json: &str,
        timeframe: Option<&str>,
        exam_date: Option<NaiveDate>,
        completed: bool,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET onboarding_completed = ?,
                onboarding_goals     = ?,
                exam_timeframe       = ?,
                target_exam_date     = ?,
                updated_at           = NOW()
            WHERE id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(completed)
        .bind(goals_json)
        .bind(timeframe)
        .bind(exam_date)
        .bind(user_id)
        .execute(&*self.pool)
        .await
        .map(|_| ())
    }

    pub async fn get_user_learning_stats(&self, user_id: &str) -> Result<UserLearningStatsResponse, Error> {
        // Query aggregate stats
        let stats_row = sqlx::query(
            r#"
            SELECT
                COUNT(*) AS total_quizzes,
                COALESCE(SUM(score), 0) AS total_score,
                COALESCE(SUM(correct_answers), 0) AS total_correct,
                COALESCE(SUM(correct_answers + incorrect_answers), 0) AS total_questions
            FROM quiz_sessions
            WHERE user_id = ? AND is_completed = TRUE
            "#,
        )
        .bind(user_id)
        .fetch_one(&*self.pool)
        .await?;

        let total_quizzes: i64 = stats_row.try_get("total_quizzes")?;
        let total_score: i64 = stats_row.try_get("total_score").unwrap_or(0);
        let total_correct: i64 = stats_row.try_get("total_correct").unwrap_or(0);
        let total_questions: i64 = stats_row.try_get("total_questions").unwrap_or(0);

        let avg_score = if total_quizzes > 0 {
            let raw = total_score as f64 / total_quizzes as f64;
            (raw * 10.0).round() / 10.0
        } else {
            0.0
        };

        // Query favorite category
        let fav_row = sqlx::query(
            r#"
            SELECT kategori_soal FROM quiz_sessions
            WHERE user_id = ? AND is_completed = TRUE
            GROUP BY kategori_soal
            ORDER BY COUNT(*) DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await?;

        let favorite_category = fav_row.and_then(|row| row.try_get::<String, _>("kategori_soal").ok());

        // Query distinct quiz dates descending for streak calculation
        let date_rows = sqlx::query(
            r#"
            SELECT DATE(updated_at) AS quiz_date
            FROM quiz_sessions
            WHERE user_id = ? AND is_completed = TRUE
            GROUP BY DATE(updated_at)
            ORDER BY quiz_date DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await?;

        // Calculate streak: count consecutive days from today backwards
        let today = Utc::now().date_naive();
        let mut streak: i64 = 0;
        let mut expected = today;

        for row in &date_rows {
            let quiz_date: chrono::NaiveDate = row.try_get("quiz_date")?;
            if quiz_date == expected {
                streak += 1;
                expected = match expected.pred_opt() {
                    Some(d) => d,
                    None => break,
                };
            } else {
                break;
            }
        }

        Ok(UserLearningStatsResponse {
            total_quizzes,
            avg_score,
            favorite_category,
            learning_streak_days: streak,
            total_correct,
            total_questions,
        })
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = ? AND deleted_at IS NULL"
        )
        .bind(username)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn username_exists(&self, username: &str) -> Result<bool, Error> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE username = ?"
        )
        .bind(username)
        .fetch_one(&*self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn update_username(&self, user_id: &str, username: &str) -> Result<(), Error> {
        sqlx::query(
            "UPDATE users SET username = ?, updated_at = NOW() WHERE id = ? AND deleted_at IS NULL"
        )
        .bind(username)
        .bind(user_id)
        .execute(&*self.pool)
        .await
        .map(|_| ())
    }

    pub async fn update_privacy(
        &self,
        user_id: &str,
        profile_public: Option<bool>,
        privacy_json: Option<&str>,
    ) -> Result<(), Error> {
        match (profile_public, privacy_json) {
            (Some(pub_flag), Some(json)) => {
                sqlx::query(
                    "UPDATE users SET profile_public = ?, privacy_settings = ?, updated_at = NOW() WHERE id = ? AND deleted_at IS NULL"
                )
                .bind(pub_flag)
                .bind(json)
                .bind(user_id)
                .execute(&*self.pool)
                .await
                .map(|_| ())
            }
            (Some(pub_flag), None) => {
                sqlx::query(
                    "UPDATE users SET profile_public = ?, updated_at = NOW() WHERE id = ? AND deleted_at IS NULL"
                )
                .bind(pub_flag)
                .bind(user_id)
                .execute(&*self.pool)
                .await
                .map(|_| ())
            }
            (None, Some(json)) => {
                sqlx::query(
                    "UPDATE users SET privacy_settings = ?, updated_at = NOW() WHERE id = ? AND deleted_at IS NULL"
                )
                .bind(json)
                .bind(user_id)
                .execute(&*self.pool)
                .await
                .map(|_| ())
            }
            (None, None) => Ok(()),
        }
    }

    pub async fn get_public_stats(&self, user_id: &str) -> Result<PublicStats, Error> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) AS total_quizzes,
                COALESCE(AVG(score), 0) AS avg_score,
                COALESCE(MAX(score), 0) AS best_score
            FROM quiz_sessions
            WHERE user_id = ? AND is_completed = TRUE
            "#,
        )
        .bind(user_id)
        .fetch_one(&*self.pool)
        .await?;

        let total_quizzes: i64 = row.try_get("total_quizzes")?;
        let avg_raw: f64 = row.try_get("avg_score").unwrap_or(0.0);
        let best_score: i64 = row.try_get("best_score").unwrap_or(0);

        let avg_score = (avg_raw * 10.0).round() / 10.0;

        // Favorite category
        let fav_row = sqlx::query(
            r#"
            SELECT kategori_soal FROM quiz_sessions
            WHERE user_id = ? AND is_completed = TRUE
            GROUP BY kategori_soal ORDER BY COUNT(*) DESC LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await?;

        let favorite_category = fav_row.and_then(|r| r.try_get::<String, _>("kategori_soal").ok());

        // Streak (reuse same logic as learning stats)
        let today = Utc::now().date_naive();
        let date_rows = sqlx::query(
            r#"
            SELECT DATE(updated_at) AS quiz_date FROM quiz_sessions
            WHERE user_id = ? AND is_completed = TRUE
            GROUP BY DATE(updated_at) ORDER BY quiz_date DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await?;

        let mut streak: i64 = 0;
        let mut expected = today;
        for date_row in &date_rows {
            let quiz_date: chrono::NaiveDate = date_row.try_get("quiz_date")?;
            if quiz_date == expected {
                streak += 1;
                expected = match expected.pred_opt() { Some(d) => d, None => break };
            } else {
                break;
            }
        }

        // Fetch XP and level from users table
        let (total_xp, current_level): (i64, i32) = sqlx::query_as(
            "SELECT total_xp, current_level FROM users WHERE id = ?"
        )
        .bind(user_id)
        .fetch_one(&*self.pool)
        .await
        .unwrap_or((0, 1));

        let (level_cfg, _, _) = compute_level_info(total_xp);

        Ok(PublicStats {
            total_quizzes,
            avg_score,
            best_score,
            learning_streak_days: streak,
            favorite_category,
            total_xp,
            current_level,
            level_name: level_cfg.name.to_string(),
            level_icon: level_cfg.icon.to_string(),
        })
    }

    pub async fn get_best_scores_by_category(&self, user_id: &str) -> Result<Vec<CategoryBestScore>, Error> {
        let rows = sqlx::query(
            r#"
            SELECT kategori_soal AS category, MAX(score) AS best_score, COUNT(*) AS total_attempts
            FROM quiz_sessions
            WHERE user_id = ? AND is_completed = TRUE
            GROUP BY kategori_soal
            ORDER BY best_score DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| CategoryBestScore {
            category: row.get("category"),
            best_score: row.get("best_score"),
            total_attempts: row.get("total_attempts"),
        }).collect())
    }

    pub async fn set_username_if_null(&self, user_id: &str, username: &str) -> Result<(), Error> {
        sqlx::query(
            "UPDATE users SET username = ?, updated_at = NOW() WHERE id = ? AND username IS NULL AND deleted_at IS NULL"
        )
        .bind(username)
        .bind(user_id)
        .execute(&*self.pool)
        .await
        .map(|_| ())
    }
}