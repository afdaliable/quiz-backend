use sqlx::MySqlPool;
use serde_json::Value as JsonValue;
use rand::seq::SliceRandom;
use crate::model::exam_simulation::ExamSimulation;

/// Generation modes recognized by `generate_questions_for_simulasi`.
pub const MODE_PAKET: &str = "paket";
/// Scored soal need a key; TKP is scored from option_scores instead. Soal whose
/// key was never scraped sit at NULL until someone fills it in.
const ANSWERABLE: &str = "(s.correct_answer IS NOT NULL OR s.question_type = 'tkp')";
pub const MODE_SUBCATEGORY: &str = "subcategory_composition";
pub const MODE_TOPIC: &str = "topic_composition";
pub const MODE_RANDOM_POOL: &str = "random_pool";

/// Generate question IDs for a simulasi based on its `generation_mode` + `generation_config`.
///
/// `generation_config` JSON shape varies per mode:
/// - paket: ignored; uses paket_soal_id
/// - subcategory_composition / topic_composition:
///   { "compositions": [ { "id": "<uuid>", "count": N, "difficulty_mix"?: {easy,medium,hard} }, ... ] }
/// - random_pool:
///   { "track_id"?: "<uuid>", "category_id"?: "<uuid>", "subcategory_id"?: "<uuid>",
///     "difficulty_mix"?: {easy,medium,hard} }
pub async fn generate_questions_for_simulasi(
    pool: &MySqlPool,
    simulasi: &ExamSimulation,
) -> Result<Vec<i32>, String> {
    match simulasi.generation_mode.as_str() {
        // "simulasi_template" is created by generate_simulasi_batch and uses paket_soal_items
        MODE_PAKET | "simulasi_template" => from_paket(pool, simulasi).await,
        MODE_SUBCATEGORY => from_composition(pool, simulasi, "subcategory_id").await,
        MODE_TOPIC => from_composition(pool, simulasi, "question_topics_join").await,
        MODE_RANDOM_POOL => from_random_pool(pool, simulasi).await,
        other => Err(format!("Unknown generation_mode: {}", other)),
    }
}

async fn from_paket(pool: &MySqlPool, sim: &ExamSimulation) -> Result<Vec<i32>, String> {
    let paket_id = sim.paket_soal_id.ok_or("paket_soal_id required for generation_mode=paket")?;
    let ids: Vec<i32> = sqlx::query_scalar(
        "SELECT soal_id FROM paket_soal_items WHERE paket_soal_id = ?"
    )
    .bind(paket_id)
    .fetch_all(pool).await
    .map_err(|e| format!("Failed to fetch paket questions: {}", e))?;

    if ids.is_empty() {
        return Err(format!("Paket soal {} has no questions", paket_id));
    }

    let mut rng = rand::thread_rng();
    let mut shuffled = ids;
    shuffled.shuffle(&mut rng);
    shuffled.truncate(sim.total_questions as usize);
    Ok(shuffled)
}

/// Composition modes: loop `compositions[]`, fetch IDs per node, respect optional difficulty_mix.
/// `mode_kind` controls how we filter:
/// - "subcategory_id" → WHERE s.subcategory_id = ?
/// - "question_topics_join" → JOIN question_topics qt ON qt.question_id = s.id AND qt.topic_id = ?
async fn from_composition(
    pool: &MySqlPool,
    sim: &ExamSimulation,
    mode_kind: &str,
) -> Result<Vec<i32>, String> {
    let cfg = sim.generation_config.as_ref()
        .ok_or("generation_config required for composition mode")?;
    let compositions = cfg.get("compositions")
        .and_then(|v| v.as_array())
        .ok_or("generation_config.compositions must be an array")?;

    // Products whose quiz UI can't show a reading passage (UPKP) opt out of
    // passage soal; everyone else keeps them.
    let exclude_passages = cfg.get("exclude_passages").and_then(|v| v.as_bool()).unwrap_or(false);

    let mut rng = rand::thread_rng();
    let mut all_ids: Vec<i32> = Vec::with_capacity(sim.total_questions as usize);

    for comp in compositions {
        let node_id = comp.get("id").and_then(|v| v.as_str())
            .ok_or("composition.id required")?;
        let count = comp.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        if count == 0 { continue; }

        let mix = comp.get("difficulty_mix");
        let buckets = difficulty_buckets(mix, count);

        for (difficulty, bucket_count) in buckets {
            if bucket_count == 0 { continue; }
            // A soal mapped to two nodes of the same composition must not
            // appear twice in one attempt.
            let candidates: Vec<i32> = fetch_by_node(pool, mode_kind, node_id, &difficulty, exclude_passages)
                .await?
                .into_iter()
                .filter(|id| !all_ids.contains(id))
                .collect();
            if candidates.len() < bucket_count {
                return Err(format!(
                    "Not enough {} questions for node {} (need {}, found {})",
                    difficulty, node_id, bucket_count, candidates.len()
                ));
            }
            let mut shuffled = candidates;
            shuffled.shuffle(&mut rng);
            all_ids.extend_from_slice(&shuffled[..bucket_count]);
        }
    }

    if all_ids.len() < sim.total_questions as usize {
        return Err(format!(
            "Composition produced only {} questions, need {}",
            all_ids.len(), sim.total_questions
        ));
    }
    all_ids.truncate(sim.total_questions as usize);
    Ok(all_ids)
}

