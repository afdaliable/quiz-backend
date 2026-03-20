use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentUser {
    pub id: String,
    pub display_name: String,
    pub picture_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentResponse {
    pub id: String,
    pub user: CommentUser,
    pub body: String,
    pub is_admin_pin: bool,
    pub upvotes: i32,
    pub has_upvoted: bool,
    pub reply_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub body: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToggleUpvoteResponse {
    pub upvotes: i32,
    pub has_upvoted: bool,
}

#[derive(Debug, Deserialize)]
pub struct PinCommentRequest {
    pub pin: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentListResponse {
    pub data: Vec<CommentResponse>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
pub struct CommentListQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}
