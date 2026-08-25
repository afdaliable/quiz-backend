//! Generate draft soal from a chunk of materi text via AiService. Mirrors
//! the manual generate.py workflow used for the UPKP materi pipeline this
//! quarter, productized as an admin endpoint. Input is plain text (paste or
//! .txt upload) -- not raw PDF; PDF extraction quality varies too much to
//! trust unattended, so materi PDFs are expected to be converted to text
//! before hitting this endpoint (same convention the manual pipeline used).
//!
//! Runs as an async job (same reasoning as the single-enrich endpoint):
//! generating 8-20 soal takes even longer than a single-field enrich call,
//! so a synchronous request would reliably trip Cloudflare's Free-plan
//! proxy timeout. POST returns 202+job_id, poll via GET for the result.

use actix_multipart::Multipart;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::admin_middleware::{AdminMiddleware, AuthentikClaims};
use crate::service::ai_service::{AiService, GeneratedSoal};
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

#[derive(Serialize, Deserialize)]
struct RejectedItem {
    reason: String,
    raw: GeneratedSoal,
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
    materi: String,
    topic_group: String,
    count_requested: i32,
    accepted: Vec<GeneratedSoal>,
    rejected: Vec<RejectedItem>,
    error_message: Option<String>,
    completed_at: Option<String>,
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

/// Reads one multipart field (text field or file) fully into a String.
async fn read_field_text(field: &mut actix_multipart::Field) -> String {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.try_next().await.unwrap_or(None) {
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Validates one generated item against the same rules the manual pipeline
/// used: no empty required field, correct_answer must reference a real
/// option. Never auto-corrects -- items that fail go to `rejected` for the
/// admin to see, not silently dropped or guessed at.
fn validate_generated(item: &GeneratedSoal) -> Option<String> {
    if item.soal.trim().is_empty() {
        return Some("field 'soal' kosong".to_string());
    }
    let opts = [&item.opt1, &item.opt2, &item.opt3, &item.opt4, &item.opt5];
    if opts.iter().any(|o| o.trim().is_empty()) {
        return Some("ada opsi (opt1-5) yang kosong".to_string());
    }
    if !["opt1", "opt2", "opt3", "opt4", "opt5"].contains(&item.correct_answer.as_str()) {
        return Some(format!(
            "correct_answer bukan salah satu dari opt1-5: {:?}",
            item.correct_answer
        ));
    }
    if item.solution.trim().is_empty() {
        return Some("field 'solution' kosong".to_string());
    }
    None
}

/// POST /admin/materi/generate
///
/// Multipart fields: `materi` (nama materi), `group` (grup/topik dalam
/// materi itu), `count` (jumlah soal diminta, default 8, di-clamp 1-20),
/// `text` (isi materi, plain text -- boleh dikirim sebagai field teks biasa
/// atau sebagai file upload, dua-duanya dibaca sama).
///
/// Balikin 202 + job_id. Endpoint ini TIDAK menyimpan apa pun ke `soal` --
/// poll GET /admin/materi/generate/{job_id} untuk draft hasilnya, lalu
/// simpan lewat POST /admin/soal yang sudah ada setelah admin review manual.
#[post("/generate")]
async fn start_generate(
    mut payload: Multipart,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    let ai_service = match &data.ai_service {
        Some(svc) => Arc::clone(svc),
        None => {
            return HttpResponse::ServiceUnavailable().json(err(
                "ai_service_unavailable",
                "AI service is not configured on this server.",
            ))
        }
    };
    let admin_email = extract_admin_email(&http_req);

    let mut materi = String::new();
    let mut group = String::new();
    let mut count: u32 = 8;
    let mut source_text = String::new();

    while let Some(mut field) = payload.try_next().await.unwrap_or(None) {
        let name = match field.content_disposition().get_name() {
            Some(n) => n.to_string(),
            None => continue,
        };
        let text = read_field_text(&mut field).await;
        match name.as_str() {
            "materi" => materi = text.trim().to_string(),
            "group" => group = text.trim().to_string(),
            "count" => count = text.trim().parse().unwrap_or(8),
            "text" | "file" => source_text = text,
            _ => {}
        }
    }

    if materi.is_empty() || group.is_empty() || source_text.trim().is_empty() {
        return HttpResponse::BadRequest().json(err(
            "bad_request",
            "Field 'materi', 'group', dan 'text' (atau file) wajib diisi.",
        ));
    }
    let count = count.clamp(1, 20);

    let pool = Arc::clone(&data.context.soal.pool);
    let job_id = Uuid::new_v4().to_string();

    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO dbquizapp.materi_generate_jobs
            (id, status, materi, topic_group, count_requested, created_by)
        VALUES (?, 'pending', ?, ?, ?, ?)
        "#,
    )
    .bind(&job_id)
    .bind(&materi)
    .bind(&group)
    .bind(count as i32)
    .bind(&admin_email)
    .execute(&*pool)
    .await
    {
        eprintln!("[materi_generate_controller] Failed to insert job: {:?}", e);
        return HttpResponse::InternalServerError().json(err("db_error", "Failed to create job."));
    }

    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        run_generate_job(job_id_clone, materi, group, source_text, count, pool, ai_service).await;
    });

