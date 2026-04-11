use super::Table;
use crate::model::passage::{Passage, CreatePassageRequest, PassageDetail};
use sqlx::Error;

impl<'c> Table<'c, Passage> {
    pub async fn create_passage(&self, req: &CreatePassageRequest) -> Result<Passage, Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO passages (content, title, source, language)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&req.content)
        .bind(&req.title)
        .bind(&req.source)
        .bind(req.language.as_deref().unwrap_or("id"))
        .execute(&*self.pool)
        .await?;

        let id = result.last_insert_id();
        self.get_passage_by_id(id as i32).await
    }

    pub async fn get_passage_by_id(&self, id: i32) -> Result<Passage, Error> {
        sqlx::query_as::<_, Passage>(
            "SELECT * FROM passages WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await
    }

    pub async fn get_passage_detail(&self, id: i32) -> Result<PassageDetail, Error> {
        let passage = self.get_passage_by_id(id).await?;

        let soal_ids: Vec<i32> = sqlx::query_scalar(
            "SELECT id FROM soal WHERE passage_id = ? ORDER BY id",
        )
        .bind(id)
        .fetch_all(&*self.pool)
        .await?;

        let soal_count = soal_ids.len() as i64;

        Ok(PassageDetail {
            passage,
            soal_ids,
            soal_count,
        })
    }

    pub async fn update_passage(&self, id: i32, req: &CreatePassageRequest) -> Result<Passage, Error> {
        let result = sqlx::query(
            r#"
            UPDATE passages
            SET content = ?, title = ?, source = ?, language = ?, updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(&req.content)
        .bind(&req.title)
        .bind(&req.source)
        .bind(req.language.as_deref().unwrap_or("id"))
        .bind(id)
        .execute(&*self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(Error::RowNotFound);
        }

        self.get_passage_by_id(id).await
    }

    pub async fn delete_passage(&self, id: i32) -> Result<u64, Error> {
        // FK is ON DELETE SET NULL so soal.passage_id will be set to NULL automatically
        let result = sqlx::query("DELETE FROM passages WHERE id = ?")
            .bind(id)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    pub async fn list_passages(&self, page: u32, limit: u32) -> Result<(Vec<Passage>, i64), Error> {
        let offset = (page - 1) * limit;

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM passages")
            .fetch_one(&*self.pool)
            .await?;

        let passages = sqlx::query_as::<_, Passage>(
            "SELECT * FROM passages ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await?;

        Ok((passages, total))
    }
}
