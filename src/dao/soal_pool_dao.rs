//! Taxonomy-based soal pools: every soal linked to a category through either
//! its direct `category_id` or the `question_topics` M2M, restricted to soal
//! that are safe to serve in an app that renders plain multiple choice.
//!
//! This is how a product (the UPKP app) draws on soal that were never put in
//! a paket -- e.g. SKD TWK soal mapped to UPKP > Wawasan Kebangsaan keep SKD as
//! their category and reach UPKP only through topics.

use serde::Serialize;
use sqlx::{MySqlPool, Row};

/// Servable = active, has an answer key, plain multiple choice, and no reading
/// passage (the consuming apps don't render passages).
const SERVABLE: &str = "s.status = 'active' \
     AND s.correct_answer IS NOT NULL \
     AND s.question_type = 'multiple_choice' \
     AND s.passage_id IS NULL";

/// No leftover AI hedge in the pembahasan. A LIKE over the solution text is the
/// expensive part of the filter, so sampling applies it only to picked rows.
const NO_HEDGE: &str = "(s.solution IS NULL OR s.solution NOT LIKE '%perlu diverifikasi dengan buku sumber%')";

/// Binds: category slug, category slug.
const IN_CATEGORY: &str = "(EXISTS (SELECT 1 FROM categories c WHERE c.id = s.category_id AND c.slug = ?) \
     OR EXISTS (SELECT 1 FROM question_topics qt JOIN topics tp ON tp.id = qt.topic_id \
                JOIN subcategories sc ON sc.id = tp.subcategory_id \
                JOIN categories c ON c.id = sc.category_id \
                WHERE qt.question_id = s.id AND c.slug = ?))";

#[derive(Debug, Serialize, Clone)]
pub struct PoolSoal {
    pub id: i32,
    pub soal: String,
    pub question_type: String,
    pub opt1: Option<String>,
    pub opt2: Option<String>,
    pub opt3: Option<String>,
    pub opt4: Option<String>,
    pub opt5: Option<String>,
    pub correct_answer: Option<String>,
    pub solution: Option<String>,
    pub tag: Option<String>,
    pub difficulty_est: Option<String>,
    /// The soal's topic within the requested category (first by sort order
    /// when it has several), for per-topic score breakdowns. The soal's own
    /// `tag` belongs to whichever exam it came from and can't be used for that.
    pub topic_name: Option<String>,
}

/// Random servable soal ids from a category, optionally narrowed to one
/// subcategory slug and/or one topic name (names are what apps show and store,
/// e.g. a weak-topic list).
pub async fn sample_ids(
    pool: &MySqlPool,
    category_slug: &str,
    subcategory_slug: Option<&str>,
    topic_name: Option<&str>,
    limit: u32,
) -> Result<Vec<i32>, sqlx::Error> {
    // Members come from two branches -- direct FK and question_topics -- each
    // narrowed the same way, so the filters run on index lookups instead of a
    // per-soal EXISTS over the whole table (~1s -> ~0.2s for UPKP).
    let mut fk = String::from("SELECT s2.id FROM soal s2 JOIN categories c ON c.id = s2.category_id");
    let mut fk_where = vec!["c.slug = ?"];
    let mut m2m = String::from(
        "SELECT qt.question_id FROM question_topics qt JOIN topics tp ON tp.id = qt.topic_id \
         JOIN subcategories sc ON sc.id = tp.subcategory_id JOIN categories c ON c.id = sc.category_id",
    );
    let mut m2m_where = vec!["c.slug = ?"];
    if subcategory_slug.is_some() {
        fk.push_str(" JOIN subcategories sc ON sc.id = s2.subcategory_id");
        fk_where.push("sc.slug = ?");
        m2m_where.push("sc.slug = ?");
    }
    if topic_name.is_some() {
        fk.push_str(" JOIN topics tp ON tp.id = s2.topic_id");
        fk_where.push("tp.name = ?");
        m2m_where.push("tp.name = ?");
    }
    // ponytail: ORDER BY RAND() sorts every member (~10k for UPKP TPA); switch
    // to id-range sampling if a pool grows toward ~1M. The +20 headroom covers
    // picked soal the hedge filter then drops.
    let sql = format!(
        "SELECT s.id FROM ( \
           SELECT s.id FROM ({fk} WHERE {} UNION {m2m} WHERE {}) m \
           JOIN soal s ON s.id = m.id WHERE {SERVABLE} ORDER BY RAND() LIMIT ? \
         ) picked JOIN soal s ON s.id = picked.id WHERE {NO_HEDGE} LIMIT ?",
        fk_where.join(" AND "),
        m2m_where.join(" AND "),
    );

    let mut q = sqlx::query_scalar::<_, i32>(&sql);
    for _branch in 0..2 {
        q = q.bind(category_slug);
        if let Some(sub) = subcategory_slug {
            q = q.bind(sub);
        }
        if let Some(topic) = topic_name {
            q = q.bind(topic);
        }
    }
    q.bind(limit + 20).bind(limit).fetch_all(pool).await
}

