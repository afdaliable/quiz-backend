//! Soal drawn from a category's taxonomy pool rather than from a paket.
//! See `dao::soal_pool_dao` for what "pool" means.
//!
//! Both endpoints return answer keys, same as /paket-soal-response does today;
//! callers (the UPKP app's server routes) strip them before reaching a browser.
//! Access is open to any logged-in user -- the pool is free for now. To gate
//! it behind a subscription later, check user_subscriptions once at the top of
//! both handlers.

use std::collections::HashSet;

use actix_web::{post, web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;

use crate::dao::soal_pool_dao::{fetch_in_pool, sample_ids};
use crate::middleware::auth_middleware::AuthenticatedUser;
use crate::AppState;

const MAX_QUESTIONS: u32 = 100;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(sample_pool).service(pool_by_ids);
}

#[derive(Debug, Deserialize)]
pub struct Composition {
    pub subcategory_slug: Option<String>,
    pub topic_name: Option<String>,
    pub count: u32,
}

#[derive(Debug, Deserialize)]
pub struct SampleRequest {
    pub category_slug: String,
    pub compositions: Vec<Composition>,
}

/// Random soal per composition entry, deduplicated across entries (a soal
/// mapped to two subcategories is never served twice). An entry with fewer
/// servable soal than requested contributes what it has; `shortfall` reports
/// the gap so the caller can decide whether that's acceptable.
#[post("/soal-pool/sample")]
async fn sample_pool(
    state: web::Data<AppState<'_>>,
    _user: AuthenticatedUser,
    req: web::Json<SampleRequest>,
) -> impl Responder {
    let total: u32 = req.compositions.iter().map(|c| c.count).sum();
    if req.category_slug.trim().is_empty() || total == 0 || total > MAX_QUESTIONS {
        return HttpResponse::BadRequest().json(json!({
            "error": format!("category_slug is required and the counts must add up to 1-{MAX_QUESTIONS}")
        }));
    }
    // Write pool, not the read replica: pool membership changes as admins map
    // and review soal, and a lagging replica would serve stale (or removed) soal.
    let pool = &*state.context.soal.pool;

    // Fetch every entry concurrently, each over-fetched by the whole request
    // size so dropping soal already picked by an earlier entry still fills it.
    let fetches = req.compositions.iter().map(|comp| {
        sample_ids(
            pool,
            &req.category_slug,
            comp.subcategory_slug.as_deref(),
            comp.topic_name.as_deref(),
            comp.count + total,
        )
    });
    let results = futures::future::join_all(fetches).await;

    let mut seen: HashSet<i32> = HashSet::new();
    let mut ids: Vec<i32> = Vec::with_capacity(total as usize);
    let mut shortfall = 0u32;
    for (comp, result) in req.compositions.iter().zip(results) {
        let candidates = match result {
            Ok(c) => c,
            Err(e) => {
                eprintln!("soal-pool sample failed: {e:?}");
                return HttpResponse::InternalServerError().json(json!({"error": "Failed to sample soal pool"}));
            }
        };
        let before = ids.len();
        for id in candidates {
            if ids.len() - before == comp.count as usize {
                break;
            }
            if seen.insert(id) {
                ids.push(id);
            }
        }
        shortfall += comp.count - (ids.len() - before) as u32;
    }

    match fetch_in_pool(pool, &req.category_slug, &ids).await {
        Ok(questions) => HttpResponse::Ok().json(json!({"questions": questions, "shortfall": shortfall})),
        Err(e) => {
            eprintln!("soal-pool fetch failed: {e:?}");
            HttpResponse::InternalServerError().json(json!({"error": "Failed to load soal"}))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ByIdsRequest {
    pub category_slug: String,
    pub ids: Vec<i32>,
}

/// Re-reads soal by id for grading. Only ids still in the category's pool
/// come back, so a soal pulled from the pool between showing and grading
/// simply drops out of the attempt.
#[post("/soal-pool/by-ids")]
async fn pool_by_ids(
    state: web::Data<AppState<'_>>,
    _user: AuthenticatedUser,
    req: web::Json<ByIdsRequest>,
) -> impl Responder {
    if req.category_slug.trim().is_empty() || req.ids.is_empty() || req.ids.len() > MAX_QUESTIONS as usize {
        return HttpResponse::BadRequest().json(json!({
            "error": format!("category_slug is required and ids must hold 1-{MAX_QUESTIONS} entries")
        }));
    }
    match fetch_in_pool(&*state.context.soal.pool, &req.category_slug, &req.ids).await {
        Ok(questions) => HttpResponse::Ok().json(json!({"questions": questions})),
        Err(e) => {
            eprintln!("soal-pool by-ids failed: {e:?}");
            HttpResponse::InternalServerError().json(json!({"error": "Failed to load soal"}))
        }
    }
}
