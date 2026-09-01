//! AI-driven taxonomy classification for a soal: primary track/category/
//! subcategory/topic (2-stage: pick subcategory from the full list, then
//! pick topic from within it) plus 0-5 cross-link "topik tambahan" from
//! other tracks (question_topics M2M), scoped to "clean" subcategories only
//! (see ai_service::filter_clean_topics).
//!
//! Runs as an async job -- 2-3 sequential LLM calls take even longer than
//! a single enrich call, same Cloudflare-timeout reasoning as the other
//! AI endpoints in this codebase.

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::dao::taxonomy_dao::TaxonomyDao;
use crate::middleware::admin_middleware::{AdminMiddleware, AuthentikClaims};
use crate::model::taxonomy::TaxonomyTree;
use crate::service::ai_service::{filter_clean_topics, AiService};
use crate::AppState;
use actix_web::HttpMessage;

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

fn extract_admin_email(req: &HttpRequest) -> String {
    if let Some(claims) = req.extensions().get::<AuthentikClaims>() {
        return claims.email.clone().unwrap_or_else(|| claims.sub.clone());
    }
    req.headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}

#[derive(sqlx::FromRow)]
struct SoalLite {
    id: i64,
    soal: String,
    opt1: Option<String>,
    opt2: Option<String>,
    opt3: Option<String>,
    opt4: Option<String>,
    opt5: Option<String>,
}

async fn fetch_soal_lite(pool: &MySqlPool, id: i64) -> Result<Option<SoalLite>, sqlx::Error> {
    sqlx::query_as::<_, SoalLite>(
        "SELECT id, soal, opt1, opt2, opt3, opt4, opt5 FROM dbquizapp.soal WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

fn options_text(s: &SoalLite) -> String {
    let mut out = String::new();
    for (label, opt) in [
        ("opt1", &s.opt1),
        ("opt2", &s.opt2),
        ("opt3", &s.opt3),
        ("opt4", &s.opt4),
        ("opt5", &s.opt5),
    ] {
        if let Some(v) = opt {
            if !v.is_empty() {
                out.push_str(&format!("{}: {}\n", label, v));
            }
        }
    }
    out
}

#[derive(Serialize, Deserialize)]
struct ClassifyResult {
    track_id: Option<String>,
    track_name: Option<String>,
    category_id: Option<String>,
    category_name: Option<String>,
    subcategory_id: Option<String>,
    subcategory_name: Option<String>,
    topic_id: Option<String>,
    topic_name: Option<String>,
    additional_topic_ids: Vec<String>,
    additional_topic_names: Vec<String>,
}

#[derive(Serialize)]
struct JobAcceptedResponse {
    job_id: String,
    status: String,
    poll_url: String,
}

#[derive(Serialize)]
struct JobStatusResponse {
    job_id: String,
    status: String,
    question_id: i64,
    result: Option<ClassifyResult>,
    error_message: Option<String>,
    completed_at: Option<String>,
}

/// POST /admin/classify/{question_id}
#[post("/{question_id}")]
async fn start_classify(
    path: web::Path<i64>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    let question_id = path.into_inner();
    let ai_service = match &data.ai_service {
        Some(svc) => Arc::clone(svc),
        None => {
            return HttpResponse::ServiceUnavailable().json(err(
                "ai_service_unavailable",
                "AI service is not configured on this server.",
            ))
        }
    };
    let pool = Arc::clone(&data.context.soal.pool);
    let admin_email = extract_admin_email(&http_req);

    match fetch_soal_lite(&pool, question_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(err("not_found", format!("Question {} not found", question_id)))
        }
        Err(e) => {
            eprintln!("[taxonomy_classify_controller] DB error fetching soal {}: {:?}", question_id, e);
            return HttpResponse::InternalServerError()
                .json(err("db_error", "Failed to fetch question."));
        }
    }

    let job_id = Uuid::new_v4().to_string();
    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO dbquizapp.taxonomy_classify_jobs (id, status, question_id, created_by)
        VALUES (?, 'pending', ?, ?)
        "#,
    )
    .bind(&job_id)
    .bind(question_id)
    .bind(&admin_email)
    .execute(&*pool)
    .await
    {
        eprintln!("[taxonomy_classify_controller] Failed to insert job: {:?}", e);
        return HttpResponse::InternalServerError().json(err("db_error", "Failed to create job."));
    }

    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        run_classify_job(job_id_clone, question_id, pool, ai_service).await;
    });

    HttpResponse::Accepted().json(JobAcceptedResponse {
        poll_url: format!("/admin/classify/{}", job_id),
        job_id,
        status: "pending".to_string(),
    })
}

