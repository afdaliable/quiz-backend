use crate::dao::db_context::Table;
use crate::model::daily_challenge::{
    DailyChallenge, DailyChallengeAttempt, LeaderboardEntry,
    SoalForChallenge, UserAttemptSummary, UserChallengeStats,
};
use chrono::{FixedOffset, NaiveDate, Utc};
use sqlx::Row;
use uuid::Uuid;

fn wib_today() -> NaiveDate {
    let wib = FixedOffset::east_opt(7 * 3600).unwrap();
    Utc::now().with_timezone(&wib).date_naive()
}

fn calc_score(is_correct: bool, time_taken_ms: i32) -> i32 {
    if !is_correct {
        return 0;
    }
    let deduction = (time_taken_ms / 1000) * 10;
    (1000 - deduction).max(100)
}

impl<'c> Table<'c, DailyChallenge> {
    /// Fetch today's challenge (WIB date).
    pub async fn get_today(&self) -> Result<Option<DailyChallenge>, sqlx::Error> {
        let today = wib_today();
        sqlx::query_as::<_, DailyChallenge>(
            "SELECT id, challenge_date, soal_id, created_by, created_at
             FROM daily_challenges
             WHERE challenge_date = ?",
        )
        .bind(today)
        .fetch_optional(&*self.pool)
        .await
    }

    /// Fetch challenge for a specific date.
    pub async fn get_by_date(&self, date: NaiveDate) -> Result<Option<DailyChallenge>, sqlx::Error> {
        sqlx::query_as::<_, DailyChallenge>(
            "SELECT id, challenge_date, soal_id, created_by, created_at
             FROM daily_challenges
             WHERE challenge_date = ?",
        )
        .bind(date)
        .fetch_optional(&*self.pool)
        .await
    }

