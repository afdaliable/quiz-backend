//! Admin-curated reference text ("materi library") scoped to a subcategory
//! (required) and optionally a narrower topic. Ground truth for AI enrich:
//! without a source, the AI answers exam questions from its own general
//! knowledge, which is generic and misses source-specific wording (see
//! `get_materi_context_for_soal`, used by ai_controller when building the
//! enrich prompt).

use crate::middleware::admin_middleware::{AdminMiddleware, AuthentikClaims};
use crate::model::materi_library::{CreateMateriLibraryRequest, MateriLibrary, UpdateMateriLibraryRequest};
use crate::AppState;
use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse, Responder};
use actix_web::HttpMessage;
use serde::Serialize;
use sqlx::MySqlPool;
use uuid::Uuid;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

fn err(msg: impl Into<String>) -> ErrorResponse {
    ErrorResponse { error: msg.into() }
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

/// Looks up reference materi for a soal's taxonomy placement -- topic-level
/// entries first (more specific), falling back to subcategory-level
/// entries (topic_id IS NULL). Concatenates matches (title + content),
/// capped so one wildly long reference doc can't blow the enrich prompt's
/// token budget.
const MAX_MATERI_CONTEXT_CHARS: usize = 6000;

pub async fn get_materi_context_for_soal(
    pool: &MySqlPool,
    subcategory_id: Option<&str>,
    topic_id: Option<&str>,
) -> Option<String> {
    let subcategory_id = subcategory_id?;

    let rows: Vec<(String, String)> = if let Some(tid) = topic_id {
        sqlx::query_as(
            "SELECT title, content FROM dbquizapp.materi_library WHERE topic_id = ? ORDER BY updated_at DESC",
        )
        .bind(tid)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    let rows = if rows.is_empty() {
        sqlx::query_as(
            "SELECT title, content FROM dbquizapp.materi_library WHERE subcategory_id = ? AND topic_id IS NULL ORDER BY updated_at DESC",
        )
        .bind(subcategory_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        rows
    };

    if rows.is_empty() {
        return None;
    }

    let mut combined = String::new();
    for (title, content) in rows {
        if combined.len() >= MAX_MATERI_CONTEXT_CHARS {
            break;
        }
        combined.push_str(&format!("### {}\n{}\n\n", title, content));
    }
    combined.truncate(MAX_MATERI_CONTEXT_CHARS);
    Some(combined)
}

/// GET /admin/materi-library?subcategory_id=&topic_id=
#[get("")]
async fn list_materi(
    query: web::Query<std::collections::HashMap<String, String>>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let pool = &*data.context.soal.pool;
    let subcategory_id = query.get("subcategory_id").cloned();
    let topic_id = query.get("topic_id").cloned();

    let mut sql = "SELECT * FROM dbquizapp.materi_library WHERE 1=1".to_string();
    let mut binds: Vec<String> = Vec::new();
    if let Some(sc) = &subcategory_id {
        sql.push_str(" AND subcategory_id = ?");
        binds.push(sc.clone());
    }
    if let Some(tp) = &topic_id {
        sql.push_str(" AND topic_id = ?");
        binds.push(tp.clone());
    }
    sql.push_str(" ORDER BY updated_at DESC");

    let mut q = sqlx::query_as::<_, MateriLibrary>(&sql);
    for b in &binds {
        q = q.bind(b);
    }

    match q.fetch_all(pool).await {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            eprintln!("[materi_library_controller] list error: {:?}", e);
            HttpResponse::InternalServerError().json(err("Failed to list materi library"))
        }
    }
}

/// GET /admin/materi-library/{id}
#[get("/{id}")]
async fn get_materi(path: web::Path<String>, data: web::Data<AppState<'_>>) -> impl Responder {
    let id = path.into_inner();
    let pool = &*data.context.soal.pool;

    match sqlx::query_as::<_, MateriLibrary>("SELECT * FROM dbquizapp.materi_library WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(row)) => HttpResponse::Ok().json(row),
        Ok(None) => HttpResponse::NotFound().json(err(format!("Materi {} not found", id))),
        Err(e) => {
            eprintln!("[materi_library_controller] get error: {:?}", e);
            HttpResponse::InternalServerError().json(err("Failed to fetch materi"))
        }
    }
}

/// POST /admin/materi-library
#[post("")]
async fn create_materi(
    req: web::Json<CreateMateriLibraryRequest>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    if req.title.trim().is_empty() || req.content.trim().is_empty() {
        return HttpResponse::BadRequest().json(err("title dan content wajib diisi"));
    }
    let pool = &*data.context.soal.pool;
    let admin_email = extract_admin_email(&http_req);
    let id = Uuid::new_v4().to_string();

    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO dbquizapp.materi_library (id, subcategory_id, topic_id, title, content, source, created_by)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.subcategory_id)
    .bind(&req.topic_id)
    .bind(&req.title)
    .bind(&req.content)
    .bind(&req.source)
    .bind(&admin_email)
    .execute(pool)
    .await
    {
        eprintln!("[materi_library_controller] create error: {:?}", e);
        return HttpResponse::InternalServerError().json(err("Failed to create materi"));
    }

    match sqlx::query_as::<_, MateriLibrary>("SELECT * FROM dbquizapp.materi_library WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await
    {
        Ok(row) => HttpResponse::Created().json(row),
        Err(e) => {
            eprintln!("[materi_library_controller] re-fetch after create error: {:?}", e);
            HttpResponse::InternalServerError().json(err("Materi created but failed to fetch it back"))
        }
    }
}

/// PUT /admin/materi-library/{id}
#[put("/{id}")]
async fn update_materi(
    path: web::Path<String>,
    req: web::Json<UpdateMateriLibraryRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let id = path.into_inner();
    if req.title.trim().is_empty() || req.content.trim().is_empty() {
        return HttpResponse::BadRequest().json(err("title dan content wajib diisi"));
    }
    let pool = &*data.context.soal.pool;

    let result = sqlx::query(
        r#"
        UPDATE dbquizapp.materi_library
        SET subcategory_id = ?, topic_id = ?, title = ?, content = ?, source = ?
        WHERE id = ?
        "#,
    )
    .bind(&req.subcategory_id)
    .bind(&req.topic_id)
    .bind(&req.title)
    .bind(&req.content)
    .bind(&req.source)
    .bind(&id)
    .execute(pool)
    .await;

    match result {
        Ok(res) if res.rows_affected() == 0 => {
            HttpResponse::NotFound().json(err(format!("Materi {} not found", id)))
        }
        Ok(_) => match sqlx::query_as::<_, MateriLibrary>("SELECT * FROM dbquizapp.materi_library WHERE id = ?")
            .bind(&id)
            .fetch_one(pool)
            .await
        {
            Ok(row) => HttpResponse::Ok().json(row),
            Err(e) => {
                eprintln!("[materi_library_controller] re-fetch after update error: {:?}", e);
                HttpResponse::InternalServerError().json(err("Materi updated but failed to fetch it back"))
            }
        },
        Err(e) => {
            eprintln!("[materi_library_controller] update error: {:?}", e);
            HttpResponse::InternalServerError().json(err("Failed to update materi"))
        }
    }
}

/// DELETE /admin/materi-library/{id}
#[delete("/{id}")]
async fn delete_materi(path: web::Path<String>, data: web::Data<AppState<'_>>) -> impl Responder {
    let id = path.into_inner();
    let pool = &*data.context.soal.pool;

    match sqlx::query("DELETE FROM dbquizapp.materi_library WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await
    {
        Ok(res) if res.rows_affected() > 0 => HttpResponse::NoContent().finish(),
        Ok(_) => HttpResponse::NotFound().json(err(format!("Materi {} not found", id))),
        Err(e) => {
            eprintln!("[materi_library_controller] delete error: {:?}", e);
            HttpResponse::InternalServerError().json(err("Failed to delete materi"))
        }
    }
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/materi-library")
            .wrap(AdminMiddleware::new())
            .service(list_materi)
            .service(create_materi)
            .service(get_materi)
            .service(update_materi)
            .service(delete_materi),
    );
}
