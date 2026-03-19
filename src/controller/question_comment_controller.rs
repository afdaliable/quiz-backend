use crate::controller::log_request;
use crate::model::question_comment::{
    CommentListQuery, CommentListResponse, CommentResponse, CommentUser,
    CreateCommentRequest, PinCommentRequest, ToggleUpvoteResponse,
};
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct CountResult {
    count: i64,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

fn extract_user_id(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/questions")
            .route(
                "/{question_id}/comments",
                web::get().to(list_comments),
            )
            .route(
                "/{question_id}/comments",
                web::post().to(create_comment),
            )
            .route(
                "/{question_id}/comments/{comment_id}/replies",
                web::get().to(list_replies),
            ),
    );
    cfg.service(
        web::scope("/comments")
            .route("/{comment_id}/upvote", web::post().to(toggle_upvote))
            .route("/{comment_id}/pin", web::patch().to(pin_comment)),
    );
}

// ─── GET /questions/{question_id}/comments ────────────────────────────────────

async fn list_comments(
    path: web::Path<i32>,
    query: web::Query<CommentListQuery>,
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("GET /questions/{id}/comments", &data.connections);
    let question_id = path.into_inner();
    let viewer_id = extract_user_id(&req).unwrap_or_default();

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;

    // Total count of top-level comments
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dbquizapp.question_comments
         WHERE question_id = ? AND parent_id IS NULL",
    )
    .bind(question_id)
    .fetch_one(&*data.context.soal.pool)
    .await
    .unwrap_or(0);

    #[derive(sqlx::FromRow)]
    struct CommentRow {
        id: String,
        user_id: String,
        display_name: String,
        picture_url: Option<String>,
        body: String,
        is_admin_pin: bool,
        upvotes: i32,
        has_upvoted: i64,
        reply_count: i64,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let rows = sqlx::query_as::<_, CommentRow>(
        r#"
        SELECT
            qc.id,
            qc.user_id,
            u.display_name,
            u.picture_url,
            qc.body,
            qc.is_admin_pin,
            qc.upvotes,
            COUNT(DISTINCT cu.user_id) AS has_upvoted,
            (SELECT COUNT(*) FROM dbquizapp.question_comments r WHERE r.parent_id = qc.id) AS reply_count,
            qc.created_at
        FROM dbquizapp.question_comments qc
        JOIN dbquizapp.users u ON u.id = qc.user_id
        LEFT JOIN dbquizapp.comment_upvotes cu ON cu.comment_id = qc.id AND cu.user_id = ?
        WHERE qc.question_id = ? AND qc.parent_id IS NULL
        GROUP BY qc.id, qc.user_id, u.display_name, u.picture_url,
                 qc.body, qc.is_admin_pin, qc.upvotes, qc.created_at
        ORDER BY qc.is_admin_pin DESC, qc.created_at ASC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(&viewer_id)
    .bind(question_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&*data.context.soal.pool)
    .await;

    match rows {
        Ok(rows) => {
            let data_vec: Vec<CommentResponse> = rows
                .into_iter()
                .map(|r| CommentResponse {
                    id: r.id,
                    user: CommentUser {
                        id: r.user_id,
                        display_name: r.display_name,
                        picture_url: r.picture_url,
                    },
                    body: r.body,
                    is_admin_pin: r.is_admin_pin,
                    upvotes: r.upvotes,
                    has_upvoted: r.has_upvoted > 0,
                    reply_count: r.reply_count,
                    created_at: r.created_at,
                })
                .collect();

            HttpResponse::Ok().json(CommentListResponse {
                data: data_vec,
                total,
                page,
                limit,
            })
        }
        Err(e) => {
            eprintln!("Error list_comments: {e}");
            HttpResponse::InternalServerError()
                .json(ErrorResponse { error: "Gagal mengambil komentar".into() })
        }
    }
}

// ─── GET /questions/{question_id}/comments/{comment_id}/replies ───────────────

async fn list_replies(
    path: web::Path<(i32, String)>,
    query: web::Query<CommentListQuery>,
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("GET /questions/{id}/comments/{id}/replies", &data.connections);
    let (question_id, comment_id) = path.into_inner();
    let viewer_id = extract_user_id(&req).unwrap_or_default();

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;

    // Verify the parent comment belongs to this question
    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dbquizapp.question_comments
         WHERE id = ? AND question_id = ? AND parent_id IS NULL",
    )
    .bind(&comment_id)
    .bind(question_id)
    .fetch_one(&*data.context.soal.pool)
    .await
    .unwrap_or(0);

    if exists == 0 {
        return HttpResponse::NotFound()
            .json(ErrorResponse { error: "Komentar tidak ditemukan".into() });
    }

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dbquizapp.question_comments WHERE parent_id = ?",
    )
    .bind(&comment_id)
    .fetch_one(&*data.context.soal.pool)
    .await
    .unwrap_or(0);

    #[derive(sqlx::FromRow)]
    struct ReplyRow {
        id: String,
        user_id: String,
        display_name: String,
        picture_url: Option<String>,
        body: String,
        is_admin_pin: bool,
        upvotes: i32,
        has_upvoted: i64,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let rows = sqlx::query_as::<_, ReplyRow>(
        r#"
        SELECT
            qc.id,
            qc.user_id,
            u.display_name,
            u.picture_url,
            qc.body,
            qc.is_admin_pin,
            qc.upvotes,
            COUNT(DISTINCT cu.user_id) AS has_upvoted,
            qc.created_at
        FROM dbquizapp.question_comments qc
        JOIN dbquizapp.users u ON u.id = qc.user_id
        LEFT JOIN dbquizapp.comment_upvotes cu ON cu.comment_id = qc.id AND cu.user_id = ?
        WHERE qc.parent_id = ?
        GROUP BY qc.id, qc.user_id, u.display_name, u.picture_url,
                 qc.body, qc.is_admin_pin, qc.upvotes, qc.created_at
        ORDER BY qc.created_at ASC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(&viewer_id)
    .bind(&comment_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&*data.context.soal.pool)
    .await;

    match rows {
        Ok(rows) => {
            let data_vec: Vec<CommentResponse> = rows
                .into_iter()
                .map(|r| CommentResponse {
                    id: r.id,
                    user: CommentUser {
                        id: r.user_id,
                        display_name: r.display_name,
                        picture_url: r.picture_url,
                    },
                    body: r.body,
                    is_admin_pin: r.is_admin_pin,
                    upvotes: r.upvotes,
                    has_upvoted: r.has_upvoted > 0,
                    reply_count: 0,
                    created_at: r.created_at,
                })
                .collect();

            HttpResponse::Ok().json(CommentListResponse {
                data: data_vec,
                total,
                page,
                limit,
            })
        }
        Err(e) => {
            eprintln!("Error list_replies: {e}");
            HttpResponse::InternalServerError()
                .json(ErrorResponse { error: "Gagal mengambil balasan".into() })
        }
    }
}

// ─── POST /questions/{question_id}/comments ───────────────────────────────────

async fn create_comment(
    path: web::Path<i32>,
    body: web::Json<CreateCommentRequest>,
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("POST /questions/{id}/comments", &data.connections);
    let question_id = path.into_inner();

    // Auth required
    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized()
                .json(ErrorResponse { error: "Login diperlukan untuk berkomentar".into() })
        }
    };

    // Validate body
    let trimmed = body.body.trim().to_string();
    if trimmed.len() < 3 {
        return HttpResponse::BadRequest()
            .json(ErrorResponse { error: "Komentar minimal 3 karakter".into() });
    }
    if trimmed.len() > 2000 {
        return HttpResponse::BadRequest()
            .json(ErrorResponse { error: "Komentar maksimal 2000 karakter".into() });
    }

    // Rate limiting: 5 komentar per menit per user
    if let Some(redis) = &data.redis_pool {
        let mut con = (*redis.rate_limits()).clone();
        let rate_key = format!("rate:comment:{}", user_id);
        let count: i64 = con.incr(&rate_key, 1_i64).await.unwrap_or(0);
        if count == 1 {
            let _: Result<(), _> = con.expire(&rate_key, 60_i64).await;
        }
        if count > 5 {
            return HttpResponse::TooManyRequests().json(ErrorResponse {
                error: "Terlalu banyak komentar. Coba lagi dalam 1 menit.".into(),
            });
        }
    }

    // Validate parent_id if provided
    if let Some(pid) = &body.parent_id {
        let parent_exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM dbquizapp.question_comments
             WHERE id = ? AND question_id = ? AND parent_id IS NULL",
        )
        .bind(pid)
        .bind(question_id)
        .fetch_one(&*data.context.soal.pool)
        .await
        .unwrap_or(0);

        if parent_exists == 0 {
            return HttpResponse::BadRequest()
                .json(ErrorResponse { error: "Komentar induk tidak ditemukan".into() });
        }
    }

    // Verify question exists
    let question_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dbquizapp.soal WHERE id = ?",
    )
    .bind(question_id)
    .fetch_one(&*data.context.soal.pool)
    .await
    .unwrap_or(0);

    if question_exists == 0 {
        return HttpResponse::NotFound()
            .json(ErrorResponse { error: "Soal tidak ditemukan".into() });
    }

    let comment_id = Uuid::new_v4().to_string();
    let parent_id: Option<&str> = body.parent_id.as_deref();

    let insert = sqlx::query(
        "INSERT INTO dbquizapp.question_comments (id, question_id, user_id, parent_id, body)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&comment_id)
    .bind(question_id)
    .bind(&user_id)
    .bind(parent_id)
    .bind(&trimmed)
    .execute(&*data.context.soal.pool)
    .await;

    match insert {
        Ok(_) => {
            // Fetch the created comment with user info
            #[derive(sqlx::FromRow)]
            struct CreatedRow {
                id: String,
                user_id: String,
                display_name: String,
                picture_url: Option<String>,
                body: String,
                is_admin_pin: bool,
                upvotes: i32,
                created_at: chrono::DateTime<chrono::Utc>,
            }

            let created = sqlx::query_as::<_, CreatedRow>(
                r#"
                SELECT qc.id, qc.user_id, u.display_name, u.picture_url,
                       qc.body, qc.is_admin_pin, qc.upvotes, qc.created_at
                FROM dbquizapp.question_comments qc
                JOIN dbquizapp.users u ON u.id = qc.user_id
                WHERE qc.id = ?
                "#,
            )
            .bind(&comment_id)
            .fetch_one(&*data.context.soal.pool)
            .await;

            match created {
                Ok(r) => HttpResponse::Created().json(CommentResponse {
                    id: r.id,
                    user: CommentUser {
                        id: r.user_id,
                        display_name: r.display_name,
                        picture_url: r.picture_url,
                    },
                    body: r.body,
                    is_admin_pin: r.is_admin_pin,
                    upvotes: r.upvotes,
                    has_upvoted: false,
                    reply_count: 0,
                    created_at: r.created_at,
                }),
                Err(e) => {
                    eprintln!("Error fetch created comment: {e}");
                    HttpResponse::InternalServerError()
                        .json(ErrorResponse { error: "Komentar dibuat tapi gagal diambil".into() })
                }
            }
        }
        Err(e) => {
            eprintln!("Error create_comment: {e}");
            HttpResponse::InternalServerError()
                .json(ErrorResponse { error: "Gagal membuat komentar".into() })
        }
    }
}

