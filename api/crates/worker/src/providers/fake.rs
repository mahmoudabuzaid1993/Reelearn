//! Deterministic providers for development and tests. They produce plausible
//! scripts, questions and moments from the source text and tiny placeholder media,
//! so the whole studio flow runs without any vendor account.

use super::*;
use std::sync::Mutex;

pub struct FakeWriter;

fn sentences(text: &str) -> Vec<String> {
    text.split(['.', '!', '?', '\n', '۔'])
        .map(|s| s.trim())
        .filter(|s| s.split_whitespace().count() >= 4)
        .map(|s| s.to_string())
        .collect()
}

/// Short teachable ideas: the clauses of a "subject: a, b, and c" brief, else its sentences.
fn key_ideas(text: &str) -> Vec<String> {
    let first = text.lines().next().unwrap_or(text);
    if let Some((_, rest)) = first.split_once(':') {
        let clauses: Vec<String> = rest
            .split([',', ';'])
            .map(|c| {
                c.trim()
                    .trim_start_matches("and ")
                    .trim()
                    .trim_end_matches('.')
                    .to_string()
            })
            .filter(|c| c.split_whitespace().count() >= 2)
            .collect();
        if clauses.len() >= 2 {
            return clauses.into_iter().take(6).collect();
        }
    }
    sentences(text).into_iter().take(6).collect()
}

fn title_case(s: &str) -> String {
    let mut out = String::new();
    for (i, w) in s.split_whitespace().take(9).enumerate() {
        if i > 0 {
            out.push(' ');
        }
        if i == 0 {
            let mut c = w.chars();
            if let Some(f) = c.next() {
                out.extend(f.to_uppercase());
                out.push_str(c.as_str());
            }
        } else {
            out.push_str(w);
        }
    }
    out
}

