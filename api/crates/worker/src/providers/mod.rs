//! Provider traits the pipeline is written against, plus the deterministic fakes
//! used in development and tests and the HTTP adapters used in production.
//!
//! A step never talks to a vendor directly: it asks a trait for a result, and the
//! result is either `Ready` (bytes the worker stores) or `Pending` (a provider
//! reference that a later `provider_callback` job completes).

pub mod blocked;
pub mod fake;
pub mod jev;
pub mod muse;
pub mod real;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// Transient: the job is retried with backoff (or paused after many attempts).
    #[error("provider unavailable: {0}")]
    Unavailable(String),
    /// Permanent: the step fails and the admin sees the message.
    #[error("provider rejected the request: {0}")]
    Rejected(String),
    #[error("provider not configured: {0}")]
    Config(String),
}

impl ProviderError {
    pub fn is_transient(&self) -> bool {
        matches!(self, ProviderError::Unavailable(_))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extraction {
    pub text: String,
    pub language: Option<String>,
    pub key_ideas: Vec<String>,
    pub suggested_reels: Vec<SuggestedReel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedReel {
    pub title: String,
    pub hook: String,
}

#[derive(Debug, Clone)]
pub struct ExtractInput {
    pub kind: String,
    pub title: String,
    pub url: Option<String>,
    pub prompt: Option<String>,
    /// Signed URL of an uploaded document or image, when there is one.
    pub media_url: Option<String>,
    pub mime: Option<String>,
    pub language_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScriptInput {
    pub source_title: String,
    pub source_text: String,
    pub key_ideas: Vec<String>,
    pub topic_name: String,
    pub language: String,
    pub reel_count: usize,
    pub brief: Option<String>,
    pub presenter_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReelScript {
    pub title: String,
    /// 45–60 s of spoken script.
    pub script: String,
    /// The one line burned into the frame.
    pub caption_line: String,
    pub source_citation: String,
    /// 2 to 5 short lowercase subject tags; the learning engine tracks interest in them.
    #[serde(default)]
    pub tags: Vec<String>,
    /// 1 (easy) to 5 (hard); the engine's prior for the question's answer rate.
    #[serde(default)]
    pub difficulty: Option<i16>,
}

impl ReelScript {
    /// Tags trimmed, lower-cased, de-duplicated and capped at five; difficulty clamped to 1..5.
    pub fn clean_features(&self) -> (Vec<String>, Option<i16>) {
        let mut tags: Vec<String> = Vec::new();
        for t in &self.tags {
            let t = t.trim().trim_start_matches('#').to_lowercase();
            let t: String = t.chars().take(32).collect();
            if !t.is_empty() && !tags.contains(&t) {
                tags.push(t);
            }
            if tags.len() == 5 {
                break;
            }
        }
        (tags, self.difficulty.map(|d| d.clamp(1, 5)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuestionDraft {
    pub prompt: String,
    pub options: Vec<(String, String)>,
    pub correct_key: String,
    pub explanation: String,
}

impl QuestionDraft {
    pub fn options_json(&self) -> serde_json::Value {
        serde_json::Value::Array(
            self.options
                .iter()
                .map(|(k, t)| serde_json::json!({ "key": k, "text": t }))
                .collect(),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentDraft {
    pub start_ms: i32,
    pub end_ms: i32,
    pub title: String,
    pub excerpt: String,
    pub speaker: Option<String>,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct TranscriptSegment {
    pub start_ms: i32,
    pub end_ms: i32,
    pub text: String,
    pub speaker: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TranscriptResult {
    Ready {
        segments: Vec<TranscriptSegment>,
        language: Option<String>,
    },
    Pending {
        provider_ref: String,
    },
}

#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub reel_id: uuid::Uuid,
    pub style: String,
    pub script: String,
    pub caption_line: String,
    pub language: String,
    pub presenter: serde_json::Value,
    /// Signed URL of a narration track when the voice step ran first.
    pub audio_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CutRequest {
    pub reel_id: uuid::Uuid,
    pub source_url: String,
    pub start_ms: i32,
    pub end_ms: i32,
}

#[derive(Debug, Clone)]
pub enum MediaResult {
    /// The worker stores these bytes under a content-addressed key.
    Ready {
        bytes: Vec<u8>,
        mime: String,
        duration_ms: i32,
        /// A keyframe (JPEG/PNG) stored as the reel's thumbnail when the provider has one.
        poster: Option<Vec<u8>>,
    },
    /// The vendor renders asynchronously and calls `/webhooks/providers/{provider}`.
    Pending { provider_ref: String },
}

#[async_trait]
pub trait ScriptWriter: Send + Sync {
    fn name(&self) -> &'static str;
    async fn extract(&self, input: ExtractInput) -> Result<Extraction, ProviderError>;
    async fn write_reels(&self, input: ScriptInput) -> Result<Vec<ReelScript>, ProviderError>;
    async fn write_question(
        &self,
        script: &str,
        language: &str,
    ) -> Result<QuestionDraft, ProviderError>;
    async fn translate(
        &self,
        script: &str,
        question: Option<&QuestionDraft>,
        from: &str,
        to: &str,
    ) -> Result<(String, Option<QuestionDraft>), ProviderError>;
    async fn find_moments(
        &self,
        transcript: &[TranscriptSegment],
    ) -> Result<Vec<MomentDraft>, ProviderError>;
}

#[async_trait]
pub trait VideoRenderer: Send + Sync {
    fn name(&self) -> &'static str;
    async fn render(&self, req: RenderRequest) -> Result<MediaResult, ProviderError>;
    async fn cut(&self, req: CutRequest) -> Result<MediaResult, ProviderError>;
}

#[async_trait]
pub trait VoiceSynth: Send + Sync {
    fn name(&self) -> &'static str;
    async fn synthesize(
        &self,
        text: &str,
        language: &str,
        voice_id: Option<&str>,
    ) -> Result<MediaResult, ProviderError>;
}

#[async_trait]
pub trait Transcriber: Send + Sync {
    fn name(&self) -> &'static str;
    async fn transcribe(
        &self,
        media_url: &str,
        language_hint: Option<&str>,
    ) -> Result<TranscriptResult, ProviderError>;
}

#[async_trait]
pub trait WhatsAppSender: Send + Sync {
    fn name(&self) -> &'static str;
    /// Returns the provider message id.
    async fn send_text(&self, to_e164: &str, body: &str) -> Result<String, ProviderError>;
    async fn send_video(
        &self,
        to_e164: &str,
        caption: &str,
        video_url: &str,
    ) -> Result<String, ProviderError>;
}

// ------------------------------------------------------------------ judging (TypeSafe Jev)

/// One draft as Jev sees it: the script, its source and its question.
#[derive(Debug, Clone)]
pub struct DraftInput {
    pub title: String,
    pub topic: String,
    pub language: String,
    pub script: String,
    pub caption_line: Option<String>,
    /// The part of the source the script was written from (truncated by the caller).
    pub source_excerpt: String,
    pub question: Option<QuestionDraft>,
    /// Tags to judge: the writer's plus the company's vocabulary.
    pub tag_candidates: Vec<String>,
}

/// A published reel, for job-title relevance.
#[derive(Debug, Clone)]
pub struct RoleReel {
    pub title: String,
    pub topic: String,
    pub tags: Vec<String>,
    pub excerpt: String,
}

#[async_trait]
pub trait Judge: Send + Sync {
    fn name(&self) -> &'static str;
    async fn review_draft(
        &self,
        input: &DraftInput,
    ) -> Result<reelearn_domain::ai::DraftAnswers, ProviderError>;
    /// P(someone working as `role` would use what each reel teaches), in input order.
    async fn role_fit(&self, role: &str, reels: &[RoleReel]) -> Result<Vec<f32>, ProviderError>;
}

// ------------------------------------------------------------------ metering

/// One billable provider call, recorded by the adapter that made it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageEvent {
    /// Integration kind: ai_claude, ai_muse, ai_typesafe, ai_higgsfield, voice, asr.
    pub provider: &'static str,
    /// write | judge | render | voice | transcribe
    pub capability: &'static str,
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub units: u32,
    pub ok: bool,
}

/// Collects usage while a job runs; the worker writes it to `ai_usage` afterwards, in its own
/// transaction, so a failed step still accounts for what the vendor charged.
#[derive(Clone, Default)]
pub struct Meter(Option<Arc<std::sync::Mutex<Vec<UsageEvent>>>>);

impl Meter {
    pub fn recording() -> Meter {
        Meter(Some(Arc::new(std::sync::Mutex::new(Vec::new()))))
    }

    pub fn record(&self, e: UsageEvent) {
        if let Some(m) = &self.0 {
            if let Ok(mut v) = m.lock() {
                v.push(e);
            }
        }
    }

    /// Tokens from an Anthropic (`input_tokens`/`output_tokens`) or OpenAI-style
    /// (`prompt_tokens`/`completion_tokens`) `usage` object.
    pub fn record_tokens(
        &self,
        provider: &'static str,
        capability: &'static str,
        model: &str,
        usage: &serde_json::Value,
    ) {
        let n = |a: &str, b: &str| usage[a].as_u64().or_else(|| usage[b].as_u64()).unwrap_or(0);
        self.record(UsageEvent {
            provider,
            capability,
            model: Some(model.to_string()),
            input_tokens: n("input_tokens", "prompt_tokens"),
            output_tokens: n("output_tokens", "completion_tokens"),
            units: 1,
            ok: true,
        });
    }

    pub fn drain(&self) -> Vec<UsageEvent> {
        match &self.0 {
            Some(m) => m
                .lock()
                .map(|mut v| std::mem::take(&mut *v))
                .unwrap_or_default(),
            None => Vec::new(),
        }
    }
}

/// The set of providers a job runs with, plus who pays for each (`sources`, by integration kind)
/// and the prices to cost them. Built per company by [`crate::Ctx::providers_for`].
#[derive(Clone)]
pub struct Providers {
    pub writer: Arc<dyn ScriptWriter>,
    pub video: Arc<dyn VideoRenderer>,
    pub voice: Arc<dyn VoiceSynth>,
    pub asr: Arc<dyn Transcriber>,
    pub whatsapp: Arc<dyn WhatsAppSender>,
    pub judge: Arc<dyn Judge>,
    pub meter: Meter,
    /// Integration kind behind each slot when it is a real vendor (for unit-billed calls).
    pub slots: Slots,
    pub sources: std::collections::HashMap<&'static str, reelearn_domain::ai::KeySource>,
    pub prices: std::collections::HashMap<&'static str, reelearn_domain::ai::Prices>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Slots {
    pub video: Option<&'static str>,
    pub voice: Option<&'static str>,
    pub asr: Option<&'static str>,
}

impl Providers {
    pub fn fake() -> Providers {
        Providers {
            writer: Arc::new(fake::FakeWriter),
            video: Arc::new(fake::FakeVideo),
            voice: Arc::new(fake::FakeVoice),
            asr: Arc::new(fake::FakeAsr),
            whatsapp: Arc::new(fake::FakeWhatsApp::default()),
            judge: Arc::new(fake::FakeJudge),
            meter: Meter::default(),
            slots: Slots::default(),
            sources: Default::default(),
            prices: Default::default(),
        }
    }

    /// Real adapters for one company from its resolved keys. Muse writes unless the company
    /// chose Claude (or only Claude has a key); a dedicated video, voice or ASR vendor takes over
    /// its slot when it has a key; TypeSafe Jev judges. A provider kept off by the company's plan
    /// or budget answers with that reason instead of silently falling back to placeholder output.
    pub fn from_resolved(
        http: reqwest::Client,
        r: &reelearn_api::services::ai::ResolvedAi,
    ) -> Providers {
        use reelearn_api::services::ai::Credential;
        use reelearn_domain::ai::WriterChoice;
        let meter = Meter::recording();
        let mut p = Providers::fake();
        p.judge = Arc::new(blocked::Blocked::off("TypeSafe Jev is not set up"));
        let used = |c: &Credential, p: &mut Providers| {
            p.sources.insert(c.kind, c.source);
            p.prices.insert(c.kind, c.prices);
        };

        let claude = r.get("ai_claude");
        let muse = r.get("ai_muse");
        let writer_cred = match r.routing.writer {
            WriterChoice::Claude => claude.clone(),
            WriterChoice::Muse => muse.clone(),
            WriterChoice::Auto => match (&muse, &claude) {
                (Ok(_), _) => muse.clone(),
                (_, Ok(_)) => claude.clone(),
                (Err(e), _) if e.blocks() => muse.clone(),
                _ => claude.clone(),
            },
        };
        match writer_cred {
            Ok(c) => {
                p.writer = match c.kind {
                    "ai_claude" => Arc::new(
                        real::ClaudeWriter::new(http.clone(), c.secret("api_key"), c.model.clone())
                            .with_meter(meter.clone()),
                    ),
                    _ => Arc::new(
                        muse::MuseWriter::new(
                            http.clone(),
                            c.secret("api_key"),
                            c.base_url(muse::DEFAULT_BASE_URL),
                            c.model.clone(),
                        )
                        .with_meter(meter.clone()),
                    ),
                };
                used(&c, &mut p);
            }
            Err(e) if e.blocks() => p.writer = Arc::new(blocked::Blocked::new(e.message)),
            Err(_) => {}
        }

        match (r.get("ai_higgsfield"), &muse) {
            (Ok(c), _) => {
                p.video = Arc::new(real::HttpRenderer::new(
                    http.clone(),
                    c.base_url(""),
                    c.secret("api_key"),
                    c.config_str("provider")
                        .unwrap_or_else(|| "higgsfield".into()),
                ));
                p.slots.video = Some(c.kind);
                used(&c, &mut p);
            }
            (_, Ok(c)) => {
                p.video = Arc::new(muse::MuseStillsRenderer::new(
                    http.clone(),
                    c.secret("api_key"),
                    c.base_url(muse::DEFAULT_BASE_URL),
                ));
                p.slots.video = Some(c.kind);
                used(c, &mut p);
            }
            _ => {}
        }
        if let Ok(c) = r.get("voice") {
            p.voice = Arc::new(real::HttpVoice::new(
                http.clone(),
                c.base_url(""),
                c.secret("api_key"),
            ));
            p.slots.voice = Some(c.kind);
            used(&c, &mut p);
        }
        match (r.get("asr"), &muse) {
            (Ok(c), _) => {
                p.asr = Arc::new(real::HttpTranscriber::new(
                    http.clone(),
                    c.base_url(""),
                    c.secret("api_key"),
                ));
                p.slots.asr = Some(c.kind);
                used(&c, &mut p);
            }
            (_, Ok(c)) => {
                p.asr = Arc::new(muse::MuseTranscriber::new(
                    http.clone(),
                    c.secret("api_key"),
                    c.base_url(muse::DEFAULT_BASE_URL),
                ));
                p.slots.asr = Some(c.kind);
                used(c, &mut p);
            }
            _ => {}
        }
        if let Ok(c) = r.get("whatsapp") {
            p.whatsapp = Arc::new(real::MetaWhatsApp::new(
                http.clone(),
                c.config_str("phone_number_id").unwrap_or_default(),
                c.secret("access_token"),
            ));
        }
        match r.get("ai_typesafe") {
            Ok(c) => {
                p.judge = Arc::new(
                    jev::JevJudge::new(
                        http,
                        c.secret("api_key"),
                        c.base_url(jev::DEFAULT_BASE_URL),
                        c.model.clone(),
                    )
                    .with_meter(meter.clone()),
                );
                used(&c, &mut p);
            }
            Err(e) => p.judge = Arc::new(blocked::Blocked::off(e.message)),
        }
        p.meter = meter;
        p
    }

    /// Everything Meta Model API can do, pointed at `base_url` (tests use a local mock server).
    pub fn muse(http: reqwest::Client, api_key: &str, base_url: &str) -> Providers {
        let mut p = Providers::fake();
        p.writer = Arc::new(muse::MuseWriter::new(
            http.clone(),
            api_key.into(),
            base_url.into(),
            muse::DEFAULT_MODEL.into(),
        ));
        p.video = Arc::new(muse::MuseStillsRenderer::new(
            http.clone(),
            api_key.into(),
            base_url.into(),
        ));
        p.asr = Arc::new(muse::MuseTranscriber::new(
            http,
            api_key.into(),
            base_url.into(),
        ));
        p
    }

    pub fn summary(&self) -> String {
        format!(
            "writer={} video={} voice={} asr={} whatsapp={} judge={}",
            self.writer.name(),
            self.video.name(),
            self.voice.name(),
            self.asr.name(),
            self.whatsapp.name(),
            self.judge.name()
        )
    }
}

/// A tiny but structurally valid MP4 container (ftyp + empty mdat) so players and CDNs accept the key.
pub fn placeholder_mp4() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&[0, 0, 0, 0x18]);
    v.extend_from_slice(b"ftypisom");
    v.extend_from_slice(&[0, 0, 0, 1]);
    v.extend_from_slice(b"isomavc1");
    v.extend_from_slice(&[0, 0, 0, 8]);
    v.extend_from_slice(b"mdat");
    v
}

/// WebVTT captions: the script split into ~6-word cues spread over the duration.
pub fn vtt_from_script(script: &str, duration_ms: i32) -> String {
    let words: Vec<&str> = script.split_whitespace().collect();
    if words.is_empty() {
        return "WEBVTT\n\n".to_string();
    }
    let cues: Vec<String> = words.chunks(6).map(|c| c.join(" ")).collect();
    let per = (duration_ms.max(1000) as f64 / cues.len() as f64).max(800.0);
    let stamp = |ms: f64| {
        let total = ms as i64;
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            total / 3_600_000,
            (total / 60_000) % 60,
            (total / 1000) % 60,
            total % 1000
        )
    };
    let mut out = String::from("WEBVTT\n\n");
    for (i, cue) in cues.iter().enumerate() {
        let start = i as f64 * per;
        out.push_str(&format!(
            "{} --> {}\n{}\n\n",
            stamp(start),
            stamp(start + per),
            cue
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vtt_covers_the_whole_script() {
        let vtt = vtt_from_script(
            "one two three four five six seven eight nine ten eleven twelve thirteen",
            45_000,
        );
        assert!(vtt.starts_with("WEBVTT"));
        assert_eq!(vtt.matches("-->").count(), 3);
        assert!(vtt.contains("thirteen"));
    }

    #[test]
    fn placeholder_mp4_has_ftyp_box() {
        let bytes = placeholder_mp4();
        assert_eq!(&bytes[4..8], b"ftyp");
    }
}
