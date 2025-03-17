use super::Table;
use crate::model::User;
use sqlx::Error;

impl<'c> Table<'c, User> {
    pub async fn create_user(&self, user: &User) -> Result<(), Error> {
        sqlx::query(
            r#"
            INSERT INTO users (id, email, display_name, phone_number)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.display_name)
        .bind(&user.phone_number)
        .execute(&*self.pool)
        .await
        .map(|_| ())
    }

    pub async fn create_user_with_picture_and_phone(&self, id: &str, email: &str, display_name: &str, picture_url: Option<&str>, phone_number: Option<&str>) -> Result<(), Error> {
        let query = match (picture_url, phone_number) {
            (Some(picture), Some(phone)) => {
                sqlx::query(
                    r#"
                    INSERT INTO users (id, email, display_name, picture_url, phone_number)
                    VALUES (?, ?, ?, ?, ?)
                    "#,
                )
                .bind(id)
                .bind(email)
                .bind(display_name)
                .bind(picture)
                .bind(phone)
            },
            (Some(picture), None) => {
                sqlx::query(
                    r#"
                    INSERT INTO users (id, email, display_name, picture_url)
                    VALUES (?, ?, ?, ?)
                    "#,
                )
                .bind(id)
                .bind(email)
                .bind(display_name)
                .bind(picture)
            },
            (None, Some(phone)) => {
                sqlx::query(
                    r#"
                    INSERT INTO users (id, email, display_name, phone_number)
                    VALUES (?, ?, ?, ?)
                    "#,
                )
                .bind(id)
                .bind(email)
                .bind(display_name)
                .bind(phone)
            },
            (None, None) => {
                sqlx::query(
                    r#"
                    INSERT INTO users (id, email, display_name)
                    VALUES (?, ?, ?)
                    "#,
                )
                .bind(id)
                .bind(email)
                .bind(display_name)
            }
        };

        query.execute(&*self.pool).await.map(|_| ())
    }

    pub async fn create_user_with_picture(&self, id: &str, email: &str, display_name: &str, picture_url: Option<&str>) -> Result<(), Error> {
        self.create_user_with_picture_and_phone(id, email, display_name, picture_url, None).await
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

    pub async fn get_user_by_id(&self, id: &str) -> Result<Option<User>, Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users WHERE id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn get_user_by_phone(&self, phone_number: &str) -> Result<Option<User>, Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM dbquizapp.users WHERE phone_number = ?"
        )
        .bind(phone_number)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn update_user_profile(&self, user_id: &str, display_name: &str, picture_url: Option<&str>, phone_number: Option<&str>) -> Result<(), Error> {
        let query = match (picture_url, phone_number) {
            (Some(picture), Some(phone)) => {
                sqlx::query(
                    r#"
                    UPDATE users 
                    SET display_name = ?, picture_url = ?, phone_number = ?, updated_at = NOW()
                    WHERE id = ? AND deleted_at IS NULL
                    "#,
                )
                .bind(display_name)
                .bind(picture)
                .bind(phone)
                .bind(user_id)
            },
            (Some(picture), None) => {
                sqlx::query(
                    r#"
                    UPDATE users 
                    SET display_name = ?, picture_url = ?, updated_at = NOW()
                    WHERE id = ? AND deleted_at IS NULL
                    "#,
                )
                .bind(display_name)
                .bind(picture)
                .bind(user_id)
            },
            (None, Some(phone)) => {
                sqlx::query(
                    r#"
                    UPDATE users 
                    SET display_name = ?, phone_number = ?, updated_at = NOW()
                    WHERE id = ? AND deleted_at IS NULL
                    "#,
                )
                .bind(display_name)
                .bind(phone)
                .bind(user_id)
            },
            (None, None) => {
                sqlx::query(
                    r#"
                    UPDATE users 
                    SET display_name = ?, updated_at = NOW()
                    WHERE id = ? AND deleted_at IS NULL
                    "#,
                )
                .bind(display_name)
                .bind(user_id)
            }
        };

        query.execute(&*self.pool).await.map(|_| ())
    }

    pub async fn update_user_profile_legacy(&self, user_id: &str, display_name: &str, picture_url: Option<&str>) -> Result<(), Error> {
        self.update_user_profile(user_id, display_name, picture_url, None).await
    }
}