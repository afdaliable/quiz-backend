//! Klaim pembayaran QRIS statis: dibuat saat QR ditampilkan, "diklaim" saat user
//! menekan "saya sudah bayar", lalu disetujui/ditolak pemilik setelah mengecek
//! mutasi. Lihat migrations/20261001_payment_claims.sql untuk alasan alurnya.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{MySqlPool, Row};

#[derive(Debug, Serialize, Clone)]
pub struct PaymentClaim {
    pub id: i64,
    pub user_id: String,
    pub plan_id: i32,
    pub base_amount: i32,
    pub unique_amount: i32,
    pub status: String,
    pub subscription_id: Option<i32>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decided_by: Option<String>,
    pub catatan: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

fn dari_row(r: &sqlx::mysql::MySqlRow) -> PaymentClaim {
    PaymentClaim {
        id: r.try_get("id").unwrap_or_default(),
        user_id: r.try_get("user_id").unwrap_or_default(),
        plan_id: r.try_get("plan_id").unwrap_or_default(),
        base_amount: r.try_get("base_amount").unwrap_or_default(),
        unique_amount: r.try_get("unique_amount").unwrap_or_default(),
        status: r.try_get("status").unwrap_or_default(),
        subscription_id: r.try_get("subscription_id").ok().flatten(),
        claimed_at: r.try_get("claimed_at").ok().flatten(),
        expires_at: r.try_get("expires_at").ok().flatten(),
        decided_at: r.try_get("decided_at").ok().flatten(),
        decided_by: r.try_get("decided_by").ok().flatten(),
        catatan: r.try_get("catatan").ok().flatten(),
        created_at: r.try_get("created_at").ok().flatten(),
    }
}

const KOLOM: &str = "id, user_id, plan_id, base_amount, unique_amount, status, subscription_id, \
                     claimed_at, expires_at, decided_at, decided_by, catatan, created_at";

/// Nominal unik hanya boleh dipakai satu klaim yang masih berjalan, supaya dua
/// pembelian tidak tertukar saat dicocokkan ke mutasi.
pub async fn nominal_dipakai(pool: &MySqlPool, unique_amount: i32) -> Result<bool, sqlx::Error> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dbquizapp.payment_claims \
         WHERE unique_amount = ? AND status IN ('menunggu','diklaim')",
    )
    .bind(unique_amount)
    .fetch_one(pool)
    .await?;
    Ok(n > 0)
}

