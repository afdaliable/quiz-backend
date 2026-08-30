//! Phase 1 of the soal analytics dashboard: precomputed coverage (how many
//! soal per track/category/subcategory/topic, broken down by difficulty and
//! status) and tag-string inconsistency clusters. A GROUP BY over ~54k soal
//! rows is fast on its own, but this still gets precomputed into
//! `soal_analytics_*` tables so the admin page loads instantly and gives a
//! stable daily snapshot instead of recomputing on every page view.
//!
//! Cross-subcategory / topik-tambahan candidate detection and the AI
//! insight summary are later phases -- those need a rate-limited AI sweep
//! over the existing soal corpus (reusing the ai_bulk_jobs pattern), not
//! something a cheap daily aggregation job can do.

use sqlx::{MySqlPool, Row};
use std::collections::HashMap;

use crate::dao::taxonomy_dao::TaxonomyDao;
use crate::model::taxonomy::TaxonomyTree;

#[derive(sqlx::FromRow)]
struct NodeCounts {
    node_id: String,
    total: i64,
    easy: i64,
    medium: i64,
    hard: i64,
    active: i64,
    draft: i64,
    archived: i64,
}

async fn fetch_counts(pool: &MySqlPool, column: &str) -> Result<HashMap<String, NodeCounts>, sqlx::Error> {
    let query = format!(
        r#"
        SELECT
            {column} AS node_id,
            COUNT(*) AS total,
            CAST(SUM(difficulty_est = 'easy') AS SIGNED) AS easy,
            CAST(SUM(difficulty_est = 'medium') AS SIGNED) AS medium,
            CAST(SUM(difficulty_est = 'hard') AS SIGNED) AS hard,
            CAST(SUM(status = 'active') AS SIGNED) AS active,
            CAST(SUM(status = 'draft') AS SIGNED) AS draft,
            CAST(SUM(status = 'archived') AS SIGNED) AS archived
        FROM dbquizapp.soal
        WHERE {column} IS NOT NULL
        GROUP BY {column}
        "#
    );
    let rows = sqlx::query_as::<_, NodeCounts>(&query).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| (r.node_id.clone(), r)).collect())
}

fn zero_counts() -> NodeCounts {
    NodeCounts { node_id: String::new(), total: 0, easy: 0, medium: 0, hard: 0, active: 0, draft: 0, archived: 0 }
}

struct CoverageRow {
    level: &'static str,
    node_id: String,
    node_name: String,
    parent_path: Option<String>,
    track_id: Option<String>,
    category_id: Option<String>,
    subcategory_id: Option<String>,
    counts: NodeCounts,
}

/// Walks the full taxonomy tree (so nodes with zero soal still show up as
/// true gaps, not just absent rows) and pairs each node with its counts.
fn build_coverage_rows(
    tree: &TaxonomyTree,
    by_track: &HashMap<String, NodeCounts>,
    by_category: &HashMap<String, NodeCounts>,
    by_subcategory: &HashMap<String, NodeCounts>,
    by_topic: &HashMap<String, NodeCounts>,
) -> Vec<CoverageRow> {
    let mut rows = Vec::new();

    for t in &tree.tracks {
        rows.push(CoverageRow {
            level: "track",
            node_id: t.track.id.clone(),
            node_name: t.track.name.clone(),
            parent_path: None,
            track_id: Some(t.track.id.clone()),
            category_id: None,
            subcategory_id: None,
            counts: by_track.get(&t.track.id).map(clone_counts).unwrap_or_else(zero_counts),
        });

        for c in &t.categories {
            rows.push(CoverageRow {
                level: "category",
                node_id: c.category.id.clone(),
                node_name: c.category.name.clone(),
                parent_path: Some(t.track.name.clone()),
                track_id: Some(t.track.id.clone()),
                category_id: Some(c.category.id.clone()),
                subcategory_id: None,
                counts: by_category.get(&c.category.id).map(clone_counts).unwrap_or_else(zero_counts),
            });

            for s in &c.subcategories {
                rows.push(CoverageRow {
                    level: "subcategory",
                    node_id: s.subcategory.id.clone(),
                    node_name: s.subcategory.name.clone(),
                    parent_path: Some(format!("{} > {}", t.track.name, c.category.name)),
                    track_id: Some(t.track.id.clone()),
                    category_id: Some(c.category.id.clone()),
                    subcategory_id: Some(s.subcategory.id.clone()),
                    counts: by_subcategory.get(&s.subcategory.id).map(clone_counts).unwrap_or_else(zero_counts),
                });

                for topic in &s.topics {
                    rows.push(CoverageRow {
                        level: "topic",
                        node_id: topic.id.clone(),
                        node_name: topic.name.clone(),
                        parent_path: Some(format!("{} > {} > {}", t.track.name, c.category.name, s.subcategory.name)),
                        track_id: Some(t.track.id.clone()),
                        category_id: Some(c.category.id.clone()),
                        subcategory_id: Some(s.subcategory.id.clone()),
                        counts: by_topic.get(&topic.id).map(clone_counts).unwrap_or_else(zero_counts),
                    });
                }
            }
        }
    }

    rows
}

fn clone_counts(c: &NodeCounts) -> NodeCounts {
    NodeCounts {
        node_id: c.node_id.clone(),
        total: c.total,
        easy: c.easy,
        medium: c.medium,
        hard: c.hard,
        active: c.active,
        draft: c.draft,
        archived: c.archived,
    }
}