async fn run_classify_job(job_id: String, question_id: i64, pool: Arc<MySqlPool>, ai_service: Arc<AiService>) {
    let _ = sqlx::query("UPDATE dbquizapp.taxonomy_classify_jobs SET status='running' WHERE id=?")
        .bind(&job_id)
        .execute(&*pool)
        .await;

    let outcome = classify_and_save(question_id, &pool, &ai_service).await;

    match outcome {
        Ok(result) => {
            let result_json = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());
            let _ = sqlx::query(
                "UPDATE dbquizapp.taxonomy_classify_jobs SET status='completed', result_json=?, completed_at=NOW() WHERE id=?",
            )
            .bind(&result_json)
            .bind(&job_id)
            .execute(&*pool)
            .await;
        }
        Err(msg) => {
            eprintln!("[taxonomy_classify_controller] job {} failed: {}", job_id, msg);
            let _ = sqlx::query(
                "UPDATE dbquizapp.taxonomy_classify_jobs SET status='failed', error_message=?, completed_at=NOW() WHERE id=?",
            )
            .bind(&msg)
            .bind(&job_id)
            .execute(&*pool)
            .await;
        }
    }
}

async fn classify_and_save(
    question_id: i64,
    pool: &Arc<MySqlPool>,
    ai_service: &AiService,
) -> Result<ClassifyResult, String> {
    let soal = fetch_soal_lite(pool.as_ref(), question_id)
        .await
        .map_err(|e| format!("DB error fetching soal: {}", e))?
        .ok_or_else(|| "Soal not found".to_string())?;
    let opts = options_text(&soal);

    let taxonomy_dao = TaxonomyDao::new(pool.clone());
    let tree: TaxonomyTree = taxonomy_dao
        .get_taxonomy_tree()
        .await
        .map_err(|e| format!("DB error fetching taxonomy tree: {}", e))?;

    // ── Stage 1: subcategory ────────────────────────────────────────────────
    let subcategory_slug = ai_service
        .classify_subcategory(&soal.soal, &opts, &tree)
        .await
        .map_err(|e| format!("AI stage 1 (subcategory) failed: {}", e))?;

    let mut found: Option<(
        String, // track_id
        String, // track_name
        String, // category_id
        String, // category_name
        String, // subcategory_id
        String, // subcategory_name
        Vec<crate::model::taxonomy::Topic>,
    )> = None;
    for t in &tree.tracks {
        for c in &t.categories {
            for s in &c.subcategories {
                if s.subcategory.slug == subcategory_slug {
                    found = Some((
                        t.track.id.clone(),
                        t.track.name.clone(),
                        c.category.id.clone(),
                        c.category.name.clone(),
                        s.subcategory.id.clone(),
                        s.subcategory.name.clone(),
                        s.topics.clone(),
                    ));
                }
            }
        }
    }
    let (track_id, track_name, category_id, category_name, subcategory_id, subcategory_name, topics) =
        found.ok_or_else(|| format!("AI returned an unknown subcategory_slug: {:?}", subcategory_slug))?;

    // ── Stage 2: topic within that subcategory ──────────────────────────────
    let topic_slug = ai_service
        .classify_topic(&soal.soal, &opts, &topics)
        .await
        .map_err(|e| format!("AI stage 2 (topic) failed: {}", e))?;
    let (topic_id, topic_name) = match topic_slug {
        Some(slug) => match topics.iter().find(|t| t.slug == slug) {
            Some(t) => (Some(t.id.clone()), Some(t.name.clone())),
            None => (None, None), // AI hallucinated a slug outside the shortlist -- don't guess
        },
        None => (None, None),
    };

    // ── Stage 3: cross-link additional topics ───────────────────────────────
    let candidates = filter_clean_topics(&tree, &subcategory_id);
    let additional_slugs = ai_service
        .classify_additional_topics(&soal.soal, &opts, &candidates)
        .await
        .map_err(|e| format!("AI stage 3 (cross-link) failed: {}", e))?;

    let mut additional_topic_ids = Vec::new();
    let mut additional_topic_names = Vec::new();
    for slug in &additional_slugs {
        'outer: for t in &tree.tracks {
            for c in &t.categories {
                for s in &c.subcategories {
                    for topic in &s.topics {
                        if &topic.slug == slug {
                            additional_topic_ids.push(topic.id.clone());
                            additional_topic_names.push(topic.name.clone());
                            break 'outer;
                        }
                    }
                }
            }
        }
    }

    // ── Save ─────────────────────────────────────────────────────────────────
    // `pelajaran` is the legacy free-text subject field, still read
    // elsewhere in the app (search filter, dropdowns, display) -- live data
    // shows it's always exactly the subcategory name (e.g. "Penalaran
    // Verbal", "TIU (Tes Intelegensi Umum)"), so keep it in sync here
    // instead of leaving it stuck empty after AI classification.
    sqlx::query(
        "UPDATE dbquizapp.soal SET track_id=?, category_id=?, subcategory_id=?, topic_id=?, pelajaran=?, updated_at=NOW() WHERE id=?",
    )
    .bind(&track_id)
    .bind(&category_id)
    .bind(&subcategory_id)
    .bind(&topic_id)
    .bind(&subcategory_name)
    .bind(question_id)
    .execute(pool.as_ref())
    .await
    .map_err(|e| format!("Failed to save classification: {}", e))?;

    if !additional_topic_ids.is_empty() {
        taxonomy_dao
            .set_question_topics(question_id as i32, &additional_topic_ids)
            .await
            .map_err(|e| format!("Failed to save topik tambahan: {}", e))?;
    }

    Ok(ClassifyResult {
        track_id: Some(track_id),
        track_name: Some(track_name),
        category_id: Some(category_id),
        category_name: Some(category_name),
        subcategory_id: Some(subcategory_id),
        subcategory_name: Some(subcategory_name),
        topic_id,
        topic_name,
        additional_topic_ids,
        additional_topic_names,
    })
}

