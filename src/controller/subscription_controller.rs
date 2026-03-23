use crate::controller::log_request;
use crate::model::midtrans::{
    CheckoutRequest, CheckoutResponse, HistoryQuery, MidtransNotification,
    MidtransTransaction, SubscriptionStatusResponse, TransactionHistoryItem,
    TransactionHistoryResponse,
};
use crate::model::user_subscription::CreateUserSubscriptionRequest;
use crate::service::midtrans_service::MidtransService;
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::{Duration, Utc};
use redis::AsyncCommands;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

fn extract_user_id(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn make_midtrans_service(data: &web::Data<AppState<'_>>) -> MidtransService {
    MidtransService::new(
        data.config.get_midtrans_server_key().to_string(),
        data.config.get_midtrans_client_key().to_string(),
        data.config.get_midtrans_is_production(),
    )
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/subscription")
            .route("/status", web::get().to(get_status))
            .route("/history", web::get().to(get_history))
            .route("/checkout", web::post().to(checkout))
            .route("/webhook/midtrans", web::post().to(midtrans_webhook))
            .route("/invoice/{transaction_id}", web::get().to(get_invoice))
            .route("/cancel", web::post().to(cancel_subscription)),
    );
}

// ─── GET /subscription/status ────────────────────────────────────────────────

async fn get_status(
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("GET /subscription/status", &data.connections);

    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse { error: "Login diperlukan".into() }),
    };

    // Auto-expire subscriptions before checking (status per-request)
    let _ = data.context.user_subscriptions.update_expired_subscriptions().await;

    match data.context.user_subscriptions.get_active_subscription(&user_id).await {
        Ok(Some(sub)) => {
            let days_remaining = if sub.is_lifetime {
                None
            } else {
                sub.end_date.map(|end| {
                    let diff = (end - Utc::now()).num_days();
                    if diff < 0 { 0 } else { diff }
                })
            };

            // Lookup price paid from last successful midtrans transaction
            let price_paid: Option<f64> = sqlx::query_scalar(
                r#"SELECT amount FROM dbquizapp.midtrans_transactions
                   WHERE user_id = ? AND plan_id = ? AND status = 'success'
                   ORDER BY paid_at DESC LIMIT 1"#,
            )
            .bind(&user_id)
            .bind(sub.plan_id)
            .fetch_optional(&*data.context.soal.pool)
            .await
            .ok()
            .flatten()
            .map(|a: i64| a as f64);

            HttpResponse::Ok().json(SubscriptionStatusResponse {
                plan: Some(sub.plan_name.to_lowercase().replace(" plan", "").replace(" ", "_")),
                plan_name: Some(sub.plan_name),
                status: "active".into(),
                started_at: Some(sub.start_date),
                expires_at: sub.end_date,
                days_remaining,
                is_lifetime: sub.is_lifetime,
                price_paid,
            })
        }
        Ok(None) => HttpResponse::Ok().json(SubscriptionStatusResponse {
            plan: None,
            plan_name: None,
            status: "none".into(),
            started_at: None,
            expires_at: None,
            days_remaining: None,
            is_lifetime: false,
            price_paid: None,
        }),
        Err(e) => {
            eprintln!("Error get_status: {e}");
            HttpResponse::InternalServerError()
                .json(ErrorResponse { error: "Gagal mengambil status langganan".into() })
        }
    }
}

// ─── GET /subscription/history ───────────────────────────────────────────────

