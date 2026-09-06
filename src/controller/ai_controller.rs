use crate::middleware::admin_middleware::{AdminMiddleware, AuthentikClaims};
use crate::model::ai_models::estimate_cost;
use crate::model::soal::AdminSoal;
use crate::service::ai_service::{AiService, SoalContext};
use crate::AppState;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types — single enrich
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct EnrichRequest {
    /// Fields to enrich. If absent or empty → enrich all.
    #[serde(default)]
    pub fields: Vec<String>,
    /// If true, persist enriched values to soal table immediately.
    #[serde(default)]
    pub save: bool,
}

// Single-question enrich now returns the same BulkJobAcceptedResponse shape
// as bulk-enrich (see below) -- it's a job with total=1, polled the same way.

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types — bulk enrich
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BulkFilterParams {
    #[serde(default)]
    pub missing_solution: bool,
    #[serde(default)]
    pub missing_tag: bool,
    pub modul: Option<String>,
    pub pelajaran: Option<String>,
    #[serde(default = "default_bulk_limit")]
    pub limit: i64,
}

fn default_bulk_limit() -> i64 {
    100
}

fn default_rate_limit() -> u32 {
    20
}

#[derive(Debug, Deserialize)]
pub struct BulkEnrichRequest {
    /// "ids" | "filter"
    pub mode: String,
    /// Used when mode = "ids"
    pub question_ids: Option<Vec<i64>>,
    /// Used when mode = "filter"
    pub filter: Option<BulkFilterParams>,
    /// Fields to enrich. If empty → enrich all.
    #[serde(default)]
    pub fields: Vec<String>,
    /// If true, persist immediately; otherwise preview only (ai_generated_content with accepted=false).
    #[serde(default)]
    pub auto_save: bool,
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
}

#[derive(Debug, Serialize)]
pub struct BulkJobAcceptedResponse {
    pub job_id: String,
    pub status: String,
    pub total_questions: usize,
    pub estimated_minutes: u64,
    pub poll_url: String,
}

#[derive(Debug, Serialize)]
pub struct BulkJobStatusResponse {
    pub job_id: String,
    pub status: String,
    pub total: i32,
    pub processed: i32,
    pub succeeded: i32,
    pub failed: i32,
    pub auto_save: bool,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub results: Vec<BulkResultItem>,
}

