//! TypeSafe Jev (System One model): typed questions about a state, answered in one parallel
//! pass with calibrated probabilities. `POST {base}/v1/systemone` with
//! `{ model, state, questions: { name: { type: noul|choice|score, instructions, criteria } } }`
//! answers `{ model, answers: { name: { type, noul | score | choice, probabilities } }, usage }`.
//!
//! Jev judges; it does not write. All question wording lives in this file so a reviewer can
//! audit it in one place, and the decisions made from the answers live in
//! `reelearn_domain::ai::review`.

use super::*;
use reelearn_domain::ai::{DraftAnswers, QuestionAnswers};
use serde_json::{json, Value};

pub const DEFAULT_BASE_URL: &str = "https://api.typesafe.ai";
pub const PROVIDER: &str = "ai_typesafe";
/// Source text sent with a draft, in characters (Jev's per-question context is 32k tokens).
pub const SOURCE_EXCERPT_CHARS: usize = 12_000;
/// Tag questions per draft.
pub const MAX_TAG_QUESTIONS: usize = 30;
/// Reels judged per role-relevance request.
pub const ROLE_BATCH: usize = 40;

pub struct JevJudge {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
    meter: Meter,
}

impl JevJudge {
    pub fn new(
        http: reqwest::Client,
        api_key: String,
        base_url: String,
        model: String,
    ) -> JevJudge {
        JevJudge {
            http,
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
            model,
            meter: Meter::default(),
        }
    }

    pub fn with_meter(mut self, meter: Meter) -> JevJudge {
        self.meter = meter;
        self
    }

