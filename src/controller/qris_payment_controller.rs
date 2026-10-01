//! Pembayaran QRIS statis milik sendiri (GoPay Merchant), tanpa payment gateway.
//!
//! Alurnya sengaja optimistis supaya pembeli tidak menunggu konfirmasi manual:
//!   1. `POST /payment/qris/start` -> klaim dibuat, QR dinamis bernominal unik;
//!   2. `POST /payment/qris/{id}/sudah-bayar` -> premium AKTIF saat itu juga,
//!      pemilik dapat notifikasi Telegram berisi dua tombol keputusan;
//!   3. pemilik menekan Setujui/Tolak dari HP -> webhook Telegram memutuskan;
//!      menolak sekaligus memblokir user dari tombol klaim berikutnya;
//!   4. klaim yang tidak diputuskan sampai batas waktu dicabut otomatis
//!      (tugas latar belakang di main.rs).
//!
//! Risiko yang disadari: siapa pun bisa menekan "sudah bayar" dan menikmati
//! akses sampai keputusan datang. Jendelanya dibatasi `auto_revoke_hours`, dan
//! penolakan memblokir pengulangan.

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;

use crate::dao::payment_claim_dao as klaim_dao;
use crate::middleware::admin_middleware::AdminMiddleware;
use crate::middleware::auth_middleware::AuthenticatedUser;
use crate::service::qris_service;
use crate::service::telegram_service::{parse_callback, TelegramService};
use crate::AppState;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(mulai)
        .service(sudah_bayar)
        .service(status_klaim)
        .service(telegram_webhook);
    // Jalur cadangan: kalau Telegram bermasalah, keputusan tetap bisa diambil
    // dari halaman admin.
    cfg.service(
        web::scope("/admin/payment-claims")
            .wrap(AdminMiddleware::new())
            .route("", web::get().to(admin_daftar))
            .route("/{id}/setujui", web::post().to(admin_setujui))
            .route("/{id}/tolak", web::post().to(admin_tolak))
            .route("/blokir/{user_id}", web::delete().to(admin_buka_blokir)),
    );
}

#[derive(Deserialize)]
pub struct DaftarQuery {
    pub status: Option<String>,
    pub limit: Option<u32>,
}

async fn admin_daftar(state: web::Data<AppState<'_>>, q: web::Query<DaftarQuery>) -> impl Responder {
    match klaim_dao::daftar(&state.context.soal.pool, q.status.as_deref(), q.limit.unwrap_or(100)).await {
        Ok(v) => HttpResponse::Ok().json(json!({"claims": v, "total": v.len()})),
        Err(e) => {
            eprintln!("gagal daftar klaim: {e:?}");
            HttpResponse::InternalServerError().json(json!({"error": "gagal_daftar"}))
        }
    }
}

async fn admin_putuskan(state: &web::Data<AppState<'_>>, id: i64, setuju: bool) -> HttpResponse {
    let pool = &*state.context.soal.pool;
    let klaim = match klaim_dao::ambil(pool, id).await {
        Ok(Some(k)) => k,
        _ => return HttpResponse::NotFound().json(json!({"error": "klaim_tidak_ada"})),
    };
    match klaim_dao::putuskan(pool, id, setuju, "admin").await {
        Ok(1) => {}
        _ => return HttpResponse::Conflict().json(json!({"error": "sudah_diproses", "status": klaim.status})),
    }
    if !setuju {
        if let Some(sub) = klaim.subscription_id {
            let _ = klaim_dao::batalkan_langganan(pool, sub).await;
        }
        let _ = klaim_dao::blokir(pool, &klaim.user_id, "klaim pembayaran ditolak", "admin").await;
    }
    HttpResponse::Ok().json(json!({"id": id, "status": if setuju {"disetujui"} else {"ditolak"}}))
}

async fn admin_setujui(path: web::Path<i64>, state: web::Data<AppState<'_>>) -> impl Responder {
    admin_putuskan(&state, path.into_inner(), true).await
}

async fn admin_tolak(path: web::Path<i64>, state: web::Data<AppState<'_>>) -> impl Responder {
    admin_putuskan(&state, path.into_inner(), false).await
}

async fn admin_buka_blokir(path: web::Path<String>, state: web::Data<AppState<'_>>) -> impl Responder {
    match klaim_dao::buka_blokir(&state.context.soal.pool, &path.into_inner()).await {
        Ok(n) => HttpResponse::Ok().json(json!({"dibuka": n})),
        Err(e) => {
            eprintln!("gagal buka blokir: {e:?}");
            HttpResponse::InternalServerError().json(json!({"error": "gagal_buka_blokir"}))
        }
    }
}

#[derive(Deserialize)]
pub struct MulaiRequest {
    pub plan_id: i32,
}