/// GET /admin/classify/{job_id}
#[get("/{job_id}")]
async fn get_classify_job(path: web::Path<String>, data: web::Data<AppState<'_>>) -> impl Responder {
    let job_id = path.into_inner();
    let pool = &*data.context.soal.pool;

    #[derive(sqlx::FromRow)]
    struct JobRow {
        status: String,
        question_id: i64,
        result_json: Option<String>,
        error_message: Option<String>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    let job: JobRow = match sqlx::query_as::<_, JobRow>(
        "SELECT status, question_id, result_json, error_message, completed_at FROM dbquizapp.taxonomy_classify_jobs WHERE id = ?",
    )
    .bind(&job_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(j)) => j,
        Ok(None) => {
            return HttpResponse::NotFound().json(err("not_found", format!("Job {} not found", job_id)))
        }
        Err(e) => {
            eprintln!("[taxonomy_classify_controller] DB error fetching job {}: {:?}", job_id, e);
            return HttpResponse::InternalServerError().json(err("db_error", "Failed to fetch job status."));
        }
    };

    let result: Option<ClassifyResult> = job
        .result_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    HttpResponse::Ok().json(JobStatusResponse {
        job_id,
        status: job.status,
        question_id: job.question_id,
        result,
        error_message: job.error_message,
        completed_at: job.completed_at.map(|t| t.to_rfc3339()),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 2 of the soal analytics work: bulk retroactive classify sweep.
//
// Each item is 3 sequential AI calls via classify_and_save. A strictly
// one-at-a-time loop (the ai_bulk_jobs / bulk-enrich pattern) would take on
// the order of a week+ for the ~20k currently-uncategorized soal. This runs
// a bounded number of items concurrently instead -- 9router already showed
// it handles overlapping requests fine (observed live during earlier
// debugging), and there's no per-admin rate limit concern here since this
// isn't interactive traffic.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct BulkClassifyRequest {
    /// "uncategorized" (default) = every soal with no track_id or no
    /// subcategory_id. "ids" = the explicit list in question_ids.
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default)]
    question_ids: Vec<i64>,
    /// How many soal to classify at once. Default 10.
    concurrency: Option<u32>,
}

fn default_mode() -> String {
    "uncategorized".to_string()
}

#[derive(Serialize)]
struct BulkClassifyAcceptedResponse {
    job_id: String,
    status: String,
    total: i64,
    poll_url: String,
}

