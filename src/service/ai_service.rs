//! AI Service Layer — provider tunggal via 9router (self-hosted OpenAI-compatible
//! gateway, model "default-soal").
//!
//! Penggunaan dari controller:
//! ```ignore
//! let enriched = state.ai_service
//!     .as_ref()
//!     .ok_or(/* 503 */)?
//!     .enrich_question(&soal_ctx)
//!     .await?;
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::Config;

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Rate limit exceeded")]
    RateLimit,
    #[error("Request timed out")]
    Timeout,
    #[error("Both providers failed — primary: {0}")]
    BothProvidersFailed(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// Domain types
// ─────────────────────────────────────────────────────────────────────────────

/// Soal yang akan diperkaya AI.
pub struct SoalContext {
    pub id: i64,
    pub soal: String,
    pub opt1: Option<String>,
    pub opt2: Option<String>,
    pub opt3: Option<String>,
    pub opt4: Option<String>,
    pub opt5: Option<String>,
    pub correct_answer: Option<String>,
    /// Field mana yang diminta: ["solution", "tag", "modul", "pelajaran"]
    pub fields_to_enrich: Vec<String>,
}

/// Hasil pengayaan dari AI.
#[derive(Debug)]
pub struct EnrichedContent {
    pub solution: Option<String>,
    pub tag: Option<String>,
    pub modul: Option<String>,
    pub pelajaran: Option<String>,
    pub provider_used: String,
    pub model_used: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// AiProvider trait
// ─────────────────────────────────────────────────────────────────────────────

/// Trait yang harus diimplementasi oleh setiap AI provider.
/// Returns `(response_text, prompt_tokens, completion_tokens)`.
#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<(String, u32, u32), AiError>;
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;
}

// ─────────────────────────────────────────────────────────────────────────────
// 9router provider (self-hosted OpenAI-compatible gateway)
// ─────────────────────────────────────────────────────────────────────────────

struct NineRouterProvider {
    base_url: String,
    api_key: String,
    model: String,
    temperature: f32,
    client: Client,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageContent,
}

#[derive(Deserialize)]
struct ChatMessageContent {
    content: String,
}

#[derive(Deserialize, Default)]
struct ChatUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Option<ChatUsage>,
}

