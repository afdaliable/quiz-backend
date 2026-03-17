use crate::controller::log_request;
use crate::model::bookmarked_questions::{
    BookmarkListResponse, BookmarkResponse, BookmarkedQuestion, BulkDeleteRequest,
    BulkDeleteResponse, CategoryCount, DeleteBookmarkResponse,
};
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use sqlx::FromRow;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, sqlx::FromRow)]
struct CountResult {
    count: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BookmarkListQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    /// Sort field: "created_at" (default) atau "pelajaran"
    pub sort_by: Option<String>,
    /// Sort direction: "desc" (default) atau "asc"
    pub sort_order: Option<String>,
    /// Filter by kategori/pelajaran
    pub category: Option<String>,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/bookmarks")
            // bulk harus didaftarkan sebelum /{question_id} agar tidak ter-capture
            .route("/bulk", web::delete().to(bulk_delete_bookmarks))
            .route("/{question_id}", web::post().to(bookmark_question))
            .route("/{question_id}", web::delete().to(unbookmark_question))
            .route("", web::get().to(get_user_bookmarks)),
    );
}

// ─── Helper ──────────────────────────────────────────────────────────────────

fn extract_user_id(http_req: &HttpRequest) -> Option<String> {
    http_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

// ─── POST /bookmarks/:question_id ────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/bookmarks/{question_id}",
    params(("question_id" = i32, Path, description = "ID soal")),
    responses(
        (status = 200, description = "Berhasil bookmark", body = BookmarkResponse),
        (status = 400, description = "Soal sudah di-bookmark"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Soal tidak ditemukan"),
        (status = 500, description = "Server error")
    ),
    tag = "Bookmarks",
    security(("bearer_auth" = []))
)]
pub async fn bookmark_question(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/bookmarks/{question_id} POST", &data.connections);
    let question_id = path.into_inner();

    let user_id = match extract_user_id(&http_req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse { error: "Unauthorized".into() }),
    };

    // Cek soal ada
    let exists: bool = sqlx::query_as::<_, CountResult>(
        "SELECT COUNT(*) as count FROM dbquizapp.soal WHERE id = ?",
    )
    .bind(question_id)
    .fetch_one(&*data.context.soal.pool)
    .await
    .map(|r| r.count > 0)
    .unwrap_or(false);

    if !exists {
        return HttpResponse::NotFound().json(ErrorResponse { error: "Soal tidak ditemukan".into() });
    }

    // Cek sudah di-bookmark
    let already: bool = sqlx::query_as::<_, CountResult>(
        "SELECT COUNT(*) as count FROM dbquizapp.bookmarked_questions WHERE user_id = ? AND question_id = ?",
    )
    .bind(&user_id)
    .bind(question_id)
    .fetch_one(&*data.context.soal.pool)
    .await
    .map(|r| r.count > 0)
    .unwrap_or(false);

    if already {
        return HttpResponse::BadRequest().json(ErrorResponse { error: "Soal sudah di-bookmark".into() });
    }

    let bookmark_id = Uuid::new_v4().to_string();

    match sqlx::query(
        "INSERT INTO dbquizapp.bookmarked_questions (id, user_id, question_id) VALUES (?, ?, ?)",
    )
    .bind(&bookmark_id)
    .bind(&user_id)
    .bind(question_id)
    .execute(&*data.context.soal.pool)
    .await
    {
        Ok(_) => {
            let bookmark_count = sqlx::query_as::<_, CountResult>(
                "SELECT COUNT(*) as count FROM dbquizapp.bookmarked_questions WHERE user_id = ?",
            )
            .bind(&user_id)
            .fetch_one(&*data.context.soal.pool)
            .await
            .map(|r| r.count as i32)
            .unwrap_or(0);

            HttpResponse::Ok().json(BookmarkResponse {
                success: true,
                message: "Soal berhasil di-bookmark".into(),
                bookmark_count,
            })
        }
        Err(e) => {
            eprintln!("Error bookmark: {e}");
            HttpResponse::InternalServerError().json(ErrorResponse { error: "Gagal bookmark soal".into() })
        }
    }
}

// ─── DELETE /bookmarks/:question_id ──────────────────────────────────────────

