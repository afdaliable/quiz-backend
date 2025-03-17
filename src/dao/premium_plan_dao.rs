use crate::dao::Table;
use crate::model::premium_plan::{PremiumPlan, CreatePremiumPlanRequest, UpdatePremiumPlanRequest};
use sqlx::Error;

impl<'c> Table<'c, PremiumPlan> {
    pub async fn get_all_premium_plans(&self) -> Result<Vec<PremiumPlan>, Error> {
        sqlx::query_as::<_, PremiumPlan>(
            "SELECT * FROM dbquizapp.premium_plans ORDER BY price ASC"
        )
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_premium_plan_by_id(&self, id: i32) -> Result<Option<PremiumPlan>, Error> {
        sqlx::query_as::<_, PremiumPlan>(
            "SELECT * FROM dbquizapp.premium_plans WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn get_premium_plan_by_mayar_product_id(&self, product_id: &str) -> Result<Option<PremiumPlan>, Error> {
        sqlx::query_as::<_, PremiumPlan>(
            "SELECT * FROM dbquizapp.premium_plans WHERE mayar_product_id = ?"
        )
        .bind(product_id)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn create_premium_plan(&self, plan: &CreatePremiumPlanRequest) -> Result<i32, Error> {
        let features_json = serde_json::to_string(&plan.features).unwrap_or_default();
        
        let result = sqlx::query(
            "INSERT INTO dbquizapp.premium_plans (name, description, price, duration_days, is_lifetime, features, mayar_product_id, mayar_link_payment) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&plan.name)
        .bind(&plan.description)
        .bind(plan.price)
        .bind(plan.duration_days)
        .bind(plan.is_lifetime)
        .bind(features_json)
        .bind(&plan.mayar_product_id)
        .bind(&plan.mayar_link_payment)
        .execute(&*self.pool)
        .await?;

        Ok(result.last_insert_id() as i32)
    }

    pub async fn update_premium_plan(&self, id: i32, plan: &UpdatePremiumPlanRequest) -> Result<bool, Error> {
        let mut query_builder = sqlx::QueryBuilder::new("UPDATE dbquizapp.premium_plans SET ");
        let mut needs_comma = false;

        if let Some(name) = &plan.name {
            query_builder.push("name = ");
            query_builder.push_bind(name);
            needs_comma = true;
        }

        if let Some(description) = &plan.description {
            if needs_comma {
                query_builder.push(", ");
            }
            query_builder.push("description = ");
            query_builder.push_bind(description);
            needs_comma = true;
        }

        if let Some(price) = plan.price {
            if needs_comma {
                query_builder.push(", ");
            }
            query_builder.push("price = ");
            query_builder.push_bind(price);
            needs_comma = true;
        }

        if let Some(duration_days) = plan.duration_days {
            if needs_comma {
                query_builder.push(", ");
            }
            query_builder.push("duration_days = ");
            query_builder.push_bind(duration_days);
            needs_comma = true;
        }

        if let Some(is_lifetime) = plan.is_lifetime {
            if needs_comma {
                query_builder.push(", ");
            }
            query_builder.push("is_lifetime = ");
            query_builder.push_bind(is_lifetime);
            needs_comma = true;
        }

        if let Some(features) = &plan.features {
            if needs_comma {
                query_builder.push(", ");
            }
            let features_json = serde_json::to_string(features).unwrap_or_default();
            query_builder.push("features = ");
            query_builder.push_bind(features_json);
            needs_comma = true;
        }

        if let Some(mayar_product_id) = &plan.mayar_product_id {
            if needs_comma {
                query_builder.push(", ");
            }
            query_builder.push("mayar_product_id = ");
            query_builder.push_bind(mayar_product_id);
            needs_comma = true;
        }

        if let Some(mayar_link_payment) = &plan.mayar_link_payment {
            if needs_comma {
                query_builder.push(", ");
            }
            query_builder.push("mayar_link_payment = ");
            query_builder.push_bind(mayar_link_payment);
            needs_comma = true;
        }

        if !needs_comma {
            return Ok(false);
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(id);

        let result = query_builder.build().execute(&*self.pool).await?;
        
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_premium_plan(&self, id: i32) -> Result<bool, Error> {
        let result = sqlx::query(
            "DELETE FROM dbquizapp.premium_plans WHERE id = ?"
        )
        .bind(id)
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
} 