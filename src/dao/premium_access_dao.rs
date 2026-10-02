//! Akses premium: langganan aktif ATAU trial yang masih berjalan.
//!
//! Selalu dibaca dari pool tulis (primary), bukan replika: user yang baru saja
//! membayar tidak boleh ditolak beberapa detik hanya karena replika belum
//! menyusul.

use serde::Serialize;
use sqlx::{MySqlPool, Row};

pub const TRIAL_HARI: i64 = 3;

#[derive(Debug, Serialize, Clone)]
pub struct Trial {
    /// detik epoch, supaya hitung mundur di browser tidak bergantung zona waktu
    pub mulai: i64,
    pub berakhir: i64,
    pub aktif: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct LanggananAktif {
    pub plan_id: i32,
    pub nama: String,
    /// None = seumur hidup
    pub berakhir: Option<i64>,
}

/// Mulai trial pada pemakaian pertama (idempoten), lalu kembalikan jendelanya.
/// Trial dihitung sejak user pertama kali menyentuh fitur premium setelah rilis,
/// jadi user lama tidak kehilangan trial-nya.
pub async fn pastikan_trial(pool: &MySqlPool, user_id: &str) -> Result<Option<Trial>, sqlx::Error> {
    sqlx::query("UPDATE dbquizapp.users SET trial_started_at = NOW() WHERE id = ? AND trial_started_at IS NULL")
        .bind(user_id)
        .execute(pool)
        .await?;
    let row = sqlx::query(
        "SELECT UNIX_TIMESTAMP(trial_started_at) AS mulai, \
                UNIX_TIMESTAMP(trial_started_at + INTERVAL ? DAY) AS berakhir, \
                (NOW() < trial_started_at + INTERVAL ? DAY) AS aktif \
         FROM dbquizapp.users WHERE id = ? AND trial_started_at IS NOT NULL",
    )
    .bind(TRIAL_HARI)
    .bind(TRIAL_HARI)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| Trial {
        mulai: r.try_get::<i64, _>("mulai").unwrap_or_default(),
        berakhir: r.try_get::<i64, _>("berakhir").unwrap_or_default(),
        aktif: r.try_get::<i64, _>("aktif").unwrap_or(0) == 1,
    }))
}

pub async fn langganan_aktif(pool: &MySqlPool, user_id: &str) -> Result<Option<LanggananAktif>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT us.plan_id, pp.name, UNIX_TIMESTAMP(us.end_date) AS berakhir \
         FROM dbquizapp.user_subscriptions us JOIN dbquizapp.premium_plans pp ON pp.id = us.plan_id \
         WHERE us.user_id = ? AND us.status = 'active' AND (us.end_date IS NULL OR us.end_date > NOW()) \
         ORDER BY us.end_date IS NULL DESC, us.end_date DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| LanggananAktif {
        plan_id: r.try_get("plan_id").unwrap_or_default(),
        nama: r.try_get("name").unwrap_or_default(),
        berakhir: r.try_get::<Option<i64>, _>("berakhir").ok().flatten(),
    }))
}

/// Satu-satunya pintu keputusan akses paket premium.
pub async fn punya_akses_premium(pool: &MySqlPool, user_id: &str) -> bool {
    if matches!(langganan_aktif(pool, user_id).await, Ok(Some(_))) {
        return true;
    }
    matches!(pastikan_trial(pool, user_id).await, Ok(Some(t)) if t.aktif)
}
