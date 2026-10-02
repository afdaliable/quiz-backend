//! Kode promo yang dikelola admin.
//!
//! - `event`: berlaku untuk semua user dalam rentang `mulai`..`berakhir`.
//! - `akhir_trial`: ditawarkan otomatis ke tiap user selama `durasi_jam` setelah
//!   trial-nya habis.
//!
//! Potongan selalu dihitung di sini (server), tidak pernah dari angka kiriman
//! browser. Pemakaian dicatat saat pembayaran LUNAS, bukan saat tagihan dibuat.

use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, Row};

#[derive(Debug, Serialize, Clone)]
pub struct Promo {
    pub id: i32,
    pub kode: String,
    pub nama: String,
    pub jenis: String,
    pub persen: i32,
    pub berlaku_untuk: String,
    pub hanya_pembayaran_pertama: bool,
    /// detik epoch; None = tanpa batas
    pub mulai: Option<i64>,
    pub berakhir: Option<i64>,
    pub durasi_jam: Option<i32>,
    pub kuota: Option<i32>,
    pub terpakai: i32,
    pub tampilkan_banner: bool,
    pub teks_banner: Option<String>,
    pub aktif: bool,
}

const KOLOM: &str = "id, kode, nama, jenis, persen, berlaku_untuk, hanya_pembayaran_pertama, \
    UNIX_TIMESTAMP(mulai) AS mulai, UNIX_TIMESTAMP(berakhir) AS berakhir, durasi_jam, kuota, terpakai, \
    tampilkan_banner, teks_banner, aktif";

fn dari_row(r: &sqlx::mysql::MySqlRow) -> Promo {
    let b = |k: &str| r.try_get::<i8, _>(k).map(|v| v != 0).unwrap_or(false);
    Promo {
        id: r.try_get("id").unwrap_or_default(),
        kode: r.try_get("kode").unwrap_or_default(),
        nama: r.try_get("nama").unwrap_or_default(),
        jenis: r.try_get("jenis").unwrap_or_default(),
        persen: r.try_get("persen").unwrap_or_default(),
        berlaku_untuk: r.try_get("berlaku_untuk").unwrap_or_default(),
        hanya_pembayaran_pertama: b("hanya_pembayaran_pertama"),
        mulai: r.try_get::<Option<i64>, _>("mulai").ok().flatten(),
        berakhir: r.try_get::<Option<i64>, _>("berakhir").ok().flatten(),
        durasi_jam: r.try_get::<Option<i32>, _>("durasi_jam").ok().flatten(),
        kuota: r.try_get::<Option<i32>, _>("kuota").ok().flatten(),
        terpakai: r.try_get("terpakai").unwrap_or_default(),
        tampilkan_banner: b("tampilkan_banner"),
        teks_banner: r.try_get::<Option<String>, _>("teks_banner").ok().flatten(),
        aktif: b("aktif"),
    }
}

// ── aturan murni (diuji) ─────────────────────────────────────────────────────

/// Cakupan promo vs periode plan ("monthly" / "yearly").
pub fn cocok_periode(berlaku_untuk: &str, period: Option<&str>) -> bool {
    match berlaku_untuk {
        "semua" => true,
        "bulanan" => period == Some("monthly"),
        "tahunan" => period == Some("yearly"),
        _ => false,
    }
}

/// Potongan dibulatkan ke rupiah terdekat dan tidak pernah melebihi harga.
pub fn hitung_potongan(harga: i64, persen: i32) -> i64 {
    let p = persen.clamp(0, 100) as i64;
    ((harga * p + 50) / 100).min(harga)
}

/// Kapan penawaran ini berakhir untuk user ini (detik epoch), atau None kalau
/// tidak berlaku sekarang. `trial_berakhir` diperlukan untuk jenis akhir_trial.
pub fn batas_berlaku(p: &Promo, sekarang: i64, trial_berakhir: Option<i64>) -> Option<i64> {
    if !p.aktif {
        return None;
    }
    match p.jenis.as_str() {
        "event" => {
            if p.mulai.map_or(false, |m| sekarang < m) {
                return None;
            }
            match p.berakhir {
                Some(b) if sekarang >= b => None,
                Some(b) => Some(b),
                None => Some(i64::MAX), // tanpa batas waktu
            }
        }
        "akhir_trial" => {
            let akhir_trial = trial_berakhir?;
            let berakhir = akhir_trial + p.durasi_jam.unwrap_or(24) as i64 * 3600;
            (sekarang >= akhir_trial && sekarang < berakhir).then_some(berakhir)
        }
        _ => None,
    }
}

