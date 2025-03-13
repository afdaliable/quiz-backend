use super::Table;
use crate::model::Session;
use sqlx::Error;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

impl<'c> Table<'c, Session> {
    pub async fn create_session(
        &self, 
        user_id: &str, 
        ip_address: Option<&str>, 
        user_agent: Option<&str>
    ) -> Result<Session, Error> {
        // Generate a random token
        let token = Uuid::new_v4().to_string();
        
        // Set expiration time (24 hours from now)
        let expires_at = Utc::now() + Duration::hours(24);
        
        // Insert the session
        sqlx::query(
            r#"
            INSERT INTO sessions (user_id, token, expires_at, ip_address, user_agent)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(user_id)
        .bind(&token)
        .bind(expires_at)
        .bind(ip_address)
        .bind(user_agent)
        .execute(&*self.pool)
        .await?;
        
        // Fetch the newly created session
        self.get_session_by_token(&token).await
    }
    
    pub async fn get_session_by_token(&self, token: &str) -> Result<Session, Error> {
        sqlx::query_as::<_, Session>(
            r#"
            SELECT * FROM sessions WHERE token = ? AND expires_at > NOW()
            "#,
        )
        .bind(token)
        .fetch_one(&*self.pool)
        .await
    }
    
    pub async fn get_sessions_by_user_id(&self, user_id: &str) -> Result<Vec<Session>, Error> {
        sqlx::query_as::<_, Session>(
            r#"
            SELECT * FROM sessions WHERE user_id = ? AND expires_at > NOW()
            "#,
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await
    }
    
    pub async fn delete_session(&self, token: &str) -> Result<(), Error> {
        sqlx::query(
            r#"
            DELETE FROM sessions WHERE token = ?
            "#,
        )
        .bind(token)
        .execute(&*self.pool)
        .await
        .map(|_| ())
    }
    
    pub async fn delete_all_user_sessions(&self, user_id: &str) -> Result<(), Error> {
        sqlx::query(
            r#"
            DELETE FROM sessions WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .execute(&*self.pool)
        .await
        .map(|_| ())
    }
    
    pub async fn delete_expired_sessions(&self) -> Result<(), Error> {
        sqlx::query(
            r#"
            DELETE FROM sessions WHERE expires_at <= NOW()
            "#,
        )
        .execute(&*self.pool)
        .await
        .map(|_| ())
    }
    
    pub async fn is_session_valid(&self, token: &str) -> Result<bool, Error> {
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM sessions WHERE token = ? AND expires_at > NOW()
            "#,
        )
        .bind(token)
        .fetch_one(&*self.pool)
        .await?;
        
        Ok(result > 0)
    }
} 