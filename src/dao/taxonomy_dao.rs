use sqlx::MySqlPool;
use std::sync::Arc;
use crate::model::taxonomy::{
    ExamTrack, Category, Subcategory, Topic, Tag,
    TaxonomyTree, TrackWithChildren, CategoryWithChildren, SubcategoryWithChildren, TaxonomyContext,
};

pub struct TaxonomyDao {
    pool: Arc<MySqlPool>,
}

impl TaxonomyDao {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    pub async fn get_all_tracks(&self) -> Result<Vec<ExamTrack>, sqlx::Error> {
        sqlx::query_as::<_, ExamTrack>(r#"
            SELECT t.id, t.slug, t.name, t.icon, t.status, t.sort_order,
                   COUNT(s.id) AS question_count
            FROM exam_tracks t
            LEFT JOIN soal s ON s.track_id = t.id
            GROUP BY t.id, t.slug, t.name, t.icon, t.status, t.sort_order
            ORDER BY t.sort_order
        "#)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_categories_by_track_slug(&self, slug: &str) -> Result<Vec<Category>, sqlx::Error> {
        sqlx::query_as::<_, Category>(r#"
            SELECT c.id, c.track_id, c.slug, c.name, c.sort_order,
                   COUNT(s.id) AS question_count
            FROM categories c
            JOIN exam_tracks t ON t.id = c.track_id
            LEFT JOIN soal s ON s.category_id = c.id
            WHERE t.slug = ?
            GROUP BY c.id, c.track_id, c.slug, c.name, c.sort_order
            ORDER BY c.sort_order
        "#)
        .bind(slug)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_subcategories_by_category_slug(&self, slug: &str) -> Result<Vec<Subcategory>, sqlx::Error> {
        sqlx::query_as::<_, Subcategory>(r#"
            SELECT sc.id, sc.category_id, sc.slug, sc.name, sc.sort_order,
                   COUNT(s.id) AS question_count
            FROM subcategories sc
            JOIN categories c ON c.id = sc.category_id
            LEFT JOIN soal s ON s.subcategory_id = sc.id
            WHERE c.slug = ?
            GROUP BY sc.id, sc.category_id, sc.slug, sc.name, sc.sort_order
            ORDER BY sc.sort_order
        "#)
        .bind(slug)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_topics_by_subcategory_slug(&self, slug: &str) -> Result<Vec<Topic>, sqlx::Error> {
        sqlx::query_as::<_, Topic>(r#"
            SELECT tp.id, tp.subcategory_id, tp.slug, tp.name, tp.sort_order,
                   COUNT(s.id) AS question_count
            FROM topics tp
            JOIN subcategories sc ON sc.id = tp.subcategory_id
            LEFT JOIN soal s ON s.topic_id = tp.id
            WHERE sc.slug = ?
            GROUP BY tp.id, tp.subcategory_id, tp.slug, tp.name, tp.sort_order
            ORDER BY tp.sort_order
        "#)
        .bind(slug)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_tags_by_topic_slug(&self, slug: &str) -> Result<Vec<Tag>, sqlx::Error> {
        sqlx::query_as::<_, Tag>(r#"
            SELECT tg.id, tg.slug, tg.label
            FROM tags tg
            JOIN question_tags qt ON qt.tag_id = tg.id
            JOIN soal s ON s.id = qt.question_id
            JOIN topics tp ON tp.id = s.topic_id
            WHERE tp.slug = ?
            GROUP BY tg.id, tg.slug, tg.label
            ORDER BY tg.label
        "#)
        .bind(slug)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn search_tags(&self, search: &str) -> Result<Vec<Tag>, sqlx::Error> {
        let pattern = format!("%{}%", search);
        sqlx::query_as::<_, Tag>(r#"
            SELECT id, slug, label FROM tags
            WHERE slug LIKE ? OR label LIKE ?
            ORDER BY label
            LIMIT 30
        "#)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_tags_for_question(&self, question_id: i32) -> Result<Vec<Tag>, sqlx::Error> {
        sqlx::query_as::<_, Tag>(r#"
            SELECT tg.id, tg.slug, tg.label
            FROM tags tg
            JOIN question_tags qt ON qt.tag_id = tg.id
            WHERE qt.question_id = ?
            ORDER BY tg.label
        "#)
        .bind(question_id)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_taxonomy_context(&self, soal: &crate::model::Soal) -> TaxonomyContext {
        let track = if let Some(ref tid) = soal.track_id {
            sqlx::query_as::<_, ExamTrack>(
                "SELECT id, slug, name, icon, status, sort_order, 0 AS question_count FROM exam_tracks WHERE id = ?"
            )
            .bind(tid)
            .fetch_optional(&*self.pool)
            .await
            .unwrap_or(None)
        } else { None };

        let category = if let Some(ref cid) = soal.category_id {
            sqlx::query_as::<_, Category>(
                "SELECT id, track_id, slug, name, sort_order, 0 AS question_count FROM categories WHERE id = ?"
            )
            .bind(cid)
            .fetch_optional(&*self.pool)
            .await
            .unwrap_or(None)
        } else { None };

        let subcategory = if let Some(ref sid) = soal.subcategory_id {
            sqlx::query_as::<_, Subcategory>(
                "SELECT id, category_id, slug, name, sort_order, 0 AS question_count FROM subcategories WHERE id = ?"
            )
            .bind(sid)
            .fetch_optional(&*self.pool)
            .await
            .unwrap_or(None)
        } else { None };

        let topic = if let Some(ref tid) = soal.topic_id {
            sqlx::query_as::<_, Topic>(
                "SELECT id, subcategory_id, slug, name, sort_order, 0 AS question_count FROM topics WHERE id = ?"
            )
            .bind(tid)
            .fetch_optional(&*self.pool)
            .await
            .unwrap_or(None)
        } else { None };

        let tags = self.get_tags_for_question(soal.id).await.unwrap_or_default();

        TaxonomyContext { track, category, subcategory, topic, tags }
    }

    pub async fn get_taxonomy_tree(&self) -> Result<TaxonomyTree, sqlx::Error> {
        let tracks = self.get_all_tracks().await?;
        let mut tree_tracks = Vec::new();

        for track in tracks {
            let categories = self.get_categories_by_track_slug(&track.slug).await?;
            let mut cat_children = Vec::new();

            for cat in categories {
                let subcats = self.get_subcategories_by_category_slug(&cat.slug).await?;
                let mut subcat_children = Vec::new();

                for sub in subcats {
                    let topics = self.get_topics_by_subcategory_slug(&sub.slug).await?;
                    subcat_children.push(SubcategoryWithChildren {
                        subcategory: sub,
                        topics,
                    });
                }

                cat_children.push(CategoryWithChildren {
                    category: cat,
                    subcategories: subcat_children,
                });
            }

            tree_tracks.push(TrackWithChildren {
                track,
                categories: cat_children,
            });
        }

        Ok(TaxonomyTree { tracks: tree_tracks })
    }

    pub async fn set_question_tags(&self, question_id: i32, tag_ids: &[String]) -> Result<(), sqlx::Error> {
        // Delete existing tags for this question
        sqlx::query("DELETE FROM question_tags WHERE question_id = ?")
            .bind(question_id)
            .execute(&*self.pool)
            .await?;

        // Insert new tags
        for tag_id in tag_ids {
            sqlx::query("INSERT IGNORE INTO question_tags (question_id, tag_id) VALUES (?, ?)")
                .bind(question_id)
                .bind(tag_id)
                .execute(&*self.pool)
                .await?;
        }

        Ok(())
    }

    // ── AFD-226: question_topics M2M ─────────────────────────────────────────

    /// Return all topic IDs associated with a question via question_topics table
    pub async fn get_topics_for_question(&self, question_id: i32) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT topic_id FROM question_topics WHERE question_id = ? ORDER BY topic_id"
        )
        .bind(question_id)
        .fetch_all(&*self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Replace all topic associations for a question (delete + insert pattern, mirrors set_question_tags)
    pub async fn set_question_topics(&self, question_id: i32, topic_ids: &[String]) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM question_topics WHERE question_id = ?")
            .bind(question_id)
            .execute(&*self.pool)
            .await?;

        for topic_id in topic_ids {
            sqlx::query("INSERT IGNORE INTO question_topics (question_id, topic_id) VALUES (?, ?)")
                .bind(question_id)
                .bind(topic_id)
                .execute(&*self.pool)
                .await?;
        }

        Ok(())
    }
}
