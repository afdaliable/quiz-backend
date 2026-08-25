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
use crate::model::taxonomy::{TaxonomyTree, Topic};

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

/// Satu soal hasil generate AI dari teks materi -- draft, belum disimpan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedSoal {
    pub soal: String,
    pub opt1: String,
    pub opt2: String,
    pub opt3: String,
    pub opt4: String,
    pub opt5: String,
    pub correct_answer: String,
    pub solution: String,
}

/// Hasil pengayaan dari AI.
#[derive(Debug)]
pub struct EnrichedContent {
    pub solution: Option<String>,
    pub tag: Option<String>,
    pub modul: Option<String>,
    pub pelajaran: Option<String>,
    /// Only set when "correct_answer" was requested AND the soal didn't
    /// already have one -- see build_prompt for why: the model is only
    /// asked to determine this when it isn't given it as known context.
    pub correct_answer: Option<String>,
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
    async fn complete(&self, system: &str, user: &str, max_tokens: u32) -> Result<(String, u32, u32), AiError>;
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
    async fn complete(&self, system: &str, user: &str, max_tokens: u32) -> Result<(String, u32, u32), AiError> {
        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage { role: "system".to_string(), content: system.to_string() },
                ChatMessage { role: "user".to_string(), content: user.to_string() },
            ],
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
    // Only requested conditionally (see build_prompt) -- default lets the
    // key be absent entirely when we didn't ask for it, not just null.
    #[serde(default)]
    correct_answer: Option<String>,
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
        let (system, user) = Self::build_prompt(soal);

