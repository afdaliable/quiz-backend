use crate::controller::log_request;
use crate::model::soal::{CreateSoalRequest, BulkImportResponse};
use crate::AppState;
use actix_web::{web, HttpResponse, HttpRequest, post};
use std::net::IpAddr;

/// Check whether an IP address is from a trusted internal network.
/// Allows: loopback, RFC-1918 private ranges, and Tailscale CGNAT range (100.64.0.0/10).
fn is_internal_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            v4.is_loopback()                          // 127.0.0.0/8
            || octets[0] == 10                         // 10.0.0.0/8
            || (octets[0] == 172 && (16..=31).contains(&octets[1]))  // 172.16.0.0/12
            || (octets[0] == 192 && octets[1] == 168) // 192.168.0.0/16
            || (octets[0] == 100 && (octets[1] & 0b1100_0000) == 0b0100_0000) // 100.64.0.0/10 (Tailscale)
        }
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/internal")
            .service(bulk_import_internal)
    );
}

/// Bulk import questions via internal endpoint (no OAuth — API key + IP restriction only).
///
/// Accepts a JSON array of questions directly (not wrapped in an object).
/// Security: requires `X-Internal-Api-Key` header matching `INTERNAL_API_KEY` env var,
/// and the request must originate from a loopback, RFC-1918, or Tailscale IP.
#[post("/soal/bulk-import")]
async fn bulk_import_internal(
    req: HttpRequest,
    questions: web::Json<Vec<CreateSoalRequest>>,
    data: web::Data<AppState<'_>>,
) -> HttpResponse {
    log_request("POST /internal/soal/bulk-import", &data.connections);

    // 1. IP check
    let peer_ip = req.peer_addr().map(|a| a.ip());
    let ip_allowed = peer_ip.map(is_internal_ip).unwrap_or(false);
    if !ip_allowed {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Forbidden: request origin not in allowed IP range"
        }));
    }

    // 2. API key check
    let expected_key = match data.config.get_internal_api_key() {
        Some(key) if !key.is_empty() => key,
        _ => {
            eprintln!("internal_api_key is not set in config.json");
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Server misconfiguration: internal_api_key not configured"
            }));
        }
    };

    let provided_key = req
        .headers()
        .get("X-Internal-Api-Key")
        .and_then(|v| v.to_str().ok());

    if provided_key != Some(expected_key) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Unauthorized: missing or invalid X-Internal-Api-Key"
        }));
    }

    // 3. Validate payload
    if questions.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "No questions provided for import"
        }));
    }

    // 4. Insert using the same DAO as the existing bulk-import endpoint
    let mut success_count: i32 = 0;
    let mut failed_count: i32 = 0;
    let mut errors: Vec<String> = Vec::new();

    for (index, question) in questions.iter().enumerate() {
        if question.soal.trim().is_empty() {
            failed_count += 1;
            errors.push(format!("Question {}: question text cannot be empty", index + 1));
            continue;
        }

        match data.context.soal.create_soal(question).await {
            Ok(_) => success_count += 1,
            Err(e) => {
                failed_count += 1;
                errors.push(format!("Question {}: {}", index + 1, e));
            }
        }
    }

    HttpResponse::Ok().json(BulkImportResponse {
        success_count,
        failed_count,
        errors,
    })
}