async fn get_history(
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
    query: web::Query<HistoryQuery>,
) -> impl Responder {
    log_request("GET /subscription/history", &data.connections);

    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse { error: "Login diperlukan".into() }),
    };

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(10).min(100);
    let offset = (page - 1) * limit;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dbquizapp.midtrans_transactions WHERE user_id = ?",
    )
    .bind(&user_id)
    .fetch_one(&*data.context.soal.pool)
    .await
    .unwrap_or(0);

    #[derive(sqlx::FromRow)]
    struct HistoryRow {
        id: String,
        plan_name: String,
        amount: i64,
        status: String,
        payment_method: Option<String>,
        paid_at: Option<chrono::DateTime<chrono::Utc>>,
        invoice_number: Option<String>,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let rows = sqlx::query_as::<_, HistoryRow>(
        r#"
        SELECT
            mt.id,
            pp.name AS plan_name,
            mt.amount,
            mt.status,
            mt.payment_method,
            mt.paid_at,
            mt.invoice_number,
            mt.created_at
        FROM dbquizapp.midtrans_transactions mt
        JOIN dbquizapp.premium_plans pp ON pp.id = mt.plan_id
        WHERE mt.user_id = ?
        ORDER BY mt.created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(&user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&*data.context.soal.pool)
    .await;

    match rows {
        Ok(rows) => {
            let data_vec: Vec<TransactionHistoryItem> = rows
                .into_iter()
                .map(|r| {
                    let plan_slug = r.plan_name.to_lowercase().replace(" plan", "").replace(" ", "_");
                    TransactionHistoryItem {
                        invoice_url: format!("/subscription/invoice/{}", r.id),
                        id: r.id,
                        plan: plan_slug,
                        plan_name: r.plan_name,
                        amount: r.amount,
                        status: r.status,
                        payment_method: r.payment_method,
                        paid_at: r.paid_at,
                        invoice_number: r.invoice_number,
                        created_at: r.created_at,
                    }
                })
                .collect();

            HttpResponse::Ok().json(TransactionHistoryResponse {
                data: data_vec,
                total,
                page,
                limit,
            })
        }
        Err(e) => {
            eprintln!("Error get_history: {e}");
            HttpResponse::InternalServerError()
                .json(ErrorResponse { error: "Gagal mengambil riwayat transaksi".into() })
        }
    }
}

// ─── POST /subscription/checkout ─────────────────────────────────────────────

async fn checkout(
    body: web::Json<CheckoutRequest>,
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("POST /subscription/checkout", &data.connections);

    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse { error: "Login diperlukan".into() }),
    };

    // Rate limit: 3 checkout per menit per user
    if let Some(redis) = &data.redis_pool {
        let mut con = (*redis.rate_limits()).clone();
        let key = format!("rate:checkout:{}", user_id);
        let count: i64 = con.incr(&key, 1_i64).await.unwrap_or(0);
        if count == 1 {
            let _: Result<(), _> = con.expire(&key, 60_i64).await;
        }
        if count > 3 {
            return HttpResponse::TooManyRequests().json(ErrorResponse {
                error: "Terlalu banyak permintaan checkout. Coba lagi dalam 1 menit.".into(),
            });
        }
    }

    let plan_slug = body.plan.trim().to_lowercase();
    if plan_slug.is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse { error: "Plan tidak boleh kosong".into() });
    }

    // Lookup plan dari premium_plans (MySQL LIKE is case-insensitive)
    #[derive(sqlx::FromRow)]
    struct PlanRow {
        id: i32,
        name: String,
        price: f64,
        is_lifetime: bool,
    }

    let plan = sqlx::query_as::<_, PlanRow>(
        "SELECT id, name, price, is_lifetime FROM dbquizapp.premium_plans WHERE LOWER(name) LIKE LOWER(CONCAT(?, '%')) LIMIT 1",
    )
    .bind(&plan_slug)
    .fetch_optional(&*data.context.soal.pool)
    .await;

    let plan = match plan {
        Ok(Some(p)) => p,
        Ok(None) => return HttpResponse::NotFound().json(ErrorResponse { error: format!("Plan '{}' tidak ditemukan", plan_slug) }),
        Err(e) => {
            eprintln!("Error lookup plan: {e}");
            return HttpResponse::InternalServerError().json(ErrorResponse { error: "Gagal mencari data plan".into() });
        }
    };

    // Ambil user info
    let user = match data.context.users.get_user_by_id(&user_id).await {
        Ok(Some(u)) => u,
        _ => return HttpResponse::InternalServerError().json(ErrorResponse { error: "Gagal mengambil data user".into() }),
    };

    let amount = plan.price as i64;

    // Generate order_id: QUIZKU-YYYYMMDD-XXXXXXXX
    let date_str = Utc::now().format("%Y%m%d").to_string();
    let rand_suffix = &Uuid::new_v4().to_string().replace("-", "").to_uppercase()[..8];
    let order_id = format!("QUIZKU-{}-{}", date_str, rand_suffix);

    // Call Midtrans Snap
    let midtrans = make_midtrans_service(&data);
    let snap = match midtrans
        .create_snap_transaction(&order_id, amount, &user.email, &user.display_name, &plan_slug, &plan.name)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Midtrans Snap error: {e}");
            return HttpResponse::BadGateway().json(ErrorResponse { error: "Gagal membuat transaksi Midtrans".into() });
        }
    };

    let tx_id = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::hours(24);

    // Simpan ke midtrans_transactions
    let insert = sqlx::query(
        r#"INSERT INTO dbquizapp.midtrans_transactions
           (id, user_id, plan_id, order_id, amount, status, payment_url, snap_token, expires_at)
           VALUES (?, ?, ?, ?, ?, 'pending', ?, ?, ?)"#,
    )
    .bind(&tx_id)
    .bind(&user_id)
    .bind(plan.id)
    .bind(&order_id)
    .bind(amount)
    .bind(&snap.redirect_url)
    .bind(&snap.token)
    .bind(expires_at)
    .execute(&*data.context.soal.pool)
    .await;

    match insert {
        Ok(_) => HttpResponse::Ok().json(CheckoutResponse {
            order_id,
            payment_url: snap.redirect_url,
            snap_token: snap.token,
            amount,
            expires_at,
        }),
        Err(e) => {
            eprintln!("Error save midtrans_transaction: {e}");
            HttpResponse::InternalServerError().json(ErrorResponse { error: "Gagal menyimpan transaksi".into() })
        }
    }
}

