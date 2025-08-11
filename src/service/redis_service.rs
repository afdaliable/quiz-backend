use redis::{Client, aio::ConnectionManager, AsyncCommands, RedisResult, RedisError};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use chrono::{DateTime, Utc, Duration};

#[derive(Clone)]
pub struct RedisPool {
    session_manager: Arc<ConnectionManager>,
    quiz_cache_manager: Arc<ConnectionManager>, 
    progress_manager: Arc<ConnectionManager>,
    leaderboard_manager: Arc<ConnectionManager>,
    rate_limit_manager: Arc<ConnectionManager>,
}

impl RedisPool {
    pub async fn new(base_url: &str) -> RedisResult<Self> {
        let session_client = Client::open(format!("{}/0", base_url))?;
        let quiz_cache_client = Client::open(format!("{}/1", base_url))?;
        let progress_client = Client::open(format!("{}/2", base_url))?;
        let leaderboard_client = Client::open(format!("{}/3", base_url))?;
        let rate_limit_client = Client::open(format!("{}/4", base_url))?;

        Ok(RedisPool {
            session_manager: Arc::new(ConnectionManager::new(session_client).await?),
            quiz_cache_manager: Arc::new(ConnectionManager::new(quiz_cache_client).await?),
            progress_manager: Arc::new(ConnectionManager::new(progress_client).await?),
            leaderboard_manager: Arc::new(ConnectionManager::new(leaderboard_client).await?),
            rate_limit_manager: Arc::new(ConnectionManager::new(rate_limit_client).await?),
        })
    }

    pub fn sessions(&self) -> Arc<ConnectionManager> {
        self.session_manager.clone()
    }

    pub fn quiz_cache(&self) -> Arc<ConnectionManager> {
        self.quiz_cache_manager.clone()
    }

    pub fn progress(&self) -> Arc<ConnectionManager> {
        self.progress_manager.clone()
    }

    pub fn leaderboards(&self) -> Arc<ConnectionManager> {
        self.leaderboard_manager.clone()
    }

    pub fn rate_limits(&self) -> Arc<ConnectionManager> {
        self.rate_limit_manager.clone()
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserSession {
    pub user_id: String,
    pub username: String,
    pub role: Option<String>,
    pub expires_at: u64,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

pub struct RedisService;

impl RedisService {
    pub async fn store_session(
        con: &mut ConnectionManager, 
        session_id: &str, 
        session: &UserSession
    ) -> RedisResult<()> {
        let session_json = serde_json::to_string(session)
            .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "JSON serialization failed", e.to_string())))?;
        
        let expiry_seconds = (session.expires_at as i64) - chrono::Utc::now().timestamp();
        if expiry_seconds > 0 {
            con.set_ex(format!("session:{}", session_id), session_json, expiry_seconds as u64).await
        } else {
            Err(RedisError::from((redis::ErrorKind::TypeError, "Session already expired")))
        }
    }

    pub async fn get_session(
        con: &mut ConnectionManager, 
        session_id: &str
    ) -> RedisResult<Option<UserSession>> {
        let session_data: Option<String> = con.get(format!("session:{}", session_id)).await?;
        
        match session_data {
            Some(data) => {
                match serde_json::from_str::<UserSession>(&data) {
                    Ok(session) => {
                        if session.expires_at > chrono::Utc::now().timestamp() as u64 {
                            Ok(Some(session))
                        } else {
                            con.del(format!("session:{}", session_id)).await?;
                            Ok(None)
                        }
                    },
                    Err(_) => Ok(None),
                }
            },
            None => Ok(None),
        }
    }

    pub async fn delete_session(
        con: &mut ConnectionManager, 
        session_id: &str
    ) -> RedisResult<()> {
        con.del(format!("session:{}", session_id)).await
    }