async fn fetch_by_node(
    pool: &MySqlPool,
    mode_kind: &str,
    node_id: &str,
    difficulty: &str,
    exclude_passages: bool,
) -> Result<Vec<i32>, String> {
    let node_filter = match mode_kind {
        "subcategory_id" => "s.subcategory_id = ?",
        "question_topics_join" => "EXISTS (SELECT 1 FROM question_topics qt WHERE qt.question_id = s.id AND qt.topic_id = ?)",
        _ => return Err(format!("Unknown mode_kind: {}", mode_kind)),
    };
    let sql = format!(
        "SELECT s.id FROM soal s WHERE s.status = 'active' AND {ANSWERABLE} AND {node_filter} \
         AND s.difficulty_est = ?{}",
        if exclude_passages { " AND s.passage_id IS NULL" } else { "" }
    );

    sqlx::query_scalar::<_, i32>(&sql)
        .bind(node_id)
        .bind(difficulty)
        .fetch_all(pool).await
        .map_err(|e| format!("Failed to fetch candidates: {}", e))
}

async fn from_random_pool(pool: &MySqlPool, sim: &ExamSimulation) -> Result<Vec<i32>, String> {
    let cfg = sim.generation_config.as_ref();
    let track_id = cfg.and_then(|c| c.get("track_id")).and_then(|v| v.as_str());
    let category_id = cfg.and_then(|c| c.get("category_id")).and_then(|v| v.as_str());
    let subcategory_id = cfg.and_then(|c| c.get("subcategory_id")).and_then(|v| v.as_str());
    let mix = cfg.and_then(|c| c.get("difficulty_mix"));

    let buckets = difficulty_buckets(mix, sim.total_questions as usize);

    let mut rng = rand::thread_rng();
    let mut all_ids: Vec<i32> = Vec::with_capacity(sim.total_questions as usize);

    for (difficulty, count) in buckets {
        if count == 0 { continue; }

        let mut conditions: Vec<&'static str> = vec!["s.status = 'active'", ANSWERABLE, "s.difficulty_est = ?"];
        let mut binds: Vec<String> = vec![difficulty.clone()];

        if let Some(t) = track_id {
            conditions.push("s.track_id = ?");
            binds.push(t.to_string());
        }
        if let Some(c) = category_id {
            conditions.push("s.category_id = ?");
            binds.push(c.to_string());
        }
        if let Some(sc) = subcategory_id {
            conditions.push("s.subcategory_id = ?");
            binds.push(sc.to_string());
        }

        let sql = format!("SELECT s.id FROM soal s WHERE {}", conditions.join(" AND "));
        let mut q = sqlx::query_scalar::<_, i32>(&sql);
        for b in &binds {
            q = q.bind(b);
        }
        let candidates = q.fetch_all(pool).await
            .map_err(|e| format!("Failed to query random pool: {}", e))?;

        if candidates.len() < count {
            return Err(format!(
                "Random pool: not enough {} questions (need {}, found {})",
                difficulty, count, candidates.len()
            ));
        }

        let mut shuffled = candidates;
        shuffled.shuffle(&mut rng);
        all_ids.extend_from_slice(&shuffled[..count]);
    }

    if all_ids.len() < sim.total_questions as usize {
        return Err(format!(
            "Random pool produced only {} questions, need {}",
            all_ids.len(), sim.total_questions
        ));
    }
    all_ids.truncate(sim.total_questions as usize);
    Ok(all_ids)
}

/// Resolve difficulty buckets. If `mix` is provided as {easy:N, medium:N, hard:N}, use it.
/// Otherwise default: split total evenly across medium (we don't have other info).
fn difficulty_buckets(mix: Option<&JsonValue>, total: usize) -> Vec<(String, usize)> {
    if let Some(m) = mix {
        let easy = m.get("easy").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let medium = m.get("medium").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let hard = m.get("hard").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        if easy + medium + hard > 0 {
            return vec![
                ("easy".to_string(), easy),
                ("medium".to_string(), medium),
                ("hard".to_string(), hard),
            ];
        }
    }
    // No mix specified — pull everything as 'medium' as the safe default.
    vec![("medium".to_string(), total)]
}