#[derive(Serialize)]
struct BulkClassifyStatusResponse {
    job_id: String,
    status: String,
    total: i32,
    processed: i32,
    succeeded: i32,
    failed: i32,
    started_at: Option<String>,
    completed_at: Option<String>,
}

/// POST /admin/classify/bulk
#[post("/bulk")]
async fn start_bulk_classify(
    body: web::Json<BulkClassifyRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    let ai_service = match &data.ai_service {
        Some(svc) => Arc::clone(svc),
        None => {
            return HttpResponse::ServiceUnavailable()
                .json(err("ai_service_unavailable", "AI service is not configured on this server."))
        }
    };
    let pool = Arc::clone(&data.context.soal.pool);
    let admin_email = extract_admin_email(&http_req);
    let concurrency = body.concurrency.unwrap_or(10).clamp(1, 30);

    let question_ids: Vec<i64> = if body.mode == "ids" {
        body.question_ids.clone()
    } else {
        match sqlx::query_scalar::<_, i64>(
            "SELECT id FROM dbquizapp.soal WHERE track_id IS NULL OR subcategory_id IS NULL",
        )
        .fetch_all(&*pool)
        .await
        {
            Ok(ids) => ids,
            Err(e) => {
                eprintln!("[taxonomy_classify_controller] Failed to resolve uncategorized ids: {:?}", e);
                return HttpResponse::InternalServerError()
                    .json(err("db_error", "Failed to resolve target questions."));
            }
        }
    };

    if question_ids.is_empty() {
        return HttpResponse::BadRequest().json(err("bad_request", "No questions matched the given criteria."));
    }

    let job_id = Uuid::new_v4().to_string();
    let total = question_ids.len() as i64;
    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO dbquizapp.taxonomy_classify_bulk_jobs (id, status, mode, total, concurrency, created_by)
        VALUES (?, 'pending', ?, ?, ?, ?)
        "#,
    )
    .bind(&job_id)
    .bind(&body.mode)
    .bind(total)
    .bind(concurrency)
    .bind(&admin_email)
    .execute(&*pool)
    .await
    {
        eprintln!("[taxonomy_classify_controller] Failed to insert bulk job: {:?}", e);
        return HttpResponse::InternalServerError().json(err("db_error", "Failed to create job."));
    }

    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        run_bulk_classify_job(job_id_clone, question_ids, concurrency as usize, pool, ai_service).await;
    });

    HttpResponse::Accepted().json(BulkClassifyAcceptedResponse {
        poll_url: format!("/admin/classify/bulk/{}", job_id),
        job_id,
        status: "pending".to_string(),
        total,
    })
}

async fn run_bulk_classify_job(
    job_id: String,
    question_ids: Vec<i64>,
    concurrency: usize,
    pool: Arc<MySqlPool>,
    ai_service: Arc<AiService>,
) {
    let _ = sqlx::query(
        "UPDATE dbquizapp.taxonomy_classify_bulk_jobs SET status='running', started_at=NOW(), heartbeat_at=NOW() WHERE id=?",
    )
    .bind(&job_id)
    .execute(&*pool)
    .await;

    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut handles = Vec::with_capacity(question_ids.len());

    for qid in question_ids {
        // Cancellation check before spawning each item -- already-running
        // items finish naturally, nothing new gets queued after this.
        let status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM dbquizapp.taxonomy_classify_bulk_jobs WHERE id=?",
        )
        .bind(&job_id)
        .fetch_optional(&*pool)
        .await
        .unwrap_or(None);
        if status.as_deref() == Some("cancelled") {
            break;
        }

        let permit = semaphore.clone().acquire_owned().await.expect("semaphore closed");
        let pool = pool.clone();
        let ai_service = ai_service.clone();
        let job_id_task = job_id.clone();

        handles.push(tokio::spawn(async move {
            let _permit = permit;
            let outcome = classify_and_save(qid, &pool, &ai_service).await;
            let succeeded = outcome.is_ok();
            if let Err(e) = &outcome {
                eprintln!("[taxonomy_classify_controller] bulk classify failed for soal {}: {}", qid, e);
            }
            let (succ_inc, fail_inc) = if succeeded { (1, 0) } else { (0, 1) };
            let _ = sqlx::query(
                "UPDATE dbquizapp.taxonomy_classify_bulk_jobs SET processed=processed+1, succeeded=succeeded+?, failed=failed+?, heartbeat_at=NOW() WHERE id=?",
            )
            .bind(succ_inc)
            .bind(fail_inc)
            .bind(&job_id_task)
            .execute(&*pool)
            .await;
        }));
    }

    for h in handles {
        let _ = h.await;
    }

    let final_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM dbquizapp.taxonomy_classify_bulk_jobs WHERE id=?")
            .bind(&job_id)
            .fetch_optional(&*pool)
            .await
            .unwrap_or(None);
    if final_status.as_deref() != Some("cancelled") {
        let _ = sqlx::query(
            "UPDATE dbquizapp.taxonomy_classify_bulk_jobs SET status='completed', completed_at=NOW() WHERE id=?",
        )
        .bind(&job_id)
        .execute(&*pool)
        .await;
    }
}

