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
use std::sync::Arc;
use uuid::Uuid;

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

#[derive(Serialize)]
struct FixJobAcceptedResponse {
    job_id: String,
    status: String,
    poll_url: String,
}

#[derive(Serialize)]
struct FixJobStatusResponse {
    job_id: String,
    status: String,
    result: Option<FixResponse>,
    error_message: Option<String>,
    completed_at: Option<String>,
}

/// POST /admin/soal-quality/merged-options/fix -- runs as an async job.
/// Looping a few hundred sequential row UPDATEs synchronously trips
/// Cloudflare's proxy timeout on the Free plan (confirmed live: 504 on
/// ~660 rows), same reasoning as the AI job endpoints even though no AI
/// is involved here.
#[post("/merged-options/fix")]
async fn fix_merged_options(
    body: web::Json<FixRequest>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let pool = Arc::clone(&data.context.soal.pool);
    let ids = body.ids.clone();

    let job_id = Uuid::new_v4().to_string();
    if let Err(e) = sqlx::query(
        "INSERT INTO dbquizapp.soal_quality_fix_jobs (id, status) VALUES (?, 'pending')",
    )
    .bind(&job_id)
    .execute(&*pool)
    .await
    {
        eprintln!("[soal_quality_controller] Failed to insert fix job: {:?}", e);
        return HttpResponse::InternalServerError().json(err("db_error", "Failed to create job."));
    }

    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        run_fix_job(job_id_clone, ids, pool).await;
    });

    HttpResponse::Accepted().json(FixJobAcceptedResponse {
        poll_url: format!("/admin/soal-quality/merged-options/fix/{}", job_id),
        job_id,
        status: "pending".to_string(),
    })
}

async fn run_fix_job(job_id: String, ids: Vec<i64>, pool: Arc<MySqlPool>) {
    let _ = sqlx::query("UPDATE dbquizapp.soal_quality_fix_jobs SET status='running' WHERE id=?")
        .bind(&job_id)
        .execute(&*pool)
        .await;

    let rows = match fetch_merged_option_rows(&pool).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[soal_quality_controller] DB error: {:?}", e);
            let msg = format!("Failed to query soal: {}", e);
            let _ = sqlx::query(
                "UPDATE dbquizapp.soal_quality_fix_jobs SET status='failed', error_message=?, completed_at=NOW() WHERE id=?",
            )
            .bind(&msg)
            .bind(&job_id)
            .execute(&*pool)
            .await;
            return;
        }
    };

    let mut fixed = 0usize;
    let mut skipped_not_auto_fixable = 0usize;
    let mut failed = 0usize;

    for row in rows {
        if !ids.is_empty() && !ids.contains(&row.id) {
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
        .execute(&*pool)
        .await;

        match result {
            Ok(_) => fixed += 1,
            Err(e) => {
                eprintln!("[soal_quality_controller] failed to fix soal {}: {:?}", row.id, e);
                failed += 1;
            }
        }
    }

    let _ = sqlx::query(
        "UPDATE dbquizapp.soal_quality_fix_jobs SET status='completed', fixed=?, skipped_not_auto_fixable=?, failed_count=?, completed_at=NOW() WHERE id=?",
    )
    .bind(fixed as i32)
    .bind(skipped_not_auto_fixable as i32)
    .bind(failed as i32)
    .bind(&job_id)
    .execute(&*pool)
    .await;
}

/// GET /admin/soal-quality/merged-options/fix/{job_id}
#[get("/merged-options/fix/{job_id}")]
async fn get_fix_job(path: web::Path<String>, data: web::Data<AppState<'_>>) -> impl Responder {
    let job_id = path.into_inner();
    let pool = &*data.context.soal.pool;

    #[derive(sqlx::FromRow)]
    struct JobRow {
        status: String,
        fixed: Option<i32>,
        skipped_not_auto_fixable: Option<i32>,
        failed_count: Option<i32>,
        error_message: Option<String>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    let job: JobRow = match sqlx::query_as::<_, JobRow>(
        "SELECT status, fixed, skipped_not_auto_fixable, failed_count, error_message, completed_at FROM dbquizapp.soal_quality_fix_jobs WHERE id = ?",
    )
    .bind(&job_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(j)) => j,
        Ok(None) => {
            return HttpResponse::NotFound().json(err("not_found", format!("Job {} not found", job_id)))
        }
        Err(e) => {
            eprintln!("[soal_quality_controller] DB error fetching fix job {}: {:?}", job_id, e);
            return HttpResponse::InternalServerError().json(err("db_error", "Failed to fetch job status."));
        }
    };

    let result = if job.status == "completed" {
        Some(FixResponse {
            fixed: job.fixed.unwrap_or(0) as usize,
            skipped_not_auto_fixable: job.skipped_not_auto_fixable.unwrap_or(0) as usize,
            failed: job.failed_count.unwrap_or(0) as usize,
        })
    } else {
        None
    };

    HttpResponse::Ok().json(FixJobStatusResponse {
        job_id,
        status: job.status,
        result,
        error_message: job.error_message,
        completed_at: job.completed_at.map(|t| t.to_rfc3339()),
    })
}

