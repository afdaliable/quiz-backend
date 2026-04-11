use crate::controller::log_request;
use crate::middleware::admin_middleware::AdminMiddleware;
use crate::model::admin_alert::{AdminAlert, AlertsQuery, PaginatedAlertsResponse};
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest, get, put};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/alerts")
            .wrap(AdminMiddleware::new())
            .service(get_alerts)
            .service(mark_alert_read)
            .service(mark_all_read)
    );
}

/// List admin alerts with pagination and optional filters
#[get("")]
async fn get_alerts(
    query: web::Query<AlertsQuery>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/alerts", &data.connections);

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;

    let mut conditions = vec!["1=1".to_string()];
    let mut binds: Vec<String> = vec![];

    match query.status.as_deref() {
        Some("read") => {
            conditions.push("is_read = TRUE".to_string());
        }
        Some("unread") => {
            conditions.push("is_read = FALSE".to_string());
        }
        _ => {}
    }

    if let Some(ref t) = query.alert_type {
        conditions.push("type = ?".to_string());
        binds.push(t.clone());
    }

    let where_clause = conditions.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) FROM admin_alerts WHERE {}", where_clause);
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for b in &binds {
        count_q = count_q.bind(b);
    }

    let total: i64 = match count_q.fetch_one(&*data.context.soal.pool).await {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Error counting alerts: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to count alerts".to_string(),
            });
        }
    };

    let list_sql = format!(
        "SELECT id, type, entity_type, entity_id, detail, is_read, created_at \
         FROM admin_alerts WHERE {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let mut list_q = sqlx::query_as::<_, AdminAlert>(&list_sql);
    for b in &binds {
        list_q = list_q.bind(b);
    }
    list_q = list_q.bind(limit).bind(offset);

    let alerts = match list_q.fetch_all(&*data.context.soal.pool).await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error fetching alerts: {:?}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch alerts".to_string(),
            });
        }
    };

    let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;

    HttpResponse::Ok().json(PaginatedAlertsResponse {
        alerts,
        total,
        page,
        limit,
        total_pages,
    })
}

/// Mark a single alert as read
#[put("/{id}/read")]
async fn mark_alert_read(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/alerts/{id}/read", &data.connections);

    let alert_id = path.into_inner();

    match sqlx::query("UPDATE admin_alerts SET is_read = TRUE WHERE id = ?")
        .bind(&alert_id)
        .execute(&*data.context.soal.pool)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Alert marked as read"
        })),
        Ok(_) => HttpResponse::NotFound().json(ErrorResponse {
            error: "Alert not found".to_string(),
        }),
        Err(e) => {
            eprintln!("Error marking alert read: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to mark alert as read".to_string(),
            })
        }
    }
}

/// Mark all unread alerts as read (bulk)
#[put("/read-all")]
async fn mark_all_read(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/admin/alerts/read-all", &data.connections);

    match sqlx::query("UPDATE admin_alerts SET is_read = TRUE WHERE is_read = FALSE")
        .execute(&*data.context.soal.pool)
        .await
    {
        Ok(r) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "marked_read": r.rows_affected()
        })),
        Err(e) => {
            eprintln!("Error marking all alerts read: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to mark all alerts as read".to_string(),
            })
        }
    }
}
