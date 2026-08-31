//! "Import Soal dari Teks" -- admin pastes raw extracted text from an
//! existing latihan-soal document (OCR output or copy-paste from a PDF/
//! image set): questions and their 5 options are already fully present,
//! just unstructured plain text, usually grouped under section headers
//! ("Persamaan Kata", "Analogi", "Kemampuan Numerik", ...) with no answer
//! key. This does NOT generate new questions (see materi_generate_controller
//! for that) -- it splits the pasted text into per-question blocks
//! deterministically here in Rust (no AI needed for that part, it's a
//! numbering pattern), then asks AI to fill in only what's actually
//! missing: correct_answer and solution. Section headers become the tag
//! suggestion.
//!
//! Runs as an async job, same Cloudflare-timeout reasoning as every other
//! AI endpoint in this codebase.

use actix_multipart::Multipart;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use futures::TryStreamExt;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::admin_middleware::{AdminMiddleware, AuthentikClaims};
use crate::service::ai_service::{AiService, ParsedSoal};
use crate::AppState;
use actix_web::HttpMessage;

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

fn err(code: &str, msg: impl Into<String>) -> ErrorResponse {
    ErrorResponse { error: code.to_string(), message: msg.into() }
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

async fn read_field_text(field: &mut actix_multipart::Field) -> String {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.try_next().await.unwrap_or(None) {
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

// ── Deterministic block splitting (no AI) ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Block {
    number: String,
    section: Option<String>,
    raw_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkippedBlock {
    number: String,
    reason: String,
}

/// Splits raw pasted text into per-question blocks. A line starting with
/// `<digits>.` opens a new block; everything after it (including a-e
/// option lines) belongs to that block until the next numbered line. Any
/// non-empty line seen while NOT inside a block is treated as a section
/// header and carried forward. Blocks that look like an image placeholder
/// rather than actual question text (no option markers found, or an
/// explicit "[gambar"/"[soal gambar" tag) are split out as skipped instead
/// of sent to AI.
fn split_into_blocks(raw: &str) -> (Vec<Block>, Vec<SkippedBlock>) {
    let question_start = Regex::new(r"^(\d+)\.\s*(.*)$").unwrap();
    let option_line = Regex::new(r"^[a-eA-E][.\)]\s+").unwrap();
    let image_marker = Regex::new(r"(?i)\[\s*(soal\s+)?gambar|\[\s*pola\s+visual").unwrap();

    let mut blocks: Vec<Block> = Vec::new();
    let mut current_section: Option<String> = None;
    // Section pinned at the moment the current block started -- a header
    // line encountered later (after the block's last option) belongs to
    // the *next* block, so finalizing must not read the live current_section.
    let mut current_block_section: Option<String> = None;
    let mut current_number: Option<String> = None;
    let mut current_lines: Vec<String> = Vec::new();
    // Once the current block has seen an a-e option line, any further
    // non-option line is a section header for the *next* block, not more
    // of this block's stem -- otherwise a header sitting between the last
    // option and the next numbered question gets swallowed as stem text.
    let mut seen_option_in_current = false;

    let finalize = |number: &Option<String>, lines: &[String], section: &Option<String>, blocks: &mut Vec<Block>| {
        if let Some(n) = number {
            if !lines.is_empty() {
                blocks.push(Block {
                    number: n.clone(),
                    section: section.clone(),
                    raw_text: lines.join("\n"),
                });
            }
        }
    };

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(caps) = question_start.captures(trimmed) {
            finalize(&current_number, &current_lines, &current_block_section, &mut blocks);
            current_number = Some(caps.get(1).unwrap().as_str().to_string());
            current_lines = vec![caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default()];
            current_block_section = current_section.clone();
            seen_option_in_current = false;
        } else if option_line.is_match(trimmed) {
            current_lines.push(trimmed.to_string());
            seen_option_in_current = true;
        } else if current_number.is_some() && !seen_option_in_current {
            // Still building the stem (multi-line question before options start).
            current_lines.push(trimmed.to_string());
        } else {
            // Not inside a block, or the current block already has its
            // options -- this is a section header / document title line.
            current_section = Some(trimmed.to_string());
        }
    }
    finalize(&current_number, &current_lines, &current_block_section, &mut blocks);

    let mut parseable = Vec::new();
    let mut skipped = Vec::new();
    for b in blocks {
        let has_options = b.raw_text.lines().filter(|l| option_line.is_match(l)).count() >= 3;
        if image_marker.is_match(&b.raw_text) || !has_options {
            skipped.push(SkippedBlock {
                number: b.number,
                reason: if image_marker.is_match(&b.raw_text) {
                    "gambar/non-teks, gak bisa diparse".to_string()
                } else {
                    "kurang dari 3 opsi jawaban terdeteksi".to_string()
                },
            });
        } else {
            parseable.push(b);
        }
    }
    (parseable, skipped)
}

/// Validates one AI-completed item, same spirit as materi_generate's
/// validate_generated -- never auto-corrects, failures go to `rejected`.
fn validate_parsed(item: &ParsedSoal) -> Option<String> {
    if item.soal.trim().is_empty() {
        return Some("field 'soal' kosong".to_string());
    }
    let opts = [&item.opt1, &item.opt2, &item.opt3, &item.opt4, &item.opt5];
    if opts.iter().any(|o| o.trim().is_empty()) {
        return Some("ada opsi (opt1-5) yang kosong".to_string());
    }
    if !["opt1", "opt2", "opt3", "opt4", "opt5"].contains(&item.correct_answer.as_str()) {
        return Some(format!("correct_answer bukan salah satu dari opt1-5: {:?}", item.correct_answer));
    }
    if item.solution.trim().is_empty() {
        return Some("field 'solution' kosong".to_string());
    }
    None
}

// ── Job types ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct AcceptedItem {
    #[serde(flatten)]
    item: ParsedSoal,
    tag: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct RejectedItem {
    reason: String,
    raw: ParsedSoal,
}

#[derive(Serialize)]
struct JobAcceptedResponse {
    job_id: String,
    status: String,
    total_blocks: usize,
    skipped_at_split: usize,
    poll_url: String,
}

#[derive(Serialize)]
struct JobStatusResponse {
    job_id: String,
    status: String,
    total_blocks: i32,
    accepted: Vec<AcceptedItem>,
    rejected: Vec<RejectedItem>,
    skipped: Vec<SkippedBlock>,
    error_message: Option<String>,
    completed_at: Option<String>,
}

const BATCH_SIZE: usize = 8;

/// POST /admin/materi/parse -- multipart field `text` (or `file`): the raw
/// pasted/extracted text. Splits synchronously (cheap, no AI), returns 202
/// immediately with the split-time skip count, then completes the
/// parseable blocks via AI in the background.
#[post("/parse")]
async fn start_parse(
    mut payload: Multipart,
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
    let admin_email = extract_admin_email(&http_req);

    let mut source_text = String::new();
    while let Some(mut field) = payload.try_next().await.unwrap_or(None) {
        let name = match field.content_disposition().get_name() {
            Some(n) => n.to_string(),
            None => continue,
        };
        let text = read_field_text(&mut field).await;
        if name == "text" || name == "file" {
            source_text = text;
        }
    }

    if source_text.trim().is_empty() {
        return HttpResponse::BadRequest().json(err("bad_request", "Field 'text' (atau file) wajib diisi."));
    }

    let (blocks, skipped_at_split) = split_into_blocks(&source_text);
    if blocks.is_empty() {
        return HttpResponse::BadRequest().json(err(
            "no_parseable_blocks",
            format!(
                "Gak ada soal yang bisa dikenali dari teks ini ({} blok dilewati).",
                skipped_at_split.len()
            ),
        ));
    }

    let pool = Arc::clone(&data.context.soal.pool);
    let job_id = Uuid::new_v4().to_string();
    let total_blocks = blocks.len();
    let skipped_json = serde_json::to_string(&skipped_at_split).unwrap_or_else(|_| "[]".to_string());

    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO dbquizapp.materi_parse_jobs (id, status, total_blocks, skipped_json, created_by)
        VALUES (?, 'pending', ?, ?, ?)
        "#,
    )
    .bind(&job_id)
    .bind(total_blocks as i32)
    .bind(&skipped_json)
    .bind(&admin_email)
    .execute(&*pool)
    .await
    {
        eprintln!("[materi_parse_controller] Failed to insert job: {:?}", e);
        return HttpResponse::InternalServerError().json(err("db_error", "Failed to create job."));
    }

    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        run_parse_job(job_id_clone, blocks, pool, ai_service).await;
    });

    HttpResponse::Accepted().json(JobAcceptedResponse {
        poll_url: format!("/admin/materi/parse/{}", job_id),
        job_id,
        status: "pending".to_string(),
        total_blocks,
        skipped_at_split: skipped_at_split.len(),
    })
}

