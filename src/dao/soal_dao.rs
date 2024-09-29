use super::Table;
use super::Soal;

impl<'c> Table<'c, Soal> {
    pub async fn drop_table(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DROP TABLE IF EXISTS soal;")
            .execute(&*self.pool)
            .await
            .map(|_|())
    }

    pub async fn create_table(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
            id INT AUTO_INCREMENT PRIMARY KEY,
            username VARCHAR(255) NOT NULL UNIQUE,
            jumlah_TO INT,
            nilai_TO DECIMAL(5, 2),
            tanggal_TO DATE,
            kategori VARCHAR(50)
        )"#,
        )
        .execute(&*self.pool)
        .await
        .map(|_|())
    }

    pub async fn get_soal_by_id(&self, soal_id: &str) -> Result<Soal, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT *
            FROM `soal`
            WHERE `id` = ?"#,
        )
        .bind(soal_id)
        .fetch_one(&*self.pool)
        .await
    }

    // pub async fn add_user(&self, user: &User) -> Result<u64, sqlx::Error> {
    //     sqlx::query(
    //         r#"
    //         INSERT INTO users (`id`, `name`, `email`)
    //         VALUES(?, ?, ?)"#,
    //     )
    //     .bind(&user.id)
    //     .bind(&user.name)
    //     .bind(&user.email)
    //     .execute(&*self.pool)
    //     .await
    //     .map(|x|x.rows_affected())
    // }

    // pub async fn update_user(&self, user: &User) -> Result<u64, sqlx::Error> {
    //     sqlx::query(
    //         r#"
    //         UPDATE users
    //         SET `name` = ?, `email` = ?
    //         WHERE `id` = ?
    //         "#,
    //     )
    //     .bind(&user.name)
    //     .bind(&user.email)
    //     .bind(&user.id)
    //     .execute(&*self.pool)
    //     .await
    //     .map(|x|x.rows_affected())
    // }

    // pub async fn delete_user(&self, user_id: &str) -> Result<u64, sqlx::Error> {
    //     sqlx::query(
    //         r#"
    //         DELETE FROM users
    //         WHERE `id` = ?
    //         "#,
    //     )
    //     .bind(user_id)
    //     .execute(&*self.pool)
    //     .await
    //     .map(|x|x.rows_affected())
    // }


}
