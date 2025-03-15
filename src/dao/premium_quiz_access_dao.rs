use crate::dao::Table;
use crate::model::premium_quiz_access::{PremiumQuizAccess, PremiumQuizAccessResponse, CreatePremiumQuizAccessRequest, UpdatePremiumQuizAccessRequest};
use sqlx::{Error, Row};

impl<'c> Table<'c, PremiumQuizAccess> {
    pub async fn get_all_premium_quiz_access(&self) -> Result<Vec<PremiumQuizAccessResponse>, Error> {
        let rows = sqlx::query(
            r#"
            SELECT 
                pqa.id,
                pqa.paket_soal_id,
                ps.nama_paket_soal as paket_soal_name,
                pqa.min_plan_id,
                pp.name as min_plan_name
            FROM 
                dbquizapp.premium_quiz_access pqa
            JOIN 
                dbquizapp.paket_soal ps ON pqa.paket_soal_id = ps.id
            JOIN 
                dbquizapp.premium_plans pp ON pqa.min_plan_id = pp.id
            ORDER BY 
                ps.nama_paket_soal
            "#
        )
        .fetch_all(&*self.pool)
        .await?;
        
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            result.push(PremiumQuizAccessResponse {
                id: row.try_get("id")?,
                paket_soal_id: row.try_get("paket_soal_id")?,
                paket_soal_name: row.try_get("paket_soal_name")?,
                min_plan_id: row.try_get("min_plan_id")?,
                min_plan_name: row.try_get("min_plan_name")?,
            });
        }
        
        Ok(result)
    }

    pub async fn get_premium_quiz_access_by_id(&self, id: i32) -> Result<Option<PremiumQuizAccessResponse>, Error> {
        let row = sqlx::query(
            r#"
            SELECT 
                pqa.id,
                pqa.paket_soal_id,
                ps.nama_paket_soal as paket_soal_name,
                pqa.min_plan_id,
                pp.name as min_plan_name
            FROM 
                dbquizapp.premium_quiz_access pqa
            JOIN 
                dbquizapp.paket_soal ps ON pqa.paket_soal_id = ps.id
            JOIN 
                dbquizapp.premium_plans pp ON pqa.min_plan_id = pp.id
            WHERE 
                pqa.id = ?
            "#
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await?;
        
        if let Some(row) = row {
            Ok(Some(PremiumQuizAccessResponse {
                id: row.try_get("id")?,
                paket_soal_id: row.try_get("paket_soal_id")?,
                paket_soal_name: row.try_get("paket_soal_name")?,
                min_plan_id: row.try_get("min_plan_id")?,
                min_plan_name: row.try_get("min_plan_name")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_premium_quiz_access_by_paket_soal_id(&self, paket_soal_id: i32) -> Result<Option<PremiumQuizAccessResponse>, Error> {
        let row = sqlx::query(
            r#"
            SELECT 
                pqa.id,
                pqa.paket_soal_id,
                ps.nama_paket_soal as paket_soal_name,
                pqa.min_plan_id,
                pp.name as min_plan_name
            FROM 
                dbquizapp.premium_quiz_access pqa
            JOIN 
                dbquizapp.paket_soal ps ON pqa.paket_soal_id = ps.id
            JOIN 
                dbquizapp.premium_plans pp ON pqa.min_plan_id = pp.id
            WHERE 
                pqa.paket_soal_id = ?
            "#
        )
        .bind(paket_soal_id)
        .fetch_optional(&*self.pool)
        .await?;
        
        if let Some(row) = row {
            Ok(Some(PremiumQuizAccessResponse {
                id: row.try_get("id")?,
                paket_soal_id: row.try_get("paket_soal_id")?,
                paket_soal_name: row.try_get("paket_soal_name")?,
                min_plan_id: row.try_get("min_plan_id")?,
                min_plan_name: row.try_get("min_plan_name")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn create_premium_quiz_access(&self, access: &CreatePremiumQuizAccessRequest) -> Result<i32, Error> {
        let result = sqlx::query(
            "INSERT INTO dbquizapp.premium_quiz_access (paket_soal_id, min_plan_id) 
             VALUES (?, ?)"
        )
        .bind(access.paket_soal_id)
        .bind(access.min_plan_id)
        .execute(&*self.pool)
        .await?;

        Ok(result.last_insert_id() as i32)
    }

    pub async fn update_premium_quiz_access(&self, id: i32, access: &UpdatePremiumQuizAccessRequest) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE dbquizapp.premium_quiz_access SET min_plan_id = ? WHERE id = ?"
        )
        .bind(access.min_plan_id)
        .bind(id)
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_premium_quiz_access(&self, id: i32) -> Result<bool, Error> {
        let result = sqlx::query(
            "DELETE FROM dbquizapp.premium_quiz_access WHERE id = ?"
        )
        .bind(id)
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn check_user_access_to_quiz(&self, user_id: &str, paket_soal_id: i32) -> Result<bool, Error> {
        // First check if the quiz requires premium access
        let premium_access = self.get_premium_quiz_access_by_paket_soal_id(paket_soal_id).await?;
        
        // If no premium access required, everyone can access
        if premium_access.is_none() {
            return Ok(true);
        }
        
        let premium_access = premium_access.unwrap();
        
        // Check if user has an active subscription with sufficient plan level
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) 
            FROM dbquizapp.user_subscriptions us
            JOIN dbquizapp.premium_plans pp ON us.plan_id = pp.id
            WHERE 
                us.user_id = ? 
                AND us.status = 'active'
                AND (us.end_date IS NULL OR us.end_date > NOW())
                AND pp.id >= ?
            "#
        )
        .bind(user_id)
        .bind(premium_access.min_plan_id)
        .fetch_one(&*self.pool)
        .await?;
        
        Ok(count > 0)
    }
} 