// --- Detector 2: soal that reads like it depends on a scenario/passage that
// was never captured -- e.g. "Hari ini Mita memiliki beberapa agenda untuk
// dilakukan..." followed by options describing an order of events ("pergi
// ke pasar", "mengunjungi nenek") that appear nowhere in the stem. Confirmed
// live: `passages` has only 28 rows total and none mention Mita/agenda --
// the source paragraph was never scraped, so it can't be recovered, only
// flagged for a human to fix or drop.
//
// Read-only detector, no auto-fix: a generic "options don't share vocabulary
// with the stem" heuristic was tried first and flagged ~38% of all soal
// (20451/54067) -- most SKD/factual-recall questions naturally have options
// that are the *answer*, not a restatement of the stem, so raw overlap is
// useless here. Narrowing to "sequence words (pertama/sebelum/setelah/...)
// appear in >=2 options but not in the stem itself, restricted to
// reasoning-type pelajaran where this actually indicates a missing clue
// list" cut it to 18 candidates on live data, all plausible.
fn has_sequence_words(text: &str, re: &Regex) -> bool {
    re.is_match(text)
}

fn detect_missing_context(soal: &str, opts: &[&str], seq_re: &Regex) -> bool {
    if opts.len() < 3 {
        return false;
    }
    let seq_opt_count = opts.iter().filter(|o| has_sequence_words(o, seq_re)).count();
    if seq_opt_count < 2 {
        return false;
    }
    if has_sequence_words(soal, seq_re) || soal.contains(':') || soal.chars().count() > 400 {
        return false;
    }
    true
}

#[derive(sqlx::FromRow)]
struct ContextCandidateRow {
    id: i64,
    soal: String,
    opt1: Option<String>,
    opt2: Option<String>,
    opt3: Option<String>,
    opt4: Option<String>,
    opt5: Option<String>,
    modul: Option<String>,
    pelajaran: Option<String>,
}

#[derive(Serialize)]
struct MissingContextItem {
    id: i64,
    soal: String,
    modul: Option<String>,
    pelajaran: Option<String>,
}

#[derive(Serialize)]
struct MissingContextResponse {
    total: usize,
    items: Vec<MissingContextItem>,
}

/// GET /admin/soal-quality/missing-context
#[get("/missing-context")]
async fn list_missing_context(data: web::Data<AppState<'_>>) -> impl Responder {
    let pool = &*data.context.soal.pool;
    let rows = match sqlx::query_as::<_, ContextCandidateRow>(
        r#"
        SELECT id, soal, opt1, opt2, opt3, opt4, opt5, modul, pelajaran
        FROM dbquizapp.soal
        WHERE passage_id IS NULL
          AND pelajaran IN ('TIU (Tes Intelegensi Umum)', 'Penalaran Verbal', 'Penalaran Analitis')
        ORDER BY id
        "#,
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[soal_quality_controller] DB error: {:?}", e);
            return HttpResponse::InternalServerError().json(err("db_error", "Failed to query soal."));
        }
    };

    let seq_re = Regex::new(
        r"(?i)\b(pertama|kedua|ketiga|sebelum|sesudah|setelah|kemudian|selanjutnya|akhirnya|terakhir)\b",
    )
    .unwrap();

    let items: Vec<MissingContextItem> = rows
        .into_iter()
        .filter(|r| {
            let opts: Vec<&str> = [
                r.opt1.as_deref(),
                r.opt2.as_deref(),
                r.opt3.as_deref(),
                r.opt4.as_deref(),
                r.opt5.as_deref(),
            ]
            .into_iter()
            .flatten()
            .filter(|o| !o.trim().is_empty())
            .collect();
            detect_missing_context(&r.soal, &opts, &seq_re)
        })
        .map(|r| MissingContextItem {
            id: r.id,
            soal: r.soal,
            modul: r.modul,
            pelajaran: r.pelajaran,
        })
        .collect();

    HttpResponse::Ok().json(MissingContextResponse {
        total: items.len(),
        items,
    })
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/soal-quality")
            .wrap(AdminMiddleware::new())
            .service(list_merged_options)
            .service(fix_merged_options)
            .service(get_fix_job)
            .service(list_missing_context),
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

    fn seq_re() -> Regex {
        Regex::new(
            r"(?i)\b(pertama|kedua|ketiga|sebelum|sesudah|setelah|kemudian|selanjutnya|akhirnya|terakhir)\b",
        )
        .unwrap()
    }

    #[test]
    fn flags_narrative_with_scattered_sequence_options() {
        let soal = "Hari ini Mita memiliki beberapa agenda untuk dilakukan. Pernyataan yang benar adalah …";
        let opts = vec![
            "Hal pertama yang dilakukan Mita adalah pergi ke pasar",
            "Mita mengajar siswa les setelah mengunjungi nenek",
            "Sebelum ke pasar, Mita mengunjungi nenek",
        ];
        assert!(detect_missing_context(soal, &opts, &seq_re()));
    }

    #[test]
    fn does_not_flag_self_contained_factual_question() {
        let soal = "Rasio pembelian ulang menunjukkan ....";
        let opts = vec!["Loyalitas pelanggan", "Tingkat retensi", "Nilai transaksi"];
        assert!(!detect_missing_context(soal, &opts, &seq_re()));
    }

    #[test]
    fn does_not_flag_when_stem_already_states_the_sequence() {
        let soal = "Setelah makan siang, budi pergi ke kantor. Pertama dia mengecek email, sebelum rapat pukul 2.";
        let opts = vec![
            "Budi rapat sebelum makan siang",
            "Budi mengecek email pertama",
            "Setelah rapat budi pulang",
        ];
        assert!(!detect_missing_context(soal, &opts, &seq_re()));
    }

    #[test]
    fn does_not_flag_with_fewer_than_two_sequence_options() {
        let soal = "Manakah pernyataan yang benar?";
        let opts = vec!["Pertama, dia lulus ujian", "Dia bekerja di bank", "Dia menikah tahun lalu"];
        assert!(!detect_missing_context(soal, &opts, &seq_re()));
    }
}