/// Nominal unik dicoba beberapa kali supaya tidak bertabrakan dengan klaim lain
/// yang masih berjalan -- pencocokan ke mutasi hanya mengandalkan nominal.
async fn nominal_unik(pool: &sqlx::MySqlPool, harga: i64) -> Option<i32> {
    for _ in 0..40 {
        let suffix: u16 = rand::random::<u16>() % 1000;
        let kandidat = qris_service::unique_amount(harga as u64, suffix) as i32;
        match klaim_dao::nominal_dipakai(pool, kandidat).await {
            Ok(false) => return Some(kandidat),
            Ok(true) => continue,
            Err(e) => {
                eprintln!("gagal cek nominal unik: {e:?}");
                return None;
            }
        }
    }
    None
}

#[post("/payment/qris/start")]
async fn mulai(
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
    req: web::Json<MulaiRequest>,
) -> impl Responder {
    let qris_cfg = match state.config.get_qris() {
        Some(q) => q,
        None => {
            return HttpResponse::ServiceUnavailable()
                .json(json!({"error": "qris_belum_disetel", "message": "Pembayaran QRIS belum dikonfigurasi."}))
        }
    };
    let pool = &*state.context.soal.pool;

    if klaim_dao::diblokir(pool, &user.user_id).await.unwrap_or(false) {
        return HttpResponse::Forbidden().json(json!({
            "error": "diblokir",
            "message": "Akun ini tidak bisa memakai konfirmasi mandiri. Hubungi admin."
        }));
    }

    let plan: Option<(i32, String, f64, i32)> = sqlx::query_as(
        "SELECT id, name, price, duration_days FROM dbquizapp.premium_plans WHERE id = ?",
    )
    .bind(req.plan_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    let (plan_id, nama_plan, harga, _durasi) = match plan {
        Some(p) => p,
        None => return HttpResponse::NotFound().json(json!({"error": "plan_tidak_ada"})),
    };

    let unique = match nominal_unik(pool, harga as i64).await {
        Some(u) => u,
        None => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": "nominal_penuh", "message": "Gagal menyiapkan nominal unik, coba lagi."}))
        }
    };
    let payload = match qris_service::to_dynamic(&qris_cfg.static_payload, unique as u64) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("QRIS statis tidak bisa dikonversi: {e}");
            return HttpResponse::InternalServerError().json(json!({"error": "qris_tidak_valid"}));
        }
    };
    match klaim_dao::buat(pool, &user.user_id, plan_id, harga as i32, unique).await {
        Ok(id) => HttpResponse::Created().json(json!({
            "claim_id": id,
            "plan": nama_plan,
            "amount": unique,
            "base_amount": harga as i64,
            "qris_payload": payload,
            "catatan": "Bayar TEPAT sejumlah nominal di atas, lalu tekan 'Saya sudah bayar'."
        })),
        Err(e) => {
            eprintln!("gagal membuat klaim: {e:?}");
            HttpResponse::InternalServerError().json(json!({"error": "gagal_membuat_klaim"}))
        }
    }
}

#[post("/payment/qris/{id}/sudah-bayar")]
async fn sudah_bayar(
    path: web::Path<i64>,
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
) -> impl Responder {
    let id = path.into_inner();
    let pool = &*state.context.soal.pool;
    let klaim = match klaim_dao::ambil(pool, id).await {
        Ok(Some(k)) => k,
        _ => return HttpResponse::NotFound().json(json!({"error": "klaim_tidak_ada"})),
    };
    if klaim.user_id != user.user_id {
        return HttpResponse::Forbidden().json(json!({"error": "bukan_klaim_anda"}));
    }
    if klaim.status != "menunggu" {
        return HttpResponse::Conflict()
            .json(json!({"error": "sudah_diproses", "status": klaim.status}));
    }
    if klaim_dao::diblokir(pool, &user.user_id).await.unwrap_or(false) {
        return HttpResponse::Forbidden().json(json!({"error": "diblokir"}));
    }

    let durasi: i32 = sqlx::query_scalar("SELECT duration_days FROM dbquizapp.premium_plans WHERE id = ?")
        .bind(klaim.plan_id)
        .fetch_one(pool)
        .await
        .unwrap_or(30);
    let sub_id = match klaim_dao::buat_langganan(pool, &klaim.user_id, klaim.plan_id, durasi).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("gagal membuat langganan: {e:?}");
            return HttpResponse::InternalServerError().json(json!({"error": "gagal_aktivasi"}));
        }
    };
    let jam = state.config.get_qris().map(|q| q.auto_revoke_hours).unwrap_or(48);
    match klaim_dao::tandai_diklaim(pool, id, sub_id, jam).await {
        Ok(1) => {}
        Ok(_) => {
            // Klaim berubah status di antara pembacaan dan penandaan: batalkan
            // langganan yang baru dibuat supaya tidak ada akses tanpa klaim.
            let _ = klaim_dao::batalkan_langganan(pool, sub_id).await;
            return HttpResponse::Conflict().json(json!({"error": "sudah_diproses"}));
        }
        Err(e) => {
            eprintln!("gagal menandai klaim: {e:?}");
            let _ = klaim_dao::batalkan_langganan(pool, sub_id).await;
            return HttpResponse::InternalServerError().json(json!({"error": "gagal_aktivasi"}));
        }
    }

    if let Some(tg) = state.config.get_telegram() {
        let batas = if jam > 0 { Some(format!("{jam} jam lagi")) } else { None };
        let label = user.email.clone().unwrap_or_else(|| user.user_id.clone());
        if let Err(e) = TelegramService::new(tg)
            .kirim_klaim(id, "plan", klaim.unique_amount as i64, &label, batas.as_deref())
            .await
        {
            // Notifikasi gagal bukan alasan membatalkan akses yang sudah
            // diberikan; pemilik masih bisa memutuskan dari halaman admin.
            eprintln!("notifikasi Telegram gagal untuk klaim {id}: {e}");
        }
    }

    HttpResponse::Ok().json(json!({
        "status": "aktif",
        "message": "Akses premium sudah aktif. Pembayaran sedang diverifikasi pemilik.",
        "auto_revoke_hours": jam
    }))
}

