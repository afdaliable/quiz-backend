use crate::dao::Table;
use crate::model::license_code::{LicenseCode, LicenseStatus};
use chrono::{DateTime, Utc};
use sqlx::{Error, Row};

impl<'c> Table<'c, LicenseCode> {
    pub async fn get_license_by_code(&self, license_code: &str) -> Result<Option<LicenseCode>, Error> {
        let row = sqlx::query(
            "SELECT * FROM dbquizapp.license_codes WHERE license_code = ?"
        )
        .bind(license_code)
        .fetch_optional(&*self.pool)
        .await?;
        
        match row {
            Some(row) => {
                let status_str: String = row.try_get("status")?;
                let status = match status_str.as_str() {
                    "active" => LicenseStatus::Active,
                    "expired" => LicenseStatus::Expired,
                    "cancelled" => LicenseStatus::Cancelled,
                    _ => LicenseStatus::Active, // Default to active if unknown
                };
                
                Ok(Some(LicenseCode {
                    id: row.try_get("id")?,
                    license_code: row.try_get("license_code")?,
                    user_id: row.try_get("user_id")?,
                    plan_id: row.try_get("plan_id")?,
                    status,
                    transaction_id: row.try_get("transaction_id")?,
                    product_id: row.try_get("product_id")?,
                    customer_id: row.try_get("customer_id")?,
                    customer_name: row.try_get("customer_name")?,
                    customer_email: row.try_get("customer_email")?,
                    expired_at: row.try_get("expired_at")?,
                    activation_limit: row.try_get("activation_limit")?,
                    use_count: row.try_get("use_count")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                    license_data: row.try_get("license_data")?,
                }))
            },
            None => Ok(None),
        }
    }

    pub async fn get_user_licenses(&self, user_id: &str) -> Result<Vec<LicenseCode>, Error> {
        let rows = sqlx::query(
            "SELECT * FROM dbquizapp.license_codes WHERE user_id = ? ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await?;
        
        let mut licenses = Vec::new();
        for row in rows {
            let status_str: String = row.try_get("status")?;
            let status = match status_str.as_str() {
                "active" => LicenseStatus::Active,
                "expired" => LicenseStatus::Expired,
                "cancelled" => LicenseStatus::Cancelled,
                _ => LicenseStatus::Active, // Default to active if unknown
            };
            
            licenses.push(LicenseCode {
                id: row.try_get("id")?,
                license_code: row.try_get("license_code")?,
                user_id: row.try_get("user_id")?,
                plan_id: row.try_get("plan_id")?,
                status,
                transaction_id: row.try_get("transaction_id")?,
                product_id: row.try_get("product_id")?,
                customer_id: row.try_get("customer_id")?,
                customer_name: row.try_get("customer_name")?,
                customer_email: row.try_get("customer_email")?,
                expired_at: row.try_get("expired_at")?,
                activation_limit: row.try_get("activation_limit")?,
                use_count: row.try_get("use_count")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                license_data: row.try_get("license_data")?,
            });
        }
        
        Ok(licenses)
    }

    pub async fn create_license(&self, 
        license_code: &str,
        user_id: Option<&str>,
        plan_id: i32,
        status: LicenseStatus,
        transaction_id: Option<&str>,
        product_id: &str,
        customer_id: Option<&str>,
        customer_name: Option<&str>,
        customer_email: Option<&str>,
        expired_at: Option<DateTime<Utc>>,
        activation_limit: Option<&str>,
        use_count: Option<i32>,
        license_data: Option<&str>
    ) -> Result<i32, Error> {
        let status_str = format!("{:?}", status).to_lowercase();
        
        let result = sqlx::query(
            "INSERT INTO dbquizapp.license_codes (
                license_code, user_id, plan_id, status, transaction_id, 
                product_id, customer_id, customer_name, customer_email, 
                expired_at, activation_limit, use_count, license_data
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(license_code)
        .bind(user_id)
        .bind(plan_id)
        .bind(status_str)
        .bind(transaction_id)
        .bind(product_id)
        .bind(customer_id)
        .bind(customer_name)
        .bind(customer_email)
        .bind(expired_at)
        .bind(activation_limit)
        .bind(use_count)
        .bind(license_data)
        .execute(&*self.pool)
        .await?;

        Ok(result.last_insert_id() as i32)
    }

    pub async fn update_license_user(&self, license_code: &str, user_id: &str) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE dbquizapp.license_codes SET user_id = ? WHERE license_code = ?"
        )
        .bind(user_id)
        .bind(license_code)
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn update_license_status(&self, license_code: &str, status: LicenseStatus) -> Result<bool, Error> {
        let status_str = format!("{:?}", status).to_lowercase();
        
        let result = sqlx::query(
            "UPDATE dbquizapp.license_codes SET status = ? WHERE license_code = ?"
        )
        .bind(status_str)
        .bind(license_code)
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_active_license_by_user_id(&self, user_id: &str) -> Result<Option<LicenseCode>, Error> {
        let row = sqlx::query(
            "SELECT * FROM dbquizapp.license_codes 
             WHERE user_id = ? 
             AND status = 'active' 
             AND (expired_at IS NULL OR expired_at > NOW())
             ORDER BY created_at DESC
             LIMIT 1"
        )
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await?;
        
        match row {
            Some(row) => {
                let status_str: String = row.try_get("status")?;
                let status = match status_str.as_str() {
                    "active" => LicenseStatus::Active,
                    "expired" => LicenseStatus::Expired,
                    "cancelled" => LicenseStatus::Cancelled,
                    _ => LicenseStatus::Active, // Default to active if unknown
                };
                
                Ok(Some(LicenseCode {
                    id: row.try_get("id")?,
                    license_code: row.try_get("license_code")?,
                    user_id: row.try_get("user_id")?,
                    plan_id: row.try_get("plan_id")?,
                    status,
                    transaction_id: row.try_get("transaction_id")?,
                    product_id: row.try_get("product_id")?,
                    customer_id: row.try_get("customer_id")?,
                    customer_name: row.try_get("customer_name")?,
                    customer_email: row.try_get("customer_email")?,
                    expired_at: row.try_get("expired_at")?,
                    activation_limit: row.try_get("activation_limit")?,
                    use_count: row.try_get("use_count")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                    license_data: row.try_get("license_data")?,
                }))
            },
            None => Ok(None),
        }
    }
} 