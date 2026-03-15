use crate::controller::log_request;
use crate::model::bookmarked_questions::{BookmarkResponse, BookmarkedQuestion};
use crate::AppState;
use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use sqlx::mysql::MySqlRow;
use sqlx::FromRow;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, sqlx::FromRow)]
struct CountResult {
    count: i64,
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/bookmarks")
            .route("/{question_id}", web::post().to(bookmark_question))
            .route("/{question_id}", web::delete().to(unbookmark_question))
            .route("", web::get().to(get_user_bookmarks))
    );
}

/// Bookmark a question for the authenticated user
#[utoipa::path(
    post,
    path = "/api/bookmarks/{question_id}",
    params(
        ("question_id" = i32, Path, description = "Question ID to bookmark")
    ),
    responses(
        (status = 200, description = "Question bookmarked successfully", body = BookmarkResponse),
        (status = 400, description = "Question already bookmarked"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Question not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Bookmarks",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn bookmark_question(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/api/bookmarks/{question_id}", &data.connections);
    let question_id = path.into_inner();

    // Get user_id from token
    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    // Check if question exists
    let question_exists: bool = match sqlx::query_as::<_, CountResult>(
        "SELECT COUNT(*) as count FROM dbquizapp.soal WHERE id = ?"
    )
    .bind(question_id)
    .fetch_one(&*data.context.soal.pool)
    .await
    {
        Ok(result) => result.map_or(0, |r| r.count) > 0,
        Err(_) => false,
    };

    if !question_exists {
        return HttpResponse::NotFound().json(ErrorResponse {
            error: "Question not found".to_string(),
        });
    }

    // Generate bookmark ID
    let bookmark_id = Uuid::new_v4().to_string();

    // Check if already bookmarked
    let already_bookmarked: bool = match sqlx::query_as::<_, CountResult>(
        "SELECT COUNT(*) as count FROM dbquizapp.bookmarked_questions WHERE user_id = ? AND question_id = ?"
    )
    .bind(&user_id)
    .bind(question_id)
    .fetch_one(&*data.context.soal.pool)
    .await
    {
        Ok(result) => result.map_or(0, |r| r.count) > 0,
        Err(_) => false,
    };

    if already_bookmarked {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Question already bookmarked".to_string(),
        });
    }

    // Insert bookmark
    match sqlx::query(
        "INSERT INTO dbquizapp.bookmarked_questions (id, user_id, question_id) VALUES (?, ?, ?)"
    )
    .bind(&bookmark_id)
    .bind(&user_id)
    .bind(question_id)
    .execute(&*data.context.soal.pool)
    .await
    {
        Ok(_) => {
            // Get total bookmark count
            let bookmark_count = match sqlx::query_as::<_, CountResult>(
                "SELECT COUNT(*) as count FROM dbquizapp.bookmarked_questions WHERE user_id = ?"
            )
            .bind(&user_id)
            .fetch_one(&*data.context.soal.pool)
            .await
            {
                Ok(result) => result.map_or(0, |r| r.count as i32),
                Err(_) => 0,
            };

            HttpResponse::Ok().json(BookmarkResponse {
                success: true,
                message: "Question bookmarked successfully".to_string(),
                bookmark_count,
            })
        }
        Err(e) => {
            eprintln!("Error bookmarking question: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to bookmark question".to_string(),
            })
        }
    }
}

/// Remove bookmark from a question
#[utoipa::path(
    delete,
    path = "/api/bookmarks/{question_id}",
    params(
        ("question_id" = i32, Path, description = "Question ID to unbookmark")
    ),
    responses(
        (status = 200, description = "Bookmark removed successfully", body = BookmarkResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Bookmark not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Bookmarks",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn unbookmark_question(
    path: web::Path<i32>,
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/api/bookmarks/{question_id}", &data.connections);
    let question_id = path.into_inner();

    // Get user_id from token
    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    // Delete bookmark
    match sqlx::query(
        "DELETE FROM dbquizapp.bookmarked_questions WHERE user_id = ? AND question_id = ?"
    )
    .bind(&user_id)
    .bind(question_id)
    .execute(&*data.context.soal.pool)
    .await
    {
        Ok(result) => {
            let rows_affected = result.rows_affected();

            if rows_affected == 0 {
                return HttpResponse::NotFound().json(ErrorResponse {
                    error: "Bookmark not found".to_string(),
                });
            }

            // Get total bookmark count
            let bookmark_count = match sqlx::query_as::<_, CountResult>(
                "SELECT COUNT(*) as count FROM dbquizapp.bookmarked_questions WHERE user_id = ?"
            )
            .bind(&user_id)
            .fetch_one(&*data.context.soal.pool)
            .await
            {
                Ok(result) => result.map_or(0, |r| r.count as i32),
                Err(_) => 0,
            };

            HttpResponse::Ok().json(BookmarkResponse {
                success: true,
                message: "Bookmark removed successfully".to_string(),
                bookmark_count,
            })
        }
        Err(e) => {
            eprintln!("Error unbookmarking question: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to remove bookmark".to_string(),
            })
        }
    }
}

/// Get all bookmarks for the authenticated user
#[utoipa::path(
    get,
    path = "/api/bookmarks",
    responses(
        (status = 200, description = "List of bookmarked questions", body = Vec<BookmarkedQuestion>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Bookmarks",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_user_bookmarks(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> impl Responder {
    log_request("/api/bookmarks", &data.connections);

    // Get user_id from token
    let user_id = match http_req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap_or_default().to_string(),
        None => return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Unauthorized".to_string(),
        }),
    };

    let query = "
        SELECT
            bq.id,
            bq.question_id,
            s.soal,
            s.opt1,
            s.opt2,
            s.opt3,
            s.opt4,
            s.opt5,
            s.correct_answer,
            s.solution,
            s.modul,
            s.pelajaran,
            s.tag,
            s.question_type,
            bq.created_at
        FROM dbquizapp.bookmarked_questions bq
        JOIN dbquizapp.soal s ON bq.question_id = s.id
        WHERE bq.user_id = ?
        ORDER BY bq.created_at DESC
    ";

    match sqlx::query(query)
        .bind(&user_id)
        .fetch_all(&*data.context.soal.pool)
        .await
    {
        Ok(rows) => {
            let bookmarks: Vec<BookmarkedQuestion> = rows
                .iter()
                .map(|row| BookmarkedQuestion::from_row(row).unwrap())
                .collect();

            HttpResponse::Ok().json(bookmarks)
        }
        Err(e) => {
            eprintln!("Error fetching bookmarks: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch bookmarks".to_string(),
            })
        }
    }
}