#[get("/payment/qris/{id}")]
async fn status_klaim(
    path: web::Path<i64>,
    state: web::Data<AppState<'_>>,
    user: AuthenticatedUser,
) -> impl Responder {
    let id = path.into_inner();
    match klaim_dao::ambil(&state.context.soal.pool, id).await {
        Ok(Some(k)) if k.user_id == user.user_id => HttpResponse::Ok().json(k),
        Ok(Some(_)) => HttpResponse::Forbidden().json(json!({"error": "bukan_klaim_anda"})),
        _ => HttpResponse::NotFound().json(json!({"error": "klaim_tidak_ada"})),
    }
}

/// Webhook Telegram. Tidak memakai JWT -- keasliannya dijamin header
/// `X-Telegram-Bot-Api-Secret-Token` yang hanya diketahui Telegram dan server.
#[post("/payment/telegram/webhook")]
async fn telegram_webhook(
    http_req: HttpRequest,
    state: web::Data<AppState<'_>>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let tg_cfg = match state.config.get_telegram() {
        Some(t) => t,
        None => return HttpResponse::ServiceUnavailable().finish(),
    };
    let diberikan = http_req
        .headers()
        .get("X-Telegram-Bot-Api-Secret-Token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if diberikan != tg_cfg.webhook_secret {
        return HttpResponse::Unauthorized().finish();
    }

    let cb = &body["callback_query"];
    let (data, cb_id, oleh) = (
        cb["data"].as_str().unwrap_or(""),
        cb["id"].as_str().unwrap_or(""),
        cb["from"]["username"].as_str().unwrap_or("telegram").to_string(),
    );
    let (setuju, claim_id) = match parse_callback(data) {
        Some(v) => v,
        None => return HttpResponse::Ok().json(json!({"ok": true})), // pesan lain diabaikan
    };

    let pool = &*state.context.soal.pool;
    let tg = TelegramService::new(tg_cfg);
    let klaim = match klaim_dao::ambil(pool, claim_id).await {
        Ok(Some(k)) => k,
        _ => {
            let _ = tg.jawab_callback(cb_id, "Klaim tidak ditemukan").await;
            return HttpResponse::Ok().json(json!({"ok": true}));
        }
    };
    match klaim_dao::putuskan(pool, claim_id, setuju, &oleh).await {
        Ok(1) => {}
        _ => {
            let _ = tg
                .jawab_callback(cb_id, &format!("Sudah diputuskan sebelumnya ({})", klaim.status))
                .await;
            return HttpResponse::Ok().json(json!({"ok": true}));
        }
    }
    if setuju {
        let _ = tg.jawab_callback(cb_id, "Disetujui. Akses tetap aktif.").await;
    } else {
        if let Some(sub) = klaim.subscription_id {
            let _ = klaim_dao::batalkan_langganan(pool, sub).await;
        }
        let _ = klaim_dao::blokir(pool, &klaim.user_id, "klaim pembayaran ditolak", &oleh).await;
        let _ = tg.jawab_callback(cb_id, "Ditolak. Akses dicabut, user diblokir.").await;
    }
    HttpResponse::Ok().json(json!({"ok": true}))
}
