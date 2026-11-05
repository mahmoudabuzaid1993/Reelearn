//! Meta Model API (Muse) adapters: https://api.meta.ai/v1, `Authorization: Bearer $MODEL_API_KEY`.
//!
//! - `MuseWriter`: Muse Spark through the OpenAI-compatible Chat Completions endpoint with
//!   `response_format: json_schema`, so every pipeline answer is schema-constrained JSON.
//!   Documents go inline (`file` content part, base64 data URL); images as `image_url`.
//! - `MuseTranscriber`: Muse Voice Transcribe (`POST /v1/asr/transcribe`, multipart, WAV only,
//!   diarized turns). Non-WAV uploads are converted with ffmpeg when it is installed.
//! - `MuseStillsRenderer`: Muse Image makes a 9:16 keyframe per reel (`/v1/images/generations`);
//!   ffmpeg turns it into a slow-zoom MP4 (with the narration track when one exists) and cuts
//!   podcast moments. Without ffmpeg the keyframe is kept as the poster and a placeholder MP4
//!   is stored, so the rest of the pipeline still runs.
//!
//! Meta Model API has no text-to-speech and no video generation endpoint, so voice stays on
//! the configured voice provider.

use super::*;
use base64::Engine;

pub const DEFAULT_BASE_URL: &str = "https://api.meta.ai/v1";
pub const DEFAULT_MODEL: &str = "muse-spark-1.3";
pub const IMAGE_MODEL: &str = "muse-image-1.0";
pub const ASR_MODEL: &str = "muse-voice-transcribe-1.0";

const DEVELOPER_PROMPT: &str = "You write one-minute vertical learning reels for frontline employees in Saudi Arabia. Plain spoken language, one idea per reel, concrete workplace moments, no slogans. Never use the words scroll, feed, binge or addictive. Answer only with JSON that matches the schema.";

fn transient(status: reqwest::StatusCode) -> bool {
    status.is_server_error() || status.as_u16() == 429 || status.as_u16() == 408
}

async fn read_response(resp: reqwest::Response) -> Result<serde_json::Value, ProviderError> {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let short: String = text.chars().take(400).collect();
    if transient(status) {
        return Err(ProviderError::Unavailable(format!("{status}: {short}")));
    }
    if !status.is_success() {
        return Err(ProviderError::Rejected(format!("{status}: {short}")));
    }
    serde_json::from_str(&text)
        .map_err(|e| ProviderError::Rejected(format!("invalid JSON from Meta Model API: {e}")))
}

// ------------------------------------------------------------------ schemas

fn obj(props: serde_json::Value, required: &[&str]) -> serde_json::Value {
    serde_json::json!({ "type": "object", "properties": props, "required": required, "additionalProperties": false })
}

fn arr(items: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "type": "array", "items": items })
}

fn s() -> serde_json::Value {
    serde_json::json!({ "type": "string" })
}

fn question_schema() -> serde_json::Value {
    obj(
        serde_json::json!({
            "prompt": s(),
            "options": arr(obj(serde_json::json!({ "key": s(), "text": s() }), &["key", "text"])),
            "correct_key": s(),
            "explanation": s()
        }),
        &["prompt", "options", "correct_key", "explanation"],
    )
}

pub fn extraction_schema() -> serde_json::Value {
    obj(
        serde_json::json!({
            "text": s(),
            "language": s(),
            "key_ideas": arr(s()),
            "suggested_reels": arr(obj(serde_json::json!({ "title": s(), "hook": s() }), &["title", "hook"]))
        }),
        &["text", "language", "key_ideas", "suggested_reels"],
    )
}

pub fn reels_schema() -> serde_json::Value {
    obj(
        serde_json::json!({
            "reels": arr(obj(
                serde_json::json!({ "title": s(), "script": s(), "caption_line": s(), "source_citation": s() }),
                &["title", "script", "caption_line", "source_citation"]
            ))
        }),
        &["reels"],
    )
}