// ─── POST /comments/{comment_id}/upvote ──────────────────────────────────────

async fn toggle_upvote(
    path: web::Path<String>,
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("POST /comments/{id}/upvote", &data.connections);
    let comment_id = path.into_inner();

    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized()
                .json(ErrorResponse { error: "Login diperlukan".into() })
        }
    };

    // Check comment exists
    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dbquizapp.question_comments WHERE id = ?",
    )
    .bind(&comment_id)
    .fetch_one(&*data.context.soal.pool)
    .await
    .unwrap_or(0);

    if exists == 0 {
        return HttpResponse::NotFound()
            .json(ErrorResponse { error: "Komentar tidak ditemukan".into() });
    }

    // Check if already upvoted
    let already_upvoted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dbquizapp.comment_upvotes WHERE comment_id = ? AND user_id = ?",
    )
    .bind(&comment_id)
    .bind(&user_id)
    .fetch_one(&*data.context.soal.pool)
    .await
    .unwrap_or(0);

    let has_upvoted: bool;

    if already_upvoted > 0 {
        // Remove upvote
        let _ = sqlx::query(
            "DELETE FROM dbquizapp.comment_upvotes WHERE comment_id = ? AND user_id = ?",
        )
        .bind(&comment_id)
        .bind(&user_id)
        .execute(&*data.context.soal.pool)
        .await;

        let _ = sqlx::query(
            "UPDATE dbquizapp.question_comments SET upvotes = GREATEST(0, upvotes - 1) WHERE id = ?",
        )
        .bind(&comment_id)
        .execute(&*data.context.soal.pool)
        .await;

        has_upvoted = false;
    } else {
        // Add upvote
        let _ = sqlx::query(
            "INSERT IGNORE INTO dbquizapp.comment_upvotes (comment_id, user_id) VALUES (?, ?)",
        )
        .bind(&comment_id)
        .bind(&user_id)
        .execute(&*data.context.soal.pool)
        .await;

        let _ = sqlx::query(
            "UPDATE dbquizapp.question_comments SET upvotes = upvotes + 1 WHERE id = ?",
        )
        .bind(&comment_id)
        .execute(&*data.context.soal.pool)
        .await;

        has_upvoted = true;
    }

    // Fetch updated upvote count
    let upvotes: i32 = sqlx::query_scalar(
        "SELECT upvotes FROM dbquizapp.question_comments WHERE id = ?",
    )
    .bind(&comment_id)
    .fetch_one(&*data.context.soal.pool)
    .await
    .unwrap_or(0);

    HttpResponse::Ok().json(ToggleUpvoteResponse { upvotes, has_upvoted })
}

