//! HTTP adapters for production providers.
//!
//! - `ClaudeWriter`: the Anthropic Messages API for extraction, scripts, questions,
//!   translation and moment finding. Documents and images are passed as content blocks.
//! - `HttpRenderer`, `HttpVoice`, `HttpTranscriber`: thin adapters over an async
//!   job-style vendor API (Higgsfield / Veo / a voice vendor / an ASR vendor) that
//!   accept `POST {base}/jobs` with a JSON body and answer `{ "id": "<ref>" }`, then
//!   call back `POST /v1/webhooks/providers/{provider}` with
//!   `{ "ref": "<ref>", "status": "done" | "failed", "url": "...", "duration_ms": N, "error": "..." }`.
//!   Each vendor's exact field names are mapped in one place here, so swapping a
//!   vendor is a change to this file only.
//! - `MetaWhatsApp`: the WhatsApp Business Cloud API (Graph API).

use super::*;

fn transient(status: reqwest::StatusCode) -> bool {
    status.is_server_error() || status.as_u16() == 429
}

async fn post_json(
    http: &reqwest::Client,
    url: &str,
    headers: &[(&str, String)],
    body: &serde_json::Value,
) -> Result<serde_json::Value, ProviderError> {
    let mut req = http.post(url).json(body);
    for (k, v) in headers {
        req = req.header(*k, v);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| ProviderError::Unavailable(e.to_string()))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if transient(status) {
        return Err(ProviderError::Unavailable(format!(
            "{status}: {}",
            text.chars().take(300).collect::<String>()
        )));
    }
    if !status.is_success() {
        return Err(ProviderError::Rejected(format!(
            "{status}: {}",
            text.chars().take(300).collect::<String>()
        )));
    }
    serde_json::from_str(&text)
        .map_err(|e| ProviderError::Rejected(format!("invalid JSON from provider: {e}")))
}

// ------------------------------------------------------------------ Claude

pub struct ClaudeWriter {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl ClaudeWriter {
    pub fn new(http: reqwest::Client, api_key: String, model: String) -> ClaudeWriter {
        ClaudeWriter {
            http,
            api_key,
            model,
        }
    }

    async fn ask(
        &self,
        system: &str,
        content: serde_json::Value,
        max_tokens: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "system": system,
            "messages": [{ "role": "user", "content": content }]
        });
        let headers = [
            ("x-api-key", self.api_key.clone()),
            ("anthropic-version", "2023-06-01".to_string()),
        ];
        let resp = post_json(
            &self.http,
            "https://api.anthropic.com/v1/messages",
            &headers,
            &body,
        )
        .await?;
        let text = resp["content"]
            .as_array()
            .and_then(|blocks| blocks.iter().find_map(|b| b["text"].as_str()))
            .unwrap_or("")
            .to_string();
        parse_json_block(&text)
            .ok_or_else(|| ProviderError::Rejected("model did not return JSON".into()))
    }
}

