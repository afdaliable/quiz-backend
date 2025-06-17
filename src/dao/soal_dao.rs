use super::Table;
use super::Soal;
use sqlx::Error;
use crate::model::CreateSoalRequest;

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

    pub async fn get_all_soal(&self) -> Result<Vec<Soal>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT *
            FROM soal 
            Order BY id"#,
        )
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn create_soal(&self, request: &CreateSoalRequest) -> Result<Soal, Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO dbquizapp.soal (soal, opt1, opt2, opt3, opt4, opt5, correct_answer, solution, sumberfile, modul, pelajaran, tag)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#)
            .bind(&request.soal)
            .bind(&request.opt1)
            .bind(&request.opt2)
            .bind(&request.opt3)
            .bind(&request.opt4)
            .bind(&request.opt5)
            .bind(&request.correct_answer)
            .bind(&request.solution)
            .bind(&request.sumberfile)
            .bind(&request.modul)
            .bind(&request.pelajaran)
            .bind(&request.tag)
            .execute(&*self.pool)
            .await?;

        let id = result.last_insert_id();

        // Fetch the inserted row
        sqlx::query_as::<_, Soal>(
            "SELECT * FROM soal WHERE id = ?"
        )
        .bind(id)
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