/// GET /admin/classify/bulk/{job_id}
#[get("/bulk/{job_id}")]
async fn get_bulk_classify_job(path: web::Path<String>, data: web::Data<AppState<'_>>) -> impl Responder {
    let job_id = path.into_inner();
    let pool = &*data.context.soal.pool;

    #[derive(sqlx::FromRow)]
    struct JobRow {
        status: String,
        total: i32,
        processed: i32,
        succeeded: i32,
        failed: i32,
        started_at: Option<chrono::DateTime<chrono::Utc>>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    let job: JobRow = match sqlx::query_as::<_, JobRow>(
        "SELECT status, total, processed, succeeded, failed, started_at, completed_at FROM dbquizapp.taxonomy_classify_bulk_jobs WHERE id = ?",
    )
    .bind(&job_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(j)) => j,
        Ok(None) => return HttpResponse::NotFound().json(err("not_found", format!("Job {} not found", job_id))),
        Err(e) => {
            eprintln!("[taxonomy_classify_controller] DB error fetching bulk job {}: {:?}", job_id, e);
            return HttpResponse::InternalServerError().json(err("db_error", "Failed to fetch job status."));
        }
    };

    HttpResponse::Ok().json(BulkClassifyStatusResponse {
        job_id,
        status: job.status,
        total: job.total,
        processed: job.processed,
        succeeded: job.succeeded,
        failed: job.failed,
        started_at: job.started_at.map(|t| t.to_rfc3339()),
        completed_at: job.completed_at.map(|t| t.to_rfc3339()),
    })
}

/// DELETE /admin/classify/bulk/{job_id} -- cancel a pending/running sweep.
#[actix_web::delete("/bulk/{job_id}")]
async fn cancel_bulk_classify_job(path: web::Path<String>, data: web::Data<AppState<'_>>) -> impl Responder {
    let job_id = path.into_inner();
    let pool = &*data.context.soal.pool;

    match sqlx::query(
        "UPDATE dbquizapp.taxonomy_classify_bulk_jobs SET status='cancelled' WHERE id=? AND status IN ('pending','running')",
    )
    .bind(&job_id)
    .execute(pool)
    .await
    {
        Ok(result) if result.rows_affected() > 0 => {
            HttpResponse::Ok().json(serde_json::json!({"status": "cancelled"}))
        }
        Ok(_) => HttpResponse::BadRequest().json(err("not_cancellable", "Job not found or already finished.")),
        Err(e) => {
            eprintln!("[taxonomy_classify_controller] Failed to cancel bulk job {}: {:?}", job_id, e);
            HttpResponse::InternalServerError().json(err("db_error", "Failed to cancel job."))
        }
    }
}

