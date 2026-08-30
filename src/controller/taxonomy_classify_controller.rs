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

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/classify")
            .wrap(AdminMiddleware::new())
            .service(start_classify)
            .service(get_classify_job),
    );
}