    /// Fetch the question details for a challenge.
    pub async fn get_soal_for_challenge(&self, soal_id: i32) -> Result<SoalForChallenge, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, soal, opt1, opt2, opt3, opt4, opt5, pelajaran, tag
             FROM soal
             WHERE id = ?",
        )
        .bind(soal_id)
        .fetch_one(&*self.pool)
        .await?;

        Ok(SoalForChallenge {
            id: row.get("id"),
            soal: row.get("soal"),
            opt1: row.get("opt1"),
            opt2: row.get("opt2"),
            opt3: row.try_get("opt3").ok(),
            opt4: row.try_get("opt4").ok(),
            opt5: row.try_get("opt5").ok(),
            pelajaran: row.try_get("pelajaran").ok(),
            tag: row.try_get("tag").ok(),
        })
    }

    /// Get correct answer for a soal (returns 1-based answer index).
    pub async fn get_correct_answer(&self, soal_id: i32) -> Result<i8, sqlx::Error> {
        let row = sqlx::query("SELECT correct_answer FROM soal WHERE id = ?")
            .bind(soal_id)
            .fetch_one(&*self.pool)
            .await?;
        let ans: String = row.get("correct_answer");
        // correct_answer is stored as "opt1".."opt5" → map to 1..5
        let idx = match ans.as_str() {
            "opt1" => 1,
            "opt2" => 2,
            "opt3" => 3,
            "opt4" => 4,
            "opt5" => 5,
            _ => 1,
        };
        Ok(idx)
    }

    /// Get user's attempt for today.
    pub async fn get_user_attempt_today(
        &self,
        user_id: &str,
    ) -> Result<Option<DailyChallengeAttempt>, sqlx::Error> {
        let today = wib_today();
        sqlx::query_as::<_, DailyChallengeAttempt>(
            "SELECT id, user_id, challenge_date, soal_id, selected_answer,
                    is_correct, time_taken_ms, score, answered_at
             FROM daily_challenge_attempts
             WHERE user_id = ? AND challenge_date = ?",
        )
        .bind(user_id)
        .bind(today)
        .fetch_optional(&*self.pool)
        .await
    }

    /// Submit an attempt (idempotent — returns existing if already answered).
    pub async fn submit_attempt(
        &self,
        user_id: &str,
        challenge: &DailyChallenge,
        selected_answer: i8,
        time_taken_ms: i32,
    ) -> Result<(DailyChallengeAttempt, bool), sqlx::Error> {
        // Check for existing attempt first (idempotent)
        if let Some(existing) = sqlx::query_as::<_, DailyChallengeAttempt>(
            "SELECT id, user_id, challenge_date, soal_id, selected_answer,
                    is_correct, time_taken_ms, score, answered_at
             FROM daily_challenge_attempts
             WHERE user_id = ? AND challenge_date = ?",
        )
        .bind(user_id)
        .bind(challenge.challenge_date)
        .fetch_optional(&*self.pool)
        .await?
        {
            return Ok((existing, false)); // false = was already answered
        }

        let correct_answer = self.get_correct_answer(challenge.soal_id).await?;
        let is_correct = selected_answer == correct_answer;
        let score = calc_score(is_correct, time_taken_ms);
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO daily_challenge_attempts
             (id, user_id, challenge_date, soal_id, selected_answer, is_correct, time_taken_ms, score)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(challenge.challenge_date)
        .bind(challenge.soal_id)
        .bind(selected_answer)
        .bind(is_correct)
        .bind(time_taken_ms)
        .bind(score)
        .execute(&*self.pool)
        .await?;

        let attempt = sqlx::query_as::<_, DailyChallengeAttempt>(
            "SELECT id, user_id, challenge_date, soal_id, selected_answer,
                    is_correct, time_taken_ms, score, answered_at
             FROM daily_challenge_attempts
             WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&*self.pool)
        .await?;

        Ok((attempt, true)) // true = freshly submitted
    }

    /// Get user's rank for today's challenge.
    pub async fn get_user_rank_today(
        &self,
        user_id: &str,
        score: i32,
        time_taken_ms: i32,
    ) -> Result<i64, sqlx::Error> {
        let today = wib_today();
        let row = sqlx::query(
            "SELECT COUNT(*) + 1 AS rank
             FROM daily_challenge_attempts
             WHERE challenge_date = ?
               AND (score > ? OR (score = ? AND time_taken_ms < ?))",
        )
        .bind(today)
        .bind(score)
        .bind(score)
        .bind(time_taken_ms)
        .fetch_one(&*self.pool)
        .await?;
        Ok(row.get::<i64, _>("rank"))
    }

    /// Count total participants for today.
    pub async fn count_participants_today(&self) -> Result<i64, sqlx::Error> {
        let today = wib_today();
        let row = sqlx::query("SELECT COUNT(*) AS cnt FROM daily_challenge_attempts WHERE challenge_date = ?")
            .bind(today)
            .fetch_one(&*self.pool)
            .await?;
        Ok(row.get::<i64, _>("cnt"))
    }

    /// Get leaderboard for today's challenge (top 50).
    pub async fn get_leaderboard_today(&self) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
        let today = wib_today();
        let rows = sqlx::query(
            "SELECT
                dca.user_id,
                COALESCE(u.display_name, u.email, dca.user_id) AS display_name,
                dca.score,
                dca.time_taken_ms,
                dca.answered_at,
                ROW_NUMBER() OVER (ORDER BY dca.score DESC, dca.time_taken_ms ASC) AS rank
             FROM daily_challenge_attempts dca
             LEFT JOIN users u ON dca.user_id = u.id
             WHERE dca.challenge_date = ?
             ORDER BY dca.score DESC, dca.time_taken_ms ASC
             LIMIT 50",
        )
        .bind(today)
        .fetch_all(&*self.pool)
        .await?;

        let entries = rows
            .iter()
            .map(|row| LeaderboardEntry {
                rank: row.get("rank"),
                user_id: row.get("user_id"),
                display_name: row.get("display_name"),
                score: row.get("score"),
                time_taken_ms: row.get("time_taken_ms"),
                answered_at: row
                    .get::<chrono::DateTime<Utc>, _>("answered_at")
                    .to_rfc3339(),
            })
            .collect();

        Ok(entries)
    }

    /// Get challenge stats for a user.
    pub async fn get_user_stats(&self, user_id: &str) -> Result<UserChallengeStats, sqlx::Error> {
        let row = sqlx::query(
            "SELECT
                COUNT(*) AS total_attempted,
                SUM(is_correct) AS total_correct,
                COALESCE(AVG(score), 0) AS avg_score
             FROM daily_challenge_attempts
             WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_one(&*self.pool)
        .await?;

        let total_attempted: i64 = row.get("total_attempted");
        let total_correct: i64 = row.try_get("total_correct").unwrap_or(0);
        let avg_score: f64 = row.try_get("avg_score").unwrap_or(0.0);

        // Compute current streak: consecutive days ending today (WIB)
        let streak_rows = sqlx::query(
            "SELECT challenge_date
             FROM daily_challenge_attempts
             WHERE user_id = ? AND is_correct = 1
             ORDER BY challenge_date DESC",
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await?;

        let today = wib_today();
        let (current_streak, best_streak) = compute_streaks(&streak_rows, today);

        Ok(UserChallengeStats {
            total_attempted,
            total_correct,
            current_streak,
            best_streak,
            avg_score,
        })
    }

    /// Admin: set (upsert) a daily challenge.
    pub async fn upsert_challenge(
        &self,
        admin_user_id: &str,
        soal_id: i32,
        challenge_date: NaiveDate,
    ) -> Result<DailyChallenge, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO daily_challenges (id, challenge_date, soal_id, created_by)
             VALUES (?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE soal_id = VALUES(soal_id), created_by = VALUES(created_by)",
        )
        .bind(&id)
        .bind(challenge_date)
        .bind(soal_id)
        .bind(admin_user_id)
        .execute(&*self.pool)
        .await?;

        // Fetch the actual row (may have different id if it was an update)
        let challenge = sqlx::query_as::<_, DailyChallenge>(
            "SELECT id, challenge_date, soal_id, created_by, created_at
             FROM daily_challenges
             WHERE challenge_date = ?",
        )
        .bind(challenge_date)
        .fetch_one(&*self.pool)
        .await?;

        Ok(challenge)
    }

    /// Admin: list upcoming/recent challenges.
    pub async fn list_challenges(&self, limit: i64) -> Result<Vec<DailyChallenge>, sqlx::Error> {
        sqlx::query_as::<_, DailyChallenge>(
            "SELECT id, challenge_date, soal_id, created_by, created_at
             FROM daily_challenges
             ORDER BY challenge_date DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&*self.pool)
        .await
    }
}

/// Compute current and best streak from ordered correct-attempt dates.
fn compute_streaks(rows: &[sqlx::mysql::MySqlRow], today: NaiveDate) -> (i64, i64) {
    if rows.is_empty() {
        return (0, 0);
    }

    let dates: Vec<NaiveDate> = rows
        .iter()
        .map(|r| r.get::<NaiveDate, _>("challenge_date"))
        .collect();

    let mut current = 0i64;
    let mut best = 0i64;
    let mut run = 1i64;

    // Current streak: check if it touches today or yesterday
    let first = dates[0];
    let days_since = (today - first).num_days();
    if days_since > 1 {
        current = 0;
    } else {
        // Walk backwards counting consecutive days
        current = 1;
        for i in 1..dates.len() {
            let diff = (dates[i - 1] - dates[i]).num_days();
            if diff == 1 {
                current += 1;
            } else {
                break;
            }
        }
    }

    // Best streak: walk all dates
    for i in 1..dates.len() {
        let diff = (dates[i - 1] - dates[i]).num_days();
        if diff == 1 {
            run += 1;
        } else {
            if run > best {
                best = run;
            }
            run = 1;
        }
    }
    if run > best {
        best = run;
    }

    (current, best)
}