/// Called once at process startup. A bulk classify job left in
/// status='running' belonged to a process that got killed mid-flight (e.g.
/// CI/CD auto-redeploy) -- the `tokio::spawn`ed worker dies with it, but
/// the job row survives. For mode='uncategorized' this is trivially
/// resumable: re-running the same "no track/subcategory" query naturally
/// excludes whatever already got classified before the crash, so this
/// just continues from wherever it left off instead of restarting from
/// zero. mode='ids' jobs don't persist their target id list, so those
/// can't be safely resumed and are marked failed instead.
pub async fn resume_orphaned_bulk_classify_jobs(pool: Arc<MySqlPool>, ai_service: Option<Arc<AiService>>) {
    #[derive(sqlx::FromRow)]
    struct OrphanedJob {
        id: String,
        mode: String,
        concurrency: i32,
    }

    let orphaned: Vec<OrphanedJob> = match sqlx::query_as::<_, OrphanedJob>(
        "SELECT id, mode, concurrency FROM dbquizapp.taxonomy_classify_bulk_jobs WHERE status='running'",
    )
    .fetch_all(&*pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("[taxonomy_classify_controller] Failed to check for orphaned bulk jobs: {:?}", e);
            return;
        }
    };
    if orphaned.is_empty() {
        return;
    }

    for job in orphaned {
        if job.mode != "uncategorized" {
            eprintln!(
                "[taxonomy_classify_controller] Orphaned bulk job {} (mode={}) can't be auto-resumed, marking failed.",
                job.id, job.mode
            );
            let _ = sqlx::query(
                "UPDATE dbquizapp.taxonomy_classify_bulk_jobs SET status='failed', error_message='orphaned by process restart, mode!=uncategorized jobs are not auto-resumed', completed_at=NOW() WHERE id=?",
            )
            .bind(&job.id)
            .execute(&*pool)
            .await;
            continue;
        }

        let Some(ai_service) = ai_service.clone() else {
            eprintln!(
                "[taxonomy_classify_controller] Orphaned bulk job {} found but AI service unavailable, marking failed.",
                job.id
            );
            let _ = sqlx::query(
                "UPDATE dbquizapp.taxonomy_classify_bulk_jobs SET status='failed', error_message='orphaned by process restart, AI service unavailable on resume', completed_at=NOW() WHERE id=?",
            )
            .bind(&job.id)
            .execute(&*pool)
            .await;
            continue;
        };

        // NAS and PC run this same startup check independently against
        // the same DB -- confirmed live, both resumed the same job at
        // once and doubled AI load. Claim atomically first: only proceed
        // if this UPDATE actually affects a row, i.e. the job's heartbeat
        // is stale (nothing else is actively ticking it right now).
        // InnoDB serializes concurrent UPDATEs to the same row, so only
        // one instance's claim can win when both race for it.
        let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string());
        let claimed = match sqlx::query(
            "UPDATE dbquizapp.taxonomy_classify_bulk_jobs \
             SET worker_hostname=?, heartbeat_at=NOW() \
             WHERE id=? AND status='running' \
               AND (heartbeat_at IS NULL OR heartbeat_at < NOW() - INTERVAL 2 MINUTE)",
        )
        .bind(&hostname)
        .bind(&job.id)
        .execute(&*pool)
        .await
        {
            Ok(res) => res.rows_affected() > 0,
            Err(e) => {
                eprintln!(
                    "[taxonomy_classify_controller] Failed to claim orphaned bulk job {}: {:?}",
                    job.id, e
                );
                false
            }
        };
        if !claimed {
            println!(
                "[taxonomy_classify_controller] Orphaned bulk job {} has a fresh heartbeat -- another instance is already working it, skipping.",
                job.id
            );
            continue;
        }

        let remaining_ids: Vec<i64> = match sqlx::query_scalar(
            "SELECT id FROM dbquizapp.soal WHERE track_id IS NULL OR subcategory_id IS NULL",
        )
        .fetch_all(&*pool)
        .await
        {
            Ok(ids) => ids,
            Err(e) => {
                eprintln!(
                    "[taxonomy_classify_controller] Failed to resolve remaining ids for orphaned job {}: {:?}",
                    job.id, e
                );
                continue;
            }
        };

        if remaining_ids.is_empty() {
            let _ = sqlx::query(
                "UPDATE dbquizapp.taxonomy_classify_bulk_jobs SET status='completed', completed_at=NOW() WHERE id=?",
            )
            .bind(&job.id)
            .execute(&*pool)
            .await;
            continue;
        }

        println!(
            "[taxonomy_classify_controller] Resuming orphaned bulk job {} -- {} soal still uncategorized.",
            job.id,
            remaining_ids.len()
        );
        let pool_clone = pool.clone();
        let job_id = job.id.clone();
        let concurrency = job.concurrency.max(1) as usize;
        tokio::spawn(async move {
            run_bulk_classify_job(job_id, remaining_ids, concurrency, pool_clone, ai_service).await;
        });
    }
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/classify")
            .wrap(AdminMiddleware::new())
            .service(start_bulk_classify)
            .service(get_bulk_classify_job)
            .service(cancel_bulk_classify_job)
            .service(start_classify)
            .service(get_classify_job),
    );
}