/// Accepts a bare JSON object or one wrapped in a ```json fence.
fn parse_json_block(text: &str) -> Option<serde_json::Value> {
    let trimmed = text.trim();
    if let Ok(v) = serde_json::from_str(trimmed) {
        return Some(v);
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    serde_json::from_str(&trimmed[start..=end]).ok()
}

const WRITER_SYSTEM: &str = "You write one-minute vertical learning reels for frontline employees in Saudi Arabia. Plain language, one idea per reel, no slogans. Never use the words scroll, feed, binge or addictive. Answer with JSON only.";

#[async_trait]
impl ScriptWriter for ClaudeWriter {
    fn name(&self) -> &'static str {
        "claude"
    }

    async fn extract(&self, input: ExtractInput) -> Result<Extraction, ProviderError> {
        let mut content = Vec::new();
        match (input.media_url.as_deref(), input.mime.as_deref()) {
            (Some(url), Some("application/pdf")) => content.push(
                serde_json::json!({ "type": "document", "source": { "type": "url", "url": url } }),
            ),
            (Some(url), Some(mime)) if mime.starts_with("image/") => content.push(
                serde_json::json!({ "type": "image", "source": { "type": "url", "url": url } }),
            ),
            (Some(_), Some(mime)) => {
                return Err(ProviderError::Rejected(format!(
                    "{mime} is not supported for extraction; export it as PDF"
                )))
            }
            _ => {}
        }
        let mut prompt = format!("Title: {}\n", input.title);
        if let Some(u) = &input.url {
            prompt.push_str(&format!("Fetch and read this page: {u}\n"));
        }
        if let Some(p) = &input.prompt {
            prompt.push_str(&format!("Topic brief: {p}\n"));
        }
        prompt.push_str("Return {\"text\": full extracted text (max 6000 words), \"language\": ISO code, \"key_ideas\": [8 short sentences], \"suggested_reels\": [{\"title\", \"hook\"} x 6]}");
        content.push(serde_json::json!({ "type": "text", "text": prompt }));
        let v = self
            .ask(WRITER_SYSTEM, serde_json::Value::Array(content), 8000)
            .await?;
        serde_json::from_value(v)
            .map_err(|e| ProviderError::Rejected(format!("extraction shape: {e}")))
    }

    async fn write_reels(&self, input: ScriptInput) -> Result<Vec<ReelScript>, ProviderError> {
        let prompt = format!(
            "Topic: {}\nLanguage: {}\nPresenter name: {}\nSource title: {}\nBrief: {}\nKey ideas:\n{}\n\nSource text:\n{}\n\nWrite {} reels. Return {{\"reels\": [{{\"title\", \"script\" (45-60 seconds spoken, first person, concrete), \"caption_line\" (<= 70 chars), \"source_citation\"}}]}}",
            input.topic_name,
            input.language,
            input.presenter_name,
            input.source_title,
            input.brief.clone().unwrap_or_default(),
            input.key_ideas.join("\n"),
            input.source_text.chars().take(24_000).collect::<String>(),
            input.reel_count
        );
        let v = self
            .ask(
                WRITER_SYSTEM,
                serde_json::json!([{ "type": "text", "text": prompt }]),
                6000,
            )
            .await?;
        let reels: Vec<ReelScript> = serde_json::from_value(v["reels"].clone())
            .map_err(|e| ProviderError::Rejected(format!("script shape: {e}")))?;
        if reels.is_empty() {
            return Err(ProviderError::Rejected("no reels returned".into()));
        }
        Ok(reels)
    }

    async fn write_question(
        &self,
        script: &str,
        language: &str,
    ) -> Result<QuestionDraft, ProviderError> {
        let prompt = format!("Language: {language}\nScript:\n{script}\n\nWrite ONE recall question with 3 options that tests the behaviour, not trivia. Return {{\"prompt\", \"options\": [{{\"key\": \"A\", \"text\"}}, ...], \"correct_key\", \"explanation\" (one sentence, warm, no judgement)}}");
        let v = self
            .ask(
                WRITER_SYSTEM,
                serde_json::json!([{ "type": "text", "text": prompt }]),
                1200,
            )
            .await?;
        question_from_json(&v)
    }

    async fn translate(
        &self,
        script: &str,
        question: Option<&QuestionDraft>,
        from: &str,
        to: &str,
    ) -> Result<(String, Option<QuestionDraft>), ProviderError> {
        let q = question.map(|q| serde_json::json!({ "prompt": q.prompt, "options": q.options_json(), "correct_key": q.correct_key, "explanation": q.explanation }));
        let prompt = format!("Translate from {from} to {to} for frontline staff; keep it spoken and simple; keep option keys.\nScript:\n{script}\nQuestion: {}\nReturn {{\"script\", \"question\": same shape or null}}", q.unwrap_or(serde_json::Value::Null));
        let v = self
            .ask(
                WRITER_SYSTEM,
                serde_json::json!([{ "type": "text", "text": prompt }]),
                3000,
            )
            .await?;
        let script_t = v["script"]
            .as_str()
            .ok_or_else(|| ProviderError::Rejected("translation missing script".into()))?
            .to_string();
        let q_t = if v["question"].is_object() {
            Some(question_from_json(&v["question"])?)
        } else {
            None
        };
        Ok((script_t, q_t))
    }

    async fn find_moments(
        &self,
        transcript: &[TranscriptSegment],
    ) -> Result<Vec<MomentDraft>, ProviderError> {
        let lines: Vec<String> = transcript
            .iter()
            .map(|s| {
                format!(
                    "[{}-{}] {}: {}",
                    s.start_ms,
                    s.end_ms,
                    s.speaker.clone().unwrap_or_default(),
                    s.text
                )
            })
            .collect();
        let prompt = format!("Transcript with millisecond ranges:\n{}\n\nFind up to 8 self-contained teachable moments of 20-60 seconds. Return {{\"moments\": [{{\"start_ms\", \"end_ms\", \"title\", \"excerpt\", \"speaker\", \"score\" 0-1}}]}}", lines.join("\n"));
        let v = self
            .ask(
                WRITER_SYSTEM,
                serde_json::json!([{ "type": "text", "text": prompt }]),
                4000,
            )
            .await?;
        serde_json::from_value(v["moments"].clone())
            .map_err(|e| ProviderError::Rejected(format!("moments shape: {e}")))
    }
}

