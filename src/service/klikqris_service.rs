//! Klien API KlikQRIS. Bentuk respons diambil dari panggilan nyata, bukan hanya
//! dokumentasi: `create` membalas 201, nominal berupa string desimal ("1084.00"),
//! dan status lunas bernilai "SUCCESS" di endpoint cek tapi "PAID" di webhook.

use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::config::KlikqrisConfig;

#[derive(Debug, Clone)]
pub struct Transaksi {
    pub order_id: String,
    pub amount: i64,
    pub total_amount: i64,
    pub status: String,
    pub signature: String,
    pub qris_url: Option<String>,
    pub qris_image: Option<String>,
    pub report_url: Option<String>,
    pub expired_at: Option<String>,
    pub paid_at: Option<String>,
}

/// Nominal KlikQRIS datang sebagai string desimal ("1084.00") atau angka.
pub fn nominal(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_f64().map(|f| f.round() as i64),
        Value::String(s) => s.trim().parse::<f64>().ok().map(|f| f.round() as i64),
        _ => None,
    }
}

/// Satukan istilah status: endpoint cek memakai SUCCESS, webhook memakai PAID.
pub fn status_baku(s: &str) -> &'static str {
    match s.trim().to_ascii_uppercase().as_str() {
        "PAID" | "SUCCESS" | "SETTLED" => "PAID",
        "EXPIRED" | "EXPIRE" => "EXPIRED",
        _ => "PENDING",
    }
}

/// Perbandingan waktu-konstan supaya signature tidak bisa ditebak per karakter.
pub fn signature_sama(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() || a.is_empty() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn dari_data(d: &Value) -> Option<Transaksi> {
    let s = |k: &str| d.get(k).and_then(|v| v.as_str()).map(|x| x.to_string());
    Some(Transaksi {
        order_id: s("order_id")?,
        amount: nominal(d.get("amount")?)?,
        total_amount: nominal(d.get("total_amount")?)?,
        status: status_baku(&s("status").unwrap_or_default()).to_string(),
        signature: s("signature").unwrap_or_default(),
        qris_url: s("qris_url"),
        qris_image: s("qris_image"),
        report_url: s("report_url"),
        expired_at: s("expired_at"),
        paid_at: s("paid_at"),
    })
}

#[derive(Deserialize)]
struct Balasan {
    status: Option<bool>,
    message: Option<String>,
    data: Option<Value>,
}

pub struct KlikqrisService {
    cfg: KlikqrisConfig,
    http: Client,
}

impl KlikqrisService {
    pub fn new(cfg: &KlikqrisConfig) -> Self {
        KlikqrisService { cfg: cfg.clone(), http: Client::new() }
    }

    fn header(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header("x-api-key", &self.cfg.api_key)
            .header("id_merchant", &self.cfg.id_merchant)
            .header("Accept", "application/json")
    }

    async fn baca(res: reqwest::Response) -> Result<Transaksi, String> {
        let kode = res.status();
        let teks = res.text().await.map_err(|e| format!("klikqris: gagal membaca balasan: {e}"))?;
        let b: Balasan = serde_json::from_str(&teks)
            .map_err(|_| format!("klikqris {kode}: balasan bukan JSON: {}", &teks[..teks.len().min(160)]))?;
        if !kode.is_success() || b.status == Some(false) {
            return Err(format!("klikqris {kode}: {}", b.message.unwrap_or_default()));
        }
        b.data.as_ref().and_then(dari_data).ok_or_else(|| "klikqris: data transaksi tidak lengkap".to_string())
    }

    pub async fn buat(&self, order_id: &str, amount: i64, keterangan: &str) -> Result<Transaksi, String> {
        let mut body = json!({
            "order_id": order_id,
            "id_merchant": self.cfg.id_merchant,
            "amount": amount,
            "keterangan": keterangan,
        });
        if let Some(cb) = self.cfg.callback_url.as_ref().filter(|s| !s.is_empty()) {
            body["callback_url"] = json!(cb);
        }
        let res = self
            .header(self.http.post(format!("{}/qris/create", self.cfg.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("klikqris create gagal: {e}"))?;
        Self::baca(res).await
    }

    pub async fn cek(&self, order_id: &str) -> Result<Transaksi, String> {
        let res = self
            .header(self.http.get(format!("{}/qris/status/{}", self.cfg.base_url, order_id)))
            .send()
            .await
            .map_err(|e| format!("klikqris status gagal: {e}"))?;
        Self::baca(res).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominal_dari_string_desimal_dan_angka() {
        assert_eq!(nominal(&json!("1084.00")), Some(1084));
        assert_eq!(nominal(&json!(1215)), Some(1215));
        assert_eq!(nominal(&json!("bukan")), None);
    }

    #[test]
    fn status_disatukan() {
        assert_eq!(status_baku("SUCCESS"), "PAID");
        assert_eq!(status_baku("paid"), "PAID");
        assert_eq!(status_baku("EXPIRED"), "EXPIRED");
        assert_eq!(status_baku("PENDING"), "PENDING");
        assert_eq!(status_baku("aneh"), "PENDING");
    }

    #[test]
    fn signature_dibanding_utuh() {
        assert!(signature_sama("abc123", "abc123"));
        assert!(!signature_sama("abc123", "abc124"));
        assert!(!signature_sama("abc", "abc123"));
        assert!(!signature_sama("", ""), "signature kosong tidak boleh dianggap cocok");
    }

    #[test]
    fn data_create_asli_terbaca() {
        // Bentuk dari respons /qris/create yang sungguhan (2026-10-02).
        let d = json!({
            "order_id": "TEST-QUIZ-1", "amount": "1000.00", "amount_uniq": "84.00",
            "total_amount": "1084.00", "status": "PENDING", "signature": "x".repeat(40),
            "qris_url": "https://klikqris.com/storage/qris_api/q.png",
            "report_url": "https://klikqris.com/laporan-buyer/abc",
            "expired_at": "2026-10-02 17:52:35", "paid_at": null,
            "qris_image": "data:image/png;base64,AAAA"
        });
        let t = dari_data(&d).expect("harus terbaca");
        assert_eq!((t.amount, t.total_amount), (1000, 1084));
        assert_eq!(t.status, "PENDING");
        assert_eq!(t.signature.len(), 40);
    }
}
