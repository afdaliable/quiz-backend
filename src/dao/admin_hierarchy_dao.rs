use sqlx::MySqlPool;
use std::sync::Arc;
use crate::model::taxonomy::{ExamTrack, Category, Subcategory, Topic, Tag};

pub struct AdminHierarchyDao {
    pool: Arc<MySqlPool>,
}

impl AdminHierarchyDao {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ── slug helper ───────────────────────────────────────────────────────────

    fn to_slug(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    // ── Tracks ────────────────────────────────────────────────────────────────

    pub async fn get_tracks(&self) -> Result<Vec<ExamTrack>, sqlx::Error> {
        // AFD-226: count includes direct FK + questions via question_topics M2M
        sqlx::query_as::<_, ExamTrack>(r#"
            SELECT t.id, t.slug, t.name, t.icon, t.status, t.sort_order,
                   (SELECT COUNT(DISTINCT s.id) FROM soal s
                    WHERE s.track_id = t.id
                       OR s.id IN (
                           SELECT qt.question_id FROM question_topics qt
                           JOIN topics tp ON tp.id = qt.topic_id
                           JOIN subcategories sc ON sc.id = tp.subcategory_id
                           JOIN categories c ON c.id = sc.category_id
                           WHERE c.track_id = t.id
                       )
                   ) AS question_count
            FROM exam_tracks t
            ORDER BY t.sort_order
        "#)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_track_by_id(&self, id: &str) -> Result<Option<ExamTrack>, sqlx::Error> {
        sqlx::query_as::<_, ExamTrack>(r#"
            SELECT id, slug, name, icon, status, sort_order, 0 AS question_count
            FROM exam_tracks WHERE id = ?
        "#)
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn create_track(
        &self,
        name: &str,
        slug: Option<&str>,
        icon: Option<&str>,
        sort_order: i32,
        status: &str,
    ) -> Result<ExamTrack, sqlx::Error> {
        let slug = slug.map(|s| s.to_string()).unwrap_or_else(|| Self::to_slug(name));

        sqlx::query(r#"
            INSERT INTO exam_tracks (name, slug, icon, sort_order, status)
            VALUES (?, ?, ?, ?, ?)
        "#)
        .bind(name)
        .bind(&slug)
        .bind(icon)
        .bind(sort_order)
        .bind(status)
        .execute(&*self.pool)
        .await?;

        let track = sqlx::query_as::<_, ExamTrack>(r#"
            SELECT id, slug, name, icon, status, sort_order, 0 AS question_count
            FROM exam_tracks WHERE slug = ?
        "#)
        .bind(&slug)
        .fetch_one(&*self.pool)
        .await?;

        Ok(track)
    }

    pub async fn update_track(
        &self,
        id: &str,
        name: Option<&str>,
        slug: Option<&str>,
        icon: Option<&str>,
        sort_order: Option<i32>,
        status: Option<&str>,
    ) -> Result<Option<ExamTrack>, sqlx::Error> {
        // Build dynamic SET clause
        let mut sets: Vec<String> = vec![];
        if name.is_some() { sets.push("name = ?".into()); }
        if slug.is_some() { sets.push("slug = ?".into()); }
        sets.push("icon = ?".into()); // always update icon (allows clearing)
        if sort_order.is_some() { sets.push("sort_order = ?".into()); }
        if status.is_some() { sets.push("status = ?".into()); }

        if sets.is_empty() {
            return self.get_track_by_id(id).await;
        }

        let sql = format!("UPDATE exam_tracks SET {} WHERE id = ?", sets.join(", "));
        let mut q = sqlx::query(&sql);
        if let Some(v) = name { q = q.bind(v); }
        if let Some(v) = slug { q = q.bind(v); }
        q = q.bind(icon); // icon can be None to clear
        if let Some(v) = sort_order { q = q.bind(v); }
        if let Some(v) = status { q = q.bind(v); }
        q = q.bind(id);

        q.execute(&*self.pool).await?;
        self.get_track_by_id(id).await
    }

    pub async fn delete_track(&self, id: &str, cascade: bool) -> Result<bool, String> {
        // Check for child categories
        let child_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories WHERE track_id = ?")
            .bind(id)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| e.to_string())?;

        if child_count > 0 && !cascade {
            return Err(format!("Track has {} categories. Use cascade=true to force delete.", child_count));
        }

        if cascade {
            // cascade: subcategories → topics first
            sqlx::query(r#"
                DELETE tp FROM topics tp
                JOIN subcategories sc ON sc.id = tp.subcategory_id
                JOIN categories c ON c.id = sc.category_id
                WHERE c.track_id = ?
            "#)
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| e.to_string())?;

            sqlx::query(r#"
                DELETE sc FROM subcategories sc
                JOIN categories c ON c.id = sc.category_id
                WHERE c.track_id = ?
            "#)
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| e.to_string())?;

            sqlx::query("DELETE FROM categories WHERE track_id = ?")
                .bind(id)
                .execute(&*self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        let result = sqlx::query("DELETE FROM exam_tracks WHERE id = ?")
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn reorder_tracks(&self, ids: &[String]) -> Result<(), sqlx::Error> {
        for (i, id) in ids.iter().enumerate() {
            sqlx::query("UPDATE exam_tracks SET sort_order = ? WHERE id = ?")
                .bind(i as i32)
                .bind(id)
                .execute(&*self.pool)
                .await?;
        }
        Ok(())
    }

    // ── Categories ────────────────────────────────────────────────────────────

    pub async fn get_categories(&self, track_id: Option<&str>) -> Result<Vec<Category>, sqlx::Error> {
        if let Some(tid) = track_id {
            sqlx::query_as::<_, Category>(r#"
                SELECT c.id, c.track_id, c.slug, c.name, c.sort_order,
                       COUNT(s.id) AS question_count
                FROM categories c
                LEFT JOIN soal s ON s.category_id = c.id
                WHERE c.track_id = ?
                GROUP BY c.id, c.track_id, c.slug, c.name, c.sort_order
                ORDER BY c.sort_order
            "#)
            .bind(tid)
            .fetch_all(&*self.pool)
            .await
        } else {
            sqlx::query_as::<_, Category>(r#"
                SELECT c.id, c.track_id, c.slug, c.name, c.sort_order,
                       COUNT(s.id) AS question_count
                FROM categories c
                LEFT JOIN soal s ON s.category_id = c.id
                GROUP BY c.id, c.track_id, c.slug, c.name, c.sort_order
                ORDER BY c.sort_order
            "#)
            .fetch_all(&*self.pool)
            .await
        }
    }

    pub async fn get_category_by_id(&self, id: &str) -> Result<Option<Category>, sqlx::Error> {
        sqlx::query_as::<_, Category>(r#"
            SELECT id, track_id, slug, name, sort_order, 0 AS question_count
            FROM categories WHERE id = ?
        "#)
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn create_category(
        &self,
        track_id: &str,
        name: &str,
        slug: Option<&str>,
        sort_order: i32,
    ) -> Result<Category, sqlx::Error> {
        let slug = slug.map(|s| s.to_string()).unwrap_or_else(|| Self::to_slug(name));

        sqlx::query("INSERT INTO categories (track_id, name, slug, sort_order) VALUES (?, ?, ?, ?)")
            .bind(track_id)
            .bind(name)
            .bind(&slug)
            .bind(sort_order)
            .execute(&*self.pool)
            .await?;

        let cat = sqlx::query_as::<_, Category>(r#"
            SELECT id, track_id, slug, name, sort_order, 0 AS question_count
            FROM categories WHERE track_id = ? AND slug = ?
        "#)
        .bind(track_id)
        .bind(&slug)
        .fetch_one(&*self.pool)
        .await?;

        Ok(cat)
    }

    pub async fn update_category(
        &self,
        id: &str,
        track_id: Option<&str>,
        name: Option<&str>,
        slug: Option<&str>,
        sort_order: Option<i32>,
    ) -> Result<Option<Category>, sqlx::Error> {
        let mut sets: Vec<String> = vec![];
        if track_id.is_some() { sets.push("track_id = ?".into()); }
        if name.is_some() { sets.push("name = ?".into()); }
        if slug.is_some() { sets.push("slug = ?".into()); }
        if sort_order.is_some() { sets.push("sort_order = ?".into()); }

        if !sets.is_empty() {
            let sql = format!("UPDATE categories SET {} WHERE id = ?", sets.join(", "));
            let mut q = sqlx::query(&sql);
            if let Some(v) = track_id { q = q.bind(v); }
            if let Some(v) = name { q = q.bind(v); }
            if let Some(v) = slug { q = q.bind(v); }
            if let Some(v) = sort_order { q = q.bind(v); }
            q = q.bind(id);
            q.execute(&*self.pool).await?;
        }

        self.get_category_by_id(id).await
    }

    pub async fn delete_category(&self, id: &str, cascade: bool) -> Result<bool, String> {
        let child_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM subcategories WHERE category_id = ?")
            .bind(id)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| e.to_string())?;

        if child_count > 0 && !cascade {
            return Err(format!("Category has {} subcategories. Use cascade=true to force delete.", child_count));
        }

        if cascade {
            sqlx::query(r#"
                DELETE tp FROM topics tp
                JOIN subcategories sc ON sc.id = tp.subcategory_id
                WHERE sc.category_id = ?
            "#)
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| e.to_string())?;

            sqlx::query("DELETE FROM subcategories WHERE category_id = ?")
                .bind(id)
                .execute(&*self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        let result = sqlx::query("DELETE FROM categories WHERE id = ?")
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn reorder_categories(&self, track_id: &str, ids: &[String]) -> Result<(), sqlx::Error> {
        for (i, id) in ids.iter().enumerate() {
            sqlx::query("UPDATE categories SET sort_order = ? WHERE id = ? AND track_id = ?")
                .bind(i as i32)
                .bind(id)
                .bind(track_id)
                .execute(&*self.pool)
                .await?;
        }
        Ok(())
    }

    // ── Subcategories ─────────────────────────────────────────────────────────

    pub async fn get_subcategories(&self, category_id: Option<&str>) -> Result<Vec<Subcategory>, sqlx::Error> {
        if let Some(cid) = category_id {
            sqlx::query_as::<_, Subcategory>(r#"
                SELECT sc.id, sc.category_id, sc.slug, sc.name, sc.sort_order,
                       COUNT(s.id) AS question_count
                FROM subcategories sc
                LEFT JOIN soal s ON s.subcategory_id = sc.id
                WHERE sc.category_id = ?
                GROUP BY sc.id, sc.category_id, sc.slug, sc.name, sc.sort_order
                ORDER BY sc.sort_order
            "#)
            .bind(cid)
            .fetch_all(&*self.pool)
            .await
        } else {
            sqlx::query_as::<_, Subcategory>(r#"
                SELECT sc.id, sc.category_id, sc.slug, sc.name, sc.sort_order,
                       COUNT(s.id) AS question_count
                FROM subcategories sc
                LEFT JOIN soal s ON s.subcategory_id = sc.id
                GROUP BY sc.id, sc.category_id, sc.slug, sc.name, sc.sort_order
                ORDER BY sc.sort_order
            "#)
            .fetch_all(&*self.pool)
            .await
        }
    }

    pub async fn get_subcategory_by_id(&self, id: &str) -> Result<Option<Subcategory>, sqlx::Error> {
        sqlx::query_as::<_, Subcategory>(r#"
            SELECT id, category_id, slug, name, sort_order, 0 AS question_count
            FROM subcategories WHERE id = ?
        "#)
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn create_subcategory(
        &self,
        category_id: &str,
        name: &str,
        slug: Option<&str>,
        sort_order: i32,
    ) -> Result<Subcategory, sqlx::Error> {
        let slug = slug.map(|s| s.to_string()).unwrap_or_else(|| Self::to_slug(name));

        sqlx::query("INSERT INTO subcategories (category_id, name, slug, sort_order) VALUES (?, ?, ?, ?)")
            .bind(category_id)
            .bind(name)
            .bind(&slug)
            .bind(sort_order)
            .execute(&*self.pool)
            .await?;

        let sub = sqlx::query_as::<_, Subcategory>(r#"
            SELECT id, category_id, slug, name, sort_order, 0 AS question_count
            FROM subcategories WHERE category_id = ? AND slug = ?
        "#)
        .bind(category_id)
        .bind(&slug)
        .fetch_one(&*self.pool)
        .await?;

        Ok(sub)
    }

    pub async fn update_subcategory(
        &self,
        id: &str,
        category_id: Option<&str>,
        name: Option<&str>,
        slug: Option<&str>,
        sort_order: Option<i32>,
    ) -> Result<Option<Subcategory>, sqlx::Error> {
        let mut sets: Vec<String> = vec![];
        if category_id.is_some() { sets.push("category_id = ?".into()); }
        if name.is_some() { sets.push("name = ?".into()); }
        if slug.is_some() { sets.push("slug = ?".into()); }
        if sort_order.is_some() { sets.push("sort_order = ?".into()); }

        if !sets.is_empty() {
            let sql = format!("UPDATE subcategories SET {} WHERE id = ?", sets.join(", "));
            let mut q = sqlx::query(&sql);
            if let Some(v) = category_id { q = q.bind(v); }
            if let Some(v) = name { q = q.bind(v); }
            if let Some(v) = slug { q = q.bind(v); }
            if let Some(v) = sort_order { q = q.bind(v); }
            q = q.bind(id);
            q.execute(&*self.pool).await?;
        }

        self.get_subcategory_by_id(id).await
    }

    pub async fn delete_subcategory(&self, id: &str, cascade: bool) -> Result<bool, String> {
        let child_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM topics WHERE subcategory_id = ?")
            .bind(id)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| e.to_string())?;

        if child_count > 0 && !cascade {
            return Err(format!("Subcategory has {} topics. Use cascade=true to force delete.", child_count));
        }

        if cascade {
            sqlx::query("DELETE FROM topics WHERE subcategory_id = ?")
                .bind(id)
                .execute(&*self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        let result = sqlx::query("DELETE FROM subcategories WHERE id = ?")
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(result.rows_affected() > 0)
    }

    // ── Topics ────────────────────────────────────────────────────────────────

    pub async fn get_topics(&self, subcategory_id: Option<&str>) -> Result<Vec<Topic>, sqlx::Error> {
        // AFD-226: count includes direct FK + questions via question_topics M2M
        if let Some(sid) = subcategory_id {
            sqlx::query_as::<_, Topic>(r#"
                SELECT tp.id, tp.subcategory_id, tp.slug, tp.name, tp.sort_order,
                       (SELECT COUNT(DISTINCT s.id) FROM soal s
                        WHERE s.topic_id = tp.id
                           OR s.id IN (SELECT question_id FROM question_topics WHERE topic_id = tp.id)
                       ) AS question_count
                FROM topics tp
                WHERE tp.subcategory_id = ?
                ORDER BY tp.sort_order
            "#)
            .bind(sid)
            .fetch_all(&*self.pool)
            .await
        } else {
            sqlx::query_as::<_, Topic>(r#"
                SELECT tp.id, tp.subcategory_id, tp.slug, tp.name, tp.sort_order,
                       (SELECT COUNT(DISTINCT s.id) FROM soal s
                        WHERE s.topic_id = tp.id
                           OR s.id IN (SELECT question_id FROM question_topics WHERE topic_id = tp.id)
                       ) AS question_count
                FROM topics tp
                ORDER BY tp.sort_order
            "#)
            .fetch_all(&*self.pool)
            .await
        }
    }

    pub async fn get_topic_by_id(&self, id: &str) -> Result<Option<Topic>, sqlx::Error> {
        sqlx::query_as::<_, Topic>(r#"
            SELECT id, subcategory_id, slug, name, sort_order, 0 AS question_count
            FROM topics WHERE id = ?
        "#)
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn create_topic(
        &self,
        subcategory_id: &str,
        name: &str,
        slug: Option<&str>,
        sort_order: i32,
    ) -> Result<Topic, sqlx::Error> {
        let slug = slug.map(|s| s.to_string()).unwrap_or_else(|| Self::to_slug(name));

        sqlx::query("INSERT INTO topics (subcategory_id, name, slug, sort_order) VALUES (?, ?, ?, ?)")
            .bind(subcategory_id)
            .bind(name)
            .bind(&slug)
            .bind(sort_order)
            .execute(&*self.pool)
            .await?;

        let topic = sqlx::query_as::<_, Topic>(r#"
            SELECT id, subcategory_id, slug, name, sort_order, 0 AS question_count
            FROM topics WHERE subcategory_id = ? AND slug = ?
        "#)
        .bind(subcategory_id)
        .bind(&slug)
        .fetch_one(&*self.pool)
        .await?;

        Ok(topic)
    }

    pub async fn update_topic(
        &self,
        id: &str,
        subcategory_id: Option<&str>,
        name: Option<&str>,
        slug: Option<&str>,
        sort_order: Option<i32>,
    ) -> Result<Option<Topic>, sqlx::Error> {
        let mut sets: Vec<String> = vec![];
        if subcategory_id.is_some() { sets.push("subcategory_id = ?".into()); }
        if name.is_some() { sets.push("name = ?".into()); }
        if slug.is_some() { sets.push("slug = ?".into()); }
        if sort_order.is_some() { sets.push("sort_order = ?".into()); }

        if !sets.is_empty() {
            let sql = format!("UPDATE topics SET {} WHERE id = ?", sets.join(", "));
            let mut q = sqlx::query(&sql);
            if let Some(v) = subcategory_id { q = q.bind(v); }
            if let Some(v) = name { q = q.bind(v); }
            if let Some(v) = slug { q = q.bind(v); }
            if let Some(v) = sort_order { q = q.bind(v); }
            q = q.bind(id);
            q.execute(&*self.pool).await?;
        }

        self.get_topic_by_id(id).await
    }

    pub async fn delete_topic(&self, id: &str) -> Result<bool, String> {
        let soal_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM soal WHERE topic_id = ?")
            .bind(id)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| e.to_string())?;

        if soal_count > 0 {
            return Err(format!("Topic has {} questions. Reassign questions before deleting.", soal_count));
        }

        let result = sqlx::query("DELETE FROM topics WHERE id = ?")
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(result.rows_affected() > 0)
    }

    // ── Tags ──────────────────────────────────────────────────────────────────

    pub async fn get_tag_by_id(&self, id: &str) -> Result<Option<Tag>, sqlx::Error> {
        sqlx::query_as::<_, Tag>("SELECT id, slug, label FROM tags WHERE id = ?")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await
    }

    pub async fn create_tag(&self, label: &str, slug: Option<&str>) -> Result<Tag, sqlx::Error> {
        let slug = slug.map(|s| s.to_string()).unwrap_or_else(|| Self::to_slug(label));

        sqlx::query("INSERT INTO tags (label, slug) VALUES (?, ?)")
            .bind(label)
            .bind(&slug)
            .execute(&*self.pool)
            .await?;

        sqlx::query_as::<_, Tag>("SELECT id, slug, label FROM tags WHERE slug = ?")
            .bind(&slug)
            .fetch_one(&*self.pool)
            .await
    }

    pub async fn update_tag(
        &self,
        id: &str,
        label: Option<&str>,
        slug: Option<&str>,
    ) -> Result<Option<Tag>, sqlx::Error> {
        let mut sets: Vec<String> = vec![];
        if label.is_some() { sets.push("label = ?".into()); }
        if slug.is_some() { sets.push("slug = ?".into()); }

        if !sets.is_empty() {
            let sql = format!("UPDATE tags SET {} WHERE id = ?", sets.join(", "));
            let mut q = sqlx::query(&sql);
            if let Some(v) = label { q = q.bind(v); }
            if let Some(v) = slug { q = q.bind(v); }
            q = q.bind(id);
            q.execute(&*self.pool).await?;
        }

        self.get_tag_by_id(id).await
    }

    pub async fn delete_tag(&self, id: &str) -> Result<bool, sqlx::Error> {
        sqlx::query("DELETE FROM question_tags WHERE tag_id = ?")
            .bind(id)
            .execute(&*self.pool)
            .await?;

        let result = sqlx::query("DELETE FROM tags WHERE id = ?")
            .bind(id)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn merge_tags(&self, source_ids: &[String], target_id: &str) -> Result<Option<Tag>, sqlx::Error> {
        for source_id in source_ids {
            // Re-assign question_tags from source to target, avoiding duplicates
            sqlx::query(r#"
                UPDATE question_tags SET tag_id = ?
                WHERE tag_id = ?
                AND question_id NOT IN (
                    SELECT question_id FROM (
                        SELECT question_id FROM question_tags WHERE tag_id = ?
                    ) AS existing
                )
            "#)
            .bind(target_id)
            .bind(source_id)
            .bind(target_id)
            .execute(&*self.pool)
            .await?;

            // Remove any remaining orphan entries from source
            sqlx::query("DELETE FROM question_tags WHERE tag_id = ?")
                .bind(source_id)
                .execute(&*self.pool)
                .await?;

            // Delete the source tag
            sqlx::query("DELETE FROM tags WHERE id = ?")
                .bind(source_id)
                .execute(&*self.pool)
                .await?;
        }

        self.get_tag_by_id(target_id).await
    }

    // ── Bulk update questions ─────────────────────────────────────────────────

    pub async fn bulk_update_questions(
        &self,
        question_ids: &[i32],
        track_id: Option<&str>,
        category_id: Option<&str>,
        subcategory_id: Option<&str>,
        topic_id: Option<&str>,
        difficulty_est: Option<&str>,
        status: Option<&str>,
    ) -> Result<u64, sqlx::Error> {
        let mut sets: Vec<String> = vec![];
        if track_id.is_some() { sets.push("track_id = ?".into()); }
        if category_id.is_some() { sets.push("category_id = ?".into()); }
        if subcategory_id.is_some() { sets.push("subcategory_id = ?".into()); }
        if topic_id.is_some() { sets.push("topic_id = ?".into()); }
        if difficulty_est.is_some() { sets.push("difficulty_est = ?".into()); }
        if status.is_some() { sets.push("status = ?".into()); }

        if sets.is_empty() || question_ids.is_empty() {
            return Ok(0);
        }

        let placeholders = question_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "UPDATE soal SET {} WHERE id IN ({})",
            sets.join(", "),
            placeholders
        );

        let mut q = sqlx::query(&sql);
        if let Some(v) = track_id { q = q.bind(v); }
        if let Some(v) = category_id { q = q.bind(v); }
        if let Some(v) = subcategory_id { q = q.bind(v); }
        if let Some(v) = topic_id { q = q.bind(v); }
        if let Some(v) = difficulty_est { q = q.bind(v); }
        if let Some(v) = status { q = q.bind(v); }
        for id in question_ids {
            q = q.bind(*id);
        }

        let result = q.execute(&*self.pool).await?;
        Ok(result.rows_affected())
    }

    // ── Stats ─────────────────────────────────────────────────────────────────

    pub async fn get_question_stats(&self) -> Result<QuestionStats, sqlx::Error> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM soal")
            .fetch_one(&*self.pool)
            .await?;

        let by_status: Vec<(String, i64)> = sqlx::query_as::<_, (String, i64)>(
            "SELECT status, COUNT(*) FROM soal GROUP BY status"
        )
        .fetch_all(&*self.pool)
        .await?;

        let by_difficulty: Vec<(String, i64)> = sqlx::query_as::<_, (String, i64)>(
            "SELECT difficulty_est, COUNT(*) FROM soal GROUP BY difficulty_est"
        )
        .fetch_all(&*self.pool)
        .await?;

        let by_track: Vec<(Option<String>, i64)> = sqlx::query_as::<_, (Option<String>, i64)>(r#"
            SELECT t.name, COUNT(s.id)
            FROM soal s
            LEFT JOIN exam_tracks t ON t.id = s.track_id
            GROUP BY t.name
        "#)
        .fetch_all(&*self.pool)
        .await?;

        let unclassified: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM soal WHERE track_id IS NULL"
        )
        .fetch_one(&*self.pool)
        .await?;

        let by_category: Vec<(Option<String>, i64)> = sqlx::query_as::<_, (Option<String>, i64)>(r#"
            SELECT c.name, COUNT(s.id)
            FROM soal s
            LEFT JOIN exam_categories c ON c.id = s.category_id
            GROUP BY c.name
            ORDER BY COUNT(s.id) DESC
        "#)
        .fetch_all(&*self.pool)
        .await?;

        let recent: Vec<RecentQuestion> = sqlx::query_as::<_, RecentQuestion>(r#"
            SELECT id, SUBSTRING(soal, 1, 80) as soal_snippet, status, COALESCE(track_id, '') as track_id, created_at
            FROM soal
            ORDER BY created_at DESC
            LIMIT 10
        "#)
        .fetch_all(&*self.pool)
        .await?;

        let anomalies: Vec<AnomalyQuestion> = sqlx::query_as::<_, AnomalyQuestion>(r#"
            SELECT id, SUBSTRING(soal, 1, 80) as soal_snippet, difficulty_est, COALESCE(difficulty_calc, '') as difficulty_calc
            FROM soal
            WHERE difficulty_calc IS NOT NULL AND difficulty_calc != difficulty_est
            LIMIT 20
        "#)
        .fetch_all(&*self.pool)
        .await?;

        let coverage: Vec<CoverageEntry> = sqlx::query_as::<_, CoverageEntry>(r#"
            SELECT t.id as topic_id, t.name as topic_name,
                SUM(CASE WHEN s.difficulty_est = 'easy' THEN 1 ELSE 0 END) as easy,
                SUM(CASE WHEN s.difficulty_est = 'medium' THEN 1 ELSE 0 END) as medium,
                SUM(CASE WHEN s.difficulty_est = 'hard' THEN 1 ELSE 0 END) as hard
            FROM exam_topics t
            LEFT JOIN soal s ON s.topic_id = t.id
            GROUP BY t.id, t.name
            ORDER BY (easy + medium + hard) ASC
            LIMIT 30
        "#)
        .fetch_all(&*self.pool)
        .await?;

        Ok(QuestionStats {
            total,
            by_status: by_status.into_iter().map(|(k, v)| StatEntry { label: k, count: v }).collect(),
            by_difficulty: by_difficulty.into_iter().map(|(k, v)| StatEntry { label: k, count: v }).collect(),
            by_track: by_track.into_iter().map(|(k, v)| StatEntry {
                label: k.unwrap_or_else(|| "Uncategorized".to_string()),
                count: v,
            }).collect(),
            unclassified,
            by_category: by_category.into_iter().map(|(k, v)| StatEntry {
                label: k.unwrap_or_else(|| "Uncategorized".to_string()),
                count: v,
            }).collect(),
            recent,
            anomalies,
            coverage,
        })
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct QuestionStats {
    pub total: i64,
    pub by_status: Vec<StatEntry>,
    pub by_difficulty: Vec<StatEntry>,
    pub by_track: Vec<StatEntry>,
    pub unclassified: i64,
    pub by_category: Vec<StatEntry>,
    pub recent: Vec<RecentQuestion>,
    pub anomalies: Vec<AnomalyQuestion>,
    pub coverage: Vec<CoverageEntry>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct StatEntry {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct RecentQuestion {
    pub id: i64,
    pub soal_snippet: String,
    pub status: String,
    pub track_id: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl<'c> sqlx::FromRow<'c, sqlx::mysql::MySqlRow> for RecentQuestion {
    fn from_row(row: &'c sqlx::mysql::MySqlRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(RecentQuestion {
            id: row.get("id"),
            soal_snippet: row.get("soal_snippet"),
            status: row.get("status"),
            track_id: row.get("track_id"),
            created_at: row.try_get("created_at").ok(),
        })
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AnomalyQuestion {
    pub id: i64,
    pub soal_snippet: String,
    pub difficulty_est: String,
    pub difficulty_calc: String,
}

impl<'c> sqlx::FromRow<'c, sqlx::mysql::MySqlRow> for AnomalyQuestion {
    fn from_row(row: &'c sqlx::mysql::MySqlRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(AnomalyQuestion {
            id: row.get("id"),
            soal_snippet: row.get("soal_snippet"),
            difficulty_est: row.get("difficulty_est"),
            difficulty_calc: row.get("difficulty_calc"),
        })
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct CoverageEntry {
    pub topic_id: String,
    pub topic_name: String,
    pub easy: i64,
    pub medium: i64,
    pub hard: i64,
}

impl<'c> sqlx::FromRow<'c, sqlx::mysql::MySqlRow> for CoverageEntry {
    fn from_row(row: &'c sqlx::mysql::MySqlRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(CoverageEntry {
            topic_id: row.get("topic_id"),
            topic_name: row.get("topic_name"),
            easy: row.get("easy"),
            medium: row.get("medium"),
            hard: row.get("hard"),
        })
    }
}