pub async fn buat(
    pool: &MySqlPool,
    user_id: &str,
    plan_id: i32,
    base_amount: i32,
    unique_amount: i32,
) -> Result<i64, sqlx::Error> {
    let r = sqlx::query(
        "INSERT INTO dbquizapp.payment_claims (user_id, plan_id, base_amount, unique_amount) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(plan_id)
    .bind(base_amount)
    .bind(unique_amount)
    .execute(pool)
    .await?;
    Ok(r.last_insert_id() as i64)
}

pub async fn ambil(pool: &MySqlPool, id: i64) -> Result<Option<PaymentClaim>, sqlx::Error> {
    let row = sqlx::query(&format!("SELECT {KOLOM} FROM dbquizapp.payment_claims WHERE id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| dari_row(&r)))
}

/// Tandai user sudah menekan "saya sudah bayar" dan catat langganan yang
/// diberikan di muka. `auto_revoke_hours` 0 berarti tanpa batas waktu.
pub async fn tandai_diklaim(
    pool: &MySqlPool,
    id: i64,
    subscription_id: i32,
    auto_revoke_hours: u32,
) -> Result<u64, sqlx::Error> {
    let sql = if auto_revoke_hours == 0 {
        "UPDATE dbquizapp.payment_claims SET status='diklaim', subscription_id=?, claimed_at=NOW(), \
         expires_at=NULL WHERE id=? AND status='menunggu'"
            .to_string()
    } else {
        format!(
            "UPDATE dbquizapp.payment_claims SET status='diklaim', subscription_id=?, claimed_at=NOW(), \
             expires_at=DATE_ADD(NOW(), INTERVAL {auto_revoke_hours} HOUR) WHERE id=? AND status='menunggu'"
        )
    };
    let r = sqlx::query(&sql).bind(subscription_id).bind(id).execute(pool).await?;
    Ok(r.rows_affected())
}

/// Setujui atau tolak. Hanya klaim berstatus 'diklaim' yang bisa diputuskan,
/// jadi tekanan tombol ganda di Telegram tidak menimpa keputusan sebelumnya.
pub async fn putuskan(
    pool: &MySqlPool,
    id: i64,
    setuju: bool,
    oleh: &str,
) -> Result<u64, sqlx::Error> {
    let status = if setuju { "disetujui" } else { "ditolak" };
    let r = sqlx::query(
        "UPDATE dbquizapp.payment_claims SET status=?, decided_at=NOW(), decided_by=?, expires_at=NULL \
         WHERE id=? AND status='diklaim'",
    )
    .bind(status)
    .bind(oleh)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected())
}

/// Klaim yang lewat batas waktu tanpa keputusan: langganannya dibatalkan dan
/// klaimnya ditandai kedaluwarsa. Lupa mengecek berarti akses mati, bukan
/// gratis selamanya.
pub async fn cabut_kedaluwarsa(pool: &MySqlPool) -> Result<Vec<PaymentClaim>, sqlx::Error> {
    let rows = sqlx::query(&format!(
        "SELECT {KOLOM} FROM dbquizapp.payment_claims \
         WHERE status='diklaim' AND expires_at IS NOT NULL AND expires_at <= NOW()"
    ))
    .fetch_all(pool)
    .await?;
    let klaim: Vec<PaymentClaim> = rows.iter().map(dari_row).collect();
    for k in &klaim {
        if let Some(sub) = k.subscription_id {
            sqlx::query("UPDATE dbquizapp.user_subscriptions SET status='cancelled', updated_at=NOW() WHERE id=?")
                .bind(sub)
                .execute(pool)
                .await?;
        }
        sqlx::query("UPDATE dbquizapp.payment_claims SET status='kedaluwarsa', decided_at=NOW(), \
                     decided_by='sistem', catatan='batas waktu konfirmasi terlewat' WHERE id=?")
            .bind(k.id)
            .execute(pool)
            .await?;
    }
    Ok(klaim)
}

pub async fn batalkan_langganan(pool: &MySqlPool, subscription_id: i32) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE dbquizapp.user_subscriptions SET status='cancelled', updated_at=NOW() WHERE id=?")
        .bind(subscription_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn diblokir(pool: &MySqlPool, user_id: &str) -> Result<bool, sqlx::Error> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dbquizapp.payment_claim_blocks WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(n > 0)
}

pub async fn blokir(pool: &MySqlPool, user_id: &str, alasan: &str, oleh: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO dbquizapp.payment_claim_blocks (user_id, alasan, blocked_by) VALUES (?, ?, ?) \
         ON DUPLICATE KEY UPDATE alasan=VALUES(alasan), blocked_at=NOW(), blocked_by=VALUES(blocked_by)",
    )
    .bind(user_id)
    .bind(alasan)
    .bind(oleh)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn buka_blokir(pool: &MySqlPool, user_id: &str) -> Result<u64, sqlx::Error> {
    let r = sqlx::query("DELETE FROM dbquizapp.payment_claim_blocks WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}

/// Buat langganan di muka. `duration_days` 0 dianggap seumur hidup (end_date NULL),
/// sesuai komentar kolom end_date.
pub async fn buat_langganan(
    pool: &MySqlPool,
    user_id: &str,
    plan_id: i32,
    duration_days: i32,
) -> Result<i32, sqlx::Error> {
    let r = if duration_days > 0 {
        sqlx::query(
            "INSERT INTO dbquizapp.user_subscriptions (user_id, plan_id, start_date, end_date, status) \
             VALUES (?, ?, NOW(), DATE_ADD(NOW(), INTERVAL ? DAY), 'active')",
        )
        .bind(user_id)
        .bind(plan_id)
        .bind(duration_days)
        .execute(pool)
        .await?
    } else {
        sqlx::query(
            "INSERT INTO dbquizapp.user_subscriptions (user_id, plan_id, start_date, end_date, status) \
             VALUES (?, ?, NOW(), NULL, 'active')",
        )
        .bind(user_id)
        .bind(plan_id)
        .execute(pool)
        .await?
    };
    Ok(r.last_insert_id() as i32)
}

pub async fn daftar(pool: &MySqlPool, status: Option<&str>, limit: u32) -> Result<Vec<PaymentClaim>, sqlx::Error> {
    let rows = match status {
        Some(s) => {
            sqlx::query(&format!(
                "SELECT {KOLOM} FROM dbquizapp.payment_claims WHERE status=? ORDER BY id DESC LIMIT ?"
            ))
            .bind(s)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query(&format!(
                "SELECT {KOLOM} FROM dbquizapp.payment_claims ORDER BY id DESC LIMIT ?"
            ))
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows.iter().map(dari_row).collect())
}