// ── akses DB ─────────────────────────────────────────────────────────────────

pub async fn sekarang(pool: &MySqlPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT UNIX_TIMESTAMP(NOW())").fetch_one(pool).await
}

async fn sudah_dipakai(pool: &MySqlPool, promo_id: i32, user_id: &str) -> Result<bool, sqlx::Error> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dbquizapp.promo_redemptions WHERE promo_id=? AND user_id=?")
        .bind(promo_id).bind(user_id).fetch_one(pool).await?;
    Ok(n > 0)
}

async fn pernah_membayar(pool: &MySqlPool, user_id: &str) -> Result<bool, sqlx::Error> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dbquizapp.klikqris_transactions WHERE user_id=? AND status='PAID'",
    )
    .bind(user_id).fetch_one(pool).await?;
    Ok(n > 0)
}

async fn layak_untuk_user(pool: &MySqlPool, p: &Promo, user_id: &str) -> Result<Result<(), &'static str>, sqlx::Error> {
    if p.kuota.map_or(false, |k| p.terpakai >= k) {
        return Ok(Err("kuota_habis"));
    }
    if sudah_dipakai(pool, p.id, user_id).await? {
        return Ok(Err("sudah_dipakai"));
    }
    if p.hanya_pembayaran_pertama && pernah_membayar(pool, user_id).await? {
        return Ok(Err("hanya_pembayaran_pertama"));
    }
    Ok(Ok(()))
}

/// Promo yang boleh ditampilkan ke user ini sekarang, beserta batas waktunya.
pub async fn untuk_user(
    pool: &MySqlPool,
    user_id: &str,
    trial_berakhir: Option<i64>,
) -> Result<Vec<(Promo, i64)>, sqlx::Error> {
    let now = sekarang(pool).await?;
    let rows = sqlx::query(&format!(
        "SELECT {KOLOM} FROM dbquizapp.promo_codes WHERE aktif = 1 AND tampilkan_banner = 1"
    ))
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    for p in rows.iter().map(dari_row) {
        if let Some(batas) = batas_berlaku(&p, now, trial_berakhir) {
            if layak_untuk_user(pool, &p, user_id).await?.is_ok() {
                out.push((p, batas));
            }
        }
    }
    Ok(out)
}

/// Validasi kode untuk satu pembelian. Ok = (promo, potongan rupiah).
pub async fn validasi(
    pool: &MySqlPool,
    user_id: &str,
    kode: &str,
    period: Option<&str>,
    harga: i64,
    trial_berakhir: Option<i64>,
) -> Result<Result<(Promo, i64), &'static str>, sqlx::Error> {
    let row = sqlx::query(&format!("SELECT {KOLOM} FROM dbquizapp.promo_codes WHERE kode = ?"))
        .bind(kode.trim().to_uppercase())
        .fetch_optional(pool)
        .await?;
    let p = match row {
        Some(r) => dari_row(&r),
        None => return Ok(Err("kode_tidak_dikenal")),
    };
    let now = sekarang(pool).await?;
    if batas_berlaku(&p, now, trial_berakhir).is_none() {
        return Ok(Err("kode_tidak_berlaku"));
    }
    if !cocok_periode(&p.berlaku_untuk, period) {
        return Ok(Err("kode_tidak_untuk_paket_ini"));
    }
    if let Err(alasan) = layak_untuk_user(pool, &p, user_id).await? {
        return Ok(Err(alasan));
    }
    let potongan = hitung_potongan(harga, p.persen);
    Ok(Ok((p, potongan)))
}

/// Catat pemakaian saat lunas. Unik per (promo, user), jadi aman dipanggil ulang.
pub async fn catat_pemakaian(
    pool: &MySqlPool,
    promo_id: i32,
    user_id: &str,
    order_id: &str,
    potongan: i64,
) -> Result<(), sqlx::Error> {
    let r = sqlx::query(
        "INSERT IGNORE INTO dbquizapp.promo_redemptions (promo_id, user_id, order_id, potongan) VALUES (?,?,?,?)",
    )
    .bind(promo_id).bind(user_id).bind(order_id).bind(potongan)
    .execute(pool).await?;
    if r.rows_affected() == 1 {
        sqlx::query("UPDATE dbquizapp.promo_codes SET terpakai = terpakai + 1 WHERE id = ?")
            .bind(promo_id).execute(pool).await?;
    }
    Ok(())
}

