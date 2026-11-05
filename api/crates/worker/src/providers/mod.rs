//! Provider traits the pipeline is written against, plus the deterministic fakes
//! used in development and tests and the HTTP adapters used in production.
//!
//! A step never talks to a vendor directly: it asks a trait for a result, and the
//! result is either `Ready` (bytes the worker stores) or `Pending` (a provider
//! reference that a later `provider_callback` job completes).

pub mod fake;
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

/// The set of providers a worker runs with.
#[derive(Clone)]
pub struct Providers {
    pub writer: Arc<dyn ScriptWriter>,
    pub video: Arc<dyn VideoRenderer>,
    pub voice: Arc<dyn VoiceSynth>,
    pub asr: Arc<dyn Transcriber>,
    pub whatsapp: Arc<dyn WhatsAppSender>,
}

impl Providers {
    pub fn fake() -> Providers {
        Providers {
            writer: Arc::new(fake::FakeWriter),
            video: Arc::new(fake::FakeVideo),
            voice: Arc::new(fake::FakeVoice),
            asr: Arc::new(fake::FakeAsr),
            whatsapp: Arc::new(fake::FakeWhatsApp::default()),
        }
    }

    /// Real adapters from the environment. Meta Model API (`MODEL_API_KEY`) is the default brain:
    /// Muse Spark writes, Muse Voice Transcribe listens, Muse Image + ffmpeg render. Claude, a video
    /// vendor, a voice vendor or an ASR vendor take over their slot when their own keys are set
    /// (`WRITER_PROVIDER=claude|muse` picks the writer explicitly). Anything unconfigured stays fake.
    pub fn from_env(http: reqwest::Client) -> Providers {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let muse_key = env("MODEL_API_KEY");
        let muse_base = env("MUSE_BASE_URL").unwrap_or_else(|| muse::DEFAULT_BASE_URL.into());
        let writer_pref = env("WRITER_PROVIDER").unwrap_or_default();
        let writer: Arc<dyn ScriptWriter> =
            match (writer_pref.as_str(), &muse_key, env("ANTHROPIC_API_KEY")) {
                ("claude", _, Some(key)) => Arc::new(real::ClaudeWriter::new(
                    http.clone(),
                    key,
                    env("CLAUDE_MODEL").unwrap_or_else(|| "claude-sonnet-5-5".into()),
                )),
                (_, Some(key), _) if writer_pref != "claude" => Arc::new(muse::MuseWriter::new(
                    http.clone(),
                    key.clone(),
                    muse_base.clone(),
                    env("MUSE_MODEL").unwrap_or_else(|| muse::DEFAULT_MODEL.into()),
                )),
                (_, None, Some(key)) => Arc::new(real::ClaudeWriter::new(
                    http.clone(),
                    key,
                    env("CLAUDE_MODEL").unwrap_or_else(|| "claude-sonnet-5-5".into()),
                )),
                _ => Arc::new(fake::FakeWriter),
            };
        let video: Arc<dyn VideoRenderer> =
            match (env("VIDEO_API_URL"), env("VIDEO_API_KEY"), &muse_key) {
                (Some(url), Some(key), _) => Arc::new(real::HttpRenderer::new(
                    http.clone(),
                    url,
                    key,
                    env("VIDEO_PROVIDER").unwrap_or_else(|| "higgsfield".into()),
                )),
                (_, _, Some(key)) => Arc::new(muse::MuseStillsRenderer::new(
                    http.clone(),
                    key.clone(),
                    muse_base.clone(),
                )),
                _ => Arc::new(fake::FakeVideo),
            };
        let voice: Arc<dyn VoiceSynth> = match (env("VOICE_API_URL"), env("VOICE_API_KEY")) {
            (Some(url), Some(key)) => Arc::new(real::HttpVoice::new(http.clone(), url, key)),
            _ => Arc::new(fake::FakeVoice),
        };
        let asr: Arc<dyn Transcriber> = match (env("ASR_API_URL"), env("ASR_API_KEY"), &muse_key) {
            (Some(url), Some(key), _) => {
                Arc::new(real::HttpTranscriber::new(http.clone(), url, key))
            }
            (_, _, Some(key)) => Arc::new(muse::MuseTranscriber::new(
                http.clone(),
                key.clone(),
                muse_base,
            )),
            _ => Arc::new(fake::FakeAsr),
        };
        let whatsapp: Arc<dyn WhatsAppSender> = match (
            env("WHATSAPP_PHONE_NUMBER_ID"),
            env("WHATSAPP_ACCESS_TOKEN"),
        ) {
            (Some(phone_id), Some(token)) => {
                Arc::new(real::MetaWhatsApp::new(http, phone_id, token))
            }
            _ => Arc::new(fake::FakeWhatsApp::default()),
        };
        Providers {
            writer,
            video,
            voice,
            asr,
            whatsapp,
        }
    }

    /// Everything Meta Model API can do, pointed at `base_url` (tests use a local mock server).
    pub fn muse(http: reqwest::Client, api_key: &str, base_url: &str) -> Providers {
        Providers {
            writer: Arc::new(muse::MuseWriter::new(
                http.clone(),
                api_key.into(),
                base_url.into(),
                muse::DEFAULT_MODEL.into(),
            )),
            video: Arc::new(muse::MuseStillsRenderer::new(
                http.clone(),
                api_key.into(),
                base_url.into(),
            )),
            voice: Arc::new(fake::FakeVoice),
            asr: Arc::new(muse::MuseTranscriber::new(
                http,
                api_key.into(),
                base_url.into(),
            )),
            whatsapp: Arc::new(fake::FakeWhatsApp::default()),
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "writer={} video={} voice={} asr={} whatsapp={}",
            self.writer.name(),
            self.video.name(),
            self.voice.name(),
            self.asr.name(),
            self.whatsapp.name()
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
