//! Read endpoints for the soal analytics dashboard (Phase 1: coverage +
//! tag-variant drift), backed by the `soal_analytics_*` summary tables
//! computed daily by soal_analytics_service. Recompute is also exposed
//! on-demand here since the aggregation itself is cheap (a few GROUP BYs
//! over ~54k rows, seconds not minutes) -- no async-job pattern needed.

use actix_web::{get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::middleware::admin_middleware::AdminMiddleware;
use crate::service::soal_analytics_service::compute_and_store_summary;
use crate::AppState;

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

fn err(code: &str, msg: impl Into<String>) -> ErrorResponse {
    ErrorResponse { error: code.to_string(), message: msg.into() }
}

#[derive(sqlx::FromRow, Serialize)]
struct CoverageItem {
    level: String,
    node_id: String,
    node_name: String,
    parent_path: Option<String>,
    track_id: Option<String>,
    category_id: Option<String>,
    subcategory_id: Option<String>,
    total_count: i32,
    easy_count: i32,
    medium_count: i32,
    hard_count: i32,
    active_count: i32,
    draft_count: i32,
    archived_count: i32,
}

#[derive(Deserialize)]
struct CoverageQuery {
    /// "track" | "category" | "subcategory" | "topic" -- defaults to "subcategory"
    level: Option<String>,
    subcategory_id: Option<String>,
    category_id: Option<String>,
    track_id: Option<String>,
    /// Only rows with total_count <= this. Useful for "which topics need more soal".
    max_count: Option<i32>,
}

/// GET /admin/soal-analytics/coverage
#[get("/coverage")]
async fn get_coverage(query: web::Query<CoverageQuery>, data: web::Data<AppState<'_>>) -> impl Responder {
    let pool = &*data.context.soal.pool;
    let level = query.level.clone().unwrap_or_else(|| "subcategory".to_string());

    let mut sql = String::from(
        "SELECT level, node_id, node_name, parent_path, track_id, category_id, subcategory_id, \
         total_count, easy_count, medium_count, hard_count, active_count, draft_count, archived_count \
         FROM dbquizapp.soal_analytics_coverage WHERE level = ?",
    );
    let mut binds: Vec<String> = vec![level.clone()];

    if let Some(ref v) = query.subcategory_id {
        sql.push_str(" AND subcategory_id = ?");
        binds.push(v.clone());
    }
    if let Some(ref v) = query.category_id {
        sql.push_str(" AND category_id = ?");
        binds.push(v.clone());
    }
    if let Some(ref v) = query.track_id {
        sql.push_str(" AND track_id = ?");
        binds.push(v.clone());
    }
    if let Some(max) = query.max_count {
        sql.push_str(&format!(" AND total_count <= {}", max.max(0)));
    }
    sql.push_str(" ORDER BY total_count ASC, node_name ASC");

    let mut q = sqlx::query_as::<_, CoverageItem>(&sql);
    for b in &binds {
        q = q.bind(b);
    }

    match q.fetch_all(pool).await {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => {
            eprintln!("[admin_soal_analytics_controller] coverage query failed: {:?}", e);
            HttpResponse::InternalServerError().json(err("db_error", "Failed to fetch coverage."))
        }
    }
}

#[derive(sqlx::FromRow, Serialize)]
struct TagVariantRow {
    normalized_tag: String,
    variant_text: String,
    soal_count: i32,
}

#[derive(Serialize)]
struct TagVariantGroup {
    normalized_tag: String,
    total_soal: i32,
    variants: Vec<TagVariantDetail>,
}

#[derive(Serialize)]
struct TagVariantDetail {
    variant_text: String,
    soal_count: i32,
}

/// GET /admin/soal-analytics/tag-variants -- only normalized tags backed by
/// more than one raw variant (the actual cleanup candidates).
#[get("/tag-variants")]
async fn get_tag_variants(data: web::Data<AppState<'_>>) -> impl Responder {
    let pool = &*data.context.soal.pool;
    let rows = match sqlx::query_as::<_, TagVariantRow>(
        "SELECT normalized_tag, variant_text, soal_count FROM dbquizapp.soal_analytics_tag_variants ORDER BY normalized_tag",
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[admin_soal_analytics_controller] tag-variants query failed: {:?}", e);
            return HttpResponse::InternalServerError().json(err("db_error", "Failed to fetch tag variants."));
        }
    };

    let mut groups: Vec<TagVariantGroup> = Vec::new();
    let mut current: Option<TagVariantGroup> = None;
    for row in rows {
        match &mut current {
            Some(g) if g.normalized_tag == row.normalized_tag => {
                g.total_soal += row.soal_count;
                g.variants.push(TagVariantDetail { variant_text: row.variant_text, soal_count: row.soal_count });
            }
            _ => {
                if let Some(g) = current.take() {
                    if g.variants.len() > 1 {
                        groups.push(g);
                    }
                }
                current = Some(TagVariantGroup {
                    normalized_tag: row.normalized_tag,
                    total_soal: row.soal_count,
                    variants: vec![TagVariantDetail { variant_text: row.variant_text, soal_count: row.soal_count }],
                });
            }
        }
    }
    if let Some(g) = current {
        if g.variants.len() > 1 {
            groups.push(g);
        }
    }
    // Biggest cleanup opportunities first
    groups.sort_by(|a, b| b.total_soal.cmp(&a.total_soal));

    HttpResponse::Ok().json(groups)
}

#[derive(sqlx::FromRow, Serialize)]
struct SummaryMeta {
    last_computed_at: Option<chrono::DateTime<chrono::Utc>>,
    total_soal: i32,
    total_uncategorized: i32,
    total_distinct_tags: i32,
}

/// GET /admin/soal-analytics/summary
#[get("/summary")]
async fn get_summary(data: web::Data<AppState<'_>>) -> impl Responder {
    let pool = &*data.context.soal.pool;
    match sqlx::query_as::<_, SummaryMeta>(
        "SELECT last_computed_at, total_soal, total_uncategorized, total_distinct_tags FROM dbquizapp.soal_analytics_meta WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    {
        Ok(Some(meta)) => HttpResponse::Ok().json(meta),
        Ok(None) => HttpResponse::Ok().json(SummaryMeta {
            last_computed_at: None,
            total_soal: 0,
            total_uncategorized: 0,
            total_distinct_tags: 0,
        }),
        Err(e) => {
            eprintln!("[admin_soal_analytics_controller] summary query failed: {:?}", e);
            HttpResponse::InternalServerError().json(err("db_error", "Failed to fetch summary."))
        }
    }
}

/// POST /admin/soal-analytics/recompute -- synchronous, the aggregation
/// itself only takes a few seconds (GROUP BY over ~54k rows), no job/poll
/// pattern needed unlike the AI-backed endpoints elsewhere in this codebase.
#[post("/recompute")]
async fn recompute(data: web::Data<AppState<'_>>) -> impl Responder {
    compute_and_store_summary(&data.context.soal.pool).await;
    HttpResponse::Ok().json(serde_json::json!({"status": "completed"}))
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/soal-analytics")
            .wrap(AdminMiddleware::new())
            .service(get_coverage)
            .service(get_tag_variants)
            .service(get_summary)
            .service(recompute),
    );
}