#[derive(Debug, Serialize)]
pub struct BulkResultItem {
    pub question_id: i64,
    pub field_name: String,
    pub generated_value: String,
    pub provider: String,
    pub model: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared error helper
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

fn err(code: &str, msg: impl Into<String>) -> ErrorResponse {
    ErrorResponse {
        error: code.to_string(),
        message: msg.into(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Route registration
// ─────────────────────────────────────────────────────────────────────────────

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/ai")
            .wrap(AdminMiddleware::new())
            .route("/questions/{id}/enrich", web::post().to(enrich_question))
            .route("/questions/bulk-enrich", web::post().to(bulk_enrich))
            .route(
                "/questions/bulk-enrich/{job_id}",
                web::get().to(get_bulk_job_status),
            )
            .route(
                "/questions/bulk-enrich/{job_id}",
                web::delete().to(cancel_bulk_job),
            )
            .route("/usage/stats", web::get().to(get_usage_stats)),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Handler: POST /ai/questions/{id}/enrich
// ─────────────────────────────────────────────────────────────────────────────

/// Runs as an async job (same machinery as bulk-enrich) instead of blocking
/// this request on the LLM call. 9router can take 20s+ for a real prompt,
/// which trips Cloudflare's Free-plan proxy timeout on a synchronous route
/// -- returning 202 + a poll_url sidesteps that entirely. Poll via the
/// existing `GET /ai/questions/bulk-enrich/{job_id}`.
async fn enrich_question(
    path: web::Path<i64>,
    body: web::Json<EnrichRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    let question_id = path.into_inner();
    let pool = &*data.context.soal.pool;

    // ── 1. Extract admin identity ─────────────────────────────────────────
    let (admin_id, admin_email) = extract_admin_identity(&http_req);

    // ── 2. Rate limit (10 req/min per admin, fail open if Redis unavailable) ─
    if let Some(ref redis_pool) = data.redis_pool {
        if !check_ai_rate_limit(redis_pool, &admin_id).await {
            return HttpResponse::TooManyRequests().json(err(
                "rate_limit_exceeded",
                "Maximum 10 AI enrich requests per minute. Try again shortly.",
            ));
        }
    }

    // ── 3. Require AI service ─────────────────────────────────────────────
    let ai_service = match &data.ai_service {
        Some(svc) => Arc::clone(svc),
        None => {
            return HttpResponse::ServiceUnavailable().json(err(
                "ai_service_unavailable",
                "AI service is not configured on this server.",
            ))
        }
    };

    // ── 4. Verify the question exists before spawning a job for it ────────
    match fetch_soal(pool, question_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return HttpResponse::NotFound().json(err(
                "not_found",
                format!("Question with id {} not found", question_id),
            ))
        }
        Err(e) => {
            eprintln!(
                "[ai_controller] DB error fetching soal {}: {:?}",
                question_id, e
            );
            return HttpResponse::InternalServerError().json(err(
                "db_error",
                "Failed to fetch question from database.",
            ));
        }
    }

    // ── 5. Resolve fields, create job, spawn worker ────────────────────────
    let fields = resolve_fields(&body.fields);
    let pool_arc = Arc::clone(&data.context.soal.pool);

    match spawn_enrich_job(
        vec![question_id],
        fields,
        body.save,
        60,
        pool_arc,
        ai_service,
        admin_email,
        data.redis_pool.clone(),
    )
    .await
    {
        Ok((job_id, total)) => HttpResponse::Accepted().json(BulkJobAcceptedResponse {
            poll_url: format!("/ai/questions/bulk-enrich/{}", job_id),
            job_id,
            status: "pending".to_string(),
            total_questions: total as usize,
            estimated_minutes: 1,
        }),
        Err(e) => {
            eprintln!(
                "[ai_controller] Failed to create enrich job for {}: {:?}",
                question_id, e
            );
            HttpResponse::InternalServerError().json(err("db_error", "Failed to create enrich job."))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handler: POST /ai/questions/bulk-enrich
// ─────────────────────────────────────────────────────────────────────────────

async fn bulk_enrich(
    body: web::Json<BulkEnrichRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    let pool = &*data.context.soal.pool;
    let (_, admin_email) = extract_admin_identity(&http_req);

    // ── 1. Require AI service ─────────────────────────────────────────────
    let ai_service = match &data.ai_service {
        Some(svc) => Arc::clone(svc),
        None => {
            return HttpResponse::ServiceUnavailable().json(err(
                "ai_service_unavailable",
                "AI service is not configured on this server.",
            ))
        }
    };

    // ── 2. Resolve question IDs ───────────────────────────────────────────
    let question_ids: Vec<i64> = match body.mode.as_str() {
        "ids" => match &body.question_ids {
            Some(ids) if !ids.is_empty() => ids.clone(),
            _ => {
                return HttpResponse::BadRequest()
                    .json(err("bad_request", "mode=ids requires non-empty question_ids"))
            }
        },
        "filter" => match &body.filter {
            Some(f) => match resolve_ids_from_filter(pool, f).await {
                Ok(ids) => ids,
                Err(e) => {
                    eprintln!("[ai_controller] filter query error: {:?}", e);
                    return HttpResponse::InternalServerError().json(err(
                        "db_error",
                        "Failed to resolve question IDs from filter.",
                    ));
                }
            },
            None => {
                return HttpResponse::BadRequest()
                    .json(err("bad_request", "mode=filter requires a filter object"))
            }
        },
        _ => {
            return HttpResponse::BadRequest().json(err(
                "bad_request",
                "mode must be 'ids' or 'filter'",
            ))
        }
    };

    if question_ids.is_empty() {
        return HttpResponse::BadRequest().json(err(
            "bad_request",
            "No questions matched the given criteria.",
        ));
    }

    // ── 3-4. Create job record + spawn background worker ───────────────────
    let fields = resolve_fields(&body.fields);
    let rate_limit = body.rate_limit_per_minute.max(1);
    let pool_arc = Arc::clone(&data.context.soal.pool);

    let (job_id, total) = match spawn_enrich_job(
        question_ids,
        fields,
        body.auto_save,
        rate_limit,
        pool_arc,
        ai_service,
        admin_email,
        data.redis_pool.clone(),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[ai_controller] Failed to insert bulk job: {:?}", e);
            return HttpResponse::InternalServerError()
                .json(err("db_error", "Failed to create bulk job."));
        }
    };

    // ── 5. Return 202 ─────────────────────────────────────────────────────
    let estimated_minutes = ((total as u64) + rate_limit as u64 - 1) / rate_limit as u64;
    HttpResponse::Accepted().json(BulkJobAcceptedResponse {
        poll_url: format!("/ai/questions/bulk-enrich/{}", job_id),
        job_id,
        status: "pending".to_string(),
        total_questions: total as usize,
        estimated_minutes,
    })
}

/// Creates an `ai_bulk_jobs` row and spawns the background worker for it.
/// Shared by the single-question and bulk enrich endpoints -- a single
/// question is just a job with total=1.
async fn spawn_enrich_job(
    question_ids: Vec<i64>,
    fields: Vec<String>,
    auto_save: bool,
    rate_limit_per_minute: u32,
    pool: Arc<MySqlPool>,
    ai_service: Arc<AiService>,
    admin_email: String,
    redis_pool: Option<Arc<crate::service::redis_service::RedisPool>>,
) -> Result<(String, i32), sqlx::Error> {
    let job_id = Uuid::new_v4().to_string();
    let fields_json = serde_json::to_string(&fields).unwrap_or_else(|_| "[]".to_string());
    let total = question_ids.len() as i32;
    let rate_limit = rate_limit_per_minute.max(1);

    sqlx::query(
        r#"
        INSERT INTO dbquizapp.ai_bulk_jobs
            (id, status, total, processed, succeeded, failed, auto_save,
             rate_limit_per_minute, fields, created_by)
        VALUES (?, 'pending', ?, 0, 0, 0, ?, ?, ?, ?)
        "#,
    )
    .bind(&job_id)
    .bind(total)
    .bind(auto_save)
    .bind(rate_limit as i32)
    .bind(&fields_json)
    .bind(&admin_email)
    .execute(&*pool)
    .await?;

    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        run_bulk_job(
            job_id_clone,
            question_ids,
            fields,
            auto_save,
            rate_limit,
            pool,
            ai_service,
            admin_email,
            redis_pool,
        )
        .await;
    });

    Ok((job_id, total))
}

// ─────────────────────────────────────────────────────────────────────────────
// Handler: GET /ai/questions/bulk-enrich/{job_id}
// ─────────────────────────────────────────────────────────────────────────────

async fn get_bulk_job_status(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let job_id = path.into_inner();
    let pool = &*data.context.soal.pool;

    #[derive(sqlx::FromRow)]
    struct JobRow {
        status: String,
        total: i32,
        processed: i32,
        succeeded: i32,
        failed: i32,
        auto_save: bool,
        started_at: Option<chrono::DateTime<chrono::Utc>>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    let job: JobRow = match sqlx::query_as::<_, JobRow>(
        r#"
        SELECT status, total, processed, succeeded, failed, auto_save,
               started_at, completed_at
        FROM dbquizapp.ai_bulk_jobs
        WHERE id = ?
        "#,
    )
    .bind(&job_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(j)) => j,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(err("not_found", format!("Job {} not found", job_id)))
        }
        Err(e) => {
            eprintln!("[ai_controller] DB error fetching job {}: {:?}", job_id, e);
            return HttpResponse::InternalServerError()
                .json(err("db_error", "Failed to fetch job status."));
        }
    };

    // ── Fetch generated results for this job ──────────────────────────────
    #[derive(sqlx::FromRow)]
    struct ContentRow {
        question_id: i64,
        field_name: String,
        generated_value: String,
        provider: String,
        model: String,
    }

    let results: Vec<BulkResultItem> = sqlx::query_as::<_, ContentRow>(
        r#"
        SELECT question_id, field_name, generated_value, provider, model
        FROM dbquizapp.ai_generated_content
        WHERE job_id = ?
        ORDER BY question_id ASC, field_name ASC
        "#,
    )
    .bind(&job_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| BulkResultItem {
        question_id: r.question_id,
        field_name: r.field_name,
        generated_value: r.generated_value,
        provider: r.provider,
        model: r.model,
    })
    .collect();

    HttpResponse::Ok().json(BulkJobStatusResponse {
        job_id,
        status: job.status,
        total: job.total,
        processed: job.processed,
        succeeded: job.succeeded,
        failed: job.failed,
        auto_save: job.auto_save,
        started_at: job.started_at.map(|t| t.to_rfc3339()),
        completed_at: job.completed_at.map(|t| t.to_rfc3339()),
        results,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Handler: DELETE /ai/questions/bulk-enrich/{job_id}
// ─────────────────────────────────────────────────────────────────────────────

async fn cancel_bulk_job(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let job_id = path.into_inner();
    let pool = &*data.context.soal.pool;

    // Only allow cancellation of pending/running jobs
    let result = sqlx::query(
        r#"
        UPDATE dbquizapp.ai_bulk_jobs
        SET status = 'cancelled'
        WHERE id = ? AND status IN ('pending', 'running')
        "#,
    )
    .bind(&job_id)
    .execute(pool)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            HttpResponse::Ok().json(serde_json::json!({ "cancelled": true }))
        }
        Ok(_) => HttpResponse::NotFound().json(err(
            "not_found",
            "Job not found or already completed/cancelled.",
        )),
        Err(e) => {
            eprintln!("[ai_controller] Cancel job error {}: {:?}", job_id, e);
            HttpResponse::InternalServerError().json(err("db_error", "Failed to cancel job."))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handler: GET /ai/usage/stats
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct UsageStatsQuery {
    /// "YYYY-MM", defaults to the current month.
    month: Option<String>,
}

#[derive(Serialize)]
struct UsagePeriodStats {
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    cost_usd: f64,
    question_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    job_count: Option<i64>,
}

#[derive(Serialize)]
struct UsageFieldBreakdown {
    field: String,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    cost_usd: f64,
    count: i64,
}

#[derive(Serialize)]
struct UsageDailyStat {
    date: String,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    cost_usd: f64,
    question_count: i64,
}

#[derive(Serialize)]
struct UsageStatsResponse {
    month: String,
    monthly: UsagePeriodStats,
    today: UsagePeriodStats,
    by_field: Vec<UsageFieldBreakdown>,
    daily: Vec<UsageDailyStat>,
}

#[derive(sqlx::FromRow)]
struct UsageLogRow {
    question_id: i64,
    job_id: Option<String>,
    fields_requested: String,
    prompt_tokens: i32,
    completion_tokens: i32,
    // sqlx can't decode DECIMAL directly into a Rust numeric type (same
    // issue hit elsewhere in this codebase, e.g. xp_service.rs) -- CAST to
    // DOUBLE in the query below instead of pulling in rust_decimal for one field.
    cost_estimate_usd: f64,
    created_at: chrono::NaiveDateTime,
}

/// Backs the "Penggunaan AI" dashboard. `ai_usage_logs` is small (one row
/// per enrich call, not per token), so this fetches the month's rows once
/// and aggregates in Rust rather than juggling several GROUP BY queries --
/// simpler, and per-field attribution needs Rust-side JSON array handling
/// anyway (`fields_requested` is a JSON array like `["solution","tag"]`;
/// a call requesting 2 fields counts its full cost toward each field's
/// breakdown -- there's no way to split a single LLM call's cost cleanly
/// between the fields it filled).
async fn get_usage_stats(
    query: web::Query<UsageStatsQuery>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let pool = &*data.context.soal.pool;
    let month = query
        .month
        .clone()
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m").to_string());

    let rows: Vec<UsageLogRow> = match sqlx::query_as::<_, UsageLogRow>(
        r#"
        SELECT question_id, job_id, fields_requested, prompt_tokens, completion_tokens,
               CAST(cost_estimate_usd AS DOUBLE) AS cost_estimate_usd, created_at
        FROM dbquizapp.ai_usage_logs
        WHERE DATE_FORMAT(created_at, '%Y-%m') = ?
        "#,
    )
    .bind(&month)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("[ai_controller] usage stats query error: {:?}", e);
            return HttpResponse::InternalServerError().json(err("db_error", "Failed to fetch usage stats."));
        }
    };

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let mut monthly = UsagePeriodStats { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0, cost_usd: 0.0, question_count: 0, job_count: Some(0) };
    let mut today_stats = UsagePeriodStats { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0, cost_usd: 0.0, question_count: 0, job_count: None };
    let mut field_totals: std::collections::HashMap<String, UsageFieldBreakdown> = std::collections::HashMap::new();
    let mut daily_totals: std::collections::BTreeMap<String, UsageDailyStat> = std::collections::BTreeMap::new();
    let mut month_question_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
    let mut month_job_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut today_question_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();

    for row in &rows {
        let cost = row.cost_estimate_usd;
        let row_date = row.created_at.format("%Y-%m-%d").to_string();

        monthly.prompt_tokens += row.prompt_tokens as i64;
        monthly.completion_tokens += row.completion_tokens as i64;
        monthly.cost_usd += cost;
        month_question_ids.insert(row.question_id);
        if let Some(jid) = &row.job_id {
            month_job_ids.insert(jid.clone());
        }

        if row_date == today {
            today_stats.prompt_tokens += row.prompt_tokens as i64;
            today_stats.completion_tokens += row.completion_tokens as i64;
            today_stats.cost_usd += cost;
            today_question_ids.insert(row.question_id);
        }

        let day_entry = daily_totals.entry(row_date.clone()).or_insert(UsageDailyStat {
            date: row_date,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            cost_usd: 0.0,
            question_count: 0,
        });
        day_entry.prompt_tokens += row.prompt_tokens as i64;
        day_entry.completion_tokens += row.completion_tokens as i64;
        day_entry.total_tokens += (row.prompt_tokens + row.completion_tokens) as i64;
        day_entry.cost_usd += cost;
        day_entry.question_count += 1;

        let fields: Vec<String> = serde_json::from_str(&row.fields_requested).unwrap_or_default();
        for field in fields {
            let entry = field_totals.entry(field.clone()).or_insert(UsageFieldBreakdown {
                field,
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                cost_usd: 0.0,
                count: 0,
            });
            entry.prompt_tokens += row.prompt_tokens as i64;
            entry.completion_tokens += row.completion_tokens as i64;
            entry.total_tokens += (row.prompt_tokens + row.completion_tokens) as i64;
            entry.cost_usd += cost;
            entry.count += 1;
        }
    }

    monthly.total_tokens = monthly.prompt_tokens + monthly.completion_tokens;
    monthly.question_count = month_question_ids.len() as i64;
    monthly.job_count = Some(month_job_ids.len() as i64);
    today_stats.total_tokens = today_stats.prompt_tokens + today_stats.completion_tokens;
    today_stats.question_count = today_question_ids.len() as i64;

    let mut by_field: Vec<UsageFieldBreakdown> = field_totals.into_values().collect();
    by_field.sort_by(|a, b| b.cost_usd.partial_cmp(&a.cost_usd).unwrap_or(std::cmp::Ordering::Equal));

    HttpResponse::Ok().json(UsageStatsResponse {
        month,
        monthly,
        today: today_stats,
        by_field,
        daily: daily_totals.into_values().collect(),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Background worker
// ─────────────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn run_bulk_job(
    job_id: String,
    question_ids: Vec<i64>,
    fields: Vec<String>,
    auto_save: bool,
    rate_limit_per_minute: u32,
    pool: Arc<MySqlPool>,
    ai_service: Arc<AiService>,
    admin_email: String,
    redis_pool: Option<Arc<crate::service::redis_service::RedisPool>>,
) {
    // Mark running
    let _ = sqlx::query(
        "UPDATE dbquizapp.ai_bulk_jobs SET status='running', started_at=NOW() WHERE id=?",
    )
    .bind(&job_id)
    .execute(&*pool)
    .await;

    let delay_ms = 60_000u64 / rate_limit_per_minute as u64;

    for qid in &question_ids {
        // Check for cancellation before each item
        let status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM dbquizapp.ai_bulk_jobs WHERE id=?",
        )
        .bind(&job_id)
        .fetch_optional(&*pool)
        .await
        .unwrap_or(None);

        if status.as_deref() == Some("cancelled") {
            break;
        }

        let success = enrich_single_for_job(
            *qid,
            &job_id,
            &fields,
            auto_save,
            &pool,
            &ai_service,
            &admin_email,
            redis_pool.as_deref(),
        )
        .await;

        if success {
            let _ = sqlx::query(
                "UPDATE dbquizapp.ai_bulk_jobs SET processed=processed+1, succeeded=succeeded+1 WHERE id=?",
            )
            .bind(&job_id)
            .execute(&*pool)
            .await;
        } else {
            let _ = sqlx::query(
                "UPDATE dbquizapp.ai_bulk_jobs SET processed=processed+1, failed=failed+1 WHERE id=?",
            )
            .bind(&job_id)
            .execute(&*pool)
            .await;
        }

        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    // Check final status (might be cancelled)
    let final_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM dbquizapp.ai_bulk_jobs WHERE id=?")
            .bind(&job_id)
            .fetch_optional(&*pool)
            .await
            .unwrap_or(None);

    if final_status.as_deref() != Some("cancelled") {
        let _ = sqlx::query(
            "UPDATE dbquizapp.ai_bulk_jobs SET status='completed', completed_at=NOW() WHERE id=?",
        )
        .bind(&job_id)
        .execute(&*pool)
        .await;
    }
}

/// Enriches a single question as part of a bulk job.
/// Returns true on success, false on failure.
async fn enrich_single_for_job(
    question_id: i64,
    job_id: &str,
    fields: &[String],
    auto_save: bool,
    pool: &MySqlPool,
    ai_service: &AiService,
    admin_email: &str,
    redis_pool: Option<&crate::service::redis_service::RedisPool>,
) -> bool {
    // Fetch soal
    let soal = match fetch_soal(pool, question_id).await {
        Ok(Some(s)) => s,
        _ => return false,
    };

    let materi_context = crate::controller::materi_library_controller::get_materi_context_for_soal(
        pool,
        soal.subcategory_id.as_deref(),
        soal.topic_id.as_deref(),
    )
    .await;

    // Reading/listening/figural soal carry their actual content in the
    // shared passage, not the stem (stems there run 35-115 chars) -- without
    // it the model would be answering a question it can't see.
    let passage = fetch_passage(pool, soal.passage_id).await;

    let ctx = build_soal_context(&soal, fields.to_vec(), materi_context, passage);

    let enriched = match ai_service.enrich_question(&ctx).await {
        Ok(e) => e,
        Err(e) => {
            let err_msg = e.to_string();
            let _ = insert_ai_usage_log(
                pool,
                question_id,
                Some(job_id),
                "unknown",
                "unknown",
                fields,
                0,
                0,
                0.0,
                false,
                Some(&err_msg),
                Some(admin_email),
            )
            .await;
            return false;
        }
    };

    let cost = estimate_cost(
        &enriched.provider_used,
        enriched.prompt_tokens as i32,
        enriched.completion_tokens as i32,
    );

    let _ = insert_ai_usage_log(
        pool,
        question_id,
        Some(job_id),
        &enriched.provider_used.clone(),
        &enriched.model_used.clone(),
        fields,
        enriched.prompt_tokens as i32,
        enriched.completion_tokens as i32,
        cost,
        true,
        None,
        Some(admin_email),
    )
    .await;

    if auto_save {
        let solution = field_if_requested(fields, "solution", enriched.solution.as_deref());
        let tag = field_if_requested(fields, "tag", enriched.tag.as_deref());
        let modul = field_if_requested(fields, "modul", enriched.modul.as_deref());
        let pelajaran = field_if_requested(fields, "pelajaran", enriched.pelajaran.as_deref());
        let correct_answer =
            field_if_requested(fields, "correct_answer", enriched.correct_answer.as_deref());

        if save_enriched_fields(pool, question_id, solution, tag, modul, pelajaran, correct_answer)
            .await
            .is_err()
        {
            return false;
        }
        // Content changed -- drop any cached /paket-soal-response for
        // packages containing this question, or students keep seeing the
        // pre-enrich version until the 1h cache TTL expires.
        if let Some(redis_pool) = redis_pool {
            crate::service::redis_service::RedisService::invalidate_paket_soal_response_for_questions(
                pool,
                redis_pool,
                &[question_id as i32],
            ).await;
        }
    }

    // Record generated content (accepted = auto_save)
    let field_map: [(&str, Option<&str>, Option<&str>); 5] = [
        (
            "solution",
            soal.solution.as_deref(),
            enriched.solution.as_deref(),
        ),
        ("tag", soal.tag.as_deref(), enriched.tag.as_deref()),
        ("modul", soal.modul.as_deref(), enriched.modul.as_deref()),
        (
            "pelajaran",
            soal.pelajaran.as_deref(),
            enriched.pelajaran.as_deref(),
        ),
        (
            "correct_answer",
            soal.correct_answer.as_deref(),
            enriched.correct_answer.as_deref(),
        ),
    ];
    for (field_name, original, generated) in &field_map {
        if fields.contains(&field_name.to_string()) {
            if let Some(gen) = generated {
                let _ = insert_ai_generated_content(
                    pool,
                    question_id,
                    Some(job_id),
                    field_name,
                    *original,
                    gen,
                    &enriched.provider_used,
                    &enriched.model_used,
                    auto_save,
                    if auto_save { Some(admin_email) } else { None },
                )
                .await;
            }
        }
    }

    true
}

// ─────────────────────────────────────────────────────────────────────────────
// Redis rate limiting
// ─────────────────────────────────────────────────────────────────────────────

/// Returns true if the request is within rate limit, false if exceeded.
/// Fails open (returns true) if Redis is unavailable.
async fn check_ai_rate_limit(
    redis_pool: &crate::service::redis_service::RedisPool,
    admin_id: &str,
) -> bool {
    let key = format!("ai_rate_limit:{}", admin_id);
    let mut con = redis_pool.rate_limits().as_ref().clone();

    let count: i64 = match con.incr::<_, i64, i64>(&key, 1i64).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ai_controller] Redis rate limit error: {:?}", e);
            return true; // fail open
        }
    };

    // Set TTL on first request (atomic enough for rate limiting)
    if count == 1 {
        let _: Result<bool, _> = con.expire(&key, 60i64).await;
    }

    count <= 10
}

// ─────────────────────────────────────────────────────────────────────────────
// DB helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Fetch a soal row by id.
async fn fetch_soal(pool: &MySqlPool, question_id: i64) -> Result<Option<AdminSoal>, sqlx::Error> {
    sqlx::query_as::<_, AdminSoal>(
        r#"
        SELECT s.*,
               COALESCE(s.created_at, NOW()) AS created_at,
               COALESCE(s.updated_at, NOW()) AS updated_at,
               0 AS usage_count
        FROM dbquizapp.soal s
        WHERE s.id = ?
        "#,
    )
    .bind(question_id)
    .fetch_optional(pool)
    .await
}

/// Fetches a soal's shared passage text, when it has one.
async fn fetch_passage(pool: &MySqlPool, passage_id: Option<i32>) -> Option<String> {
    let pid = passage_id?;
    sqlx::query_scalar::<_, String>("SELECT content FROM dbquizapp.passages WHERE id = ?")
        .bind(pid)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

/// Insert a row into ai_usage_logs.
async fn insert_ai_usage_log(
    pool: &MySqlPool,
    question_id: i64,
    job_id: Option<&str>,
    provider: &str,
    model: &str,
    fields: &[String],
    prompt_tokens: i32,
    completion_tokens: i32,
    cost_estimate_usd: f64,
    success: bool,
    error_message: Option<&str>,
    created_by_admin: Option<&str>,
) -> Result<(), sqlx::Error> {
    let fields_json = serde_json::to_string(fields).unwrap_or_else(|_| "[]".to_string());

    sqlx::query(
        r#"
        INSERT INTO dbquizapp.ai_usage_logs
            (question_id, job_id, provider, model, fields_requested,
             prompt_tokens, completion_tokens, cost_estimate_usd,
             success, error_message, created_by_admin)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(question_id)
    .bind(job_id)
    .bind(provider)
    .bind(model)
    .bind(fields_json)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(cost_estimate_usd)
    .bind(success)
    .bind(error_message)
    .bind(created_by_admin)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Insert a row into ai_generated_content.
async fn insert_ai_generated_content(
    pool: &MySqlPool,
    question_id: i64,
    job_id: Option<&str>,
    field_name: &str,
    original_value: Option<&str>,
    generated_value: &str,
    provider: &str,
    model: &str,
    accepted: bool,
    accepted_by: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO dbquizapp.ai_generated_content
            (question_id, job_id, field_name, original_value, generated_value,
             provider, model, accepted, accepted_at, accepted_by)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, IF(?, NOW(), NULL), ?)
        "#,
    )
    .bind(question_id)
    .bind(job_id)
    .bind(field_name)
    .bind(original_value)
    .bind(generated_value)
    .bind(provider)
    .bind(model)
    .bind(accepted)
    .bind(accepted)
    .bind(accepted_by)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Dynamically update only the requested fields on the soal row.
async fn save_enriched_fields(
    pool: &MySqlPool,
    question_id: i64,
    solution: Option<&str>,
    tag: Option<&str>,
    modul: Option<&str>,
    pelajaran: Option<&str>,
    correct_answer: Option<&str>,
) -> Result<(), sqlx::Error> {
    // Build dynamic SET clause — always include updated_at
    let mut set_parts: Vec<&str> = vec!["updated_at = NOW()"];
    if solution.is_some() {
        set_parts.push("solution = ?");
    }
    if tag.is_some() {
        set_parts.push("tag = ?");
    }
    if modul.is_some() {
        set_parts.push("modul = ?");
    }
    if pelajaran.is_some() {
        set_parts.push("pelajaran = ?");
    }
    if correct_answer.is_some() {
        set_parts.push("correct_answer = ?");
    }

    let sql = format!(
        "UPDATE dbquizapp.soal SET {} WHERE id = ?",
        set_parts.join(", ")
    );

    let mut q = sqlx::query(&sql);
    if let Some(s) = solution {
        q = q.bind(s);
    }
    if let Some(t) = tag {
        q = q.bind(t);
    }
    if let Some(m) = modul {
        q = q.bind(m);
    }
    if let Some(p) = pelajaran {
        q = q.bind(p);
    }
    if let Some(c) = correct_answer {
        q = q.bind(c);
    }
    q = q.bind(question_id);

    q.execute(pool).await.map(|_| ())
}

/// Resolve question IDs from a filter (mode=filter).
async fn resolve_ids_from_filter(
    pool: &MySqlPool,
    filter: &BulkFilterParams,
) -> Result<Vec<i64>, sqlx::Error> {
    let mut conditions: Vec<String> = vec!["1=1".to_string()];

    if filter.missing_solution {
        conditions.push("(s.solution IS NULL OR s.solution = '')".to_string());
    }
    if filter.missing_tag {
        conditions.push("(s.tag IS NULL OR s.tag = '')".to_string());
    }
    if filter.modul.is_some() {
        conditions.push("s.modul = ?".to_string());
    }
    if filter.pelajaran.is_some() {
        conditions.push("s.pelajaran = ?".to_string());
    }

    let sql = format!(
        "SELECT s.id FROM dbquizapp.soal s WHERE {} ORDER BY s.id ASC LIMIT ?",
        conditions.join(" AND ")
    );

    let mut q = sqlx::query_scalar::<_, i64>(&sql);
    if let Some(ref m) = filter.modul {
        q = q.bind(m);
    }
    if let Some(ref p) = filter.pelajaran {
        q = q.bind(p);
    }
    q = q.bind(filter.limit);

    q.fetch_all(pool).await
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin identity extraction
// ─────────────────────────────────────────────────────────────────────────────

/// Returns (admin_id, admin_email) from the request.
/// Prefers AuthentikClaims injected by AdminMiddleware; falls back to user_id header.
fn extract_admin_identity(http_req: &HttpRequest) -> (String, String) {
    if let Some(claims) = http_req.extensions().get::<AuthentikClaims>() {
        let email = claims.email.clone().unwrap_or_else(|| claims.sub.clone());
        return (claims.sub.clone(), email);
    }
    let user_id = http_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    (user_id.clone(), user_id)
}

// ─────────────────────────────────────────────────────────────────────────────
// Small helpers
// ─────────────────────────────────────────────────────────────────────────────

fn resolve_fields(requested: &[String]) -> Vec<String> {
    if requested.is_empty() {
        vec![
            "solution".into(),
            "tag".into(),
            "modul".into(),
            "pelajaran".into(),
        ]
    } else {
        requested.to_vec()
    }
}

fn build_soal_context(
    soal: &AdminSoal,
    fields: Vec<String>,
    materi_context: Option<String>,
    passage: Option<String>,
) -> SoalContext {
    SoalContext {
        id: soal.id as i64,
        soal: soal.soal.clone(),
        opt1: soal.opt1.clone(),
        opt2: soal.opt2.clone(),
        opt3: soal.opt3.clone(),
        opt4: soal.opt4.clone(),
        opt5: soal.opt5.clone(),
        correct_answer: soal.correct_answer.clone(),
        fields_to_enrich: fields,
        passage,
        materi_context,
    }
}

/// Returns `value` if `field_name` is in `fields`, otherwise `None`.
fn field_if_requested<'a>(
    fields: &[String],
    field_name: &str,
    value: Option<&'a str>,
) -> Option<&'a str> {
    if fields.contains(&field_name.to_string()) {
        value
    } else {
        None
    }
}
