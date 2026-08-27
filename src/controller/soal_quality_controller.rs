//! Data-quality checks for `soal` -- currently one detector: options that
//! never got split from raw scraped/imported text, so opt1 contains all the
//! choices concatenated ("...jawaban benar B. pilihan lain C. ...") while
//! opt2-5 sit empty. Confirmed live on 726 rows (SKD + LPDP) via direct DB
//! query before building this. No AI involved -- the merged text is
//! regular enough (a "B./C./D./E." marker before each choice, case-
//! insensitive, first choice unmarked) that a deterministic split handles
//! ~91% of cases; the rest are flagged for manual fix rather than guessed.

use actix_web::{get, post, web, HttpResponse, Responder};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

use crate::middleware::admin_middleware::AdminMiddleware;
use crate::AppState;

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

#[derive(sqlx::FromRow)]
struct MergedOptionRow {
    id: i64,
    soal: String,
    opt1: String,
    modul: Option<String>,
}

/// Splits opt1 on "B./C./D./E." markers (case-insensitive, whitespace
/// tolerant). Returns Some([a,b,c,d,e]) only when all four markers are
/// found in order with nothing extra/missing/out-of-order -- anything less
/// clean is left for a human, never guessed at.
fn split_merged_options(opt1: &str) -> Option<[String; 5]> {
    let re = Regex::new(r"(?i)\s*([b-e])\.\s+").unwrap();

    let mut parts: Vec<String> = Vec::new();
    let mut letters: Vec<char> = Vec::new();
    let mut last_end = 0;
    for cap in re.captures_iter(opt1) {
        let m = cap.get(0).unwrap();
        parts.push(opt1[last_end..m.start()].trim().to_string());
        letters.push(cap.get(1).unwrap().as_str().to_ascii_uppercase().chars().next().unwrap());
        last_end = m.end();
    }
    parts.push(opt1[last_end..].trim().to_string());

    if letters != ['B', 'C', 'D', 'E'] || parts.len() != 5 || parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    Some([
        parts[0].clone(),
        parts[1].clone(),
        parts[2].clone(),
        parts[3].clone(),
        parts[4].clone(),
    ])
}

async fn fetch_merged_option_rows(pool: &MySqlPool) -> Result<Vec<MergedOptionRow>, sqlx::Error> {
    sqlx::query_as::<_, MergedOptionRow>(
        r#"
        SELECT id, soal, opt1, modul FROM dbquizapp.soal
        WHERE opt1 REGEXP '[B-E]\\.[[:space:]]' AND (opt2 IS NULL OR opt2 = '')
        ORDER BY id
        "#,
    )
    .fetch_all(pool)
    .await
}

#[derive(Serialize)]
struct QualityItem {
    id: i64,
    soal: String,
    opt1: String,
    modul: Option<String>,
    auto_fixable: bool,
    preview: Option<[String; 5]>,
}

#[derive(Serialize)]
struct QualityListResponse {
    total: usize,
    auto_fixable_count: usize,
    needs_review_count: usize,
    items: Vec<QualityItem>,
}

/// GET /admin/soal-quality/merged-options
#[get("/merged-options")]
async fn list_merged_options(data: web::Data<AppState<'_>>) -> impl Responder {
    let pool = &*data.context.soal.pool;
    let rows = match fetch_merged_option_rows(pool).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[soal_quality_controller] DB error: {:?}", e);
            return HttpResponse::InternalServerError().json(err("db_error", "Failed to query soal."));
        }
    };

    let mut auto_fixable_count = 0;
    let mut needs_review_count = 0;
    let items: Vec<QualityItem> = rows
        .into_iter()
        .map(|r| {
            let preview = split_merged_options(&r.opt1);
            if preview.is_some() {
                auto_fixable_count += 1;
            } else {
                needs_review_count += 1;
            }
            QualityItem {
                id: r.id,
                soal: r.soal,
                opt1: r.opt1,
                modul: r.modul,
                auto_fixable: preview.is_some(),
                preview,
            }
        })
        .collect();

    HttpResponse::Ok().json(QualityListResponse {
        total: items.len(),
        auto_fixable_count,
        needs_review_count,
        items,
    })
}

