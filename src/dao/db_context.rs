use super::Soal;
use super::KategoriSoal;
use super::PaketSoal;
use super::PaketSoalItem;
use crate::model::PaketSoalResponse;
use crate::model::ListPaketSoal;

use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, MySqlPool};
use std::marker::PhantomData;
use std::sync::Arc;

pub struct Table<'c, T>
where
    T: FromRow<'c, MySqlRow>,
{
    pub pool: Arc<MySqlPool>,
    _from_row: fn(&'c MySqlRow) -> Result<T, sqlx::Error>,
    _marker: PhantomData<&'c T>,
}

impl<'c, T> Table<'c, T>
where
    T: FromRow<'c, MySqlRow>,
{
    fn new(pool: Arc<MySqlPool>) -> Self {
        Table {
            pool,
            _from_row: T::from_row,
            _marker: PhantomData,
        }
    }
}

pub struct JoinTable<'c, T1, T2, T3,T4>
where
    T1: FromRow<'c, MySqlRow>,
    T2: FromRow<'c, MySqlRow>,
    T3: FromRow<'c, MySqlRow>,
    T4: FromRow<'c, MySqlRow>,
{
    pub pool: Arc<MySqlPool>,
    _from_row: (
        fn(&'c MySqlRow) -> Result<T1, sqlx::Error>,
        fn(&'c MySqlRow) -> Result<T2, sqlx::Error>,
        fn(&'c MySqlRow) -> Result<T3, sqlx::Error>,
        fn(&'c MySqlRow) -> Result<T4, sqlx::Error>,
    ),
    _marker_t1: PhantomData<&'c T1>,
    _marker_t2: PhantomData<&'c T2>,
    _marker_t3: PhantomData<&'c T3>,
    _marker_t4: PhantomData<&'c T4>
}

impl<'c, T1, T2, T3, T4> JoinTable<'c, T1, T2, T3, T4>
where
    T1: FromRow<'c, MySqlRow>,
    T2: FromRow<'c, MySqlRow>,
    T3: FromRow<'c, MySqlRow>,
    T4: FromRow<'c, MySqlRow>,
{
    fn new(pool: Arc<MySqlPool>) -> Self {
        JoinTable {
            pool,
            _from_row: (T1::from_row, T2::from_row, T3::from_row, T4::from_row),
            _marker_t1: PhantomData,
            _marker_t2: PhantomData,
            _marker_t3: PhantomData,
            _marker_t4: PhantomData,
        }
    }
}

impl<'c> JoinTable<'c, KategoriSoal, PaketSoal, PaketSoalItem, Soal>{
    pub async fn get_paket_soal_response(&self, nama_kategori: &String, nama_paket_soal: &String) -> Result<PaketSoalResponse, sqlx::Error> {
        let mut results = sqlx::query_as::<_, PaketSoalResponse>(
            r#"
            SELECT ks.id as kategori_id, ks.nama_kategori, ps.id as paket_soal_id, ps.nama_paket_soal, 
                   s.id as soal_id, s.soal, s.opt1, s.opt2, s.opt3, s.opt4, s.opt5, s.correct_answer, s.solution
            FROM kategori_soal ks 
            JOIN paket_soal ps ON ks.id = ps.kategori_id 
            JOIN paket_soal_items psi ON psi.paket_soal_id = ps.id 
            JOIN soal s ON psi.soal_id = s.id 
            WHERE ks.nama_kategori = ? AND ps.nama_paket_soal = ?
            "#,
        )
        .bind(nama_kategori)
        .bind(nama_paket_soal)
        .fetch_all(&*self.pool)
        .await?;

        if results.is_empty() {
            return Err(sqlx::Error::RowNotFound);
        }

        let mut response = results.remove(0);
        response.kumpulan_soal.extend(results.into_iter().flat_map(|r| r.kumpulan_soal));
        Ok(response)
    }
}

impl<'c> JoinTable<'c, KategoriSoal, PaketSoal, PaketSoalItem, Soal> {
    pub async fn get_list_paket_soal(&self) -> Result<Vec<ListPaketSoal>, sqlx::Error> {
        sqlx::query_as::<_, ListPaketSoal>(
            r#"
            SELECT 
                ps.id as id_nama_paket_soal,
            ps.nama_paket_soal,
            ks.id as id_kategori_soal,
            ks.nama_kategori AS kategori_soal,
            COUNT(psi.soal_id) AS jumlah_soal
        FROM 
            paket_soal ps
        JOIN 
            kategori_soal ks ON ps.kategori_id = ks.id
        LEFT JOIN 
            paket_soal_items psi ON ps.id = psi.paket_soal_id
        GROUP BY 
            ps.id, ps.nama_paket_soal, ks.id, ks.nama_kategori
        ORDER BY 
            ks.nama_kategori, ps.nama_paket_soal
        "#,
    )
    .fetch_all(&*self.pool)
    .await
}

}

pub struct Database<'c> {
    pub soal: Arc<Table<'c, Soal>>,
    pub paket_soal_response: Arc<JoinTable<'c, KategoriSoal, PaketSoal, PaketSoalItem, Soal>>,
}

impl<'a> Database<'a> {
    pub async fn new(sql_url: &String) -> Database<'a> {
        let connection = MySqlPool::connect(&sql_url).await.unwrap();
        let pool = Arc::new(connection);

        Database {
            // groups: Arc::from(Table::new(pool.clone())),
            soal: Arc::from(Table::new(pool.clone())),
            paket_soal_response: Arc::from(JoinTable::new(pool.clone())),
            // users_to_groups: Arc::from(JoinTable::new(pool.clone())),
        }
    }
}