pub fn translation_schema() -> serde_json::Value {
    let mut q = question_schema();
    q["type"] = serde_json::json!(["object", "null"]);
    obj(
        serde_json::json!({ "script": s(), "question": q }),
        &["script", "question"],
    )
}

pub fn moments_schema() -> serde_json::Value {
    obj(
        serde_json::json!({
            "moments": arr(obj(
                serde_json::json!({
                    "start_ms": { "type": "integer" }, "end_ms": { "type": "integer" }, "title": s(),
                    "excerpt": s(), "speaker": s(), "score": { "type": "number" }
                }),
                &["start_ms", "end_ms", "title", "excerpt", "speaker", "score"]
            ))
        }),
        &["moments"],
    )
}

// ------------------------------------------------------------------ Muse Spark

pub struct MuseWriter {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl MuseWriter {
    pub fn new(
        http: reqwest::Client,
        api_key: String,
        base_url: String,
        model: String,
    ) -> MuseWriter {
        MuseWriter {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
        }
    }

    /// One schema-constrained chat completion. Returns the parsed JSON object.
    async fn complete(
        &self,
        schema_name: &str,
        schema: serde_json::Value,
        content: serde_json::Value,
        max_tokens: u32,
    ) -> Result<serde_json::Value, ProviderError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "developer", "content": DEVELOPER_PROMPT },
                { "role": "user", "content": content }
            ],
            "response_format": { "type": "json_schema", "json_schema": { "name": schema_name, "schema": schema } },
            "reasoning_effort": "low",
            "max_completion_tokens": max_tokens
        });
        let resp = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(e.to_string()))?;
        let v = read_response(resp).await?;
        parse_completion(&v)
    }
}

/// `choices[0].message.content` holds the JSON text (string, or already-parsed content parts).
pub fn parse_completion(v: &serde_json::Value) -> Result<serde_json::Value, ProviderError> {
    let message = &v["choices"][0]["message"];
    if message["refusal"].is_string() {
        return Err(ProviderError::Rejected(format!(
            "model refused: {}",
            message["refusal"]
        )));
    }
    let text = match &message["content"] {
        serde_json::Value::String(t) => t.clone(),
        serde_json::Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join(""),
        _ => return Err(ProviderError::Rejected("completion has no content".into())),
    };
    let trimmed = text.trim();
    serde_json::from_str(trimmed).or_else(|_| {
        let start = trimmed.find('{').unwrap_or(0);
        let end = trimmed.rfind('}').map(|i| i + 1).unwrap_or(trimmed.len());
        serde_json::from_str(&trimmed[start..end])
            .map_err(|e| ProviderError::Rejected(format!("model did not return JSON: {e}")))
    })
}

fn text_part(text: String) -> serde_json::Value {
    serde_json::json!({ "type": "text", "text": text })
}