// ─── POST /subscription/webhook/midtrans ─────────────────────────────────────

async fn midtrans_webhook(
    body: web::Json<MidtransNotification>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    log_request("POST /subscription/webhook/midtrans", &data.connections);

    // Verifikasi signature
    let midtrans = make_midtrans_service(&data);
    if !midtrans.verify_signature(
        &body.order_id,
        &body.status_code,
        &body.gross_amount,
        &body.signature_key,
    ) {
        eprintln!("Midtrans webhook: invalid signature for order {}", body.order_id);
        return HttpResponse::Unauthorized().json(ErrorResponse { error: "Invalid signature".into() });
    }

    // Map transaction_status → internal status
    let new_status = match body.transaction_status.as_str() {
        "capture" | "settlement" => "success",
        "deny" | "cancel" | "failure" => "failed",
        "expire" => "expired",
        _ => "pending",
    };

    // Cari transaksi berdasarkan order_id
    let tx: Option<MidtransTransaction> = sqlx::query_as(
        "SELECT * FROM dbquizapp.midtrans_transactions WHERE order_id = ?",
    )
    .bind(&body.order_id)
    .fetch_optional(&*data.context.soal.pool)
    .await
    .unwrap_or(None);

    let tx = match tx {
        Some(t) => t,
        None => {
            eprintln!("Midtrans webhook: order {} not found", body.order_id);
            return HttpResponse::NotFound().json(ErrorResponse { error: format!("Order {} tidak ditemukan", body.order_id) });
        }
    };

    // Update status + payment_method + midtrans_transaction_id
    let paid_at = if new_status == "success" { Some(Utc::now()) } else { None };

    let _ = sqlx::query(
        r#"UPDATE dbquizapp.midtrans_transactions
           SET status = ?, payment_method = ?, midtrans_transaction_id = ?,
               midtrans_status_code = ?, paid_at = ?, updated_at = NOW()
           WHERE order_id = ?"#,
    )
    .bind(new_status)
    .bind(body.payment_type.as_deref())
    .bind(body.transaction_id.as_deref())
    .bind(&body.status_code)
    .bind(paid_at)
    .bind(&body.order_id)
    .execute(&*data.context.soal.pool)
    .await;

    // Jika pembayaran sukses, aktivasi/perpanjang subscription
    if new_status == "success" {
        // Deaktivasi subscription lama jika ada
        let _ = sqlx::query(
            "UPDATE dbquizapp.user_subscriptions SET status = 'cancelled', updated_at = NOW() WHERE user_id = ? AND status = 'active'",
        )
        .bind(&tx.user_id)
        .execute(&*data.context.soal.pool)
        .await;

        // Buat subscription baru
        let sub_req = CreateUserSubscriptionRequest {
            user_id: tx.user_id.clone(),
            plan_id: tx.plan_id,
        };

        match data.context.user_subscriptions.create_subscription(&sub_req).await {
            Ok(sub_id) => {
                // Catat subscription_id + generate invoice_number di transaksi
                let invoice_number = format!("INV-{}-{}", Utc::now().format("%Y"), sub_id);
                let _ = sqlx::query(
                    "UPDATE dbquizapp.midtrans_transactions SET subscription_id = ?, invoice_number = ? WHERE order_id = ?",
                )
                .bind(sub_id)
                .bind(&invoice_number)
                .bind(&body.order_id)
                .execute(&*data.context.soal.pool)
                .await;

                eprintln!("Midtrans webhook: subscription {} activated for user {}", sub_id, tx.user_id);
            }
            Err(e) => {
                eprintln!("Midtrans webhook: failed to create subscription: {e}");
                return HttpResponse::InternalServerError()
                    .json(ErrorResponse { error: "Gagal mengaktifkan langganan".into() });
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "message": "Webhook processed",
        "order_id": body.order_id,
        "status": new_status,
    }))
}