async fn run_parse_job(job_id: String, blocks: Vec<Block>, pool: Arc<MySqlPool>, ai_service: Arc<AiService>) {
    let _ = sqlx::query("UPDATE dbquizapp.materi_parse_jobs SET status='running' WHERE id=?")
        .bind(&job_id)
        .execute(&*pool)
        .await;

    // number -> section, so a result can be tagged once matched back by the
    // AI's echoed `number` field.
    let section_by_number: std::collections::HashMap<String, Option<String>> =
        blocks.iter().map(|b| (b.number.clone(), b.section.clone())).collect();

    let mut accepted: Vec<AcceptedItem> = Vec::new();
    let mut rejected: Vec<RejectedItem> = Vec::new();
    let mut had_error: Option<String> = None;

    for chunk in blocks.chunks(BATCH_SIZE) {
        let batch: Vec<(String, String)> = chunk.iter().map(|b| (b.number.clone(), b.raw_text.clone())).collect();
        match ai_service.parse_soal_batch(&batch).await {
            Ok(items) => {
                for item in items {
                    let tag = section_by_number.get(&item.number).cloned().flatten();
                    match validate_parsed(&item) {
                        None => accepted.push(AcceptedItem { item, tag }),
                        Some(reason) => rejected.push(RejectedItem { reason, raw: item }),
                    }
                }
            }
            Err(e) => {
                eprintln!("[materi_parse_controller] batch failed for job {}: {}", job_id, e);
                had_error = Some(e.to_string());
                // Keep going with the remaining batches instead of aborting
                // the whole job over one bad batch -- admin still gets
                // partial results plus the error message.
            }
        }
    }

    let accepted_json = serde_json::to_string(&accepted).unwrap_or_else(|_| "[]".to_string());
    let rejected_json = serde_json::to_string(&rejected).unwrap_or_else(|_| "[]".to_string());

    let _ = sqlx::query(
        r#"
        UPDATE dbquizapp.materi_parse_jobs
        SET status='completed', accepted_json=?, rejected_json=?, error_message=?, completed_at=NOW()
        WHERE id=?
        "#,
    )
    .bind(&accepted_json)
    .bind(&rejected_json)
    .bind(&had_error)
    .bind(&job_id)
    .execute(&*pool)
    .await;
}