    async fn ask(&self, state: Value, questions: Value) -> Result<Value, ProviderError> {
        let body = json!({ "model": self.model, "state": state, "questions": questions });
        let resp = self
            .http
            .post(format!("{}/v1/systemone", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(e.to_string()))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let short = || text.chars().take(300).collect::<String>();
        // 429 rate limit and 529 overloaded are retried with backoff, as TypeSafe asks.
        if status.is_server_error() || status.as_u16() == 429 {
            return Err(ProviderError::Unavailable(format!("{status}: {}", short())));
        }
        if !status.is_success() {
            return Err(ProviderError::Rejected(format!("{status}: {}", short())));
        }
        let v: Value = serde_json::from_str(&text)
            .map_err(|e| ProviderError::Rejected(format!("invalid JSON from TypeSafe: {e}")))?;
        let model = v["model"].as_str().unwrap_or(&self.model).to_string();
        self.meter
            .record_tokens(PROVIDER, "judge", &model, &v["usage"]);
        Ok(v)
    }
}

/// The probability of yes for a Noul answer.
fn noul(answers: &Value, name: &str) -> Result<f64, ProviderError> {
    answers[name]["noul"]
        .as_f64()
        .ok_or_else(|| ProviderError::Rejected(format!("TypeSafe answer `{name}` has no noul")))
}

/// The expected level of a Score answer.
fn score(answers: &Value, name: &str) -> Result<f64, ProviderError> {
    answers[name]["score"]
        .as_f64()
        .ok_or_else(|| ProviderError::Rejected(format!("TypeSafe answer `{name}` has no score")))
}

fn excerpt(text: &str, chars: usize) -> String {
    text.chars().take(chars).collect()
}

/// The state and questions for one draft. Public so tests and reviewers can read the payload.
pub fn draft_request(input: &DraftInput) -> (Value, Value) {
    let mut reel = json!({
        "title": input.title,
        "topic": input.topic,
        "language": input.language,
        "script": input.script,
    });
    if let Some(c) = &input.caption_line {
        reel["caption_line"] = json!(c);
    }
    let mut state = json!({
        "reel": reel,
        "source_excerpt": excerpt(&input.source_excerpt, SOURCE_EXCERPT_CHARS),
    });
    let mut q = serde_json::Map::new();
    q.insert(
        "grounded".into(),
        json!({
            "type": "noul",
            "instructions": "Every factual claim in `reel.script` is supported by `source_excerpt`.",
            "criteria": {
                "true": "Each fact, number, rule or step in the script appears in the source excerpt or follows directly from it. Greetings and encouragement are not factual claims.",
                "false": "The script states at least one fact, number, rule or step that the source excerpt does not contain or that contradicts it."
            }
        }),
    );
    q.insert(
        "on_topic".into(),
        json!({
            "type": "noul",
            "instructions": "`reel.script` teaches something about the subject named in `reel.topic`."
        }),
    );
    q.insert(
        "unsafe_content".into(),
        json!({
            "type": "noul",
            "instructions": "`reel.script` contains content that is unsuitable for training at work.",
            "criteria": {
                "true": "Harassment, discrimination, sexual or graphic content, insults, or advice that would put someone in danger if followed.",
                "false": "Ordinary workplace training, including training that describes hazards, mistakes or misconduct so people can avoid them."
            }
        }),
    );
    if input.caption_line.is_some() {
        q.insert(
            "caption_fits".into(),
            json!({
                "type": "noul",
                "instructions": "`reel.caption_line` states something that `reel.script` also says."
            }),
        );
    }
    q.insert(
        "clarity".into(),
        json!({
            "type": "score",
            "instructions": "How easy is `reel.script` to follow for an employee who hears it once, in its language?",
            "criteria": [
                "Confusing: the main point cannot be followed",
                "Hard to follow: jargon or jumps the listener cannot fill in",
                "Followable with effort",
                "Clear: one main point, plain words",
                "Very clear: one main point, plain words, a concrete example"
            ]
        }),
    );
    q.insert(
        "hook".into(),
        json!({
            "type": "score",
            "instructions": "How strongly do the first one or two sentences of `reel.script` give an employee a reason to keep watching?",
            "criteria": [
                "No reason: a generic greeting or a title read aloud",
                "Weak: a general statement of the subject",
                "Some: a relevant fact or claim",
                "Strong: a question, a surprising fact or a situation from the employee's day",
                "Very strong: a concrete, surprising situation the employee recognises, in the first sentence"
            ]
        }),
    );
    q.insert(
        "difficulty".into(),
        json!({
            "type": "score",
            "instructions": "How much prior knowledge does an employee need to understand `reel.script`?",
            "criteria": [
                "None: anyone on their first day",
                "A little: basic familiarity with the workplace",
                "Some: experience in this kind of role",
                "A lot: specialist training in this area",
                "Expert: deep specialist knowledge"
            ]
        }),
    );
    if let Some(question) = &input.question {
        state["question"] = json!({
            "prompt": question.prompt,
            "options": question.options.iter().map(|(k, t)| json!({ "key": k, "text": t })).collect::<Vec<_>>(),
            "correct_key": question.correct_key,
        });
        q.insert(
            "key_correct".into(),
            json!({
                "type": "noul",
                "instructions": "According to `reel.script`, the option whose key is `question.correct_key` correctly answers `question.prompt`."
            }),
        );
        q.insert(
            "single_answer".into(),
            json!({
                "type": "noul",
                "instructions": "Exactly one option in `question.options` correctly answers `question.prompt`.",
                "criteria": {
                    "true": "One option is right and every other option is clearly wrong according to the script.",
                    "false": "No option is right, or two or more options could be defended as right."
                }
            }),
        );
        q.insert(
            "distractors".into(),
            json!({
                "type": "score",
                "instructions": "How believable are the wrong options in `question.options` to an employee who did not watch the reel?",
                "criteria": [
                    "Obviously wrong or joke options",
                    "Mostly easy to rule out",
                    "Some are believable",
                    "Most are believable",
                    "All are believable mistakes a real employee might make"
                ]
            }),
        );
    }
    for (i, tag) in input
        .tag_candidates
        .iter()
        .take(MAX_TAG_QUESTIONS)
        .enumerate()
    {
        q.insert(
            format!("tag_{i}"),
            json!({
                "type": "noul",
                "instructions": format!("A main subject of `reel.script` is \"{tag}\"."),
                "criteria": {
                    "true": format!("The script teaches something about \"{tag}\"."),
                    "false": format!("\"{tag}\" is absent from the script or only mentioned in passing.")
                }
            }),
        );
    }
    (state, Value::Object(q))
}

/// Read Jev's answers for [`draft_request`] back into typed answers.
pub fn draft_answers(input: &DraftInput, answers: &Value) -> Result<DraftAnswers, ProviderError> {
    let mut tags = std::collections::BTreeMap::new();
    for (i, tag) in input
        .tag_candidates
        .iter()
        .take(MAX_TAG_QUESTIONS)
        .enumerate()
    {
        if let Ok(p) = noul(answers, &format!("tag_{i}")) {
            tags.insert(tag.clone(), p);
        }
    }
    Ok(DraftAnswers {
        grounded: noul(answers, "grounded")?,
        on_topic: noul(answers, "on_topic")?,
        unsafe_content: noul(answers, "unsafe_content")?,
        caption_fits: if input.caption_line.is_some() {
            noul(answers, "caption_fits")?
        } else {
            1.0
        },
        clarity: score(answers, "clarity")?,
        hook: score(answers, "hook")?,
        // Score levels start at 0; difficulty starts at 1.
        difficulty: score(answers, "difficulty")? + 1.0,
        question: match &input.question {
            Some(_) => Some(QuestionAnswers {
                key_correct: noul(answers, "key_correct")?,
                single_answer: noul(answers, "single_answer")?,
                distractors: score(answers, "distractors")?,
            }),
            None => None,
        },
        tags,
    })
}

/// One request per batch of reels: the role is the state, each reel its own question.
pub fn role_request(role: &str, reels: &[RoleReel]) -> (Value, Value) {
    let state = json!({ "role": role });
    let mut q = serde_json::Map::new();
    for (i, r) in reels.iter().enumerate() {
        q.insert(
            format!("reel_{i}"),
            json!({
                "type": "noul",
                "instructions": {
                    "question": "Someone working as `role` would use what this reel teaches in their own job.",
                    "reel": {
                        "title": r.title,
                        "topic": r.topic,
                        "tags": r.tags,
                        "opening": excerpt(&r.excerpt, 500),
                    }
                },
                "criteria": {
                    "true": "The reel covers a task, rule, risk or skill that comes up in this role's normal work.",
                    "false": "The reel is about work this role does not do, or only general knowledge."
                }
            }),
        );
    }
    (state, Value::Object(q))
}

#[async_trait]
impl Judge for JevJudge {
    fn name(&self) -> &'static str {
        "typesafe-jev"
    }