#[async_trait]
impl ScriptWriter for MuseWriter {
    fn name(&self) -> &'static str {
        "muse-spark"
    }

    async fn extract(&self, input: ExtractInput) -> Result<Extraction, ProviderError> {
        let mut content = Vec::new();
        match (input.media_url.as_deref(), input.mime.as_deref()) {
            (Some(url), Some(mime)) if mime.starts_with("image/") => {
                content
                    .push(serde_json::json!({ "type": "image_url", "image_url": { "url": url } }));
            }
            (Some(url), Some("application/pdf")) => {
                let bytes = self
                    .http
                    .get(url)
                    .send()
                    .await
                    .map_err(|e| ProviderError::Unavailable(format!("fetching the upload: {e}")))?
                    .bytes()
                    .await
                    .map_err(|e| ProviderError::Unavailable(e.to_string()))?;
                if bytes.len() > 50_000_000 {
                    return Err(ProviderError::Rejected(
                        "PDF is larger than the 50 MB inline limit".into(),
                    ));
                }
                let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                content.push(serde_json::json!({ "type": "file", "file": { "filename": "source.pdf", "file_data": format!("data:application/pdf;base64,{data}") } }));
            }
            (Some(_), Some(mime)) => {
                return Err(ProviderError::Rejected(format!(
                    "{mime} is not supported for extraction on Meta Model API; export it as PDF"
                )));
            }
            _ => {}
        }
        let mut prompt = format!("Title: {}\n", input.title);
        if let Some(u) = &input.url {
            prompt.push_str(&format!("Read this page and extract its content: {u}\n"));
        }
        if let Some(p) = &input.prompt {
            prompt.push_str(&format!("Topic brief: {p}\n"));
        }
        if let Some(l) = &input.language_hint {
            prompt.push_str(&format!("Expected language: {l}\n"));
        }
        prompt.push_str("Extract the full teachable text (up to 6000 words), detect the language (ISO code), list 8 key ideas as short sentences, and suggest 6 reels with a title and a one-line hook.");
        content.push(text_part(prompt));
        let v = self
            .complete(
                "extraction",
                extraction_schema(),
                serde_json::Value::Array(content),
                8000,
            )
            .await?;
        let out: Extraction = serde_json::from_value(v)
            .map_err(|e| ProviderError::Rejected(format!("extraction shape: {e}")))?;
        if out.text.trim().is_empty() {
            return Err(ProviderError::Rejected(
                "nothing could be extracted from the source".into(),
            ));
        }
        Ok(out)
    }

    async fn write_reels(&self, input: ScriptInput) -> Result<Vec<ReelScript>, ProviderError> {
        let prompt = format!(
            "Topic: {}\nLanguage: {}\nPresenter name: {}\nSource title: {}\nBrief: {}\nKey ideas:\n{}\n\nSource text:\n{}\n\nWrite exactly {} reels. Each script is 45-60 seconds spoken, first person, with one concrete workplace moment. caption_line is at most 70 characters.",
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
            .complete(
                "reels",
                reels_schema(),
                serde_json::json!([text_part(prompt)]),
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
        let prompt = format!("Language: {language}\nScript:\n{script}\n\nWrite one recall question with three options (keys A, B, C) that tests the behaviour, not trivia, plus a one-sentence warm explanation.");
        let v = self
            .complete(
                "question",
                question_schema(),
                serde_json::json!([text_part(prompt)]),
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
        let prompt = format!(
            "Translate from {from} to {to} for frontline staff: spoken, simple, same meaning. Keep option keys unchanged. If there is no question, return null for it.\nScript:\n{script}\nQuestion: {}",
            q.unwrap_or(serde_json::Value::Null)
        );
        let v = self
            .complete(
                "translation",
                translation_schema(),
                serde_json::json!([text_part(prompt)]),
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
            .map(|t| {
                format!(
                    "[{}-{}] {}: {}",
                    t.start_ms,
                    t.end_ms,
                    t.speaker.clone().unwrap_or_default(),
                    t.text
                )
            })
            .collect();
        let prompt = format!("Transcript with millisecond ranges:\n{}\n\nFind up to 8 self-contained teachable moments of 20-60 seconds that stand alone as a reel. Use the transcript's millisecond ranges; score 0-1 by how teachable the moment is.", lines.join("\n"));
        let v = self
            .complete(
                "moments",
                moments_schema(),
                serde_json::json!([text_part(prompt)]),
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

// ------------------------------------------------------------------ Muse Voice Transcribe

pub struct MuseTranscriber {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl MuseTranscriber {
    pub fn new(http: reqwest::Client, api_key: String, base_url: String) -> MuseTranscriber {
        MuseTranscriber {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }
}

/// `turns[]` of the transcription response → segments.
pub fn segments_from_turns(v: &serde_json::Value) -> Vec<TranscriptSegment> {
    let turns = v["turns"].as_array().cloned().unwrap_or_default();
    if turns.is_empty() {
        let text = v["transcript"].as_str().unwrap_or("").trim().to_string();
        let duration = v["audioDurationMs"].as_i64().unwrap_or(0) as i32;
        return if text.is_empty() {
            Vec::new()
        } else {
            vec![TranscriptSegment {
                start_ms: 0,
                end_ms: duration.max(1000),
                text,
                speaker: None,
            }]
        };
    }
    turns
        .iter()
        .filter_map(|t| {
            Some(TranscriptSegment {
                start_ms: t["startMs"].as_i64()? as i32,
                end_ms: t["endMs"].as_i64()? as i32,
                text: t["transcript"].as_str()?.trim().to_string(),
                speaker: t["speaker"].as_str().map(|s| s.to_string()),
            })
        })
        .filter(|s| !s.text.is_empty())
        .collect()
}

#[async_trait]
impl Transcriber for MuseTranscriber {
    fn name(&self) -> &'static str {
        "muse-voice-transcribe"
    }

    async fn transcribe(
        &self,
        media_url: &str,
        language_hint: Option<&str>,
    ) -> Result<TranscriptResult, ProviderError> {
        let resp = self
            .http
            .get(media_url)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("fetching the upload: {e}")))?;
        let mime = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ProviderError::Unavailable(e.to_string()))?
            .to_vec();
        let wav = if bytes.starts_with(b"RIFF") {
            bytes
        } else {
            ffmpeg::to_wav(&bytes, &mime).await?
        };
        if wav.len() > 32_000_000 {
            return Err(ProviderError::Rejected(
                "audio is larger than the 32 MB transcription limit; split it first".into(),
            ));
        }
        let mut settings = serde_json::json!({ "mode": "DIARIZATION", "model": ASR_MODEL, "audioEncoding": "WAV" });
        if let Some(l) = language_hint {
            settings["languageBias"] = serde_json::json!([l]);
        }
        let form = reqwest::multipart::Form::new()
            .part(
                "request",
                reqwest::multipart::Part::text(settings.to_string())
                    .mime_str("application/json")
                    .map_err(|e| ProviderError::Rejected(e.to_string()))?,
            )
            .part(
                "audio",
                reqwest::multipart::Part::bytes(wav)
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .map_err(|e| ProviderError::Rejected(e.to_string()))?,
            );
        let resp = self
            .http
            .post(format!("{}/asr/transcribe", self.base_url))
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(e.to_string()))?;
        let v = read_response(resp).await?;
        let segments = segments_from_turns(&v);
        if segments.is_empty() {
            return Err(ProviderError::Rejected(
                "transcription came back empty".into(),
            ));
        }
        Ok(TranscriptResult::Ready {
            segments,
            language: language_hint.map(|s| s.to_string()),
        })
    }
}

// ------------------------------------------------------------------ Muse Image + ffmpeg

pub struct MuseStillsRenderer {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl MuseStillsRenderer {
    pub fn new(http: reqwest::Client, api_key: String, base_url: String) -> MuseStillsRenderer {
        MuseStillsRenderer {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    async fn keyframe(&self, prompt: &str) -> Result<Vec<u8>, ProviderError> {
        let body = serde_json::json!({ "model": IMAGE_MODEL, "prompt": prompt, "n": 1, "size": "1080x1920", "response_format": "b64_json", "output_format": "jpeg" });
        let resp = self
            .http
            .post(format!("{}/images/generations", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(e.to_string()))?;
        let v = read_response(resp).await?;
        let b64 = v["data"][0]["b64_json"]
            .as_str()
            .ok_or_else(|| ProviderError::Rejected("image response has no b64_json".into()))?;
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| ProviderError::Rejected(format!("image is not valid base64: {e}")))
    }
}

/// The visual brief for one reel: the presenter and the moment, framed for a phone.
pub fn keyframe_prompt(req: &RenderRequest) -> String {
    let presenter = req.presenter["name"]
        .as_str()
        .unwrap_or("a store colleague");
    let style = match req.style.as_str() {
        "motion" => "cinematic motion-graphics style, bold shapes, high contrast",
        "broll" => "documentary b-roll photo of the real workplace, natural light",
        _ => "friendly presenter speaking to camera on a retail store floor, realistic photo",
    };
    format!(
        "Vertical 9:16 keyframe for a one-minute workplace learning reel. {style}. Presenter: {presenter}. Moment: {}. Leave the lower third clean for captions; no text in the image; no logos.",
        req.caption_line.chars().take(140).collect::<String>()
    )
}

#[async_trait]
impl VideoRenderer for MuseStillsRenderer {
    fn name(&self) -> &'static str {
        "muse-image+ffmpeg"
    }

    async fn render(&self, req: RenderRequest) -> Result<MediaResult, ProviderError> {
        let image = self.keyframe(&keyframe_prompt(&req)).await?;
        let words = req.script.split_whitespace().count() as i32;
        let duration_ms = (words * 400).clamp(20_000, 60_000);
        let audio = match &req.audio_url {
            Some(url) => self
                .http
                .get(url)
                .send()
                .await
                .ok()
                .filter(|r| r.status().is_success())
                .map(|r| r.bytes()),
            None => None,
        };
        let audio = match audio {
            Some(fut) => fut
                .await
                .ok()
                .map(|b| b.to_vec())
                .filter(|b| b.len() > 1024),
            None => None,
        };
        match ffmpeg::still_to_video(&image, duration_ms, audio.as_deref()).await {
            Ok(mp4) => Ok(MediaResult::Ready {
                bytes: mp4,
                mime: "video/mp4".into(),
                duration_ms,
                poster: Some(image),
            }),
            Err(e) => {
                tracing::warn!(error = %e, "ffmpeg unavailable; storing the keyframe as poster with a placeholder video");
                Ok(MediaResult::Ready {
                    bytes: placeholder_mp4(),
                    mime: "video/mp4".into(),
                    duration_ms,
                    poster: Some(image),
                })
            }
        }
    }

    async fn cut(&self, req: CutRequest) -> Result<MediaResult, ProviderError> {
        let mp4 = ffmpeg::cut(&req.source_url, req.start_ms, req.end_ms).await?;
        Ok(MediaResult::Ready {
            bytes: mp4,
            mime: "video/mp4".into(),
            duration_ms: (req.end_ms - req.start_ms).clamp(1000, 90_000),
            poster: None,
        })
    }
}

/// Thin ffmpeg wrapper: every call runs the binary on temp files and returns bytes.
pub mod ffmpeg {
    use super::ProviderError;
    use std::path::PathBuf;
    use tokio::process::Command;

    fn tmp(ext: &str) -> PathBuf {
        std::env::temp_dir().join(format!("reelearn-{}.{ext}", uuid::Uuid::new_v4().simple()))
    }

    pub async fn available() -> bool {
        Command::new("ffmpeg")
            .arg("-version")
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn run(args: &[String]) -> Result<(), ProviderError> {
        let out = Command::new("ffmpeg")
            .args(args)
            .output()
            .await
            .map_err(|e| ProviderError::Config(format!("ffmpeg not available: {e}")))?;
        if out.status.success() {
            Ok(())
        } else {
            let err = String::from_utf8_lossy(&out.stderr);
            Err(ProviderError::Rejected(format!(
                "ffmpeg failed: {}",
                err.lines().last().unwrap_or("unknown error")
            )))
        }
    }

    async fn read_and_remove(
        paths: &[&PathBuf],
        output: &PathBuf,
    ) -> Result<Vec<u8>, ProviderError> {
        let bytes = tokio::fs::read(output)
            .await
            .map_err(|e| ProviderError::Rejected(format!("ffmpeg produced no file: {e}")))?;
        for p in paths {
            let _ = tokio::fs::remove_file(p).await;
        }
        let _ = tokio::fs::remove_file(output).await;
        Ok(bytes)
    }

    /// A slow-zoom 1080×1920 H.264 video from one keyframe, with an optional narration track.
    pub async fn still_to_video(
        image: &[u8],
        duration_ms: i32,
        audio: Option<&[u8]>,
    ) -> Result<Vec<u8>, ProviderError> {
        let img = tmp("jpg");
        let out = tmp("mp4");
        tokio::fs::write(&img, image)
            .await
            .map_err(|e| ProviderError::Rejected(e.to_string()))?;
        let seconds = f64::from(duration_ms) / 1000.0;
        let frames = (seconds * 25.0).ceil() as i64;
        let vf = format!("scale=1080:1920:force_original_aspect_ratio=increase,crop=1080:1920,zoompan=z='1+0.10*on/{frames}':d=1:s=1080x1920:fps=25,format=yuv420p");
        let mut args: Vec<String> = vec![
            "-y".into(),
            "-loglevel".into(),
            "error".into(),
            "-loop".into(),
            "1".into(),
            "-i".into(),
            img.display().to_string(),
        ];
        let mut audio_path = None;
        if let Some(a) = audio {
            let ap = tmp("audio");
            tokio::fs::write(&ap, a)
                .await
                .map_err(|e| ProviderError::Rejected(e.to_string()))?;
            args.extend(["-i".into(), ap.display().to_string()]);
            audio_path = Some(ap);
        }
        args.extend([
            "-t".into(),
            format!("{seconds:.2}"),
            "-vf".into(),
            vf,
            "-r".into(),
            "25".into(),
            "-c:v".into(),
            "libx264".into(),
            "-preset".into(),
            "veryfast".into(),
            "-pix_fmt".into(),
            "yuv420p".into(),
        ]);
        if audio_path.is_some() {
            args.extend(["-c:a".into(), "aac".into(), "-shortest".into()]);
        }
        args.extend([
            "-movflags".into(),
            "+faststart".into(),
            out.display().to_string(),
        ]);
        let result = run(&args).await;
        if result.is_err() && audio_path.is_some() {
            // The narration track may be a placeholder; render video-only rather than fail the reel.
            let video_only: Vec<String> = args
                .iter()
                .filter(|a| *a != "-shortest")
                .cloned()
                .collect::<Vec<_>>();
            let mut cleaned = Vec::new();
            let mut skip = 0;
            for (i, a) in video_only.iter().enumerate() {
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                if a == "-i"
                    && audio_path
                        .as_ref()
                        .map(|p| video_only.get(i + 1) == Some(&p.display().to_string()))
                        .unwrap_or(false)
                {
                    skip = 1;
                    continue;
                }
                if a == "-c:a" {
                    skip = 1;
                    continue;
                }
                cleaned.push(a.clone());
            }
            run(&cleaned).await?;
        } else {
            result?;
        }
        let mut inputs = vec![&img];
        if let Some(ap) = &audio_path {
            inputs.push(ap);
        }
        read_and_remove(&inputs, &out).await
    }

    /// Cut `[start_ms, end_ms]` out of a podcast/talk recording as a 9:16 clip (audio-only sources get a dark frame).
    pub async fn cut(
        source_url: &str,
        start_ms: i32,
        end_ms: i32,
    ) -> Result<Vec<u8>, ProviderError> {
        let out = tmp("mp4");
        let start = f64::from(start_ms) / 1000.0;
        let dur = f64::from((end_ms - start_ms).max(1000)) / 1000.0;
        let args: Vec<String> = vec![
            "-y".into(),
            "-loglevel".into(),
            "error".into(),
            "-ss".into(),
            format!("{start:.3}"),
            "-t".into(),
            format!("{dur:.3}"),
            "-i".into(),
            source_url.to_string(),
            "-vf".into(),
            "scale=1080:1920:force_original_aspect_ratio=increase,crop=1080:1920,format=yuv420p"
                .into(),
            "-c:v".into(),
            "libx264".into(),
            "-preset".into(),
            "veryfast".into(),
            "-c:a".into(),
            "aac".into(),
            "-movflags".into(),
            "+faststart".into(),
            out.display().to_string(),
        ];
        if run(&args).await.is_err() {
            // Audio-only source: paint a dark 9:16 frame under the audio.
            let args: Vec<String> = vec![
                "-y".into(),
                "-loglevel".into(),
                "error".into(),
                "-f".into(),
                "lavfi".into(),
                "-i".into(),
                "color=c=0x081822:s=1080x1920:r=25".into(),
                "-ss".into(),
                format!("{start:.3}"),
                "-t".into(),
                format!("{dur:.3}"),
                "-i".into(),
                source_url.to_string(),
                "-t".into(),
                format!("{dur:.3}"),
                "-c:v".into(),
                "libx264".into(),
                "-preset".into(),
                "veryfast".into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                "-c:a".into(),
                "aac".into(),
                "-shortest".into(),
                "-movflags".into(),
                "+faststart".into(),
                out.display().to_string(),
            ];
            run(&args).await?;
        }
        read_and_remove(&[], &out).await
    }

    /// Any audio/video container → mono 16-bit PCM WAV at 24 kHz, what Muse Voice Transcribe accepts.
    pub async fn to_wav(bytes: &[u8], mime: &str) -> Result<Vec<u8>, ProviderError> {
        if !available().await {
            return Err(ProviderError::Rejected(format!("Meta Voice Transcribe needs WAV (mono, 16-bit, 16/24 kHz) and ffmpeg is not installed to convert {mime}; upload a WAV or install ffmpeg")));
        }
        let input = tmp("media");
        let out = tmp("wav");
        tokio::fs::write(&input, bytes)
            .await
            .map_err(|e| ProviderError::Rejected(e.to_string()))?;
        let args: Vec<String> = vec![
            "-y".into(),
            "-loglevel".into(),
            "error".into(),
            "-i".into(),
            input.display().to_string(),
            "-vn".into(),
            "-ac".into(),
            "1".into(),
            "-ar".into(),
            "24000".into(),
            "-c:a".into(),
            "pcm_s16le".into(),
            "-map_metadata".into(),
            "-1".into(),
            out.display().to_string(),
        ];
        run(&args).await?;
        read_and_remove(&[&input], &out).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_content_is_parsed_as_json() {
        let v = serde_json::json!({ "choices": [{ "message": { "role": "assistant", "content": "{\"prompt\":\"Q?\",\"options\":[{\"key\":\"A\",\"text\":\"x\"},{\"key\":\"B\",\"text\":\"y\"}],\"correct_key\":\"B\",\"explanation\":\"because\"}" } }] });
        let parsed = parse_completion(&v).expect("json");
        assert_eq!(
            question_from_json(&parsed).expect("question").correct_key,
            "B"
        );
        let refusal = serde_json::json!({ "choices": [{ "message": { "refusal": "no" } }] });
        assert!(parse_completion(&refusal).is_err());
    }

    #[test]
    fn schemas_are_strict_objects() {
        for schema in [
            extraction_schema(),
            reels_schema(),
            question_schema(),
            translation_schema(),
            moments_schema(),
        ] {
            assert_eq!(schema["type"], "object");
            assert_eq!(schema["additionalProperties"], false);
            let props = schema["properties"].as_object().expect("props");
            let required: Vec<&str> = schema["required"]
                .as_array()
                .expect("req")
                .iter()
                .filter_map(|v| v.as_str())
                .collect();
            for key in props.keys() {
                assert!(required.contains(&key.as_str()), "{key} must be required");
            }
        }
    }

    #[test]
    fn diarized_turns_become_segments() {
        let v = serde_json::json!({ "transcript": "How is the weather? It is raining.", "audioDurationMs": 8240, "turns": [
            { "turnId": 1, "startMs": 1520, "endMs": 4640, "transcript": "How is the weather?", "speaker": "A" },
            { "turnId": 2, "startMs": 5900, "endMs": 8240, "transcript": "It is raining.", "speaker": "B" }
        ] });
        let segs = segments_from_turns(&v);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[1].speaker.as_deref(), Some("B"));
        let flat =
            serde_json::json!({ "transcript": "Only text.", "audioDurationMs": 3000, "turns": [] });
        assert_eq!(segments_from_turns(&flat).len(), 1);
    }

    #[test]
    fn keyframe_prompt_is_vertical_and_text_free() {
        let req = RenderRequest {
            reel_id: uuid::Uuid::nil(),
            style: "ai_avatar".into(),
            script: String::new(),
            caption_line: "Cone first, mop second".into(),
            language: "en".into(),
            presenter: serde_json::json!({ "name": "Maha" }),
            audio_url: None,
        };
        let p = keyframe_prompt(&req);
        assert!(p.contains("9:16") && p.contains("Maha") && p.contains("no text in the image"));
    }
}