/// GET /admin/materi/parse/{job_id}
#[get("/parse/{job_id}")]
async fn get_parse_job(path: web::Path<String>, data: web::Data<AppState<'_>>) -> impl Responder {
    let job_id = path.into_inner();
    let pool = &*data.context.soal.pool;

    #[derive(sqlx::FromRow)]
    struct JobRow {
        status: String,
        total_blocks: i32,
        skipped_json: Option<String>,
        accepted_json: Option<String>,
        rejected_json: Option<String>,
        error_message: Option<String>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    let job: JobRow = match sqlx::query_as::<_, JobRow>(
        r#"
        SELECT status, total_blocks, skipped_json, accepted_json, rejected_json, error_message, completed_at
        FROM dbquizapp.materi_parse_jobs
        WHERE id = ?
        "#,
    )
    .bind(&job_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(j)) => j,
        Ok(None) => return HttpResponse::NotFound().json(err("not_found", format!("Job {} not found", job_id))),
        Err(e) => {
            eprintln!("[materi_parse_controller] DB error fetching job {}: {:?}", job_id, e);
            return HttpResponse::InternalServerError().json(err("db_error", "Failed to fetch job status."));
        }
    };

    let accepted: Vec<AcceptedItem> =
        job.accepted_json.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();
    let rejected: Vec<RejectedItem> =
        job.rejected_json.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();
    let skipped: Vec<SkippedBlock> =
        job.skipped_json.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();

    HttpResponse::Ok().json(JobStatusResponse {
        job_id,
        status: job.status,
        total_blocks: job.total_blocks,
        accepted,
        rejected,
        skipped,
        error_message: job.error_message,
        completed_at: job.completed_at.map(|t| t.to_rfc3339()),
    })
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/materi")
            .wrap(AdminMiddleware::new())
            .service(start_parse)
            .service(get_parse_job),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_numbered_questions_with_options() {
        let text = "Persamaan Kata\n\n1. PANDIR\na. Agak Pintar\nb. Bebal\nc. Cerdas\nd. Tidak Jenius\ne. Pemandangan\n\n2. OBESITAS\na. Kurus\nb. Tambun\nc. Tinggi\nd. Cebol\ne. Pendek\n";
        let (blocks, skipped) = split_into_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].number, "1");
        assert_eq!(blocks[0].section.as_deref(), Some("Persamaan Kata"));
        assert!(blocks[0].raw_text.contains("PANDIR"));
        assert!(blocks[0].raw_text.contains("e. Pemandangan"));
        assert_eq!(blocks[1].number, "2");
        assert!(skipped.is_empty());
    }

    #[test]
    fn skips_image_placeholder_blocks() {
        let text = "54. [Soal Gambar / Pola Visual - segitiga]\n\n55. [Soal Gambar / Pola Visual - lingkaran]\n";
        let (blocks, skipped) = split_into_blocks(text);
        assert!(blocks.is_empty());
        assert_eq!(skipped.len(), 2);
        assert_eq!(skipped[0].number, "54");
    }

    #[test]
    fn skips_blocks_with_too_few_options() {
        let text = "1. Some question with no real options\na. Only one\n";
        let (blocks, skipped) = split_into_blocks(text);
        assert!(blocks.is_empty());
        assert_eq!(skipped.len(), 1);
    }

    #[test]
    fn carries_section_header_forward_across_questions() {
        let text = "Lawan Kata\n11. BHINEKA\na. Tunggal\nb. Jua\nc. Berbeda beda\nd. Tetap\ne. Heterogen\n\nAnalogi\n21. Pohon : Berlindung\na. Rambut : Hitam\nb. Telinga : Anting\nc. Buku : Pena\nd. Kaki : Melangkah\ne. Kepala : Kaki\n";
        let (blocks, _) = split_into_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].section.as_deref(), Some("Lawan Kata"));
        assert_eq!(blocks[1].section.as_deref(), Some("Analogi"));
    }
}
