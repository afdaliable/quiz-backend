use actix_web::{web, HttpResponse, HttpRequest};
use crate::AppState;
use crate::model::xp::{LeaderboardXpEntry, LeaderboardXpResponse};
use crate::levels::compute_level_info;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/leaderboard")
            .route("/xp", web::get().to(get_xp_leaderboard)),
    );
}

/// Mask a display name for privacy: keep first word, abbreviate the rest.
/// "Afdhal Kurniawan" → "Afdhal K."
fn mask_name(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() <= 1 {
        return name.to_string();
    }
    let mut result = parts[0].to_string();
    for part in &parts[1..] {
        if let Some(c) = part.chars().next() {
            result.push(' ');
            result.push(c);
            result.push('.');
        }
    }
    result
}

async fn get_xp_leaderboard(
    data: web::Data<AppState<'_>>,
    http_req: HttpRequest,
) -> HttpResponse {
    let current_user_id = http_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let pool = &*data.context.users.pool;

    let rows: Vec<(String, String, i64, i32)> = match sqlx::query_as(
        "SELECT id, display_name, total_xp, current_level FROM users WHERE deleted_at IS NULL ORDER BY total_xp DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error fetching XP leaderboard: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "Failed to fetch leaderboard"
            }));
        }
    };

    let mut current_user_rank: Option<i64> = None;

    let entries: Vec<LeaderboardXpEntry> = rows
        .into_iter()
        .enumerate()
        .map(|(i, (id, display_name, total_xp, _))| {
            let rank = (i + 1) as i64;
            let is_current_user = current_user_id.as_deref() == Some(id.as_str());
            if is_current_user {
                current_user_rank = Some(rank);
            }
            let (cfg, _, _) = compute_level_info(total_xp);
            LeaderboardXpEntry {
                rank,
                display_name: mask_name(&display_name),
                total_xp,
                current_level: cfg.level,
                level_name: cfg.name.to_string(),
                level_icon: cfg.icon.to_string(),
                is_current_user,
            }
        })
        .collect();

    // If current user not in top 100, fetch their rank separately
    if current_user_rank.is_none() {
        if let Some(uid) = &current_user_id {
            let rank: Option<i64> = sqlx::query_scalar(
                "SELECT COUNT(*) + 1 FROM users WHERE total_xp > (SELECT total_xp FROM users WHERE id = ?) AND deleted_at IS NULL",
            )
            .bind(uid)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);
            current_user_rank = rank;
        }
    }

    HttpResponse::Ok().json(LeaderboardXpResponse {
        entries,
        current_user_rank,
    })
}