// ─── PATCH /comments/{comment_id}/pin ────────────────────────────────────────

async fn pin_comment(
    path: web::Path<String>,
    body: web::Json<PinCommentRequest>,
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
) -> impl Responder {
    log_request("PATCH /comments/{id}/pin", &data.connections);
    let comment_id = path.into_inner();

    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized()
                .json(ErrorResponse { error: "Login diperlukan".into() })
        }
    };

    // Check admin role
    let is_admin: bool = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dbquizapp.users
         WHERE id = ? AND role IN ('admin', 'superadmin') AND deleted_at IS NULL",
    )
    .bind(&user_id)
    .fetch_one(&*data.context.users.pool)
    .await
    .map(|c| c > 0)
    .unwrap_or(false);

    if !is_admin {
        return HttpResponse::Forbidden()
            .json(ErrorResponse { error: "Hanya admin yang dapat pin komentar".into() });
    }

    // Fetch comment to get question_id
    let question_id: Option<i32> = sqlx::query_scalar(
        "SELECT question_id FROM dbquizapp.question_comments WHERE id = ?",
    )
    .bind(&comment_id)
    .fetch_optional(&*data.context.soal.pool)
    .await
    .unwrap_or(None);

    let question_id = match question_id {
        Some(id) => id,
        None => {
            return HttpResponse::NotFound()
                .json(ErrorResponse { error: "Komentar tidak ditemukan".into() })
        }
    };

    if body.pin {
        // Unpin all other comments for this question first
        let _ = sqlx::query(
            "UPDATE dbquizapp.question_comments SET is_admin_pin = FALSE WHERE question_id = ?",
        )
        .bind(question_id)
        .execute(&*data.context.soal.pool)
        .await;
    }

    // Set pin status on the target comment
    let result = sqlx::query(
        "UPDATE dbquizapp.question_comments SET is_admin_pin = ? WHERE id = ?",
    )
    .bind(body.pin)
    .bind(&comment_id)
    .execute(&*data.context.soal.pool)
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "comment_id": comment_id,
            "is_admin_pin": body.pin,
        })),
        Err(e) => {
            eprintln!("Error pin_comment: {e}");
            HttpResponse::InternalServerError()
                .json(ErrorResponse { error: "Gagal mengubah status pin".into() })
        }
    }
}
