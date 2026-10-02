//! Pembayaran premium lewat KlikQRIS.
//!
//!   POST /payment/klikqris/create        {plan_id} -> QR dinamis + total yang harus dibayar
//!   GET  /payment/klikqris/{order_id}    status (dan menyembuhkan diri bila webhook terlewat)
//!   POST /payment/klikqris/webhook       notifikasi KlikQRIS: PAID / EXPIRED
//!
//! Webhook divalidasi dengan signature yang tersimpan saat transaksi dibuat,
//! jadi instance backend mana pun bisa memprosesnya walau tidak punya
//! kredensial API (trafik publik terbagi antara PC dan NAS). Kalau kredensial
//! ada, status juga dikonfirmasi ulang ke API KlikQRIS.
//!
//! Penyelesaian pembayaran idempoten: hanya satu UPDATE yang bisa memindahkan
//! transaksi dari PENDING/EXPIRED ke PAID, jadi webhook ganda atau webhook yang
//! berbarengan dengan polling tidak pernah membuat langganan dua kali.

use actix_web::{get, post, web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::dao::payment_claim_dao::buat_langganan;
use crate::middleware::auth_middleware::AuthenticatedUser;
use crate::service::klikqris_service::{nominal, signature_sama, status_baku, KlikqrisService};
use crate::AppState;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(buat_transaksi).service(webhook).service(status_transaksi);
}

