use super::Table;
use super::KategoriSoal;

impl<'c> Table<'c, KategoriSoal> {

    pub async fn get_all_category(&self) -> Result<Vec<KategoriSoal>, sqlx::Error>{
        sqlx::query_as(
            r#"
            SELECT * from kategori_soal
            "#
        ).fetch_all(&*self.pool).await
    }




}