        match self.primary.complete(&system, &user, self.max_tokens).await {
            Ok((text, prompt_tokens, completion_tokens)) => {
                let parsed = Self::parse_ai_response(&text)?;
                Ok(EnrichedContent {
                    solution: parsed.solution,
                    tag: parsed.tag,
                    modul: parsed.modul,
                    pelajaran: parsed.pelajaran,
                    correct_answer: parsed.correct_answer,
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
                        match fallback.complete(&system, &user, self.max_tokens).await {
                            Ok((text, prompt_tokens, completion_tokens)) => {
                                let parsed = Self::parse_ai_response(&text)?;
                                Ok(EnrichedContent {
                                    solution: parsed.solution,
                                    tag: parsed.tag,
                                    modul: parsed.modul,
                                    pelajaran: parsed.pelajaran,
                                    correct_answer: parsed.correct_answer,
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

    /// Returns (system_prompt, user_prompt). Kept as two separate messages
    /// (not one blob) because a dedicated system role holds instruction-
    /// following better than stuffing rules into the user turn -- some
    /// models (observed: gemini-pro-agent via 9router) otherwise "think out
    /// loud" in the response instead of emitting bare JSON, burning the
    /// token budget on prose and getting cut off by finish_reason=max_tokens
    /// before a single '{' appears.
    fn build_prompt(soal: &SoalContext) -> (String, String) {
        // Determine the answer ourselves only when it's actually missing or
        // explicitly requested -- never second-guess an answer key that's
        // already there just because "correct_answer" wasn't in the request.
        let has_answer = soal
            .correct_answer
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let wants_answer = soal.fields_to_enrich.iter().any(|f| f == "correct_answer");
        let determine_answer = wants_answer || !has_answer;

        let example = if determine_answer {
            "{\"correct_answer\": \"opt2\", \"solution\": \"2 + 2 = 4 karena penjumlahan dua bilangan cacah.\", \"tag\": \"matematika-dasar\", \"modul\": \"Operasi Hitung\", \"pelajaran\": \"Matematika\"}"
        } else {
            "{\"solution\": \"2 + 2 = 4 karena penjumlahan dua bilangan cacah.\", \"tag\": \"matematika-dasar\", \"modul\": \"Operasi Hitung\", \"pelajaran\": \"Matematika\"}"
        };
        let system = format!(
            "Kamu adalah asisten pendidikan yang menganalisis soal ujian berbagai \
            topik ujian dengan bahasa Indonesia. Tugasmu HANYA mengembalikan satu objek JSON \
            berisi penjelasan dan metadata soal -- jangan menjelaskan proses berpikirmu, \
            jangan menyapa, jangan membungkus dengan markdown code fence, jangan menulis apa \
            pun sebelum atau sesudah objek JSON itu. Balasanmu akan di-parse langsung sebagai \
            JSON, jadi karakter pertama balasanmu harus '{{' dan karakter terakhir harus '}}'.\n\n\
            Contoh balasan yang benar persis:\n{example}"
        );

        let opts = [
            ("opt1", soal.opt1.as_deref().unwrap_or("")),
            ("opt2", soal.opt2.as_deref().unwrap_or("")),
            ("opt3", soal.opt3.as_deref().unwrap_or("")),
            ("opt4", soal.opt4.as_deref().unwrap_or("")),
            ("opt5", soal.opt5.as_deref().unwrap_or("")),
        ];

        let mut options_text = String::new();
        for (key, opt) in opts.iter() {
            if !opt.is_empty() {
                options_text.push_str(&format!("{}: {}\n", key, opt));
            }
        }

        let answer_line = if determine_answer {
            String::new()
        } else {
            format!("Jawaban benar: {}\n\n", soal.correct_answer.as_deref().unwrap_or("-"))
        };

        let mut field_instructions = String::new();
        if determine_answer {
            field_instructions.push_str(
                "- \"correct_answer\": tentukan sendiri jawaban yang paling benar berdasarkan analisismu -- salah satu dari \"opt1\", \"opt2\", \"opt3\", \"opt4\", \"opt5\" (hanya opsi yang ada di atas)\n",
            );
        }
        field_instructions.push_str(
            "- \"solution\": penjelasan singkat (2-4 kalimat) kenapa jawaban di atas benar\n\
             - \"tag\": satu topik-utama singkat (kebab-case atau frasa pendek)\n\
             - \"modul\": nama modul/bab jika bisa dideteksi dari isi soal, string kosong \"\" jika tidak\n\
             - \"pelajaran\": nama mata pelajaran jika bisa dideteksi, string kosong \"\" jika tidak",
        );

        let user = format!(
            "Soal:\n{soal_text}\n\n\
             Pilihan jawaban:\n{options}\n\
             {answer_line}Isi field berikut untuk soal di atas, balas sebagai objek JSON tunggal:\n\
             {field_instructions}",
            soal_text = soal.soal,
            options = options_text,
        );

        (system, user)
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

impl AiService {
    /// Generate soal pilihan ganda dari potongan teks materi. Sama pola
    /// retry (primary lalu fallback) seperti enrich_question. Butuh
    /// max_tokens lebih besar dari base config karena generate `count` soal
    /// sekaligus dalam satu balasan, bukan satu field.
    pub async fn generate_soal(
        &self,
        materi: &str,
        group: &str,
        source_text: &str,
        count: u32,
    ) -> Result<Vec<GeneratedSoal>, AiError> {
        let (system, user) = Self::build_generate_prompt(materi, group, source_text, count);
        let max_tokens = self.max_tokens.max(500 * count.max(1));

        match self.primary.complete(&system, &user, max_tokens).await {
            Ok((text, _, _)) => Self::parse_generated_soal(&text),
            Err(primary_err) => match &self.fallback {
                Some(fallback) => {
                    eprintln!(
                        "[AiService] generate_soal primary ({}) failed: {}. Trying fallback ({})...",
                        self.primary.provider_name(),
                        primary_err,
                        fallback.provider_name(),
                    );
                    match fallback.complete(&system, &user, max_tokens).await {
                        Ok((text, _, _)) => Self::parse_generated_soal(&text),
                        Err(fallback_err) => Err(AiError::BothProvidersFailed(format!(
                            "primary: {}, fallback: {}",
                            primary_err, fallback_err
                        ))),
                    }
                }
                None => Err(primary_err),
            },
        }
    }

    fn build_generate_prompt(materi: &str, group: &str, source_text: &str, count: u32) -> (String, String) {
        let system = "Kamu penyusun soal ujian tingkat menengah berbahasa Indonesia. Berdasarkan \
            potongan materi yang diberikan, buat soal pilihan ganda (5 opsi, A-E) yang menguji \
            PEMAHAMAN, bukan hafalan kata-per-kata. Satu jawaban benar tegas, 4 pengecoh masuk akal \
            tapi jelas salah kalau materi dipahami. Balas HANYA sebagai array JSON -- jangan \
            menjelaskan proses berpikirmu, jangan menyapa, jangan membungkus dengan markdown code \
            fence, jangan menulis apa pun sebelum atau sesudah array JSON itu. Karakter pertama \
            balasanmu harus '[' dan karakter terakhir harus ']'.\n\n\
            Contoh format satu item (ulangi untuk tiap soal, semua field wajib diisi):\n\
            {\"soal\": \"Apa fungsi utama dari X?\", \"opt1\": \"...\", \"opt2\": \"...\", \"opt3\": \"...\", \"opt4\": \"...\", \"opt5\": \"...\", \"correct_answer\": \"opt2\", \"solution\": \"Penjelasan singkat kenapa opt2 benar, mengutip bagian materi.\"}"
            .to_string();

        let user = format!(
            "Materi: {materi}\n\
             Grup/topik: {group}\n\n\
             Teks:\n\"{source_text}\"\n\n\
             Buat {count} soal dari teks ini.",
        );

        (system, user)
    }

    fn parse_generated_soal(text: &str) -> Result<Vec<GeneratedSoal>, AiError> {
        let extracted = extract_json_array(text);
        serde_json::from_str::<Vec<GeneratedSoal>>(extracted).map_err(|e| {
            AiError::ParseError(format!(
                "Invalid JSON array from AI: {} (raw snippet: {})",
                e,
                &text[..text.len().min(500)]
            ))
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Taxonomy classification (2-stage: subcategory, then topic within it) +
// cross-link topic suggestion. Callers (materi_generate_controller-style job
// wiring) supply the already-fetched TaxonomyTree -- this service stays
// DB-agnostic, matching how it doesn't own the DB pool for enrich/generate
// either.
// ─────────────────────────────────────────────────────────────────────────────

impl AiService {
    async fn complete_with_retry(&self, system: &str, user: &str, max_tokens: u32) -> Result<String, AiError> {
        match self.primary.complete(system, user, max_tokens).await {
            Ok((text, _, _)) => Ok(text),
            Err(primary_err) => match &self.fallback {
                Some(fallback) => match fallback.complete(system, user, max_tokens).await {
                    Ok((text, _, _)) => Ok(text),
                    Err(fallback_err) => Err(AiError::BothProvidersFailed(format!(
                        "primary: {}, fallback: {}",
                        primary_err, fallback_err
                    ))),
                },
                None => Err(primary_err),
            },
        }
    }

    /// Stage 1: pick the best-fit subcategory slug for a soal from the full
    /// track > category > subcategory list (topics excluded here -- stage 2
    /// picks the topic from the already-fetched tree, no second DB call).
    pub async fn classify_subcategory(
        &self,
        soal_text: &str,
        options_text: &str,
        tree: &TaxonomyTree,
    ) -> Result<String, AiError> {
        let mut lines = String::new();
        for t in &tree.tracks {
            for c in &t.categories {
                for s in &c.subcategories {
                    lines.push_str(&format!(
                        "{} | {} > {} > {}\n",
                        s.subcategory.slug, t.track.name, c.category.name, s.subcategory.name
                    ));
                }
            }
        }

        let system = "Kamu asisten klasifikasi soal ujian. Tugasmu: pilih SATU subkategori yang \
            paling cocok buat soal yang diberikan, dari daftar yang disediakan. Balas HANYA satu \
            objek JSON, karakter pertama '{' terakhir '}', tanpa teks lain.\n\n\
            Contoh: {\"subcategory_slug\": \"twk-tes-wawasan-kebangsaan\"}"
            .to_string();
        let user = format!(
            "Daftar subkategori (format: slug | track > kategori > subkategori):\n{lines}\n\n\
             Soal:\n{soal_text}\n\n\
             Pilihan jawaban:\n{options_text}\n\n\
             Balas: {{\"subcategory_slug\": \"<slug persis dari daftar di atas>\"}}",
        );

        let text = self.complete_with_retry(&system, &user, self.max_tokens).await?;
        #[derive(Deserialize)]
        struct Resp {
            subcategory_slug: String,
        }
        let extracted = extract_json_object(&text);
        let parsed: Resp = serde_json::from_str(extracted).map_err(|e| {
            AiError::ParseError(format!(
                "Invalid JSON from AI (stage 1 - subcategory): {} (raw: {})",
                e,
                &text[..text.len().min(500)]
            ))
        })?;
        Ok(parsed.subcategory_slug)
    }

    /// Stage 2: pick the primary topic within an already-chosen subcategory.
    /// Returns None if the subcategory has no topics or the model can't
    /// find a good fit.
    pub async fn classify_topic(
        &self,
        soal_text: &str,
        options_text: &str,
        topics: &[Topic],
    ) -> Result<Option<String>, AiError> {
        if topics.is_empty() {
            return Ok(None);
        }
        let mut lines = String::new();
        for t in topics {
            lines.push_str(&format!("{} | {}\n", t.slug, t.name));
        }

        let system = "Kamu asisten klasifikasi soal ujian. Tugasmu: pilih SATU topik yang paling \
            cocok buat soal yang diberikan, dari daftar yang disediakan. Kalau gak ada yang cocok \
            sama sekali, balas null. Balas HANYA satu objek JSON, karakter pertama '{' terakhir \
            '}', tanpa teks lain.\n\n\
            Contoh: {\"topic_slug\": \"analogi-kata\"}"
            .to_string();
        let user = format!(
            "Daftar topik (format: slug | nama):\n{lines}\n\n\
             Soal:\n{soal_text}\n\n\
             Pilihan jawaban:\n{options_text}\n\n\
             Balas: {{\"topic_slug\": \"<slug persis dari daftar di atas>\" atau null}}",
        );

        let text = self.complete_with_retry(&system, &user, self.max_tokens).await?;
        #[derive(Deserialize)]
        struct Resp {
            topic_slug: Option<String>,
        }
        let extracted = extract_json_object(&text);
        let parsed: Resp = serde_json::from_str(extracted).map_err(|e| {
            AiError::ParseError(format!(
                "Invalid JSON from AI (stage 2 - topic): {} (raw: {})",
                e,
                &text[..text.len().min(500)]
            ))
        })?;
        Ok(parsed.topic_slug)
    }

    /// Stage 3: pick 0-5 additional cross-link topics from a pre-filtered
    /// "clean" candidate list (see filter_clean_topics -- excludes
    /// subcategories whose topics are tryout-package names, not real
    /// semantic categories).
    pub async fn classify_additional_topics(
        &self,
        soal_text: &str,
        options_text: &str,
        candidates: &[(String, String)], // (slug, "track > kategori > subkategori > topik")
    ) -> Result<Vec<String>, AiError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let mut lines = String::new();
        for (slug, label) in candidates {
            lines.push_str(&format!("{} | {}\n", slug, label));
        }

        let system = "Kamu asisten klasifikasi soal ujian. Soal ini SUDAH punya kategori utama -- \
            tugasmu sekarang cari topik TAMBAHAN dari jalur/track LAIN yang temanya juga relevan \
            (misal soal penalaran umum UTBK yang temanya juga cocok dipakai di TIU CPNS). Pilih \
            0 sampai 5 topik dari daftar, HANYA yang beneran relevan -- kosongkan array kalau \
            gak ada yang cocok, jangan maksain. Balas HANYA satu objek JSON, karakter pertama \
            '{' terakhir '}', tanpa teks lain.\n\n\
            Contoh: {\"additional_topic_slugs\": [\"analogi-kata\", \"sinonim-antonim\"]}"
            .to_string();
        let user = format!(
            "Daftar kandidat topik lintas-jalur (format: slug | track > kategori > subkategori > topik):\n{lines}\n\n\
             Soal:\n{soal_text}\n\n\
             Pilihan jawaban:\n{options_text}\n\n\
             Balas: {{\"additional_topic_slugs\": [<0-5 slug persis dari daftar di atas>]}}",
        );

        let text = self.complete_with_retry(&system, &user, self.max_tokens).await?;
        #[derive(Deserialize)]
        struct Resp {
            additional_topic_slugs: Vec<String>,
        }
        let extracted = extract_json_object(&text);
        let parsed: Resp = serde_json::from_str(extracted).map_err(|e| {
            AiError::ParseError(format!(
                "Invalid JSON from AI (stage 3 - cross-link): {} (raw: {})",
                e,
                &text[..text.len().min(500)]
            ))
        })?;
        Ok(parsed.additional_topic_slugs)
    }
}

/// Marks a whole subcategory's topics as excluded from cross-link
/// candidates if ANY of its topic names look like a tryout-package name
/// (numbered, or containing "tryout"/"paket"/"simulasi") rather than a real
/// semantic category -- same heuristic the soal-enrichment pipeline used
/// (src/cross_link.py in that project), ported here so this feature doesn't
/// need a Python sidecar.
pub fn filter_clean_topics(tree: &TaxonomyTree, exclude_subcategory_id: &str) -> Vec<(String, String)> {
    let messy_re = regex::Regex::new(r"(?i)^\d|tryout|paket|simulasi").unwrap();
    let mut out = Vec::new();
    for t in &tree.tracks {
        for c in &t.categories {
            for s in &c.subcategories {
                if s.subcategory.id == exclude_subcategory_id {
                    continue;
                }
                let messy = s.topics.iter().any(|topic| messy_re.is_match(&topic.name));
                if messy {
                    continue;
                }
                for topic in &s.topics {
                    let label = format!(
                        "{} > {} > {} > {}",
                        t.track.name, c.category.name, s.subcategory.name, topic.name
                    );
                    out.push((topic.slug.clone(), label));
                }
            }
        }
    }
    out
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

/// Ekstrak blok `[ ... ]` terluar dari teks AI (strip markdown fences + leading text).
fn extract_json_array(text: &str) -> &str {
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

    if let (Some(start), Some(end)) = (inner.find('['), inner.rfind(']')) {
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
            _system: &str,
            _user: &str,
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
        let (_system, user) = AiService::build_prompt(&soal);
        assert!(user.contains("2 + 2"));
        assert!(user.contains("opt2: 4"));
        assert!(user.contains("Jawaban benar: B"));
    }

    #[test]
    fn test_build_prompt_skips_empty_options() {
        let soal = sample_soal(); // opt5 = None
        let (_system, user) = AiService::build_prompt(&soal);
        assert!(!user.contains("opt5:"));
    }

    #[test]
    fn test_build_prompt_asks_for_answer_when_missing() {
        let mut soal = sample_soal();
        soal.correct_answer = None;
        let (system, user) = AiService::build_prompt(&soal);
        assert!(user.contains("\"correct_answer\""));
        assert!(!user.contains("Jawaban benar:"));
        assert!(system.contains("\"correct_answer\": \"opt2\""));
    }

    #[test]
    fn test_build_prompt_asks_for_answer_when_explicitly_requested() {
        let mut soal = sample_soal(); // has correct_answer = Some("B")
        soal.fields_to_enrich = vec!["correct_answer".to_string()];
        let (_system, user) = AiService::build_prompt(&soal);
        assert!(user.contains("\"correct_answer\""));
        assert!(!user.contains("Jawaban benar:"));
    }

    #[test]
    fn test_build_prompt_does_not_ask_for_answer_when_present_and_not_requested() {
        let soal = sample_soal(); // has correct_answer, fields = [solution, tag]
        let (_system, user) = AiService::build_prompt(&soal);
        assert!(!user.contains("\"correct_answer\""));
        assert!(user.contains("Jawaban benar: B"));
    }

    #[test]
    fn test_build_prompt_system_demands_json_only() {
        let soal = sample_soal();
        let (system, _user) = AiService::build_prompt(&soal);
        assert!(system.contains("JSON"));
        assert!(system.contains('{') && system.contains('}'));
    }
}