async fn compute_coverage(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    let taxonomy_dao = TaxonomyDao::new(std::sync::Arc::new(pool.clone()));
    let tree = taxonomy_dao.get_taxonomy_tree().await?;

    let by_track = fetch_counts(pool, "track_id").await?;
    let by_category = fetch_counts(pool, "category_id").await?;
    let by_subcategory = fetch_counts(pool, "subcategory_id").await?;
    let by_topic = fetch_counts(pool, "topic_id").await?;

    let rows = build_coverage_rows(&tree, &by_track, &by_category, &by_subcategory, &by_topic);

    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM dbquizapp.soal_analytics_coverage").execute(&mut *tx).await?;
    for r in &rows {
        sqlx::query(
            r#"
            INSERT INTO dbquizapp.soal_analytics_coverage
                (level, node_id, node_name, parent_path, track_id, category_id, subcategory_id,
                 total_count, easy_count, medium_count, hard_count, active_count, draft_count, archived_count)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(r.level)
        .bind(&r.node_id)
        .bind(&r.node_name)
        .bind(&r.parent_path)
        .bind(&r.track_id)
        .bind(&r.category_id)
        .bind(&r.subcategory_id)
        .bind(r.counts.total)
        .bind(r.counts.easy)
        .bind(r.counts.medium)
        .bind(r.counts.hard)
        .bind(r.counts.active)
        .bind(r.counts.draft)
        .bind(r.counts.archived)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Splits every soal.tag on commas, normalizes each token (trim + lowercase
/// + collapse internal whitespace), and records every distinct raw variant
/// seen per normalized token. A normalized token backed by >1 raw variant
/// is free-text drift worth merging (case/whitespace/formatting only --
/// genuine near-duplicates like typos need fuzzy matching, out of scope
/// for this phase).
async fn compute_tag_variants(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    let rows = sqlx::query("SELECT tag FROM dbquizapp.soal WHERE tag IS NOT NULL AND tag != ''")
        .fetch_all(pool)
        .await?;

    // normalized -> (raw_variant -> count)
    let mut variants: HashMap<String, HashMap<String, i64>> = HashMap::new();
    for row in &rows {
        let tag_field: String = row.try_get("tag")?;
        for raw in tag_field.split(',') {
            let raw = raw.trim();
            if raw.is_empty() {
                continue;
            }
            let normalized = raw.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
            *variants.entry(normalized).or_default().entry(raw.to_string()).or_insert(0) += 1;
        }
    }

    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM dbquizapp.soal_analytics_tag_variants").execute(&mut *tx).await?;
    for (normalized, by_variant) in &variants {
        for (variant_text, count) in by_variant {
            sqlx::query(
                "INSERT INTO dbquizapp.soal_analytics_tag_variants (normalized_tag, variant_text, soal_count) VALUES (?, ?, ?)",
            )
            .bind(normalized)
            .bind(variant_text)
            .bind(count)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

async fn compute_meta(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    let total_soal: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dbquizapp.soal").fetch_one(pool).await?;
    let total_uncategorized: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dbquizapp.soal WHERE track_id IS NULL OR subcategory_id IS NULL",
    )
    .fetch_one(pool)
    .await?;
    let total_distinct_tags: i64 =
        sqlx::query_scalar("SELECT COUNT(DISTINCT tag) FROM dbquizapp.soal WHERE tag IS NOT NULL AND tag != ''")
            .fetch_one(pool)
            .await?;

    sqlx::query(
        r#"
        INSERT INTO dbquizapp.soal_analytics_meta (id, last_computed_at, total_soal, total_uncategorized, total_distinct_tags)
        VALUES (1, NOW(), ?, ?, ?)
        ON DUPLICATE KEY UPDATE
            last_computed_at = NOW(),
            total_soal = VALUES(total_soal),
            total_uncategorized = VALUES(total_uncategorized),
            total_distinct_tags = VALUES(total_distinct_tags)
        "#,
    )
    .bind(total_soal)
    .bind(total_uncategorized)
    .bind(total_distinct_tags)
    .execute(pool)
    .await?;

    Ok(())
}

/// Returns Err with a short message on failure instead of silently
/// swallowing it -- the background scheduled call just logs the Err (a
/// stale summary table isn't a crash), but the manual recompute endpoint
/// needs this to actually report failure to the caller instead of always
/// saying "completed" regardless of what happened (confirmed live: this
/// masked a real decode bug for a while -- recompute returned 200 while
/// every table stayed empty).
pub async fn compute_and_store_summary(pool: &MySqlPool) -> Result<(), String> {
    println!("[soal_analytics_service] Computing coverage...");
    compute_coverage(pool).await.map_err(|e| {
        let msg = format!("compute_coverage failed: {:?}", e);
        eprintln!("[soal_analytics_service] {}", msg);
        msg
    })?;
    println!("[soal_analytics_service] Computing tag variants...");
    compute_tag_variants(pool).await.map_err(|e| {
        let msg = format!("compute_tag_variants failed: {:?}", e);
        eprintln!("[soal_analytics_service] {}", msg);
        msg
    })?;
    println!("[soal_analytics_service] Computing meta...");
    compute_meta(pool).await.map_err(|e| {
        let msg = format!("compute_meta failed: {:?}", e);
        eprintln!("[soal_analytics_service] {}", msg);
        msg
    })?;
    println!("[soal_analytics_service] Done.");
    Ok(())
}
