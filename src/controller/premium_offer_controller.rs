//! Penawaran premium untuk user dan pengelolaan kode promo oleh admin.
//!
//!   GET  /premium/offer                    trial, langganan, plan aktif, promo yang berlaku + batas waktunya
//!   GET  /admin/promo-codes                daftar kode promo (admin)
//!   POST /admin/promo-codes                buat kode promo (admin)
//!   PUT  /admin/promo-codes/{id}           ubah kode promo, termasuk menyalakan/mematikan (admin)

use actix_web::{web, HttpResponse, Responder};
use serde_json::json;

use crate::dao::{premium_access_dao, promo_dao};
use crate::middleware::auth_middleware::AuthenticatedUser;
use crate::model::premium_plan::PremiumPlanResponse;
use crate::AppState;

pub async fn get_offer(state: web::Data<AppState<'_>>, user: AuthenticatedUser) -> impl Responder {
    let pool = &*state.context.soal.pool;
    let gagal = |e: sqlx::Error| {
        eprintln!("[premium/offer] {e:?}");
        HttpResponse::InternalServerError().json(json!({"error": "gagal_memuat_penawaran"}))
    };
    let trial = match premium_access_dao::pastikan_trial(pool, &user.user_id).await {
        Ok(t) => t,
        Err(e) => return gagal(e),
    };
    let langganan = match premium_access_dao::langganan_aktif(pool, &user.user_id).await {
        Ok(l) => l,
        Err(e) => return gagal(e),
    };
    let plans: Vec<PremiumPlanResponse> = match state.context.premium_plans.get_all_premium_plans().await {
        Ok(p) => p.into_iter().map(PremiumPlanResponse::from).collect(),
        Err(e) => return gagal(e),
    };
    // Pelanggan aktif tidak perlu melihat banner diskon.
    let promos = if langganan.is_some() {
        Vec::new()
    } else {
        match promo_dao::untuk_user(pool, &user.user_id, trial.as_ref().map(|t| t.berakhir)).await {
            Ok(v) => v,
            Err(e) => return gagal(e),
        }
    };
    let promos: Vec<_> = promos
        .into_iter()
        .map(|(p, batas)| {
            let harga: Vec<_> = plans
                .iter()
                .filter(|pl| promo_dao::cocok_periode(&p.berlaku_untuk, pl.period.as_deref()))
                .map(|pl| {
                    let h = pl.price.round() as i64;
                    json!({"plan_id": pl.id, "harga": h, "harga_akhir": h - promo_dao::hitung_potongan(h, p.persen)})
                })
                .collect();
            json!({
                "kode": p.kode,
                "nama": p.nama,
                "jenis": p.jenis,
                "persen": p.persen,
                "berlaku_untuk": p.berlaku_untuk,
                // null = tanpa batas waktu
                "berakhir": (batas != i64::MAX).then_some(batas),
                "teks_banner": p.teks_banner,
                "harga": harga,
            })
        })
        .collect();
    let server_time = promo_dao::sekarang(pool).await.unwrap_or_else(|_| chrono::Utc::now().timestamp());
    HttpResponse::Ok().json(json!({
        "server_time": server_time,
        "akses_premium": langganan.is_some() || trial.as_ref().map_or(false, |t| t.aktif),
        "trial": trial,
        "langganan": langganan,
        "plans": plans,
        "promos": promos,
    }))
}

pub fn init_admin(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/promo-codes")
            .wrap(crate::middleware::admin_middleware::AdminMiddleware::new())
            .route("", web::get().to(daftar))
            .route("", web::post().to(buat))
            .route("/{id}", web::put().to(ubah)),
    );
}

async fn daftar(state: web::Data<AppState<'_>>) -> impl Responder {
    match promo_dao::daftar(&state.context.soal.pool).await {
        Ok(v) => HttpResponse::Ok().json(json!({"data": v})),
        Err(e) => {
            eprintln!("[admin/promo-codes] {e:?}");
            HttpResponse::InternalServerError().json(json!({"error": "gagal_memuat"}))
        }
    }
}

async fn simpan(state: &AppState<'_>, id: Option<i32>, input: &promo_dao::PromoInput) -> HttpResponse {
    if let Err(pesan) = promo_dao::periksa_input(input) {
        return HttpResponse::BadRequest().json(json!({"error": "isian_tidak_valid", "message": pesan}));
    }
    match promo_dao::simpan(&state.context.soal.pool, id, input).await {
        Ok(id) => HttpResponse::Ok().json(json!({"id": id})),
        Err(sqlx::Error::Database(e)) if e.message().contains("Duplicate") => HttpResponse::Conflict()
            .json(json!({"error": "kode_sudah_ada", "message": "Kode itu sudah dipakai promo lain."})),
        Err(e) => {
            eprintln!("[admin/promo-codes] simpan: {e:?}");
            HttpResponse::InternalServerError().json(json!({"error": "gagal_menyimpan"}))
        }
    }
}

async fn buat(state: web::Data<AppState<'_>>, input: web::Json<promo_dao::PromoInput>) -> impl Responder {
    simpan(&state, None, &input).await
}

async fn ubah(state: web::Data<AppState<'_>>, path: web::Path<i32>, input: web::Json<promo_dao::PromoInput>) -> impl Responder {
    simpan(&state, Some(path.into_inner()), &input).await
}