#[derive(Deserialize)]
struct FixRequest {
    /// If empty, fixes every auto-fixable row found right now.
    #[serde(default)]
    ids: Vec<i64>,
}

#[derive(Serialize)]
struct FixResponse {
    fixed: usize,
    skipped_not_auto_fixable: usize,
    failed: usize,
}

/// POST /admin/soal-quality/merged-options/fix
#[post("/merged-options/fix")]
async fn fix_merged_options(
    body: web::Json<FixRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let pool = &*data.context.soal.pool;
    let rows = match fetch_merged_option_rows(pool).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[soal_quality_controller] DB error: {:?}", e);
            return HttpResponse::InternalServerError().json(err("db_error", "Failed to query soal."));
        }
    };

    let target: Box<dyn Fn(i64) -> bool> = if body.ids.is_empty() {
        Box::new(|_| true)
    } else {
        let ids = body.ids.clone();
        Box::new(move |id| ids.contains(&id))
    };

    let mut fixed = 0;
    let mut skipped_not_auto_fixable = 0;
    let mut failed = 0;

    for row in rows {
        if !target(row.id) {
            continue;
        }
        let Some(split) = split_merged_options(&row.opt1) else {
            skipped_not_auto_fixable += 1;
            continue;
        };
        let result = sqlx::query(
            "UPDATE dbquizapp.soal SET opt1=?, opt2=?, opt3=?, opt4=?, opt5=?, updated_at=NOW() WHERE id=?",
        )
        .bind(&split[0])
        .bind(&split[1])
        .bind(&split[2])
        .bind(&split[3])
        .bind(&split[4])
        .bind(row.id)
        .execute(pool)
        .await;

        match result {
            Ok(_) => fixed += 1,
            Err(e) => {
                eprintln!("[soal_quality_controller] failed to fix soal {}: {:?}", row.id, e);
                failed += 1;
            }
        }
    }

    HttpResponse::Ok().json(FixResponse {
        fixed,
        skipped_not_auto_fixable,
        failed,
    })
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/soal-quality")
            .wrap(AdminMiddleware::new())
            .service(list_merged_options)
            .service(fix_merged_options),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_clean_four_way_merge() {
        let opt1 = "Berani mengambil yang baik dari luar untuk memperbaiki bangsa sendiri B. Bersikap apa adanya sesuai kondisi bangsa C. Memberikan kebebasan kepada warga negara untuk memberikan ide terkait nasionalis D. Memberi ruang bagi semua dan melampaui batas-batas suku, agama dan ras E. Membebaskan kebebasan kepada warga negara untuk memberikan ide terkait nasionalis";
        let result = split_merged_options(opt1).expect("should split cleanly");
        assert_eq!(result[0], "Berani mengambil yang baik dari luar untuk memperbaiki bangsa sendiri");
        assert_eq!(result[1], "Bersikap apa adanya sesuai kondisi bangsa");
        assert_eq!(result[4], "Membebaskan kebebasan kepada warga negara untuk memberikan ide terkait nasionalis");
    }

    #[test]
    fn splits_lowercase_markers() {
        let opt1 = "Alur dana b. Pengeluaran dana c. Pemasukan dana d. Dana pribadi e. Penyediaan dana";
        let result = split_merged_options(opt1).expect("should split lowercase markers too");
        assert_eq!(result[0], "Alur dana");
        assert_eq!(result[3], "Dana pribadi");
    }

    #[test]
    fn rejects_missing_marker() {
        // Only B, C, D present -- E missing, don't guess.
        let opt1 = "Satu B. Dua C. Tiga D. Empat";
        assert!(split_merged_options(opt1).is_none());
    }

    #[test]
    fn rejects_out_of_order_or_duplicate_markers() {
        let opt1 = "Satu E. Dua B. Tiga C. Empat D. Lima E. Enam";
        assert!(split_merged_options(opt1).is_none());
    }

    #[test]
    fn rejects_garbage_two_letter_only() {
        assert!(split_merged_options("B. C.").is_none());
    }
}