#[async_trait]
impl AiProvider for NineRouterProvider {
    async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<(String, u32, u32), AiError> {
        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens,
            temperature: self.temperature,
            stream: false,
        };

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AiError::Timeout
                } else {
                    AiError::ProviderError(e.to_string())
                }
            })?;

        if resp.status().as_u16() == 429 {
            return Err(AiError::RateLimit);
        }
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(AiError::ProviderError(format!("HTTP {}: {}", status, body_text)));
        }

        let parsed: ChatResponse = resp
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;

        let content = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();
        let usage = parsed.usage.unwrap_or_default();

        Ok((content, usage.prompt_tokens, usage.completion_tokens))
    }

    fn provider_name(&self) -> &str {
        "9router"
    }
    fn model_name(&self) -> &str {
        &self.model
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// JSON response struct dari AI
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AiResponseJson {
    solution: Option<String>,
    tag: Option<String>,
    modul: Option<String>,
    pelajaran: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// AiService
// ─────────────────────────────────────────────────────────────────────────────

pub struct AiService {
    primary: Box<dyn AiProvider>,
    fallback: Option<Box<dyn AiProvider>>,
    max_tokens: u32,
}

impl AiService {
    /// Buat AiService dari config. Returns None jika section `ai` tidak ada di config.json.
    pub fn from_config(config: &Config) -> Option<Self> {
        let ai_cfg = config.get_ai_config()?;

        let timeout = Duration::from_secs(ai_cfg.timeout_secs);
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to build HTTP client for AI service");

        let primary: Box<dyn AiProvider> = Box::new(NineRouterProvider {
            base_url: ai_cfg.base_url.clone(),
            api_key: ai_cfg.api_key.clone(),
            model: ai_cfg.model.clone(),
            temperature: ai_cfg.temperature,
            client: client.clone(),
        });

        // Single provider -- the "fallback" slot is a retry-once against the
        // same 9router endpoint, since it's the only backend we call now.
        let fallback: Option<Box<dyn AiProvider>> = Some(Box::new(NineRouterProvider {
            base_url: ai_cfg.base_url.clone(),
            api_key: ai_cfg.api_key.clone(),
            model: ai_cfg.model.clone(),
            temperature: ai_cfg.temperature,
            client,
        }));

        Some(AiService {
            primary,
            fallback,
            max_tokens: ai_cfg.max_tokens,
        })
    }

    /// Constructor untuk testing (mock provider).
    #[cfg(test)]
    pub fn new_with_providers(
        primary: Box<dyn AiProvider>,
        fallback: Option<Box<dyn AiProvider>>,
        max_tokens: u32,
    ) -> Self {
        AiService {
            primary,
            fallback,
            max_tokens,
        }
    }

    /// Perkaya sebuah soal menggunakan AI. Coba primary provider dulu;
    /// jika gagal, fallback ke provider kedua.
    pub async fn enrich_question(&self, soal: &SoalContext) -> Result<EnrichedContent, AiError> {
        let prompt = Self::build_prompt(soal);

        match self.primary.complete(&prompt, self.max_tokens).await {
            Ok((text, prompt_tokens, completion_tokens)) => {
                let parsed = Self::parse_ai_response(&text)?;
                Ok(EnrichedContent {
                    solution: parsed.solution,
                    tag: parsed.tag,
                    modul: parsed.modul,
                    pelajaran: parsed.pelajaran,
                    provider_used: self.primary.provider_name().to_string(),
                    model_used: self.primary.model_name().to_string(),
                    prompt_tokens,
                    completion_tokens,
                })
            }
            Err(primary_err) => {
                match &self.fallback {
                    Some(fallback) => {
                        eprintln!(
                            "[AiService] Primary ({}) failed: {}. Trying fallback ({})...",
                            self.primary.provider_name(),
                            primary_err,
                            fallback.provider_name(),
                        );
                        match fallback.complete(&prompt, self.max_tokens).await {
                            Ok((text, prompt_tokens, completion_tokens)) => {
                                let parsed = Self::parse_ai_response(&text)?;
                                Ok(EnrichedContent {
                                    solution: parsed.solution,
                                    tag: parsed.tag,
                                    modul: parsed.modul,
                                    pelajaran: parsed.pelajaran,
                                    provider_used: fallback.provider_name().to_string(),
                                    model_used: fallback.model_name().to_string(),
                                    prompt_tokens,
                                    completion_tokens,
                                })
                            }
                            Err(fallback_err) => Err(AiError::BothProvidersFailed(format!(
                                "primary: {}, fallback: {}",
                                primary_err, fallback_err
                            ))),
                        }
                    }
                    None => Err(primary_err),
                }
            }
        }
    }

    fn build_prompt(soal: &SoalContext) -> String {
        let opts = [
            soal.opt1.as_deref().unwrap_or(""),
            soal.opt2.as_deref().unwrap_or(""),
            soal.opt3.as_deref().unwrap_or(""),
            soal.opt4.as_deref().unwrap_or(""),
            soal.opt5.as_deref().unwrap_or(""),
        ];
        let labels = ["A", "B", "C", "D", "E"];

        let mut options_text = String::new();
        for (label, opt) in labels.iter().zip(opts.iter()) {
            if !opt.is_empty() {
                options_text.push_str(&format!("{}. {}\n", label, opt));
            }
        }

        let correct = soal.correct_answer.as_deref().unwrap_or("-");

        format!(
            "Kamu adalah asisten pendidikan yang menganalisis soal ujian berbagai topik ujian dengan bahasa Indonesia.\n\n\
             Soal:\n{soal_text}\n\n\
             Pilihan jawaban:\n{options}\n\
             Jawaban benar: {correct}\n\n\
             Berikan output JSON dengan format TEPAT ini (tanpa penjelasan tambahan):\n\
             {{\n  \
               \"solution\": \"penjelasan mengapa jawaban benar, 2-4 kalimat\",\n  \
               \"tag\": \"topik-utama\",\n  \
               \"modul\": \"nama modul/bab jika bisa dideteksi, kosong jika tidak\",\n  \
               \"pelajaran\": \"nama mata pelajaran jika bisa dideteksi, kosong jika tidak\"\n\
             }}",
            soal_text = soal.soal,
            options = options_text,
            correct = correct,
        )
    }

    /// Parse JSON response dari AI. Handles markdown code fences dan leading/trailing text.
    fn parse_ai_response(text: &str) -> Result<AiResponseJson, AiError> {
        let extracted = extract_json_object(text);
        serde_json::from_str::<AiResponseJson>(extracted).map_err(|e| {
            AiError::ParseError(format!(
                "Invalid JSON from AI: {} (raw snippet: {})",
                e,
                &text[..text.len().min(300)]
            ))
        })
    }
}

/// Ekstrak blok `{ ... }` terluar dari teks AI (strip markdown fences + leading text).
fn extract_json_object(text: &str) -> &str {
    // 1. Strip markdown code fences: ```json ... ``` atau ``` ... ```
    let text = text.trim();
    let inner = if text.starts_with("```") {
        let after = if text.starts_with("```json") {
            &text["```json".len()..]
        } else {
            &text["```".len()..]
        };
        let trimmed = after.trim_start();
        if let Some(end) = trimmed.rfind("```") {
            trimmed[..end].trim()
        } else {
            trimmed
        }
    } else {
        text
    };

    // 2. Cari { ... } terluar
    if let (Some(start), Some(end)) = (inner.find('{'), inner.rfind('}')) {
        &inner[start..=end]
    } else {
        inner
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Mock provider ──────────────────────────────────────────────────────────

    struct MockProvider {
        name: &'static str,
        model: &'static str,
        /// Some(response_text) → success; None → error
        response: Option<&'static str>,
    }

    #[async_trait]
    impl AiProvider for MockProvider {
        async fn complete(
            &self,
            _prompt: &str,
            _max_tokens: u32,
        ) -> Result<(String, u32, u32), AiError> {
            match self.response {
                Some(text) => Ok((text.to_string(), 10, 20)),
                None => Err(AiError::ProviderError("mock provider error".to_string())),
            }
        }
        fn provider_name(&self) -> &str {
            self.name
        }
        fn model_name(&self) -> &str {
            self.model
        }
    }

    fn sample_soal() -> SoalContext {
        SoalContext {
            id: 1,
            soal: "Berapakah hasil dari 2 + 2?".to_string(),
            opt1: Some("3".to_string()),
            opt2: Some("4".to_string()),
            opt3: Some("5".to_string()),
            opt4: Some("6".to_string()),
            opt5: None,
            correct_answer: Some("B".to_string()),
            fields_to_enrich: vec!["solution".to_string(), "tag".to_string()],
        }
    }

    const VALID_JSON_RESPONSE: &str = r#"{
  "solution": "2 + 2 = 4 karena penjumlahan dua bilangan cacah.",
  "tag": "matematika-dasar",
  "modul": "Operasi Hitung",
  "pelajaran": "Matematika"
}"#;

    // ── Test: primary berhasil ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_primary_success() {
        let service = AiService::new_with_providers(
            Box::new(MockProvider {
                name: "deepseek",
                model: "deepseek-chat",
                response: Some(VALID_JSON_RESPONSE),
            }),
            None,
            2048,
        );

        let result = service.enrich_question(&sample_soal()).await.unwrap();
        assert_eq!(result.provider_used, "deepseek");
        assert_eq!(result.model_used, "deepseek-chat");
        assert_eq!(result.prompt_tokens, 10);
        assert_eq!(result.completion_tokens, 20);
        assert!(result.solution.is_some());
        assert_eq!(result.tag.as_deref(), Some("matematika-dasar"));
        assert_eq!(result.modul.as_deref(), Some("Operasi Hitung"));
        assert_eq!(result.pelajaran.as_deref(), Some("Matematika"));
    }

    // ── Test: primary gagal → fallback dipanggil ──────────────────────────────

    #[tokio::test]
    async fn test_fallback_on_primary_failure() {
        let service = AiService::new_with_providers(
            Box::new(MockProvider {
                name: "deepseek",
                model: "deepseek-chat",
                response: None, // primary gagal
            }),
            Some(Box::new(MockProvider {
                name: "gemini",
                model: "gemini-2.0-flash",
                response: Some(VALID_JSON_RESPONSE),
            })),
            2048,
        );

        let result = service.enrich_question(&sample_soal()).await.unwrap();
        assert_eq!(result.provider_used, "gemini");
        assert_eq!(result.model_used, "gemini-2.0-flash");
        assert!(result.solution.is_some());
    }

    // ── Test: kedua provider gagal ────────────────────────────────────────────

    #[tokio::test]
    async fn test_both_providers_fail() {
        let service = AiService::new_with_providers(
            Box::new(MockProvider {
                name: "deepseek",
                model: "deepseek-chat",
                response: None,
            }),
            Some(Box::new(MockProvider {
                name: "gemini",
                model: "gemini-2.0-flash",
                response: None,
            })),
            2048,
        );

        let err = service.enrich_question(&sample_soal()).await.unwrap_err();
        assert!(matches!(err, AiError::BothProvidersFailed(_)));
    }

    // ── Test: tidak ada fallback, primary gagal ───────────────────────────────

    #[tokio::test]
    async fn test_no_fallback_primary_failure() {
        let service = AiService::new_with_providers(
            Box::new(MockProvider {
                name: "deepseek",
                model: "deepseek-chat",
                response: None,
            }),
            None,
            2048,
        );

        let err = service.enrich_question(&sample_soal()).await.unwrap_err();
        assert!(matches!(err, AiError::ProviderError(_)));
    }

    // ── Test: JSON parse error dari response AI ───────────────────────────────

    #[tokio::test]
    async fn test_json_parse_error() {
        let service = AiService::new_with_providers(
            Box::new(MockProvider {
                name: "deepseek",
                model: "deepseek-chat",
                response: Some("Ini bukan JSON sama sekali."),
            }),
            None,
            2048,
        );

        let err = service.enrich_question(&sample_soal()).await.unwrap_err();
        assert!(matches!(err, AiError::ParseError(_)));
    }

    // ── Test: AI response dibungkus markdown code fence ───────────────────────

    #[tokio::test]
    async fn test_json_in_markdown_fence() {
        let fenced = format!("```json\n{}\n```", VALID_JSON_RESPONSE);
        let service = AiService::new_with_providers(
            Box::new(MockProvider {
                name: "deepseek",
                model: "deepseek-chat",
                response: Some(Box::leak(fenced.into_boxed_str())),
            }),
            None,
            2048,
        );

        let result = service.enrich_question(&sample_soal()).await.unwrap();
        assert!(result.solution.is_some());
    }

    // ── Test: extract_json_object ─────────────────────────────────────────────

    #[test]
    fn test_extract_json_plain() {
        let text = r#"{"solution": "x", "tag": "y", "modul": "", "pelajaran": ""}"#;
        assert_eq!(extract_json_object(text), text);
    }

    #[test]
    fn test_extract_json_with_fences() {
        let text = "```json\n{\"solution\": \"x\"}\n```";
        assert_eq!(extract_json_object(text), "{\"solution\": \"x\"}");
    }

    #[test]
    fn test_extract_json_with_leading_text() {
        let text = "Berikut adalah jawaban:\n{\"solution\": \"x\", \"tag\": \"y\", \"modul\": \"\", \"pelajaran\": \"\"}";
        assert!(extract_json_object(text).starts_with('{'));
    }

    // ── Test: build_prompt berisi soal & pilihan ──────────────────────────────

    #[test]
    fn test_build_prompt_contains_question() {
        let soal = sample_soal();
        let prompt = AiService::build_prompt(&soal);
        assert!(prompt.contains("2 + 2"));
        assert!(prompt.contains("B. 4"));
        assert!(prompt.contains("Jawaban benar: B"));
    }

    #[test]
    fn test_build_prompt_skips_empty_options() {
        let soal = sample_soal(); // opt5 = None
        let prompt = AiService::build_prompt(&soal);
        assert!(!prompt.contains("E. "));
    }
}