#[derive(Debug, PartialEq)]
pub enum Aksi {
    Bayar,
    Kedaluwarsa,
    Abaikan(&'static str),
    Tolak(&'static str),
}

/// Keputusan atas satu notifikasi, dari data tersimpan vs data kiriman.
pub fn putuskan(
    status_db: &str,
    total_db: i64,
    signature_db: &str,
    status_kiriman: &str,
    total_kiriman: Option<i64>,
    signature_kiriman: &str,
) -> Aksi {
    if !signature_sama(signature_db, signature_kiriman) {
        return Aksi::Tolak("signature tidak cocok");
    }
    match status_baku(status_kiriman) {
        "PAID" => {
            if total_kiriman != Some(total_db) {
                return Aksi::Tolak("nominal tidak cocok");
            }
            if status_db == "PAID" {
                Aksi::Abaikan("sudah lunas")
            } else {
                Aksi::Bayar
            }
        }
        "EXPIRED" if status_db == "PENDING" => Aksi::Kedaluwarsa,
        _ => Aksi::Abaikan("tidak ada perubahan"),
    }
}

#[derive(sqlx::FromRow)]
struct Baris {
    user_id: String,
    plan_id: i32,
    total_amount: i32,
    status: String,
    signature: String,
}

async fn ambil(pool: &sqlx::MySqlPool, order_id: &str) -> Option<Baris> {
    sqlx::query_as::<_, Baris>(
        "SELECT user_id, plan_id, total_amount, status, signature FROM dbquizapp.klikqris_transactions WHERE order_id = ?",
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

/// Pindahkan ke PAID lalu buat langganan. Hanya pemenang UPDATE yang membuat
/// langganan, sehingga aman dipanggil berkali-kali dari webhook maupun polling.
async fn lunasi(pool: &sqlx::MySqlPool, order_id: &str, paid_at: Option<&str>, payload: &str) -> Result<bool, String> {
    let r = sqlx::query(
        "UPDATE dbquizapp.klikqris_transactions SET status='PAID', paid_at=?, webhook_payload=? \
         WHERE order_id=? AND status IN ('PENDING','EXPIRED')",
    )
    .bind(paid_at)
    .bind(payload)
    .bind(order_id)
    .execute(pool)
    .await
    .map_err(|e| format!("gagal menandai lunas: {e}"))?;
    if r.rows_affected() == 0 {
        return Ok(false); // sudah diproses oleh panggilan lain
    }
    let b = ambil(pool, order_id).await.ok_or("transaksi hilang setelah dilunasi")?;
    let durasi: i32 = sqlx::query_scalar("SELECT duration_days FROM dbquizapp.premium_plans WHERE id = ?")
        .bind(b.plan_id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("plan tidak ditemukan: {e}"))?;
    let sub = buat_langganan(pool, &b.user_id, b.plan_id, durasi)
        .await
        .map_err(|e| format!("gagal membuat langganan: {e}"))?;
    sqlx::query("UPDATE dbquizapp.klikqris_transactions SET subscription_id=? WHERE order_id=?")
        .bind(sub)
        .bind(order_id)
        .execute(pool)
        .await
        .map_err(|e| format!("gagal mencatat langganan: {e}"))?;
    println!("[klikqris] {order_id} lunas -> langganan #{sub} untuk {}", b.user_id);
    Ok(true)
}

#[derive(Deserialize)]
pub struct BuatRequest {
    pub plan_id: i32,
}

#[post("/payment/klikqris/create")]
async fn buat_transaksi(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
    req: web::Json<BuatRequest>,
) -> impl Responder {
    let cfg = match state.config.get_klikqris() {
        Some(c) => c,
        None => {
            return HttpResponse::ServiceUnavailable()
                .json(json!({"error": "pembayaran_belum_disetel", "message": "Pembayaran sedang tidak tersedia."}))
        }
    };
    let pool = &*state.context.soal.pool;
    let plan: Option<(i32, String, f64)> =
        sqlx::query_as("SELECT id, name, price FROM dbquizapp.premium_plans WHERE id = ?")
            .bind(req.plan_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);
    let (plan_id, nama, harga) = match plan {
        Some(p) => p,
        None => return HttpResponse::NotFound().json(json!({"error": "plan_tidak_ada"})),
    };

    let order_id = format!(
        "QZ{plan_id}-{}-{:04x}",
        chrono::Utc::now().timestamp_millis(),
        rand::random::<u16>()
    );
    let trx = match KlikqrisService::new(cfg)
        .buat(&order_id, harga as i64, &format!("Langganan {nama}"))
        .await
    {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[klikqris] create {order_id} gagal: {e}");
            return HttpResponse::BadGateway()
                .json(json!({"error": "gateway_gagal", "message": "Gagal membuat tagihan QRIS, coba lagi."}));
        }
    };
    if trx.signature.is_empty() {
        eprintln!("[klikqris] create {order_id}: balasan tanpa signature, transaksi tidak disimpan");
        return HttpResponse::BadGateway().json(json!({"error": "gateway_tanpa_signature"}));
    }
    if let Err(e) = sqlx::query(
        "INSERT INTO dbquizapp.klikqris_transactions \
         (order_id, user_id, plan_id, amount, total_amount, status, signature, qris_url, report_url, expired_at) \
         VALUES (?, ?, ?, ?, ?, 'PENDING', ?, ?, ?, ?)",
    )
    .bind(&trx.order_id)
    .bind(&user.user_id)
    .bind(plan_id)
    .bind(trx.amount as i32)
    .bind(trx.total_amount as i32)
    .bind(&trx.signature)
    .bind(&trx.qris_url)
    .bind(&trx.report_url)
    .bind(&trx.expired_at)
    .execute(pool)
    .await
    {
        eprintln!("[klikqris] gagal menyimpan {order_id}: {e:?}");
        return HttpResponse::InternalServerError().json(json!({"error": "gagal_menyimpan"}));
    }

    HttpResponse::Created().json(json!({
        "order_id": trx.order_id,
        "plan": nama,
        "amount": trx.amount,
        "total_amount": trx.total_amount,
        "qris_image": trx.qris_image,
        "qris_url": trx.qris_url,
        "expired_at": trx.expired_at,
        "report_url": trx.report_url,
        // dipakai Snap KlikQRIS (data-signature) bila frontend memilih popup mereka
        "signature": trx.signature,
        "status": "PENDING",
    }))
}

#[get("/payment/klikqris/{order_id}")]
async fn status_transaksi(
    path: web::Path<String>,
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
) -> impl Responder {
    let order_id = path.into_inner();
    let pool = &*state.context.soal.pool;
    let b = match ambil(pool, &order_id).await {
        Some(b) if b.user_id == user.user_id => b,
        Some(_) => return HttpResponse::Forbidden().json(json!({"error": "bukan_transaksi_anda"})),
        None => return HttpResponse::NotFound().json(json!({"error": "transaksi_tidak_ada"})),
    };

    // Penyembuhan diri: webhook bisa terlewat, jadi transaksi yang masih PENDING
    // dicek langsung ke KlikQRIS (bila instance ini punya kredensial).
    let mut status = b.status.clone();
    if status == "PENDING" {
        if let Some(cfg) = state.config.get_klikqris() {
            if let Ok(t) = KlikqrisService::new(cfg).cek(&order_id).await {
                match putuskan(&b.status, b.total_amount as i64, &b.signature, &t.status,
                               Some(t.total_amount), &t.signature) {
                    Aksi::Bayar => {
                        if let Err(e) = lunasi(pool, &order_id, t.paid_at.as_deref(), "via status API").await {
                            eprintln!("[klikqris] gagal melunasi {order_id} lewat polling: {e}");
                        }
                    }
                    Aksi::Kedaluwarsa => {
                        let _ = sqlx::query("UPDATE dbquizapp.klikqris_transactions SET status='EXPIRED' \
                                             WHERE order_id=? AND status='PENDING'")
                            .bind(&order_id).execute(pool).await;
                    }
                    _ => {}
                }
                if let Some(baru) = ambil(pool, &order_id).await {
                    status = baru.status;
                }
            }
        }
    }
    HttpResponse::Ok().json(json!({"order_id": order_id, "status": status, "total_amount": b.total_amount}))
}

#[post("/payment/klikqris/webhook")]
async fn webhook(state: web::Data<AppState<'_>>, body: web::Bytes) -> impl Responder {
    let mentah = String::from_utf8_lossy(&body).to_string();
    let p: Value = match serde_json::from_str(&mentah) {
        Ok(v) => v,
        Err(_) => return HttpResponse::BadRequest().json(json!({"error": "payload_bukan_json"})),
    };
    let order_id = p["order_id"].as_str().unwrap_or_default().to_string();
    let pool = &*state.context.soal.pool;

    // Transaksi yang bukan milik app ini (mis. dibuat dari panel KlikQRIS) juga
    // bisa dikirim ke webhook global. Balas 200 supaya tidak dikirim ulang terus.
    let b = match ambil(pool, &order_id).await {
        Some(b) => b,
        None => return HttpResponse::Ok().json(json!({"ok": true, "ignored": "order_id tidak dikenal"})),
    };

    let aksi = putuskan(
        &b.status,
        b.total_amount as i64,
        &b.signature,
        p["status"].as_str().unwrap_or_default(),
        nominal(&p["total_amount"]),
        p["signature"].as_str().unwrap_or_default(),
    );
    match aksi {
        Aksi::Tolak(alasan) => {
            eprintln!("[klikqris] webhook {order_id} DITOLAK: {alasan}");
            HttpResponse::Unauthorized().json(json!({"error": alasan}))
        }
        Aksi::Abaikan(alasan) => HttpResponse::Ok().json(json!({"ok": true, "ignored": alasan})),
        Aksi::Kedaluwarsa => {
            let _ = sqlx::query("UPDATE dbquizapp.klikqris_transactions SET status='EXPIRED', webhook_payload=? \
                                 WHERE order_id=? AND status='PENDING'")
                .bind(&mentah).bind(&order_id).execute(pool).await;
            HttpResponse::Ok().json(json!({"ok": true, "status": "EXPIRED"}))
        }
        Aksi::Bayar => {
            // Lapis kedua bila kredensial ada: pastikan KlikQRIS sendiri menyatakan lunas.
            if let Some(cfg) = state.config.get_klikqris() {
                if let Ok(t) = KlikqrisService::new(cfg).cek(&order_id).await {
                    if t.status != "PAID" {
                        eprintln!("[klikqris] webhook {order_id} PAID tapi API bilang {}", t.status);
                        return HttpResponse::Conflict().json(json!({"error": "status_api_belum_lunas"}));
                    }
                }
            }
            match lunasi(pool, &order_id, p["payment_date"].as_str(), &mentah).await {
                Ok(_) => HttpResponse::Ok().json(json!({"ok": true, "status": "PAID"})),
                Err(e) => {
                    eprintln!("[klikqris] webhook {order_id}: {e}");
                    // non-200 supaya KlikQRIS mengirim ulang dan pembayaran tidak hilang
                    HttpResponse::InternalServerError().json(json!({"error": "gagal_memproses"}))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SIG: &str = "ohRISdH4ABDvOlDTUCBTEnBndwUYK0177659abcd";

    #[test]
    fn lunas_dengan_signature_dan_nominal_benar() {
        assert_eq!(putuskan("PENDING", 1084, SIG, "PAID", Some(1084), SIG), Aksi::Bayar);
        // status cek API memakai SUCCESS
        assert_eq!(putuskan("PENDING", 1084, SIG, "SUCCESS", Some(1084), SIG), Aksi::Bayar);
    }

    #[test]
    fn webhook_palsu_ditolak() {
        assert_eq!(putuskan("PENDING", 1084, SIG, "PAID", Some(1084), "palsu"), Aksi::Tolak("signature tidak cocok"));
        assert_eq!(putuskan("PENDING", 1084, SIG, "PAID", Some(1084), ""), Aksi::Tolak("signature tidak cocok"));
    }

    #[test]
    fn nominal_dimanipulasi_ditolak() {
        assert_eq!(putuskan("PENDING", 99084, SIG, "PAID", Some(1000), SIG), Aksi::Tolak("nominal tidak cocok"));
        assert_eq!(putuskan("PENDING", 99084, SIG, "PAID", None, SIG), Aksi::Tolak("nominal tidak cocok"));
    }

    #[test]
    fn notifikasi_ganda_tidak_memberi_akses_dua_kali() {
        assert_eq!(putuskan("PAID", 1084, SIG, "PAID", Some(1084), SIG), Aksi::Abaikan("sudah lunas"));
    }

    #[test]
    fn bayar_setelah_kedaluwarsa_tetap_diterima() {
        // dokumentasi KlikQRIS: PENDING atau EXPIRED -> PAID
        assert_eq!(putuskan("EXPIRED", 1084, SIG, "PAID", Some(1084), SIG), Aksi::Bayar);
    }

    #[test]
    fn kedaluwarsa_hanya_dari_pending() {
        assert_eq!(putuskan("PENDING", 1084, SIG, "EXPIRED", Some(1084), SIG), Aksi::Kedaluwarsa);
        assert_eq!(putuskan("PAID", 1084, SIG, "EXPIRED", Some(1084), SIG), Aksi::Abaikan("tidak ada perubahan"));
    }
}