#[async_trait]
impl ScriptWriter for FakeWriter {
    fn name(&self) -> &'static str {
        "fake-writer"
    }

    async fn extract(&self, input: ExtractInput) -> Result<Extraction, ProviderError> {
        let mut text = String::new();
        if let Some(p) = &input.prompt {
            text.push_str(p);
            text.push('\n');
        }
        if let Some(u) = &input.url {
            text.push_str(&format!("Source page: {u}. "));
        }
        if text.trim().is_empty() {
            text = format!(
                "{t}. Greet every customer within ten seconds and use their name once at hello and once at goodbye. \
                 Keep the floor clear: a spill gets a cone before a mop. Count cash twice and report any single cash transaction of SAR 60,000 or more. \
                 Refunds without a receipt go to a supervisor; a refund is not a loss. End every shift by checking the back door is locked.",
                t = input.title
            );
        }
        let ideas = key_ideas(&text);
        let suggested = ideas
            .iter()
            .take(4)
            .map(|i| SuggestedReel {
                title: title_case(i),
                hook: format!("Why it matters: {}", i.chars().take(80).collect::<String>()),
            })
            .collect();
        Ok(Extraction {
            text,
            language: input.language_hint.or_else(|| Some("en".into())),
            key_ideas: ideas,
            suggested_reels: suggested,
        })
    }

    async fn write_reels(&self, input: ScriptInput) -> Result<Vec<ReelScript>, ProviderError> {
        let mut ideas = input.key_ideas.clone();
        if ideas.is_empty() {
            ideas = key_ideas(&input.source_text);
        }
        if ideas.is_empty() {
            ideas.push(format!(
                "{}: the one thing to remember today",
                input.topic_name
            ));
        }
        let subject = input.source_title.trim().to_string();
        let short_subject = subject.split_whitespace().count() <= 6;
        let n = input.reel_count.max(1);
        Ok((0..n)
            .map(|i| {
                let idea = ideas[i % ideas.len()].trim().trim_end_matches('.').to_string();
                let title = if short_subject && !idea.to_lowercase().starts_with(&subject.to_lowercase()) {
                    format!("{subject}: {}", idea.split_whitespace().take(8).collect::<Vec<_>>().join(" "))
                } else {
                    title_case(&idea)
                };
                ReelScript {
                    title,
                    script: format!(
                        "Hi, I'm {p}. Today's minute on {t}: {idea}. Picture the moment on the floor. First, notice it. Then do the small thing right away, and say it out loud so your colleague hears it too. That's it. One thing, done well, every day.",
                        p = input.presenter_name,
                        t = input.topic_name.to_lowercase()
                    ),
                    caption_line: title_case(&idea).chars().take(70).collect(),
                    source_citation: input.source_title.clone(),
                    tags: fake_tags(&input.topic_name, &idea),
                    difficulty: Some(i16::try_from(2 + i % 3).unwrap_or(3)),
                }
            })
            .collect())
    }

    async fn write_question(
        &self,
        script: &str,
        language: &str,
    ) -> Result<QuestionDraft, ProviderError> {
        let line = sentences(script)
            .into_iter()
            .nth(1)
            .unwrap_or_else(|| "Do the small thing right away".into());
        let (prompt, right, wrong1, wrong2, why) = match language {
            "ar" => (
                "ما هو أول شيء تفعله في هذا الموقف؟",
                "ألاحظ اللحظة وأتصرّف فوراً",
                "أنتظر حتى يطلب المدير",
                "أتجاهل الأمر إذا كان صغيراً",
                "الشيء الصغير الذي يُنجز فوراً هو ما يبقى في الذاكرة.",
            ),
            "ur" => (
                "اس صورتحال میں سب سے پہلے کیا کریں گے؟",
                "لمحے کو پہچانیں اور فوراً عمل کریں",
                "مینیجر کے کہنے کا انتظار کریں",
                "چھوٹی بات ہو تو نظرانداز کریں",
                "جو چھوٹا کام فوراً ہو جائے وہی یاد رہتا ہے۔",
            ),
            _ => (
                "What is the first thing you do in this situation?",
                "Notice the moment and act right away",
                "Wait until a manager asks",
                "Ignore it if it looks small",
                "The small thing done right away is what sticks.",
            ),
        };
        let _ = line;
        Ok(QuestionDraft {
            prompt: prompt.to_string(),
            options: vec![
                ("A".into(), wrong1.to_string()),
                ("B".into(), right.to_string()),
                ("C".into(), wrong2.to_string()),
            ],
            correct_key: "B".into(),
            explanation: why.to_string(),
        })
    }

    async fn translate(
        &self,
        script: &str,
        question: Option<&QuestionDraft>,
        _from: &str,
        to: &str,
    ) -> Result<(String, Option<QuestionDraft>), ProviderError> {
        let translated = format!("[{to}] {script}");
        let q = match question {
            Some(_) => Some(self.write_question(script, to).await?),
            None => None,
        };
        Ok((translated, q))
    }

    async fn find_moments(
        &self,
        transcript: &[TranscriptSegment],
    ) -> Result<Vec<MomentDraft>, ProviderError> {
        Ok(transcript
            .chunks(3)
            .enumerate()
            .take(6)
            .map(|(i, chunk)| {
                let start = chunk.first().map(|s| s.start_ms).unwrap_or(0);
                let end = chunk
                    .last()
                    .map(|s| s.end_ms)
                    .unwrap_or(start + 30_000)
                    .min(start + 60_000);
                let excerpt = chunk
                    .iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                MomentDraft {
                    start_ms: start,
                    end_ms: end,
                    title: title_case(&excerpt),
                    excerpt,
                    speaker: chunk.first().and_then(|s| s.speaker.clone()),
                    score: 0.9 - i as f32 * 0.1,
                }
            })
            .collect())
    }
}

pub struct FakeVideo;

#[async_trait]
impl VideoRenderer for FakeVideo {
    fn name(&self) -> &'static str {
        "fake-video"
    }
    async fn render(&self, req: RenderRequest) -> Result<MediaResult, ProviderError> {
        let words = req.script.split_whitespace().count() as i32;
        Ok(MediaResult::Ready {
            bytes: placeholder_mp4(),
            mime: "video/mp4".into(),
            duration_ms: (words * 400).clamp(20_000, 60_000),
            poster: None,
        })
    }
    async fn cut(&self, req: CutRequest) -> Result<MediaResult, ProviderError> {
        Ok(MediaResult::Ready {
            bytes: placeholder_mp4(),
            mime: "video/mp4".into(),
            duration_ms: (req.end_ms - req.start_ms).clamp(1000, 90_000),
            poster: None,
        })
    }
}