    async fn review_draft(&self, input: &DraftInput) -> Result<DraftAnswers, ProviderError> {
        let (state, questions) = draft_request(input);
        let v = self.ask(state, questions).await?;
        draft_answers(input, &v["answers"])
    }

    async fn role_fit(&self, role: &str, reels: &[RoleReel]) -> Result<Vec<f32>, ProviderError> {
        let mut out = Vec::with_capacity(reels.len());
        for chunk in reels.chunks(ROLE_BATCH) {
            let (state, questions) = role_request(role, chunk);
            let v = self.ask(state, questions).await?;
            for i in 0..chunk.len() {
                out.push(noul(&v["answers"], &format!("reel_{i}"))? as f32);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> DraftInput {
        DraftInput {
            title: "Exits stay clear".into(),
            topic: "Fire safety".into(),
            language: "en".into(),
            script: "Blocked exits cost lives. Keep every exit clear.".into(),
            caption_line: Some("Keep exits clear".into()),
            source_excerpt: "x".repeat(20_000),
            question: Some(QuestionDraft {
                prompt: "What do you do with exits?".into(),
                options: vec![
                    ("a".into(), "Keep clear".into()),
                    ("b".into(), "Store boxes".into()),
                ],
                correct_key: "a".into(),
                explanation: String::new(),
            }),
            tag_candidates: vec!["fire safety".into(), "exits".into()],
        }
    }

    #[test]
    fn draft_request_asks_one_question_per_judgement() {
        let (state, q) = draft_request(&draft());
        assert_eq!(
            state["source_excerpt"].as_str().unwrap().len(),
            SOURCE_EXCERPT_CHARS
        );
        assert_eq!(state["question"]["correct_key"], "a");
        let names: Vec<&String> = q.as_object().unwrap().keys().collect();
        for n in [
            "grounded",
            "on_topic",
            "unsafe_content",
            "caption_fits",
            "clarity",
            "hook",
            "difficulty",
            "key_correct",
            "single_answer",
            "distractors",
            "tag_0",
            "tag_1",
        ] {
            assert!(names.iter().any(|k| *k == n), "missing {n}");
        }
        assert_eq!(q["clarity"]["criteria"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn answers_map_back_with_difficulty_starting_at_one() {
        let input = draft();
        let answers = json!({
            "grounded": { "type": "noul", "noul": 0.9 },
            "on_topic": { "type": "noul", "noul": 0.95 },
            "unsafe_content": { "type": "noul", "noul": 0.01 },
            "caption_fits": { "type": "noul", "noul": 0.8 },
            "clarity": { "type": "score", "score": 3.2, "confidence": 0.8 },
            "hook": { "type": "score", "score": 2.5, "confidence": 0.7 },
            "difficulty": { "type": "score", "score": 0.6, "confidence": 0.9 },
            "key_correct": { "type": "noul", "noul": 0.97 },
            "single_answer": { "type": "noul", "noul": 0.9 },
            "distractors": { "type": "score", "score": 1.8, "confidence": 0.6 },
            "tag_0": { "type": "noul", "noul": 0.93 },
            "tag_1": { "type": "noul", "noul": 0.4 }
        });
        let a = draft_answers(&input, &answers).unwrap();
        assert!((a.difficulty - 1.6).abs() < 1e-9);
        assert_eq!(a.question.unwrap().distractors, 1.8);
        assert_eq!(a.tags["fire safety"], 0.93);
        // A missing answer is an error, never a silent "no".
        let mut broken = answers.clone();
        broken.as_object_mut().unwrap().remove("grounded");
        assert!(draft_answers(&input, &broken).is_err());
    }

    #[test]
    fn role_request_puts_each_reel_in_its_own_question() {
        let reels = vec![
            RoleReel {
                title: "A".into(),
                topic: "T".into(),
                tags: vec![],
                excerpt: "e".into(),
            },
            RoleReel {
                title: "B".into(),
                topic: "T".into(),
                tags: vec![],
                excerpt: "e".into(),
            },
        ];
        let (state, q) = role_request("shift supervisor", &reels);
        assert_eq!(state["role"], "shift supervisor");
        assert_eq!(q["reel_1"]["instructions"]["reel"]["title"], "B");
    }
}
