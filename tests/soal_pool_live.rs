//! Read-only checks of the soal pool SQL against a real database.
//! Skipped by default; run with:
//!   SOAL_POOL_DB_URL=mysql://... cargo test --test soal_pool_live -- --ignored

use quiz_backend::dao::soal_pool_dao::{fetch_in_pool, sample_ids};
use sqlx::MySqlPool;

async fn pool() -> MySqlPool {
    let url = std::env::var("SOAL_POOL_DB_URL").expect("SOAL_POOL_DB_URL not set");
    MySqlPool::connect(&url).await.expect("connect")
}

#[tokio::test]
#[ignore]
async fn samples_are_random_servable_and_carry_the_upkp_topic() {
    let pool = pool().await;

    let tpa = sample_ids(&pool, "upkp", Some("upkp-tpa"), None, 9).await.unwrap();
    assert_eq!(tpa.len(), 9);
    let again = sample_ids(&pool, "upkp", Some("upkp-tpa"), None, 9).await.unwrap();
    assert_ne!(tpa, again, "two samples of 9 from ~9k soal should differ");

    let full = fetch_in_pool(&pool, "upkp", &tpa).await.unwrap();
    assert_eq!(full.iter().map(|s| s.id).collect::<Vec<_>>(), tpa, "order preserved");
    for s in &full {
        assert!(s.correct_answer.is_some());
        let topic = s.topic_name.as_deref().unwrap_or_default();
        assert!(topic.starts_with("TPA"), "soal {} topic {topic:?}", s.id);
    }

    let numerik = sample_ids(&pool, "upkp", None, Some("TPA Numerik"), 10).await.unwrap();
    assert_eq!(numerik.len(), 10);
    for s in fetch_in_pool(&pool, "upkp", &numerik).await.unwrap() {
        assert_eq!(s.topic_name.as_deref(), Some("TPA Numerik"), "soal {}", s.id);
    }

    assert!(sample_ids(&pool, "upkp", None, Some("Tidak Ada"), 10).await.unwrap().is_empty());
    assert!(sample_ids(&pool, "tidak-ada", None, None, 10).await.unwrap().is_empty());
}

#[tokio::test]
#[ignore]
async fn fetch_in_pool_drops_soal_outside_the_category() {
    let pool = pool().await;
    // Any soal that is not UPKP by FK or topic, e.g. an SNBT soal.
    let outside: i32 = sqlx::query_scalar(
        "SELECT s.id FROM soal s JOIN categories c ON c.id = s.category_id \
         WHERE c.slug <> 'upkp' AND NOT EXISTS (SELECT 1 FROM question_topics qt JOIN topics tp ON tp.id = qt.topic_id \
           JOIN subcategories sc ON sc.id = tp.subcategory_id JOIN categories c2 ON c2.id = sc.category_id \
           WHERE qt.question_id = s.id AND c2.slug = 'upkp') LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let inside = sample_ids(&pool, "upkp", None, None, 1).await.unwrap();
    let got = fetch_in_pool(&pool, "upkp", &[outside, inside[0]]).await.unwrap();
    assert_eq!(got.iter().map(|s| s.id).collect::<Vec<_>>(), vec![inside[0]]);
}
