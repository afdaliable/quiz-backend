//! Ubah QRIS statis milik merchant (mis. dari GoPay Merchant) jadi QRIS dinamis
//! berisi nominal, mengikuti format EMVCo yang dipakai QRIS.
//!
//! Yang dilakukan sama persis dengan konverter QRIS dinamis pada umumnya:
//!   1. buang 4 digit CRC di ekor payload;
//!   2. ganti penanda `010211` (statis) jadi `010212` (dinamis);
//!   3. sisipkan tag `54` (nominal) tepat sebelum `5802ID`;
//!   4. hitung ulang CRC16-CCITT (init 0xFFFF, polinomial 0x1021) dan tempel.
//!
//! Catatan penting: QR hasilnya sah dibayar, tapi QRIS statis TIDAK memberi
//! callback ke server. Konfirmasi pembayaran harus ditangani terpisah --
//! lihat `unique_amount` untuk mencocokkan pembayaran masuk.

const TAG_AMOUNT: &str = "54";
const COUNTRY_TAG: &str = "5802ID";
const STATIC_MARKER: &str = "010211";
const DYNAMIC_MARKER: &str = "010212";

#[derive(Debug, PartialEq)]
pub enum QrisError {
    TooShort,
    MissingCountryTag,
    AmountTooLarge,
}

impl std::fmt::Display for QrisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QrisError::TooShort => write!(f, "payload QRIS terlalu pendek"),
            QrisError::MissingCountryTag => write!(f, "payload QRIS tidak memuat 5802ID"),
            QrisError::AmountTooLarge => write!(f, "nominal melebihi 13 digit"),
        }
    }
}

/// CRC16-CCITT (False) seperti yang diminta spesifikasi QRIS.
pub fn crc16(data: &str) -> String {
    let mut crc: u16 = 0xFFFF;
    for byte in data.bytes() {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    format!("{crc:04X}")
}

/// QRIS statis -> dinamis dengan nominal rupiah (bilangan bulat).
pub fn to_dynamic(static_payload: &str, amount: u64) -> Result<String, QrisError> {
    let trimmed = static_payload.trim();
    if trimmed.len() < 8 {
        return Err(QrisError::TooShort);
    }
    // Ekor 4 digit terakhir adalah CRC lama; dibuang, nanti dihitung ulang.
    let body = &trimmed[..trimmed.len() - 4];
    let body = body.replacen(STATIC_MARKER, DYNAMIC_MARKER, 1);

    let amount_str = amount.to_string();
    if amount_str.len() > 13 {
        return Err(QrisError::AmountTooLarge);
    }
    let amount_tag = format!("{TAG_AMOUNT}{:02}{amount_str}", amount_str.len());

    let idx = body.find(COUNTRY_TAG).ok_or(QrisError::MissingCountryTag)?;
    let mut payload = String::with_capacity(body.len() + amount_tag.len() + 4);
    payload.push_str(&body[..idx]);
    payload.push_str(&amount_tag);
    payload.push_str(&body[idx..]);

    let crc = crc16(&payload);
    payload.push_str(&crc);
    Ok(payload)
}

/// Nominal unik: harga asli + 3 digit acak sebagai penanda transaksi.
///
/// QRIS statis tidak mengirim notifikasi ke server, jadi pembayaran masuk hanya
/// bisa dicocokkan lewat nominalnya. Sisipan ini membuat dua pembelian dengan
/// harga sama tidak tertukar. Pemanggil wajib memastikan nominal hasilnya belum
/// dipakai transaksi lain yang masih menunggu.
pub fn unique_amount(base: u64, suffix: u16) -> u64 {
    base + (suffix % 1000) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Payload statis contoh, strukturnya sama dengan QRIS asli
    /// (penanda statis, merchant account, 5802ID, lalu CRC).
    fn sample_static() -> String {
        let body = "00020101021126610014COM.GO-JEK.WWW01189360091400000000000210G0000000000303UMI51440014ID.CO.QRIS.WWW0215ID00000000000000303UMI5204481253033605802ID5910TOKO CONTOH6007JAKARTA61051234062070703A01";
        format!("{body}6304{}", crc16(&format!("{body}6304")))
    }

    #[test]
    fn crc16_sesuai_contoh_spesifikasi() {
        // Nilai uji baku CRC-16/CCITT-FALSE untuk string "123456789".
        assert_eq!(crc16("123456789"), "29B1");
    }

    #[test]
    fn statis_jadi_dinamis_dengan_nominal() {
        let statis = sample_static();
        let dinamis = to_dynamic(&statis, 99000).expect("harus berhasil");

        assert!(dinamis.contains(DYNAMIC_MARKER), "penanda harus jadi dinamis");
        assert!(!dinamis.contains(STATIC_MARKER), "penanda statis harus hilang");
        assert!(dinamis.contains("540599000"), "tag 54 berisi nominal 99000");
        // Tag nominal harus berada sebelum 5802ID.
        assert!(dinamis.find("540599000").unwrap() < dinamis.find(COUNTRY_TAG).unwrap());
    }

    #[test]
    fn crc_hasil_konversi_valid() {
        let dinamis = to_dynamic(&sample_static(), 150000).unwrap();
        let (body, crc) = dinamis.split_at(dinamis.len() - 4);
        assert_eq!(crc16(body), crc, "CRC harus cocok dengan isi payload");
    }

    #[test]
    fn panjang_tag_nominal_mengikuti_jumlah_digit() {
        let d = to_dynamic(&sample_static(), 5000).unwrap();
        assert!(d.contains("54045000"), "4 digit -> panjang 04");
        let d = to_dynamic(&sample_static(), 1234567).unwrap();
        assert!(d.contains("54071234567"), "7 digit -> panjang 07");
    }

    #[test]
    fn payload_tanpa_5802id_ditolak() {
        let err = to_dynamic("00020101021154041000ABCD", 1000).unwrap_err();
        assert_eq!(err, QrisError::MissingCountryTag);
    }

    #[test]
    fn nominal_unik_menambah_tiga_digit() {
        assert_eq!(unique_amount(99000, 7), 99007);
        assert_eq!(unique_amount(99000, 1234), 99234);
    }
}

#[cfg(test)]
mod lintas_implementasi {
    use super::*;

    /// Hasil harus sama persis dengan konverter QRIS lain (dihitung terpisah
    /// di luar Rust), bukan hanya konsisten dengan dirinya sendiri.
    #[test]
    fn cocok_dengan_implementasi_pembanding() {
        // Dihitung oleh implementasi lain (Python), ditanam supaya test ini
        // tidak bergantung file di luar repo.
        let harapan = "00020101021226610014COM.GO-JEK.WWW01189360091400000000000210G0000000000303UMI51440014ID.CO.QRIS.WWW0215ID00000000000000303UMI5204481253033605405990005802ID5910TOKO CONTOH6007JAKARTA61051234062070703A016304E84E";
        let body = "00020101021126610014COM.GO-JEK.WWW01189360091400000000000210G0000000000303UMI51440014ID.CO.QRIS.WWW0215ID00000000000000303UMI5204481253033605802ID5910TOKO CONTOH6007JAKARTA61051234062070703A01";
        let statis = format!("{body}6304{}", crc16(&format!("{body}6304")));
        assert_eq!(to_dynamic(&statis, 99000).unwrap(), harapan);
    }
}