fn question_from_json(v: &serde_json::Value) -> Result<QuestionDraft, ProviderError> {
    let options = v["options"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|o| {
                    Some((
                        o["key"].as_str()?.to_string(),
                        o["text"].as_str()?.to_string(),
                    ))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let draft = QuestionDraft {
        prompt: v["prompt"].as_str().unwrap_or_default().to_string(),
        options,
        correct_key: v["correct_key"].as_str().unwrap_or_default().to_string(),
        explanation: v["explanation"].as_str().unwrap_or_default().to_string(),
    };
    if draft.prompt.is_empty()
        || draft.options.len() < 2
        || !draft.options.iter().any(|(k, _)| *k == draft.correct_key)
    {
        return Err(ProviderError::Rejected("question is incomplete".into()));
    }
    Ok(draft)
}

// ------------------------------------------------------------------ async media vendors

pub struct HttpRenderer {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    provider: String,
}

impl HttpRenderer {
    pub fn new(
        http: reqwest::Client,
        base_url: String,
        api_key: String,
        provider: String,
    ) -> HttpRenderer {
        HttpRenderer {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            provider,
        }
    }
}

async fn submit_job(
    http: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    body: serde_json::Value,
) -> Result<MediaResult, ProviderError> {
    let v = post_json(
        http,
        &format!("{base_url}/jobs"),
        &[("authorization", format!("Bearer {api_key}"))],
        &body,
    )
    .await?;
    let id = v["id"]
        .as_str()
        .or_else(|| v["job_id"].as_str())
        .ok_or_else(|| ProviderError::Rejected("provider returned no job id".into()))?;
    Ok(MediaResult::Pending {
        provider_ref: id.to_string(),
    })
}

#[async_trait]
impl VideoRenderer for HttpRenderer {
    fn name(&self) -> &'static str {
        "http-video"
    }
    async fn render(&self, req: RenderRequest) -> Result<MediaResult, ProviderError> {
        submit_job(
            &self.http,
            &self.base_url,
            &self.api_key,
            serde_json::json!({
                "provider": self.provider, "kind": "render", "reel_id": req.reel_id, "style": req.style, "script": req.script,
                "caption": req.caption_line, "language": req.language, "presenter": req.presenter, "audio_url": req.audio_url,
                "aspect": "9:16", "max_seconds": 60
            }),
        )
        .await
    }
    async fn cut(&self, req: CutRequest) -> Result<MediaResult, ProviderError> {
        submit_job(&self.http, &self.base_url, &self.api_key, serde_json::json!({ "provider": self.provider, "kind": "cut", "reel_id": req.reel_id, "source_url": req.source_url, "start_ms": req.start_ms, "end_ms": req.end_ms, "aspect": "9:16" })).await
    }
}

pub struct HttpVoice {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl HttpVoice {
    pub fn new(http: reqwest::Client, base_url: String, api_key: String) -> HttpVoice {
        HttpVoice {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }
}

#[async_trait]
impl VoiceSynth for HttpVoice {
    fn name(&self) -> &'static str {
        "http-voice"
    }
    async fn synthesize(
        &self,
        text: &str,
        language: &str,
        voice_id: Option<&str>,
    ) -> Result<MediaResult, ProviderError> {
        submit_job(&self.http, &self.base_url, &self.api_key, serde_json::json!({ "kind": "tts", "text": text, "language": language, "voice_id": voice_id })).await
    }
}

pub struct HttpTranscriber {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl HttpTranscriber {
    pub fn new(http: reqwest::Client, base_url: String, api_key: String) -> HttpTranscriber {
        HttpTranscriber {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }
}

#[async_trait]
impl Transcriber for HttpTranscriber {
    fn name(&self) -> &'static str {
        "http-asr"
    }
    async fn transcribe(
        &self,
        media_url: &str,
        language_hint: Option<&str>,
    ) -> Result<TranscriptResult, ProviderError> {
        match submit_job(&self.http, &self.base_url, &self.api_key, serde_json::json!({ "kind": "transcribe", "url": media_url, "language": language_hint, "diarize": true })).await? {
            MediaResult::Pending { provider_ref } => Ok(TranscriptResult::Pending { provider_ref }),
            MediaResult::Ready { .. } => Err(ProviderError::Rejected("transcriber answered synchronously with media".into())),
        }
    }
}

/// Parses the segments a transcription callback carries: `[{start_ms, end_ms, text, speaker}]`.
pub fn segments_from_callback(v: &serde_json::Value) -> Vec<TranscriptSegment> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| {
                    Some(TranscriptSegment {
                        start_ms: s["start_ms"].as_i64()? as i32,
                        end_ms: s["end_ms"].as_i64()? as i32,
                        text: s["text"].as_str()?.to_string(),
                        speaker: s["speaker"].as_str().map(|x| x.to_string()),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

// ------------------------------------------------------------------ WhatsApp Cloud API

pub struct MetaWhatsApp {
    http: reqwest::Client,
    phone_number_id: String,
    token: String,
}

impl MetaWhatsApp {
    pub fn new(http: reqwest::Client, phone_number_id: String, token: String) -> MetaWhatsApp {
        MetaWhatsApp {
            http,
            phone_number_id,
            token,
        }
    }
    async fn send(&self, body: serde_json::Value) -> Result<String, ProviderError> {
        let url = format!(
            "https://graph.facebook.com/v21.0/{}/messages",
            self.phone_number_id
        );
        let v = post_json(
            &self.http,
            &url,
            &[("authorization", format!("Bearer {}", self.token))],
            &body,
        )
        .await?;
        v["messages"][0]["id"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ProviderError::Rejected("no message id in WhatsApp response".into()))
    }
}

#[async_trait]
impl WhatsAppSender for MetaWhatsApp {
    fn name(&self) -> &'static str {
        "whatsapp-cloud"
    }
    async fn send_text(&self, to_e164: &str, body: &str) -> Result<String, ProviderError> {
        self.send(serde_json::json!({ "messaging_product": "whatsapp", "to": to_e164.trim_start_matches('+'), "type": "text", "text": { "body": body } })).await
    }
    async fn send_video(
        &self,
        to_e164: &str,
        caption: &str,
        video_url: &str,
    ) -> Result<String, ProviderError> {
        self.send(serde_json::json!({ "messaging_product": "whatsapp", "to": to_e164.trim_start_matches('+'), "type": "video", "video": { "link": video_url, "caption": caption } })).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_block_is_found_inside_a_fence() {
        let v = parse_json_block("Sure:\n```json\n{\"a\": 1}\n```").expect("json");
        assert_eq!(v["a"], 1);
    }

    #[test]
    fn question_needs_a_valid_correct_key() {
        let bad = serde_json::json!({ "prompt": "Q?", "options": [{ "key": "A", "text": "x" }, { "key": "B", "text": "y" }], "correct_key": "Z" });
        assert!(question_from_json(&bad).is_err());
        let good = serde_json::json!({ "prompt": "Q?", "options": [{ "key": "A", "text": "x" }, { "key": "B", "text": "y" }], "correct_key": "B", "explanation": "because" });
        assert_eq!(question_from_json(&good).expect("ok").correct_key, "B");
    }
}