#[utoipa::path(
    delete,
    path = "/api/bookmarks/{question_id}",
    params(("question_id" = i32, Path, description = "ID soal")),
    responses(
        (status = 200, description = "Bookmark dihapus", body = DeleteBookmarkResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Bookmark tidak ditemukan"),
        (status = 500, description = "Server error")
    ),
    tag = "Bookmarks",
    security(("bearer_auth" = []))
)]
pub async fn unbookmark_question(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/bookmarks/{question_id} DELETE", &data.connections);
    let question_id = path.into_inner();

    let user_id = match extract_user_id(&http_req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse { error: "Unauthorized".into() }),
    };

    match sqlx::query(
        "DELETE FROM dbquizapp.bookmarked_questions WHERE user_id = ? AND question_id = ?",
    )
    .bind(&user_id)
    .bind(question_id)
    .execute(&*data.context.soal.pool)
    .await
    {
        Ok(result) => {
            if result.rows_affected() == 0 {
                return HttpResponse::NotFound().json(ErrorResponse {
                    error: "Bookmark tidak ditemukan".into(),
                });
            }

            let bookmark_count = sqlx::query_as::<_, CountResult>(
                "SELECT COUNT(*) as count FROM dbquizapp.bookmarked_questions WHERE user_id = ?",
            )
            .bind(&user_id)
            .fetch_one(&*data.context.soal.pool)
            .await
            .map(|r| r.count as i32)
            .unwrap_or(0);

            HttpResponse::Ok().json(DeleteBookmarkResponse {
                success: true,
                deleted_question_id: question_id,
                bookmark_count,
            })
        }
        Err(e) => {
            eprintln!("Error unbookmark: {e}");
            HttpResponse::InternalServerError().json(ErrorResponse { error: "Gagal hapus bookmark".into() })
        }
    }
}

// ─── DELETE /bookmarks/bulk ───────────────────────────────────────────────────

#[utoipa::path(
    delete,
    path = "/api/bookmarks/bulk",
    request_body = BulkDeleteRequest,
    responses(
        (status = 200, description = "Bulk bookmark dihapus", body = BulkDeleteResponse),
        (status = 400, description = "question_ids kosong"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "Bookmarks",
    security(("bearer_auth" = []))
)]
pub async fn bulk_delete_bookmarks(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
    body: web::Json<BulkDeleteRequest>,
) -> impl Responder {
    log_request("/bookmarks/bulk DELETE", &data.connections);

    let user_id = match extract_user_id(&http_req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse { error: "Unauthorized".into() }),
    };

    if body.question_ids.is_empty() {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "question_ids tidak boleh kosong".into(),
        });
    }

    let placeholders = body.question_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "DELETE FROM dbquizapp.bookmarked_questions WHERE user_id = ? AND question_id IN ({placeholders})"
    );

    let mut q = sqlx::query(&sql).bind(&user_id);
    for id in &body.question_ids {
        q = q.bind(id);
    }

    match q.execute(&*data.context.soal.pool).await {
        Ok(result) => {
            let deleted_count = result.rows_affected();

            let bookmark_count = sqlx::query_as::<_, CountResult>(
                "SELECT COUNT(*) as count FROM dbquizapp.bookmarked_questions WHERE user_id = ?",
            )
            .bind(&user_id)
            .fetch_one(&*data.context.soal.pool)
            .await
            .map(|r| r.count as i32)
            .unwrap_or(0);

            HttpResponse::Ok().json(BulkDeleteResponse {
                success: true,
                deleted_count,
                bookmark_count,
            })
        }
        Err(e) => {
            eprintln!("Error bulk delete bookmark: {e}");
            HttpResponse::InternalServerError().json(ErrorResponse { error: "Gagal bulk delete bookmark".into() })
        }
    }
}

