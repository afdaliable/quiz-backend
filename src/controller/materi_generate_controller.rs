//! Generate draft soal from a chunk of materi text via AiService. Mirrors
//! the manual generate.py workflow used for the UPKP materi pipeline this
//! quarter, productized as an admin endpoint. Input is plain text (paste or
//! .txt upload) -- not raw PDF; PDF extraction quality varies too much to
//! trust unattended, so materi PDFs are expected to be converted to text
//! before hitting this endpoint (same convention the manual pipeline used).

use actix_multipart::Multipart;
use actix_web::{post, web, HttpResponse, Responder};
use futures::TryStreamExt;
use serde::Serialize;

use crate::middleware::admin_middleware::AdminMiddleware;
use crate::service::ai_service::GeneratedSoal;
use crate::AppState;

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

#[derive(Serialize)]
struct RejectedItem {
    reason: String,
    raw: GeneratedSoal,
}

#[derive(Serialize)]
struct GenerateSoalResponse {
    materi: String,
    group: String,
    accepted: Vec<GeneratedSoal>,
    rejected: Vec<RejectedItem>,
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
/// Balikin draft soal untuk direview -- endpoint ini TIDAK menyimpan
/// apa pun ke `soal`. Simpan lewat `POST /admin/soal` yang sudah ada
/// setelah admin review manual, sama seperti alur bulk-enrich/review-soal.
#[post("/generate")]
async fn generate_soal(mut payload: Multipart, data: web::Data<AppState<'_>>) -> impl Responder {
    let ai_service = match &data.ai_service {
        Some(svc) => svc,
        None => {
            return HttpResponse::ServiceUnavailable().json(err(
                "ai_service_unavailable",
                "AI service is not configured on this server.",
            ))
        }
    };

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

    let generated = match ai_service
        .generate_soal(&materi, &group, &source_text, count)
        .await
    {
        Ok(items) => items,
        Err(e) => {
            eprintln!("[materi_generate_controller] generate_soal failed: {}", e);
            return HttpResponse::BadGateway().json(err("ai_error", e.to_string()));
        }
    };

    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for item in generated {
        match validate_generated(&item) {
            None => accepted.push(item),
            Some(reason) => rejected.push(RejectedItem { reason, raw: item }),
        }
    }

    HttpResponse::Ok().json(GenerateSoalResponse {
        materi,
        group,
        accepted,
        rejected,
    })
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/materi")
            .wrap(AdminMiddleware::new())
            .service(generate_soal),
    );
}