    HttpResponse::Accepted().json(JobAcceptedResponse {
        poll_url: format!("/admin/materi/generate/{}", job_id),
        job_id,
        status: "pending".to_string(),
    })
}

async fn run_generate_job(
    job_id: String,
    materi: String,
    group: String,
    source_text: String,
    count: u32,
    pool: Arc<MySqlPool>,
    ai_service: Arc<AiService>,
) {
    let _ = sqlx::query(
        "UPDATE dbquizapp.materi_generate_jobs SET status='running' WHERE id=?",
    )
    .bind(&job_id)
    .execute(&*pool)
    .await;

    match ai_service.generate_soal(&materi, &group, &source_text, count).await {
        Ok(items) => {
            let mut accepted = Vec::new();
            let mut rejected = Vec::new();
            for item in items {
                match validate_generated(&item) {
                    None => accepted.push(item),
                    Some(reason) => rejected.push(RejectedItem { reason, raw: item }),
                }
            }
            let accepted_json = serde_json::to_string(&accepted).unwrap_or_else(|_| "[]".to_string());
            let rejected_json = serde_json::to_string(&rejected).unwrap_or_else(|_| "[]".to_string());

            let _ = sqlx::query(
                r#"
                UPDATE dbquizapp.materi_generate_jobs
                SET status='completed', accepted_json=?, rejected_json=?, completed_at=NOW()
                WHERE id=?
                "#,
            )
            .bind(&accepted_json)
            .bind(&rejected_json)
            .bind(&job_id)
            .execute(&*pool)
            .await;
        }
        Err(e) => {
            eprintln!("[materi_generate_controller] generate_soal failed for job {}: {}", job_id, e);
            let _ = sqlx::query(
                "UPDATE dbquizapp.materi_generate_jobs SET status='failed', error_message=?, completed_at=NOW() WHERE id=?",
            )
            .bind(e.to_string())
            .bind(&job_id)
            .execute(&*pool)
            .await;
        }
    }
}

/// GET /admin/materi/generate/{job_id}
#[get("/generate/{job_id}")]
async fn get_generate_job(path: web::Path<String>, data: web::Data<AppState<'_>>) -> impl Responder {
    let job_id = path.into_inner();
    let pool = &*data.context.soal.pool;

    #[derive(sqlx::FromRow)]
    struct JobRow {
        status: String,
        materi: String,
        topic_group: String,
        count_requested: i32,
        accepted_json: Option<String>,
        rejected_json: Option<String>,
        error_message: Option<String>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    let job: JobRow = match sqlx::query_as::<_, JobRow>(
        r#"
        SELECT status, materi, topic_group, count_requested, accepted_json,
               rejected_json, error_message, completed_at
        FROM dbquizapp.materi_generate_jobs
        WHERE id = ?
        "#,
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
            eprintln!("[materi_generate_controller] DB error fetching job {}: {:?}", job_id, e);
            return HttpResponse::InternalServerError().json(err("db_error", "Failed to fetch job status."));
        }
    };

    let accepted: Vec<GeneratedSoal> = job
        .accepted_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let rejected: Vec<RejectedItem> = job
        .rejected_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    HttpResponse::Ok().json(JobStatusResponse {
        job_id,
        status: job.status,
        materi: job.materi,
        topic_group: job.topic_group,
        count_requested: job.count_requested,
        accepted,
        rejected,
        error_message: job.error_message,
        completed_at: job.completed_at.map(|t| t.to_rfc3339()),
    })
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/materi")
            .wrap(AdminMiddleware::new())
            .service(start_generate)
            .service(get_generate_job),
    );
}
