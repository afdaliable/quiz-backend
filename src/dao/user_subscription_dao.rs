use crate::dao::Table;
use crate::model::user_subscription::{UserSubscription, UserSubscriptionWithPlan, CreateUserSubscriptionRequest, UpdateUserSubscriptionRequest, UserSubscriptionResponse};
use chrono::{DateTime, Duration, Utc};
use sqlx::{Error, Row};

impl<'c> Table<'c, UserSubscription> {
    pub async fn get_user_subscriptions(&self, user_id: &str) -> Result<Vec<UserSubscription>, Error> {
        sqlx::query_as::<_, UserSubscription>(
            "SELECT * FROM dbquizapp.user_subscriptions WHERE user_id = ? ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_user_subscription_by_id(&self, id: i32) -> Result<Option<UserSubscription>, Error> {
        sqlx::query_as::<_, UserSubscription>(
            "SELECT * FROM dbquizapp.user_subscriptions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn get_active_subscription(&self, user_id: &str) -> Result<Option<UserSubscriptionWithPlan>, Error> {
        let row = sqlx::query(
            r#"
            SELECT 
                us.id, 
                us.user_id, 
                us.plan_id, 
                pp.name as plan_name, 
                us.start_date, 
                us.end_date, 
                us.status as status, 
                pp.is_lifetime
            FROM 
                dbquizapp.user_subscriptions us
            JOIN 
                dbquizapp.premium_plans pp ON us.plan_id = pp.id
            WHERE 
                us.user_id = ? 
                AND us.status = 'active'
                AND (us.end_date IS NULL OR us.end_date > NOW())
            ORDER BY 
                us.created_at DESC
            LIMIT 1
            "#
        )
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await?;
        
        if let Some(row) = row {
            let is_lifetime: i32 = row.try_get("is_lifetime")?;
            
            Ok(Some(UserSubscriptionWithPlan {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                plan_id: row.try_get("plan_id")?,
                plan_name: row.try_get("plan_name")?,
                start_date: row.try_get("start_date")?,
                end_date: row.try_get("end_date")?,
                status: row.try_get("status")?,
                is_lifetime: is_lifetime != 0,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn create_subscription(&self, subscription: &CreateUserSubscriptionRequest) -> Result<i32, Error> {
        // Get the plan details to determine end_date
        let plan_result = sqlx::query_as::<_, (i32, i32)>(
            "SELECT duration_days, is_lifetime FROM dbquizapp.premium_plans WHERE id = ?"
        )
        .bind(subscription.plan_id)
        .fetch_optional(&*self.pool)
        .await?;
        
        // Default values if plan not found (30 days, not lifetime)
        let (duration_days, is_lifetime) = plan_result.unwrap_or((30, 0));

        let end_date = if is_lifetime != 0 {
            None
        } else {
            Some(Utc::now() + Duration::days(duration_days as i64))
        };

        let result = sqlx::query(
            "INSERT INTO dbquizapp.user_subscriptions (user_id, plan_id, start_date, end_date, status) 
             VALUES (?, ?, NOW(), ?, 'active')"
        )
        .bind(&subscription.user_id)
        .bind(subscription.plan_id)
        .bind(end_date)
        .execute(&*self.pool)
        .await?;

        Ok(result.last_insert_id() as i32)
    }

    pub async fn update_subscription(&self, id: i32, update: &UpdateUserSubscriptionRequest) -> Result<bool, Error> {
        let mut query = "UPDATE dbquizapp.user_subscriptions SET ".to_string();
        let mut has_updates = false;

        if let Some(status) = &update.status {
            query.push_str("status = ?");
            has_updates = true;
        }

        if let Some(end_date) = update.end_date {
            if has_updates {
                query.push_str(", ");
            }
            query.push_str("end_date = ?");
            has_updates = true;
        }

        if !has_updates {
            return Ok(false);
        }

        query.push_str(" WHERE id = ?");

        let mut query_builder = sqlx::query(&query);

        if let Some(status) = &update.status {
            query_builder = query_builder.bind(status);
        }

        if let Some(end_date) = update.end_date {
            query_builder = query_builder.bind(end_date);
        }

        query_builder = query_builder.bind(id);

        let result = query_builder.execute(&*self.pool).await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn cancel_subscription(&self, id: i32) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE dbquizapp.user_subscriptions SET status = 'cancelled' WHERE id = ?"
        )
        .bind(id)
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn update_expired_subscriptions(&self) -> Result<u64, Error> {
        let result = sqlx::query(
            "UPDATE dbquizapp.user_subscriptions 
             SET status = 'expired' 
             WHERE status = 'active' 
             AND end_date IS NOT NULL 
             AND end_date < NOW()"
        )
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn has_active_subscription(&self, user_id: &str) -> Result<bool, Error> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM dbquizapp.user_subscriptions 
             WHERE user_id = ? 
             AND status = 'active' 
             AND (end_date IS NULL OR end_date > NOW())"
        )
        .bind(user_id)
        .fetch_one(&*self.pool)
        .await?;

        Ok(count > 0)
    }

    pub async fn get_user_subscriptions_by_user_id(&self, user_id: &str) -> Result<Vec<UserSubscriptionResponse>, Error> {
        let rows = sqlx::query(
            r#"
            SELECT 
                us.id,
                us.user_id,
                us.plan_id,
                pp.name as plan_name,
                us.status,
                us.start_date,
                us.end_date,
                pp.is_lifetime
            FROM 
                dbquizapp.user_subscriptions us
            JOIN 
                dbquizapp.premium_plans pp ON us.plan_id = pp.id
            WHERE 
                us.user_id = ?
            ORDER BY 
                us.created_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await?;
        
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let is_lifetime: i32 = row.try_get("is_lifetime")?;
            let end_date: Option<DateTime<Utc>> = row.try_get("end_date")?;
            
            // Calculate days remaining
            let days_remaining = if let Some(end_date) = end_date {
                let now = Utc::now();
                if end_date > now {
                    Some((end_date - now).num_days())
                } else {
                    Some(0)
                }
            } else {
                None
            };
            
            result.push(UserSubscriptionResponse {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                plan_id: row.try_get("plan_id")?,
                plan_name: row.try_get("plan_name")?,
                status: row.try_get("status")?,
                start_date: row.try_get("start_date")?,
                end_date,
                is_lifetime: is_lifetime != 0,
                days_remaining,
            });
        }
        
        Ok(result)
    }
} 