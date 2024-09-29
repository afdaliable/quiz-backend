// use super::Group;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};

#[derive(Serialize, Deserialize, Clone)]
pub struct Soal {
    pub id: i32,
    pub soal: String,
    pub opt1: String,
    pub opt2: String,
    pub opt3: String,
    pub opt4: String,
    pub opt5: String,
    pub correct_answer: String,
    pub solution: String,
}

impl<'c> FromRow<'c, MySqlRow> for Soal {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(Soal {
            id: row.get(0),
            soal: row.get(1),
            opt1: row.get(2),
            opt2: row.get(3),
            opt3: row.get(4),
            opt4: row.get(5),
            opt5: row.get(6),
            correct_answer: row.get(7),
            solution: row.get(8),
        })
    }
}