// ─── GET /bookmarks ───────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/bookmarks",
    params(
        ("page" = Option<u32>, Query, description = "Halaman (default: 1)"),
        ("limit" = Option<u32>, Query, description = "Item per halaman (default: 20, max: 100)"),
        ("sort_by" = Option<String>, Query, description = "Field sort: created_at (default) | pelajaran"),
        ("sort_order" = Option<String>, Query, description = "Arah sort: desc (default) | asc"),
        ("category" = Option<String>, Query, description = "Filter by kategori/pelajaran"),
    ),
    responses(
        (status = 200, description = "List bookmark", body = BookmarkListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Server error")
    ),
    tag = "Bookmarks",
    security(("bearer_auth" = []))
)]
pub async fn get_user_bookmarks(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
    query: web::Query<BookmarkListQuery>,
) -> impl Responder {
    log_request("/bookmarks GET", &data.connections);

    let user_id = match extract_user_id(&http_req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(ErrorResponse { error: "Unauthorized".into() }),
    };

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;

    // Validasi sort field & direction terhadap whitelist (cegah SQL injection)
    let sort_col = match query.sort_by.as_deref() {
        Some("pelajaran") => "s.pelajaran",
        _ => "bq.created_at",
    };
    let sort_dir = match query.sort_order.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };

    // ── Total count (dengan filter opsional) ──
    let (total, category_filter_sql) = if let Some(cat) = &query.category {
        let total = sqlx::query_as::<_, CountResult>(
            "SELECT COUNT(*) as count
             FROM dbquizapp.bookmarked_questions bq
             JOIN dbquizapp.soal s ON bq.question_id = s.id
             WHERE bq.user_id = ? AND s.pelajaran = ?",
        )
        .bind(&user_id)
        .bind(cat)
        .fetch_one(&*data.context.soal.pool)
        .await
        .map(|r| r.count)
        .unwrap_or(0);

        (total, format!("AND s.pelajaran = '{}'", cat.replace('\'', "\\'")))
    } else {
        let total = sqlx::query_as::<_, CountResult>(
            "SELECT COUNT(*) as count FROM dbquizapp.bookmarked_questions WHERE user_id = ?",
        )
        .bind(&user_id)
        .fetch_one(&*data.context.soal.pool)
        .await
        .map(|r| r.count)
        .unwrap_or(0);

        (total, String::new())
    };

    let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;

    // ── Main query dengan quiz_name & question_number ──
    let main_sql = format!(
        "SELECT
            bq.id,
            bq.question_id,
            s.soal,
            s.opt1, s.opt2, s.opt3, s.opt4, s.opt5,
            s.correct_answer,
            s.solution,
            s.modul,
            s.pelajaran,
            s.tag,
            s.question_type,
            bq.created_at AS bookmarked_at,
            COALESCE(ps.nama_paket_soal, '') AS quiz_name,
            COALESCE(
                (SELECT COUNT(*)
                 FROM dbquizapp.paket_soal_items psi2
                 WHERE psi2.paket_soal_id = psi.paket_soal_id
                   AND psi2.id <= psi.item_id),
                0
            ) AS question_number
        FROM dbquizapp.bookmarked_questions bq
        JOIN dbquizapp.soal s ON bq.question_id = s.id
        LEFT JOIN (
            SELECT soal_id, MIN(id) AS item_id, MIN(paket_soal_id) AS paket_soal_id
            FROM dbquizapp.paket_soal_items
            GROUP BY soal_id
        ) psi ON psi.soal_id = s.id
        LEFT JOIN dbquizapp.paket_soal ps ON ps.id = psi.paket_soal_id
        WHERE bq.user_id = ? {category_filter_sql}
        ORDER BY {sort_col} {sort_dir}
        LIMIT ? OFFSET ?"
    );

    let rows = sqlx::query(&main_sql)
        .bind(&user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*data.context.soal.pool)
        .await;

    let bookmarks = match rows {
        Ok(rows) => rows
            .iter()
            .map(|row| BookmarkedQuestion::from_row(row).unwrap())
            .collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("Error fetch bookmarks: {e}");
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Gagal mengambil data bookmark".into(),
            });
        }
    };

    // ── Count per kategori ──
    let categories = sqlx::query_as::<_, CategoryCount>(
        "SELECT COALESCE(s.pelajaran, 'Lainnya') AS category, COUNT(*) AS count
         FROM dbquizapp.bookmarked_questions bq
         JOIN dbquizapp.soal s ON bq.question_id = s.id
         WHERE bq.user_id = ?
         GROUP BY s.pelajaran
         ORDER BY count DESC",
    )
    .bind(&user_id)
    .fetch_all(&*data.context.soal.pool)
    .await
    .unwrap_or_default();

    HttpResponse::Ok().json(BookmarkListResponse {
        bookmarks,
        total,
        page,
        limit,
        total_pages,
        categories,
    })
}
