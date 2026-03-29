use sqlx::MySqlPool;
use chrono::{Utc, Datelike, FixedOffset, TimeZone};
use crate::levels::{compute_level_info, XP_DAILY_CAP};

// ---- XP constants per source ----
pub const XP_QUIZ_COMPLETE: i32 = 50;
pub const XP_PER_CORRECT_ANSWER: i32 = 5;
pub const XP_BONUS_SCORE_80: i32 = 50;
pub const XP_BONUS_SCORE_100: i32 = 100;
pub const XP_STUDY_MODE_COMPLETE: i32 = 30;

#[derive(Debug, serde::Serialize)]
pub struct XpAwardResult {
    pub xp_awarded: i32,
    pub xp_capped: bool,
    pub total_xp: i64,
    pub leveled_up: bool,
    pub old_level: i32,
    pub new_level: i32,
    pub new_level_name: String,
    pub new_level_icon: String,
}

#[derive(Debug)]
pub struct QuizXpBreakdown {
    pub quiz_complete: i32,
    pub correct_answers: i32,
    pub score_bonus: i32,
    pub total: i32,
}

/// Compute XP breakdown for a completed quiz session.
pub fn compute_quiz_xp(score: i32, correct_answers: i32, is_study_mode: bool) -> QuizXpBreakdown {
    let complete_xp = if is_study_mode { XP_STUDY_MODE_COMPLETE } else { XP_QUIZ_COMPLETE };
    let answer_xp = correct_answers * XP_PER_CORRECT_ANSWER;
    let bonus_xp = if score == 100 {
        XP_BONUS_SCORE_100
    } else if score >= 80 {
        XP_BONUS_SCORE_80
    } else {
        0
    };

    QuizXpBreakdown {
        quiz_complete: complete_xp,
        correct_answers: answer_xp,
        score_bonus: bonus_xp,
        total: complete_xp + answer_xp + bonus_xp,
    }
}

/// Award XP to a user atomically. Enforces daily cap (WIB = UTC+7).
/// Returns Err only on DB errors — callers should use `.ok()` to avoid failing the quiz request.
pub async fn award_quiz_xp(
    pool: &MySqlPool,
    user_id: &str,
    session_id: &str,
    breakdown: &QuizXpBreakdown,
) -> Result<XpAwardResult, sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Today's date in WIB (UTC+7)
    let wib_today = {
        let wib_offset = FixedOffset::east_opt(7 * 3600).unwrap();
        let now_wib = Utc::now().with_timezone(&wib_offset);
        format!("{}-{:02}-{:02}", now_wib.year(), now_wib.month(), now_wib.day())
    };

    // Fetch current XP, today's XP total, and current level
    let row: (i64, i64, i32) = sqlx::query_as(
        r#"
        SELECT
            u.total_xp,
            COALESCE(SUM(CASE
                WHEN DATE(CONVERT_TZ(t.created_at, '+00:00', '+07:00')) = ? THEN t.amount
                ELSE 0
            END), 0) AS today_xp,
            u.current_level
        FROM users u
        LEFT JOIN xp_transactions t ON t.user_id = u.id
        WHERE u.id = ?
        GROUP BY u.id, u.total_xp, u.current_level
        "#,
    )
    .bind(&wib_today)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    let (total_xp, today_xp, old_level) = row;

    // Apply daily cap
    let remaining_cap = (XP_DAILY_CAP - today_xp).max(0);
    let actual_xp = (breakdown.total as i64).min(remaining_cap) as i32;
    let capped = actual_xp < breakdown.total;

    if actual_xp > 0 {
        let tx_id = uuid::Uuid::new_v4().to_string();

        // Insert XP transaction
        sqlx::query(
            r#"INSERT INTO xp_transactions (id, user_id, amount, source, source_id, description)
               VALUES (?, ?, ?, 'quiz_complete', ?, ?)"#,
        )
        .bind(&tx_id)
        .bind(user_id)
        .bind(actual_xp)
        .bind(session_id)
        .bind(format!("Quiz: +{} XP", actual_xp))
        .execute(&mut *tx)
        .await?;

        let new_total_xp = total_xp + actual_xp as i64;
        let (new_level_cfg, _, _) = compute_level_info(new_total_xp);
        let new_level = new_level_cfg.level;

        // Update user's XP and level
        sqlx::query(
            r#"UPDATE users
               SET total_xp = ?,
                   current_level = ?,
                   level_updated_at = CASE WHEN current_level != ? THEN NOW() ELSE level_updated_at END
               WHERE id = ?"#,
        )
        .bind(new_total_xp)
        .bind(new_level)
        .bind(new_level)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let (cfg, _, _) = compute_level_info(new_total_xp);
        Ok(XpAwardResult {
            xp_awarded: actual_xp,
            xp_capped: capped,
            total_xp: new_total_xp,
            leveled_up: new_level > old_level,
            old_level,
            new_level,
            new_level_name: cfg.name.to_string(),
            new_level_icon: cfg.icon.to_string(),
        })
    } else {
        tx.rollback().await?;
        let (cfg, _, _) = compute_level_info(total_xp);
        Ok(XpAwardResult {
            xp_awarded: 0,
            xp_capped: true,
            total_xp,
            leveled_up: false,
            old_level,
            new_level: old_level,
            new_level_name: cfg.name.to_string(),
            new_level_icon: cfg.icon.to_string(),
        })
    }
}