// ─── GET /subscription/invoice/:transaction_id ────────────────────────────────

async fn get_invoice(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("GET /subscription/invoice/{id}", &data.connections);

    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse { error: "Login diperlukan".into() }),
    };

    let tx_id = path.into_inner();

    #[derive(sqlx::FromRow)]
    struct InvoiceRow {
        id: String,
        user_id: String,
        plan_name: String,
        amount: i64,
        status: String,
        payment_method: Option<String>,
        order_id: String,
        invoice_number: Option<String>,
        paid_at: Option<chrono::DateTime<chrono::Utc>>,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let row = sqlx::query_as::<_, InvoiceRow>(
        r#"
        SELECT mt.id, mt.user_id, pp.name AS plan_name, mt.amount,
               mt.status, mt.payment_method, mt.order_id,
               mt.invoice_number, mt.paid_at, mt.created_at
        FROM dbquizapp.midtrans_transactions mt
        JOIN dbquizapp.premium_plans pp ON pp.id = mt.plan_id
        WHERE mt.id = ?
        "#,
    )
    .bind(&tx_id)
    .fetch_optional(&*data.context.soal.pool)
    .await;

    match row {
        Ok(Some(r)) => {
            if r.user_id != user_id {
                return HttpResponse::Forbidden().json(ErrorResponse { error: "Akses ditolak".into() });
            }
            HttpResponse::Ok().json(serde_json::json!({
                "id": r.id,
                "order_id": r.order_id,
                "invoice_number": r.invoice_number,
                "plan_name": r.plan_name,
                "amount": r.amount,
                "status": r.status,
                "payment_method": r.payment_method,
                "paid_at": r.paid_at,
                "issued_at": r.created_at,
            }))
        }
        Ok(None) => HttpResponse::NotFound().json(ErrorResponse { error: "Invoice tidak ditemukan".into() }),
        Err(e) => {
            eprintln!("Error get_invoice: {e}");
            HttpResponse::InternalServerError().json(ErrorResponse { error: "Gagal mengambil invoice".into() })
        }
    }
}

// ─── POST /subscription/cancel ────────────────────────────────────────────────

async fn cancel_subscription(
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("POST /subscription/cancel", &data.connections);

    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse { error: "Login diperlukan".into() }),
    };

    // Cari active subscription
    let active = match data.context.user_subscriptions.get_active_subscription(&user_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().json(ErrorResponse { error: "Tidak ada langganan aktif".into() }),
        Err(e) => {
            eprintln!("Error cancel lookup: {e}");
            return HttpResponse::InternalServerError().json(ErrorResponse { error: "Gagal mencari langganan".into() });
        }
    };

    match data.context.user_subscriptions.cancel_subscription(active.id).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Langganan dibatalkan. Akses premium tetap aktif sampai masa habis.",
            "expires_at": active.end_date,
        })),
        Ok(false) => HttpResponse::InternalServerError().json(ErrorResponse { error: "Gagal membatalkan langganan".into() }),
        Err(e) => {
            eprintln!("Error cancel_subscription: {e}");
            HttpResponse::InternalServerError().json(ErrorResponse { error: "Gagal membatalkan langganan".into() })
        }
    }
}