pub struct FakeVoice;

#[async_trait]
impl VoiceSynth for FakeVoice {
    fn name(&self) -> &'static str {
        "fake-voice"
    }
    async fn synthesize(
        &self,
        text: &str,
        _language: &str,
        _voice_id: Option<&str>,
    ) -> Result<MediaResult, ProviderError> {
        let words = text.split_whitespace().count() as i32;
        Ok(MediaResult::Ready {
            bytes: b"ID3\x04\x00\x00\x00\x00\x00\x00".to_vec(),
            mime: "audio/mpeg".into(),
            duration_ms: (words * 400).clamp(5_000, 60_000),
            poster: None,
        })
    }
}

pub struct FakeAsr;

#[async_trait]
impl Transcriber for FakeAsr {
    fn name(&self) -> &'static str {
        "fake-asr"
    }
    async fn transcribe(
        &self,
        _media_url: &str,
        language_hint: Option<&str>,
    ) -> Result<TranscriptResult, ProviderError> {
        let lines = [
            ("Host", "Welcome back. Today we talk about what a great first minute with a customer looks like."),
            ("Guest", "Say the name twice. Once at hello, once at goodbye. More than that sounds like a script."),
            ("Guest", "And when you refund, remember a refund is not a loss, it is the cheapest marketing you will ever buy."),
            ("Host", "Let's move to safety. A spill on the floor, what is the first move?"),
            ("Guest", "Cone first, mop second. The cone protects people while you are away getting the mop."),
            ("Host", "And on the cash side, which transactions have to be reported?"),
            ("Guest", "Any single cash transaction of sixty thousand riyals or more. Suspicion is a separate report."),
            ("Host", "Great. Last one: what do you check before you lock up?"),
            ("Guest", "The back door, the safe, and that nobody is still in the store room."),
        ];
        Ok(TranscriptResult::Ready {
            segments: lines
                .iter()
                .enumerate()
                .map(|(i, (sp, t))| TranscriptSegment {
                    start_ms: i as i32 * 12_000,
                    end_ms: i as i32 * 12_000 + 11_000,
                    text: (*t).into(),
                    speaker: Some((*sp).into()),
                })
                .collect(),
            language: language_hint.map(|s| s.to_string()).or(Some("en".into())),
        })
    }
}

/// Records what would have been sent, so tests can assert on it.
#[derive(Default)]
pub struct FakeWhatsApp {
    pub sent: Mutex<Vec<(String, String)>>,
}

#[async_trait]
impl WhatsAppSender for FakeWhatsApp {
    fn name(&self) -> &'static str {
        "fake-whatsapp"
    }
    async fn send_text(&self, to_e164: &str, body: &str) -> Result<String, ProviderError> {
        tracing::info!(to = %to_e164, body = %body, "fake WhatsApp text");
        if let Ok(mut s) = self.sent.lock() {
            s.push((to_e164.to_string(), body.to_string()));
        }
        Ok(format!("wamid.fake.{}", uuid::Uuid::new_v4().simple()))
    }
    async fn send_video(
        &self,
        to_e164: &str,
        caption: &str,
        _video_url: &str,
    ) -> Result<String, ProviderError> {
        self.send_text(to_e164, caption).await
    }
}

/// Deterministic tags: the topic, then the longest distinct words of the idea.
fn fake_tags(topic: &str, idea: &str) -> Vec<String> {
    const STOP: [&str; 12] = [
        "about", "after", "before", "their", "there", "these", "those", "which", "while", "would",
        "every", "where",
    ];
    let mut tags = vec![topic.trim().to_lowercase()];
    let mut words: Vec<String> = idea
        .split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_lowercase())
        .filter(|w| w.chars().count() >= 5 && !STOP.contains(&w.as_str()))
        .collect();
    words.sort_by(|a, b| b.chars().count().cmp(&a.chars().count()).then(a.cmp(b)));
    for w in words {
        if !tags.contains(&w) {
            tags.push(w);
        }
        if tags.len() == 4 {
            break;
        }
    }
    tags
}