/// Full soal (with answer key and topic name) for ids that are still in the
/// category's servable pool, in the order given. Ids outside the pool are
/// silently dropped, so callers can't use this to read arbitrary soal.
pub async fn fetch_in_pool(
    pool: &MySqlPool,
    category_slug: &str,
    ids: &[i32],
) -> Result<Vec<PoolSoal>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = vec!["?"; ids.len()].join(", ");
    let sql = format!(
        "SELECT s.id, s.soal, s.question_type, s.opt1, s.opt2, s.opt3, s.opt4, s.opt5, \
                s.correct_answer, s.solution, s.tag, s.difficulty_est, \
                COALESCE( \
                  (SELECT tp.name FROM question_topics qt JOIN topics tp ON tp.id = qt.topic_id \
                     JOIN subcategories sc ON sc.id = tp.subcategory_id \
                     JOIN categories c ON c.id = sc.category_id \
                   WHERE qt.question_id = s.id AND c.slug = ? \
                   ORDER BY sc.sort_order, tp.sort_order LIMIT 1), \
                  (SELECT tp.name FROM topics tp JOIN subcategories sc ON sc.id = tp.subcategory_id \
                     JOIN categories c ON c.id = sc.category_id \
                   WHERE tp.id = s.topic_id AND c.slug = ?) \
                ) AS topic_name \
         FROM soal s WHERE s.id IN ({placeholders}) AND {SERVABLE} AND {NO_HEDGE} AND {IN_CATEGORY}"
    );
    let mut q = sqlx::query(&sql).bind(category_slug).bind(category_slug);
    for id in ids {
        q = q.bind(id);
    }
    let rows = q.bind(category_slug).bind(category_slug).fetch_all(pool).await?;

    let mut by_id: std::collections::HashMap<i32, PoolSoal> = rows
        .into_iter()
        .map(|r| {
            let s = PoolSoal {
                id: r.try_get("id").unwrap_or_default(),
                soal: r.try_get("soal").unwrap_or_default(),
                question_type: r.try_get("question_type").unwrap_or_default(),
                opt1: r.try_get("opt1").ok().flatten(),
                opt2: r.try_get("opt2").ok().flatten(),
                opt3: r.try_get("opt3").ok().flatten(),
                opt4: r.try_get("opt4").ok().flatten(),
                opt5: r.try_get("opt5").ok().flatten(),
                correct_answer: r.try_get("correct_answer").ok().flatten(),
                solution: r.try_get("solution").ok().flatten(),
                tag: r.try_get("tag").ok().flatten(),
                difficulty_est: r.try_get("difficulty_est").ok().flatten(),
                topic_name: r.try_get("topic_name").ok().flatten(),
            };
            (s.id, s)
        })
        .collect();
    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
}