// ── admin ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PromoInput {
    pub kode: String,
    pub nama: String,
    pub jenis: String,
    pub persen: i32,
    pub berlaku_untuk: String,
    #[serde(default = "benar")]
    pub hanya_pembayaran_pertama: bool,
    /// "YYYY-MM-DD HH:MM:SS" waktu WIB, atau kosong
    pub mulai: Option<String>,
    pub berakhir: Option<String>,
    pub durasi_jam: Option<i32>,
    pub kuota: Option<i32>,
    #[serde(default = "benar")]
    pub tampilkan_banner: bool,
    pub teks_banner: Option<String>,
    #[serde(default = "benar")]
    pub aktif: bool,
}

fn benar() -> bool {
    true
}

/// Periksa isian admin sebelum disimpan. Err berisi pesan untuk admin.
pub fn periksa_input(p: &PromoInput) -> Result<(), String> {
    let kode = p.kode.trim();
    if kode.is_empty() || kode.len() > 40 || !kode.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Kode harus 1–40 karakter: huruf, angka, '-' atau '_'.".into());
    }
    if !(1..=100).contains(&p.persen) {
        return Err("Persen diskon harus 1–100.".into());
    }
    if !["bulanan", "tahunan", "semua"].contains(&p.berlaku_untuk.as_str()) {
        return Err("Berlaku untuk harus bulanan, tahunan, atau semua.".into());
    }
    match p.jenis.as_str() {
        "event" => {
            if let (Some(m), Some(b)) = (&p.mulai, &p.berakhir) {
                if !m.is_empty() && !b.is_empty() && b <= m {
                    return Err("Waktu berakhir harus setelah waktu mulai.".into());
                }
            }
        }
        "akhir_trial" => {
            if p.durasi_jam.map_or(true, |d| d < 1) {
                return Err("Penawaran akhir trial butuh durasi minimal 1 jam.".into());
            }
        }
        _ => return Err("Jenis harus event atau akhir_trial.".into()),
    }
    if p.kuota.map_or(false, |k| k < 1) {
        return Err("Kuota minimal 1, atau kosongkan untuk tanpa batas.".into());
    }
    Ok(())
}

