use super::Table;
use crate::model::Session;
use crate::service::redis_service::{RedisService, UserSession};
use crate::AppState;
use sqlx::Error;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use redis::aio::ConnectionManager;

impl<'c> Table<'c, Session> {
    pub async fn create_session_with_redis(
        &self,
        user_id: &str,
        username: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        redis_pool: Option<&crate::service::redis_service::RedisPool>
    ) -> Result<Session, Error> {
        let token = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + Duration::hours(24);

        // INSERT — tidak perlu SELECT kembali setelah ini
        sqlx::query(
            "INSERT INTO sessions (user_id, token, expires_at, ip_address, user_agent) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(user_id)
        .bind(&token)
        .bind(expires_at)
        .bind(ip_address)
        .bind(user_agent)
        .execute(&*self.pool)
        .await?;

        // Store in Redis (fire-and-forget style — tidak blok response)
        if let Some(pool) = redis_pool {
            let redis_session = UserSession {
                user_id: user_id.to_string(),
                username: username.to_string(),
                role: None,
                expires_at: expires_at.timestamp() as u64,
                ip_address: ip_address.map(|s| s.to_string()),
                user_agent: user_agent.map(|s| s.to_string()),
            };
            let mut con = pool.sessions().as_ref().clone();
            if let Err(e) = RedisService::store_session(&mut con, &token, &redis_session).await {
                eprintln!("Failed to cache session in Redis: {:?}", e);
            }
        }

        // Return session langsung dari data yang sudah kita punya — tanpa extra SELECT
        Ok(Session {
            id: 0, // auto-increment, tidak dipakai di response
            user_id: user_id.to_string(),
            token,
            expires_at,
            ip_address: ip_address.map(|s| s.to_string()),
            user_agent: user_agent.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn create_session(
        &self, 
        user_id: &str, 
        ip_address: Option<&str>, 
        user_agent: Option<&str>
    ) -> Result<Session, Error> {
        self.create_session_with_redis(user_id, "unknown", ip_address, user_agent, None).await
    }
    
    pub async fn get_session_by_token_with_redis(
        &self, 
        token: &str,
        redis_pool: Option<&crate::service::redis_service::RedisPool>
    ) -> Result<Session, Error> {
        // Try Redis first if available
        if let Some(pool) = redis_pool {
            let mut con = pool.sessions().as_ref().clone();
            if let Ok(Some(_redis_session)) = RedisService::get_session(&mut con, token).await {
                // If found in Redis, also check MySQL to return proper Session struct
                if let Ok(session) = sqlx::query_as::<_, Session>(
                    r#"
                    SELECT * FROM sessions WHERE token = ? AND expires_at > NOW()
                    "#,
                )
                .bind(token)
                .fetch_one(&*self.pool)
                .await {
                    return Ok(session);
                }
            }
        }
        
        // Fallback to MySQL
        sqlx::query_as::<_, Session>(
            r#"
            SELECT * FROM sessions WHERE token = ? AND expires_at > NOW()
            "#,
        )
        .bind(token)
        .fetch_one(&*self.pool)
        .await
    }

    pub async fn get_session_by_token(&self, token: &str) -> Result<Session, Error> {
        self.get_session_by_token_with_redis(token, None).await
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
    
    pub async fn delete_session_with_redis(
        &self, 
        token: &str,
        redis_pool: Option<&crate::service::redis_service::RedisPool>
    ) -> Result<(), Error> {
        // Delete from Redis first if available
        if let Some(pool) = redis_pool {
            let mut con = pool.sessions().as_ref().clone();
            if let Err(e) = RedisService::delete_session(&mut con, token).await {
                eprintln!("Failed to delete session from Redis: {:?}", e);
            }
        }
        
        // Delete from MySQL
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

    pub async fn delete_session(&self, token: &str) -> Result<(), Error> {
        self.delete_session_with_redis(token, None).await
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