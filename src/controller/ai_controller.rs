use crate::middleware::admin_middleware::{AdminMiddleware, AuthentikClaims};
use crate::model::ai_models::estimate_cost;
use crate::model::soal::AdminSoal;
use crate::service::ai_service::{AiError, AiService, SoalContext};
use crate::AppState;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types
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

#[derive(Debug, Serialize)]
pub struct EnrichResponse {
    pub question_id: i64,
    pub original: FieldValues,
    pub enriched: FieldValues,
    pub saved: bool,
    pub provider_used: String,
    pub model_used: String,
    pub tokens_used: TokensUsed,
}

#[derive(Debug, Serialize)]
pub struct FieldValues {
    pub solution: Option<String>,
    pub tag: Option<String>,
    pub modul: Option<String>,
    pub pelajaran: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TokensUsed {
    pub prompt: u32,
    pub completion: u32,
    pub total: u32,
}

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
            .route("/questions/{id}/enrich", web::post().to(enrich_question)),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Handler: POST /ai/questions/{id}/enrich
// ─────────────────────────────────────────────────────────────────────────────

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
    let ai_service: &AiService = match &data.ai_service {
        Some(svc) => svc,
        None => {
            return HttpResponse::ServiceUnavailable().json(err(
                "ai_service_unavailable",
                "AI service is not configured on this server.",
            ))
        }
    };

    // ── 4. Fetch soal ─────────────────────────────────────────────────────
    let query = r#"
        SELECT s.*,
               COALESCE(s.created_at, NOW()) AS created_at,
               COALESCE(s.updated_at, NOW()) AS updated_at,
               0 AS usage_count
        FROM dbquizapp.soal s
        WHERE s.id = ?
    "#;

    let soal: AdminSoal = match sqlx::query_as::<_, AdminSoal>(query)
        .bind(question_id)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            return HttpResponse::NotFound().json(err(
                "not_found",
                format!("Question with id {} not found", question_id),
            ))
        }
        Err(e) => {
            eprintln!("[ai_controller] DB error fetching soal {}: {:?}", question_id, e);
            return HttpResponse::InternalServerError().json(err(
                "db_error",
                "Failed to fetch question from database.",
            ));
        }
    };

    // ── 5. Resolve fields to enrich ───────────────────────────────────────
    let fields: Vec<String> = if body.fields.is_empty() {
        vec![
            "solution".into(),
            "tag".into(),
            "modul".into(),
            "pelajaran".into(),
        ]
    } else {
        body.fields.clone()
    };

    // ── 6. Build SoalContext ──────────────────────────────────────────────
    let ctx = SoalContext {
        id: soal.id as i64,
        soal: soal.soal.clone(),
        opt1: soal.opt1.clone(),
        opt2: soal.opt2.clone(),
        opt3: soal.opt3.clone(),
        opt4: soal.opt4.clone(),
        opt5: soal.opt5.clone(),
        correct_answer: soal.correct_answer.clone(),
        fields_to_enrich: fields.clone(),
    };

    // ── 7. Call AI ────────────────────────────────────────────────────────
    let enriched = match ai_service.enrich_question(&ctx).await {
        Ok(e) => e,
        Err(e) => {
            let err_msg = e.to_string();
            let _ = insert_ai_usage_log(
                pool,
                question_id,
                None,
                "unknown",
                "unknown",
                &fields,
                0,
                0,
                0.0,
                false,
                Some(&err_msg),
                Some(&admin_email),
            )
            .await;

            return match e {
                AiError::BothProvidersFailed(_) => {
                    HttpResponse::BadGateway().json(err("ai_providers_failed", err_msg))
                }
                _ => HttpResponse::BadGateway().json(err("ai_error", err_msg)),
            };
        }
    };

    // ── 8. Estimate cost & log usage ──────────────────────────────────────
    let cost = estimate_cost(
        &enriched.provider_used,
        enriched.prompt_tokens as i32,
        enriched.completion_tokens as i32,
    );

    let _ = insert_ai_usage_log(
        pool,
        question_id,
        None,
        &enriched.provider_used.clone(),
        &enriched.model_used.clone(),
        &fields,
        enriched.prompt_tokens as i32,
        enriched.completion_tokens as i32,
        cost,
        true,
        None,
        Some(&admin_email),
    )
    .await;

    // ── 9. Persist if save=true ───────────────────────────────────────────
    let saved = if body.save {
        // Only save fields that were requested AND the AI returned a value for
        let solution = if fields.contains(&"solution".to_string()) {
            enriched.solution.as_deref()
        } else {
            None
        };
        let tag = if fields.contains(&"tag".to_string()) {
            enriched.tag.as_deref()
        } else {
            None
        };
        let modul = if fields.contains(&"modul".to_string()) {
            enriched.modul.as_deref()
        } else {
            None
        };
        let pelajaran = if fields.contains(&"pelajaran".to_string()) {
            enriched.pelajaran.as_deref()
        } else {
            None
        };

        let save_ok = save_enriched_fields(pool, question_id, solution, tag, modul, pelajaran)
            .await
            .is_ok();

        if save_ok {
            // Insert ai_generated_content for each saved field
            let field_map: [(&str, Option<&str>, Option<&str>); 4] = [
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
            ];
            for (field_name, original, generated) in &field_map {
                if fields.contains(&field_name.to_string()) {
                    if let Some(gen) = generated {
                        let _ = insert_ai_generated_content(
                            pool,
                            question_id,
                            None,
                            field_name,
                            *original,
                            gen,
                            &enriched.provider_used,
                            &enriched.model_used,
                            true,
                            Some(&admin_email),
                        )
                        .await;
                    }
                }
            }
        }

        save_ok
    } else {
        false
    };

    // ── 10. Build response ────────────────────────────────────────────────
    let total_tokens = enriched.prompt_tokens + enriched.completion_tokens;
    HttpResponse::Ok().json(EnrichResponse {
        question_id,
        original: FieldValues {
            solution: soal.solution.clone(),
            tag: soal.tag.clone(),
            modul: soal.modul.clone(),
            pelajaran: soal.pelajaran.clone(),
        },
        enriched: FieldValues {
            solution: enriched.solution,
            tag: enriched.tag,
            modul: enriched.modul,
            pelajaran: enriched.pelajaran,
        },
        saved,
        provider_used: enriched.provider_used,
        model_used: enriched.model_used,
        tokens_used: TokensUsed {
            prompt: enriched.prompt_tokens,
            completion: enriched.completion_tokens,
            total: total_tokens,
        },
    })
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
    let fields_json =
        serde_json::to_string(fields).unwrap_or_else(|_| "[]".to_string());

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
    q = q.bind(question_id);

    q.execute(pool).await.map(|_| ())
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