fn kosong_jadi_null(v: &Option<String>) -> Option<String> {
    v.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub async fn daftar(pool: &MySqlPool) -> Result<Vec<Promo>, sqlx::Error> {
    let rows = sqlx::query(&format!("SELECT {KOLOM} FROM dbquizapp.promo_codes ORDER BY aktif DESC, id DESC"))
        .fetch_all(pool).await?;
    Ok(rows.iter().map(dari_row).collect())
}

pub async fn simpan(pool: &MySqlPool, id: Option<i32>, p: &PromoInput) -> Result<i32, sqlx::Error> {
    let q = if id.is_some() {
        "UPDATE dbquizapp.promo_codes SET kode=?, nama=?, jenis=?, persen=?, berlaku_untuk=?, \
         hanya_pembayaran_pertama=?, mulai=?, berakhir=?, durasi_jam=?, kuota=?, tampilkan_banner=?, \
         teks_banner=?, aktif=? WHERE id=?"
    } else {
        "INSERT INTO dbquizapp.promo_codes (kode, nama, jenis, persen, berlaku_untuk, hanya_pembayaran_pertama, \
         mulai, berakhir, durasi_jam, kuota, tampilkan_banner, teks_banner, aktif) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)"
    };
    let mut query = sqlx::query(q)
        .bind(p.kode.trim().to_uppercase())
        .bind(p.nama.trim())
        .bind(&p.jenis)
        .bind(p.persen)
        .bind(&p.berlaku_untuk)
        .bind(p.hanya_pembayaran_pertama)
        .bind(kosong_jadi_null(&p.mulai))
        .bind(kosong_jadi_null(&p.berakhir))
        .bind(p.durasi_jam)
        .bind(p.kuota)
        .bind(p.tampilkan_banner)
        .bind(kosong_jadi_null(&p.teks_banner))
        .bind(p.aktif);
    if let Some(i) = id {
        query = query.bind(i);
    }
    let r = query.execute(pool).await?;
    Ok(id.unwrap_or(r.last_insert_id() as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn promo(jenis: &str) -> Promo {
        Promo {
            id: 1, kode: "X".into(), nama: "x".into(), jenis: jenis.into(), persen: 50,
            berlaku_untuk: "bulanan".into(), hanya_pembayaran_pertama: true,
            mulai: None, berakhir: None, durasi_jam: Some(24), kuota: None, terpakai: 0,
            tampilkan_banner: true, teks_banner: None, aktif: true,
        }
    }

    #[test]
    fn potongan_50_persen_bulanan() {
        assert_eq!(hitung_potongan(200_000, 50), 100_000);
        assert_eq!(hitung_potongan(1_920_000, 20), 384_000);
        assert_eq!(hitung_potongan(199_999, 50), 100_000, "dibulatkan ke rupiah terdekat");
        assert_eq!(hitung_potongan(200_000, 150), 200_000, "tidak pernah melebihi harga");
    }

    #[test]
    fn cakupan_periode() {
        assert!(cocok_periode("bulanan", Some("monthly")));
        assert!(!cocok_periode("bulanan", Some("yearly")));
        assert!(cocok_periode("tahunan", Some("yearly")));
        assert!(cocok_periode("semua", Some("yearly")));
        assert!(!cocok_periode("bulanan", None), "plan lama tanpa periode tidak bisa dipromo");
    }

    #[test]
    fn event_hanya_dalam_rentang_waktu() {
        let mut p = promo("event");
        p.mulai = Some(1_000);
        p.berakhir = Some(2_000);
        assert_eq!(batas_berlaku(&p, 999, None), None, "belum mulai");
        assert_eq!(batas_berlaku(&p, 1_500, None), Some(2_000));
        assert_eq!(batas_berlaku(&p, 2_000, None), None, "tepat di batas sudah berakhir");
        p.aktif = false;
        assert_eq!(batas_berlaku(&p, 1_500, None), None, "dimatikan admin");
    }

    #[test]
    fn akhir_trial_hanya_24_jam_setelah_trial_habis() {
        let p = promo("akhir_trial");
        let trial_habis = 10_000;
        assert_eq!(batas_berlaku(&p, 9_999, Some(trial_habis)), None, "trial masih jalan");
        assert_eq!(batas_berlaku(&p, 10_000, Some(trial_habis)), Some(10_000 + 86_400));
        assert_eq!(batas_berlaku(&p, 10_000 + 86_400, Some(trial_habis)), None, "24 jam lewat");
        assert_eq!(batas_berlaku(&p, 10_000, None), None, "belum pernah trial");
    }

    #[test]
    fn isian_admin_diperiksa() {
        let ok = PromoInput {
            kode: "MERDEKA50".into(), nama: "Kemerdekaan".into(), jenis: "event".into(), persen: 50,
            berlaku_untuk: "bulanan".into(), hanya_pembayaran_pertama: true,
            mulai: Some("2026-08-10 00:00:00".into()), berakhir: Some("2026-08-17 23:59:59".into()),
            durasi_jam: None, kuota: None, tampilkan_banner: true, teks_banner: None, aktif: true,
        };
        assert!(periksa_input(&ok).is_ok());
        assert!(periksa_input(&PromoInput { persen: 0, ..clone(&ok) }).is_err());
        assert!(periksa_input(&PromoInput { kode: "ada spasi".into(), ..clone(&ok) }).is_err());
        assert!(periksa_input(&PromoInput {
            mulai: Some("2026-08-17 00:00:00".into()), berakhir: Some("2026-08-10 00:00:00".into()), ..clone(&ok)
        }).is_err(), "berakhir sebelum mulai");
        assert!(periksa_input(&PromoInput { jenis: "akhir_trial".into(), durasi_jam: None, ..clone(&ok) }).is_err());
    }

    fn clone(p: &PromoInput) -> PromoInput {
        PromoInput {
            kode: p.kode.clone(), nama: p.nama.clone(), jenis: p.jenis.clone(), persen: p.persen,
            berlaku_untuk: p.berlaku_untuk.clone(), hanya_pembayaran_pertama: p.hanya_pembayaran_pertama,
            mulai: p.mulai.clone(), berakhir: p.berakhir.clone(), durasi_jam: p.durasi_jam, kuota: p.kuota,
            tampilkan_banner: p.tampilkan_banner, teks_banner: p.teks_banner.clone(), aktif: p.aktif,
        }
    }
}
