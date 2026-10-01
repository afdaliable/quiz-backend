//! Notifikasi ke pemilik lewat bot Telegram, dengan tombol Setujui/Tolak
//! langsung di pesannya supaya keputusan bisa diambil dari HP tanpa membuka
//! halaman admin.

use reqwest::Client;
use serde_json::json;

use crate::config::TelegramConfig;

pub struct TelegramService {
    bot_token: String,
    chat_id: String,
    http: Client,
}

impl TelegramService {
    pub fn new(cfg: &TelegramConfig) -> Self {
        TelegramService {
            bot_token: cfg.bot_token.clone(),
            chat_id: cfg.chat_id.clone(),
            http: Client::new(),
        }
    }

    fn url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.bot_token, method)
    }

    /// Kirim pemberitahuan klaim + dua tombol keputusan. `callback_data` dibatasi
    /// 64 byte oleh Telegram, jadi isinya hanya aksi dan id klaim.
    pub async fn kirim_klaim(
        &self,
        claim_id: i64,
        nama_plan: &str,
        unique_amount: i64,
        user_label: &str,
        batas: Option<&str>,
    ) -> Result<(), String> {
        let catatan_batas = match batas {
            Some(t) => format!("\nOtomatis dicabut: {t}"),
            None => String::new(),
        };
        let teks = format!(
            "💰 Klaim pembayaran #{claim_id}\n\nPaket: {nama_plan}\nNominal: Rp{unique_amount}\nUser: {user_label}\n\nAkses premium SUDAH diaktifkan. Cek mutasi, lalu putuskan.{catatan_batas}"
        );
        let body = json!({
            "chat_id": self.chat_id,
            "text": teks,
            "reply_markup": {"inline_keyboard": [[
                {"text": "✅ Lunas, setujui", "callback_data": format!("ok:{claim_id}")},
                {"text": "❌ Tidak ada, tolak", "callback_data": format!("no:{claim_id}")}
            ]]}
        });
        self.post("sendMessage", body).await
    }

    /// Jawab tekanan tombol supaya loading di Telegram berhenti.
    pub async fn jawab_callback(&self, callback_id: &str, teks: &str) -> Result<(), String> {
        self.post("answerCallbackQuery", json!({"callback_query_id": callback_id, "text": teks}))
            .await
    }

    pub async fn kirim_teks(&self, teks: &str) -> Result<(), String> {
        self.post("sendMessage", json!({"chat_id": self.chat_id, "text": teks})).await
    }

    async fn post(&self, method: &str, body: serde_json::Value) -> Result<(), String> {
        let res = self
            .http
            .post(self.url(method))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("telegram {method} gagal: {e}"))?;
        if res.status().is_success() {
            Ok(())
        } else {
            let kode = res.status();
            let isi = res.text().await.unwrap_or_default();
            Err(format!("telegram {method} status {kode}: {}", &isi[..isi.len().min(200)]))
        }
    }
}

/// Pecah `callback_data` jadi (aksi, id klaim). Hanya "ok" dan "no" diterima.
pub fn parse_callback(data: &str) -> Option<(bool, i64)> {
    let (aksi, id) = data.split_once(':')?;
    let id = id.parse::<i64>().ok()?;
    match aksi {
        "ok" => Some((true, id)),
        "no" => Some((false, id)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_dibaca_benar() {
        assert_eq!(parse_callback("ok:42"), Some((true, 42)));
        assert_eq!(parse_callback("no:7"), Some((false, 7)));
    }

    #[test]
    fn callback_asing_ditolak() {
        assert_eq!(parse_callback("hapus:42"), None);
        assert_eq!(parse_callback("ok:bukanangka"), None);
        assert_eq!(parse_callback("ok"), None);
        assert_eq!(parse_callback(""), None);
    }
}
