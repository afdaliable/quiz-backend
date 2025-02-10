use super::Table;
use crate::model::User;
use sqlx::Error;

impl<'c> Table<'c, User> {
    pub async fn create_user(&self, user: &User) -> Result<(), Error> {
        sqlx::query(
            r#"
            INSERT INTO users (id, email, display_name)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.display_name)
        .execute(&*self.pool)
        .await
        .map(|_| ())
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<User, Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users WHERE email = ? AND deleted_at IS NULL
            "#,
        )
        .bind(email)
        .fetch_one(&*self.pool)
        .await
    }
}