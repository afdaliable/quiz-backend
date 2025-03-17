use sqlx::MySqlPool;
use crate::model::payment_transaction::License;
use sqlx::Row;

/// Create a new license in the database
pub async fn create_license(db_context: &MySqlPool, license: &License) -> Result<i32, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO licenses (
            license_code, user_id, plan_id, status, expired_at, is_lifetime, product_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&license.license_code)
    .bind(&license.user_id)
    .bind(license.plan_id)
    .bind(&license.status)
    .bind(license.expired_at)
    .bind(license.is_lifetime)
    .bind(&license.product_id)
    .execute(db_context)
    .await?;

    Ok(result.last_insert_id() as i32)
}

/// Get a license by its code
pub async fn get_license_by_code(db_context: &MySqlPool, license_code: &str) -> Result<Option<License>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, license_code, user_id, plan_id, status, created_at, expired_at, is_lifetime, product_id
        FROM licenses
        WHERE license_code = ?
        "#
    )
    .bind(license_code)
    .fetch_optional(db_context)
    .await?;
    
    match row {
        Some(row) => Ok(Some(License {
            id: row.try_get("id")?,
            license_code: row.try_get("license_code")?,
            user_id: row.try_get("user_id")?,
            plan_id: row.try_get("plan_id")?,
            status: row.try_get("status")?,
            created_at: row.try_get("created_at")?,
            expired_at: row.try_get("expired_at")?,
            is_lifetime: row.try_get("is_lifetime")?,
            product_id: row.try_get("product_id")?,
        })),
        None => Ok(None),
    }
}

/// Get a license by its ID
pub async fn get_license_by_id(db_context: &MySqlPool, license_id: i32) -> Result<Option<License>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, license_code, user_id, plan_id, status, created_at, expired_at, is_lifetime, product_id
        FROM licenses
        WHERE id = ?
        "#
    )
    .bind(license_id)
    .fetch_optional(db_context)
    .await?;
    
    match row {
        Some(row) => Ok(Some(License {
            id: row.try_get("id")?,
            license_code: row.try_get("license_code")?,
            user_id: row.try_get("user_id")?,
            plan_id: row.try_get("plan_id")?,
            status: row.try_get("status")?,
            created_at: row.try_get("created_at")?,
            expired_at: row.try_get("expired_at")?,
            is_lifetime: row.try_get("is_lifetime")?,
            product_id: row.try_get("product_id")?,
        })),
        None => Ok(None),
    }
}

/// Get all licenses for a user
pub async fn get_licenses_by_user_id(db_context: &MySqlPool, user_id: &str) -> Result<Vec<License>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, license_code, user_id, plan_id, status, created_at, expired_at, is_lifetime, product_id
        FROM licenses
        WHERE user_id = ?
        ORDER BY created_at DESC
        "#
    )
    .bind(user_id)
    .fetch_all(db_context)
    .await?;
    
    let mut licenses = Vec::with_capacity(rows.len());
    for row in rows {
        licenses.push(License {
            id: row.try_get("id")?,
            license_code: row.try_get("license_code")?,
            user_id: row.try_get("user_id")?,
            plan_id: row.try_get("plan_id")?,
            status: row.try_get("status")?,
            created_at: row.try_get("created_at")?,
            expired_at: row.try_get("expired_at")?,
            is_lifetime: row.try_get("is_lifetime")?,
            product_id: row.try_get("product_id")?,
        });
    }
    
    Ok(licenses)
}

/// Update the status of a license
pub async fn update_license_status(db_context: &MySqlPool, license_id: i32, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE licenses
        SET status = ?
        WHERE id = ?
        "#
    )
    .bind(status)
    .bind(license_id)
    .execute(db_context)
    .await?;

    Ok(())
}

/// Delete a license
pub async fn delete_license(db_context: &MySqlPool, license_id: i32) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM licenses
        WHERE id = ?
        "#
    )
    .bind(license_id)
    .execute(db_context)
    .await?;

    Ok(())
} 