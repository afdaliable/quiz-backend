use crate::dao::Table;
use crate::model::payment_transaction::{PaymentTransaction, PaymentStatus, PaymentTransactionResponse, CreatePaymentRequest};
use sqlx::{Error, Row};
use serde_json;

impl<'c> Table<'c, PaymentTransaction> {
    pub async fn get_user_payment_transactions(&self, user_id: &str) -> Result<Vec<PaymentTransactionResponse>, Error> {
        let rows = sqlx::query(
            r#"
            SELECT 
                pt.id,
                pt.user_id,
                pt.plan_id,
                pp.name as plan_name,
                pt.amount,
                pt.transaction_id,
                pt.payment_link,
                pt.status as status,
                pt.payment_method,
                pt.created_at
            FROM 
                dbquizapp.payment_transactions pt
            JOIN 
                dbquizapp.premium_plans pp ON pt.plan_id = pp.id
            WHERE 
                pt.user_id = ?
            ORDER BY 
                pt.created_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await?;
        
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            result.push(PaymentTransactionResponse {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                plan_id: row.try_get("plan_id")?,
                plan_name: row.try_get("plan_name")?,
                amount: row.try_get("amount")?,
                transaction_id: row.try_get("transaction_id")?,
                payment_link: row.try_get("payment_link")?,
                status: row.try_get("status")?,
                payment_method: row.try_get("payment_method")?,
                created_at: row.try_get("created_at")?,
            });
        }
        
        Ok(result)
    }

    pub async fn get_payment_transaction_by_id(&self, id: i32) -> Result<Option<PaymentTransactionResponse>, Error> {
        let row = sqlx::query(
            r#"
            SELECT 
                pt.id,
                pt.user_id,
                pt.plan_id,
                pp.name as plan_name,
                pt.amount,
                pt.transaction_id,
                pt.payment_link,
                pt.status as status,
                pt.payment_method,
                pt.created_at
            FROM 
                dbquizapp.payment_transactions pt
            JOIN 
                dbquizapp.premium_plans pp ON pt.plan_id = pp.id
            WHERE 
                pt.id = ?
            "#
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await?;
        
        if let Some(row) = row {
            Ok(Some(PaymentTransactionResponse {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                plan_id: row.try_get("plan_id")?,
                plan_name: row.try_get("plan_name")?,
                amount: row.try_get("amount")?,
                transaction_id: row.try_get("transaction_id")?,
                payment_link: row.try_get("payment_link")?,
                status: row.try_get("status")?,
                payment_method: row.try_get("payment_method")?,
                created_at: row.try_get("created_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_payment_transaction_by_transaction_id(&self, transaction_id: &str) -> Result<Option<PaymentTransaction>, Error> {
        sqlx::query_as::<_, PaymentTransaction>(
            "SELECT * FROM dbquizapp.payment_transactions WHERE transaction_id = ?"
        )
        .bind(transaction_id)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn create_payment_transaction(&self, 
        user_id: &str, 
        plan_id: i32, 
        amount: f64, 
        transaction_id: &str, 
        payment_link: &str
    ) -> Result<i32, Error> {
        let result = sqlx::query(
            "INSERT INTO dbquizapp.payment_transactions 
             (user_id, plan_id, amount, transaction_id, payment_link, status) 
             VALUES (?, ?, ?, ?, ?, 'pending')"
        )
        .bind(user_id)
        .bind(plan_id)
        .bind(amount)
        .bind(transaction_id)
        .bind(payment_link)
        .execute(&*self.pool)
        .await?;

        Ok(result.last_insert_id() as i32)
    }

    pub async fn update_payment_status(&self, transaction_id: &str, status: PaymentStatus, payment_method: Option<&str>, webhook_data: Option<&str>) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE dbquizapp.payment_transactions 
             SET status = ?, payment_method = ?, webhook_data = ? 
             WHERE transaction_id = ?"
        )
        .bind(status)
        .bind(payment_method)
        .bind(webhook_data)
        .bind(transaction_id)
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn update_payment_details(&self, transaction_id: &str, payment_details: &str) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE dbquizapp.payment_transactions 
             SET payment_details = ? 
             WHERE transaction_id = ?"
        )
        .bind(payment_details)
        .bind(transaction_id)
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_all_transaction_ids(&self) -> Result<Vec<String>, Error> {
        let rows = sqlx::query(
            "SELECT transaction_id FROM dbquizapp.payment_transactions ORDER BY created_at DESC LIMIT 50"
        )
        .fetch_all(&*self.pool)
        .await?;
        
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            result.push(row.try_get("transaction_id")?);
        }
        
        Ok(result)
    }

    pub async fn create_payment_transaction_with_mayar_id(
        &self, 
        user_id: &str, 
        plan_id: i32, 
        amount: f64, 
        transaction_id: &str, 
        payment_link: &str,
        mayar_id: &str
    ) -> Result<i32, Error> {
        let result = sqlx::query(
            "INSERT INTO dbquizapp.payment_transactions 
             (user_id, plan_id, amount, transaction_id, payment_link, status, payment_details) 
             VALUES (?, ?, ?, ?, ?, 'pending', ?)"
        )
        .bind(user_id)
        .bind(plan_id)
        .bind(amount)
        .bind(transaction_id)
        .bind(payment_link)
        .bind(serde_json::json!({ "mayar_id": mayar_id }).to_string())
        .execute(&*self.pool)
        .await?;

        Ok(result.last_insert_id() as i32)
    }

    pub async fn get_payment_transaction_by_mayar_id(&self, mayar_id: &str) -> Result<Option<PaymentTransaction>, Error> {
        let rows = sqlx::query_as::<_, PaymentTransaction>(
            "SELECT * FROM dbquizapp.payment_transactions WHERE payment_details LIKE ?"
        )
        .bind(format!("%\"mayar_id\":\"%{}%", mayar_id))
        .fetch_all(&*self.pool)
        .await?;
        
        // Find the transaction with the exact mayar_id
        for transaction in rows {
            if let Some(details) = &transaction.payment_details {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(details) {
                    if let Some(id) = json.get("mayar_id").and_then(|id| id.as_str()) {
                        if id == mayar_id {
                            return Ok(Some(transaction));
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
} 