    pub async fn delete_user_sessions(
        con: &mut ConnectionManager, 
        user_id: &str
    ) -> RedisResult<()> {
        let pattern = format!("session:*");
        let keys: Vec<String> = con.keys(pattern).await?;
        
        for key in keys {
            if let Ok(Some(session)) = Self::get_session(con, &key.replace("session:", "")).await {
                if session.user_id == user_id {
                    con.del(&key).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn cache_quiz<T: Serialize>(
        con: &mut ConnectionManager, 
        quiz_id: u64, 
        quiz: &T,
        ttl_seconds: usize
    ) -> RedisResult<()> {
        let quiz_json = serde_json::to_string(quiz)
            .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "JSON serialization failed", e.to_string())))?;
        
        con.set_ex(format!("quiz:{}", quiz_id), quiz_json, ttl_seconds as u64).await
    }

    pub async fn cache_quiz_by_key<T: Serialize>(
        con: &mut ConnectionManager, 
        cache_key: &str, 
        quiz: &T,
        ttl_seconds: usize
    ) -> RedisResult<()> {
        let quiz_json = serde_json::to_string(quiz)
            .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "JSON serialization failed", e.to_string())))?;
        
        con.set_ex(format!("quiz:{}", cache_key), quiz_json, ttl_seconds as u64).await
    }

    pub async fn get_cached_quiz<T: for<'de> Deserialize<'de>>(
        con: &mut ConnectionManager, 
        quiz_id: u64
    ) -> RedisResult<Option<T>> {
        let quiz_data: Option<String> = con.get(format!("quiz:{}", quiz_id)).await?;
        
        match quiz_data {
            Some(data) => {
                match serde_json::from_str::<T>(&data) {
                    Ok(quiz) => Ok(Some(quiz)),
                    Err(_) => Ok(None),
                }
            },
            None => Ok(None),
        }
    }

    pub async fn get_cached_quiz_by_key<T: for<'de> Deserialize<'de>>(
        con: &mut ConnectionManager, 
        cache_key: &str
    ) -> RedisResult<Option<T>> {
        let quiz_data: Option<String> = con.get(format!("quiz:{}", cache_key)).await?;
        
        match quiz_data {
            Some(data) => {
                match serde_json::from_str::<T>(&data) {
                    Ok(quiz) => Ok(Some(quiz)),
                    Err(_) => Ok(None),
                }
            },
            None => Ok(None),
        }
    }

    pub async fn invalidate_quiz_cache(
        con: &mut ConnectionManager, 
        quiz_id: u64
    ) -> RedisResult<()> {
        con.del(format!("quiz:{}", quiz_id)).await
    }

    pub async fn start_quiz_attempt(
        con: &mut ConnectionManager, 
        user_id: u64, 
        quiz_id: u64
    ) -> RedisResult<()> {
        let attempt_key = format!("attempt:{}:{}", user_id, quiz_id);
        let start_time = chrono::Utc::now().timestamp() as u64;
        
        con.hset_multiple(&attempt_key, &[
            ("start_time", start_time.to_string()),
            ("current_question", "0".to_string()),
            ("score", "0".to_string()),
            ("status", "1".to_string()), // 1 = active, 2 = completed, 3 = abandoned
        ]).await?;
        
        con.expire(&attempt_key, 7200).await // 2 hours expiry
    }

    pub async fn update_quiz_progress(
        con: &mut ConnectionManager, 
        user_id: u64, 
        quiz_id: u64, 
        question_idx: u32, 
        score: u32
    ) -> RedisResult<()> {
        let attempt_key = format!("attempt:{}:{}", user_id, quiz_id);
        let last_activity = chrono::Utc::now().timestamp() as u64;
        
        con.hset_multiple(&attempt_key, &[
            ("current_question", question_idx.to_string()),
            ("score", score.to_string()),
            ("last_activity", last_activity.to_string()),
        ]).await
    }

    pub async fn complete_quiz_attempt(
        con: &mut ConnectionManager, 
        user_id: u64, 
        quiz_id: u64, 
        final_score: u32
    ) -> RedisResult<()> {
        let attempt_key = format!("attempt:{}:{}", user_id, quiz_id);
        let completed_at = chrono::Utc::now().timestamp() as u64;
        
        con.hset_multiple(&attempt_key, &[
            ("score", final_score.to_string()),
            ("status", "2".to_string()), // completed
            ("completed_at", completed_at.to_string()),
        ]).await?;
        
        con.expire(&attempt_key, 86400).await // Keep for 24 hours after completion
    }

    pub async fn get_quiz_attempt(
        con: &mut ConnectionManager, 
        user_id: u64, 
        quiz_id: u64
    ) -> RedisResult<Option<std::collections::HashMap<String, String>>> {
        let attempt_key = format!("attempt:{}:{}", user_id, quiz_id);
        let exists: bool = con.exists(&attempt_key).await?;
        
        if exists {
            let attempt_data: std::collections::HashMap<String, String> = con.hgetall(&attempt_key).await?;
            Ok(Some(attempt_data))
        } else {
            Ok(None)
        }
    }

    pub async fn update_leaderboard(
        con: &mut ConnectionManager, 
        quiz_id: u64, 
        user_id: u64, 
        score: f64
    ) -> RedisResult<()> {
        con.zadd(format!("leaderboard:{}", quiz_id), user_id, score).await
    }

    pub async fn get_top_scores(
        con: &mut ConnectionManager, 
        quiz_id: u64, 
        count: isize
    ) -> RedisResult<Vec<(u64, f64)>> {
        con.zrevrange_withscores(format!("leaderboard:{}", quiz_id), 0, count - 1).await
    }

    pub async fn get_user_rank(
        con: &mut ConnectionManager, 
        quiz_id: u64, 
        user_id: u64
    ) -> RedisResult<Option<usize>> {
        con.zrevrank(format!("leaderboard:{}", quiz_id), user_id).await
    }

    pub async fn check_rate_limit(
        con: &mut ConnectionManager, 
        user_id: u64, 
        limit: i32, 
        window_seconds: usize
    ) -> RedisResult<bool> {
        let key = format!("rate_limit:quiz:{}", user_id);
        let current: i32 = con.incr(&key, 1).await?;
        
        if current == 1 {
            con.expire(&key, window_seconds as i64).await?;
        }
        
        Ok(current <= limit)
    }

    pub async fn ping_connection(con: &mut ConnectionManager) -> RedisResult<String> {
        match con.get::<&str, String>("ping").await {
            Ok(result) => Ok(result),
            Err(_) => {
                redis::cmd("PING").query_async(con).await
            }
        }
    }
}