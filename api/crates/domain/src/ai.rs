//! AI providers as a commercial and editorial concern, with no I/O:
//!
//! - which provider does each job for a company ([`AiRouting`], set by the company admin),
//! - whose key pays for it ([`resolve_source`], from the company's [`AiPlan`] set by the platform),
//! - what a call costs ([`Prices`]),
//! - and how the calibrated answers of TypeSafe Jev turn into a draft's quality score and flags
//!   ([`review`]). Jev makes the judgements; this module makes the decisions, so a reviewer can
//!   read every rule.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The AI providers a company can route work to.
pub const AI_KINDS: [&str; 3] = ["ai_claude", "ai_muse", "ai_typesafe"];

pub const DEFAULT_CLAUDE_MODEL: &str = "claude-sonnet-5-5";
pub const DEFAULT_MUSE_MODEL: &str = "muse-spark-1.3";
pub const DEFAULT_JEV_MODEL: &str = "jev-latest";

// ---------------------------------------------------------------- routing (company admin)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WriterChoice {
    /// Meta Muse when it has a key, else Claude.
    #[default]
    Auto,
    Claude,
    Muse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AiRouting {
    /// Who writes scripts, questions and translations.
    pub writer: WriterChoice,
    /// Run the TypeSafe Jev quality check on every new draft.
    pub quality_check: bool,
    /// Drafts scoring below this are flagged "needs a careful look" for reviewers (0..100).
    pub review_threshold: u8,
    /// Let Jev judge how relevant each reel is to each job title (a starting point for new people).
    pub role_matching: bool,
    /// Let Jev check the subject tags the writer proposed against the company's tag vocabulary.
    pub tag_check: bool,
}

impl Default for AiRouting {
    fn default() -> Self {
        AiRouting {
            writer: WriterChoice::Auto,
            quality_check: true,
            review_threshold: 70,
            role_matching: true,
            tag_check: true,
        }
    }
}

impl AiRouting {
    pub fn sanitized(mut self) -> AiRouting {
        self.review_threshold = self.review_threshold.min(100);
        self
    }
}

// ---------------------------------------------------------------- plan (platform operator)

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AiPlan {
    /// The company may use the platform's keys when it has none of its own.
    pub platform_keys: bool,
    /// Ceiling on platform-key spend per calendar month, in USD; `None` is unlimited.
    pub monthly_budget_usd: Option<f64>,
    /// Added to platform-key cost on the company's bill, in percent.
    pub markup_pct: f64,
    /// Providers the company may use through the platform's keys. Empty means all.
    pub allowed: Vec<String>,
}

impl Default for AiPlan {
    fn default() -> Self {
        AiPlan {
            platform_keys: true,
            monthly_budget_usd: None,
            markup_pct: 0.0,
            allowed: Vec::new(),
        }
    }
}

impl AiPlan {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(b) = self.monthly_budget_usd {
            if !b.is_finite() || b < 0.0 {
                return Err("monthly_budget_usd must be zero or more".into());
            }
        }
        if !self.markup_pct.is_finite() || !(0.0..=500.0).contains(&self.markup_pct) {
            return Err("markup_pct must be between 0 and 500".into());
        }
        Ok(())
    }

    pub fn allows(&self, kind: &str) -> bool {
        self.platform_keys && (self.allowed.is_empty() || self.allowed.iter().any(|k| k == kind))
    }

    /// What the company is billed for platform-key usage that cost `cost` to the platform.
    pub fn billable(&self, cost: f64) -> f64 {
        cost * (1.0 + self.markup_pct / 100.0)
    }
}

// ---------------------------------------------------------------- key resolution

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeySource {
    Company,
    Platform,
}

impl KeySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            KeySource::Company => "company",
            KeySource::Platform => "platform",
        }
    }
}

/// Why a provider has no usable key for a company.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unavailable {
    /// Neither the company nor the platform has a key.
    NotConfigured,
    /// The platform has a key but the company's plan does not include it.
    NotInPlan,
    /// The platform key is in the plan but this month's budget is spent.
    BudgetReached,
}

impl Unavailable {
    pub fn message(&self) -> &'static str {
        match self {
            Unavailable::NotConfigured => "no API key is set for this provider",
            Unavailable::NotInPlan => {
                "your plan doesn't include the platform's key for this provider; add your own key"
            }
            Unavailable::BudgetReached => {
                "this month's AI budget on the platform's key is used up; add your own key or raise the plan"
            }
        }
    }
}

/// The company's key always wins (it pays its own vendor bill); otherwise the platform's, if the
/// plan allows it and there is budget left this month.
pub fn resolve_source(
    kind: &str,
    company_has_key: bool,
    platform_has_key: bool,
    plan: &AiPlan,
    platform_spend_this_month: f64,
) -> Result<KeySource, Unavailable> {
    if company_has_key {
        return Ok(KeySource::Company);
    }
    if !platform_has_key {
        return Err(Unavailable::NotConfigured);
    }
    if !plan.allows(kind) {
        return Err(Unavailable::NotInPlan);
    }
    if let Some(budget) = plan.monthly_budget_usd {
        if plan.billable(platform_spend_this_month) >= budget {
            return Err(Unavailable::BudgetReached);
        }
    }
    Ok(KeySource::Platform)
}

// ---------------------------------------------------------------- prices

/// Prices the platform operator enters per provider (USD). Unset prices make cost unknown, never zero.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prices {
    pub input_per_mtok: Option<f64>,
    pub output_per_mtok: Option<f64>,
    pub per_call: Option<f64>,
}

impl Prices {
    /// TypeSafe's published Jev price: $0.042 per million input tokens, output free.
    pub fn jev() -> Prices {
        Prices {
            input_per_mtok: Some(0.042),
            output_per_mtok: Some(0.0),
            per_call: None,
        }
    }

    pub fn from_config(config: &serde_json::Value) -> Prices {
        let num = |k: &str| {
            config.get(k).and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
            })
        };
        Prices {
            input_per_mtok: num("price_input_per_mtok"),
            output_per_mtok: num("price_output_per_mtok"),
            per_call: num("price_per_call"),
        }
    }

    /// Fill unset prices from `defaults`.
    pub fn or(self, defaults: Prices) -> Prices {
        Prices {
            input_per_mtok: self.input_per_mtok.or(defaults.input_per_mtok),
            output_per_mtok: self.output_per_mtok.or(defaults.output_per_mtok),
            per_call: self.per_call.or(defaults.per_call),
        }
    }

    pub fn cost(&self, input_tokens: u64, output_tokens: u64, units: u32) -> Option<f64> {
        let mut total = 0.0;
        let mut known = false;
        if input_tokens > 0 || output_tokens > 0 {
            match (self.input_per_mtok, self.output_per_mtok) {
                (Some(i), Some(o)) => {
                    total += i * input_tokens as f64 / 1e6 + o * output_tokens as f64 / 1e6;
                    known = true;
                }
                (Some(i), None) if output_tokens == 0 => {
                    total += i * input_tokens as f64 / 1e6;
                    known = true;
                }
                _ => return None,
            }
        }
        if units > 0 {
            match self.per_call {
                Some(p) => {
                    total += p * f64::from(units);
                    known = true;
                }
                None if !known => return None,
                None => {}
            }
        }
        known.then_some(total)
    }
}

// ---------------------------------------------------------------- Jev quality check

/// The calibrated answers Jev gives about one draft (probabilities 0..1, scores 0..4).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DraftAnswers {
    /// P(every factual claim in the script is supported by the source).
    pub grounded: f64,
    /// P(the script is about the topic it was filed under).
    pub on_topic: f64,
    /// P(the script contains content unsuitable for a workplace).
    pub unsafe_content: f64,
    /// P(the caption line matches what the script says).
    pub caption_fits: f64,
    /// 0 (confusing) .. 4 (very clear).
    pub clarity: f64,
    /// 0 (no reason to keep watching) .. 4 (strong opening).
    pub hook: f64,
    /// 1 (easy) .. 5 (expert), Jev's expected level.
    pub difficulty: f64,
    /// Present when the reel has a question.
    pub question: Option<QuestionAnswers>,
    /// Tag → P(the reel is about it), for the writer's tags and the company's vocabulary.
    pub tags: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct QuestionAnswers {
    /// P(the marked answer is correct according to the script).
    pub key_correct: f64,
    /// P(exactly one option is correct).
    pub single_answer: f64,
    /// 0 (obviously wrong) .. 4 (all plausible).
    pub distractors: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flag {
    NotGrounded,
    OffTopic,
    UnsafeContent,
    CaptionMismatch,
    Unclear,
    WeakHook,
    AnswerKeyWrong,
    AmbiguousQuestion,
    WeakDistractors,
}

/// Probability above which a yes/no judgement counts as yes.
pub const YES: f64 = 0.5;
/// Tags Jev is at least this sure about are kept or added.
pub const TAG_KEEP: f64 = 0.6;
/// At most this many tags per reel.
pub const MAX_TAGS: usize = 5;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftReview {
    pub score: u8,
    pub flags: Vec<Flag>,
    /// Below the company's threshold, or any blocking flag.
    pub needs_attention: bool,
    pub difficulty: i16,
    pub tags: Vec<String>,
}

/// Turn Jev's answers into a 0..100 score, flags, a difficulty and the tag set. The weights are
/// constants here, not words in a prompt, so changing a policy is a reviewed code change.
pub fn review(a: &DraftAnswers, threshold: u8, writer_tags: &[String]) -> DraftReview {
    let mut flags = Vec::new();
    let mut check = |cond: bool, f: Flag| {
        if cond {
            flags.push(f)
        }
    };
    check(a.grounded < YES, Flag::NotGrounded);
    check(a.on_topic < YES, Flag::OffTopic);
    check(a.unsafe_content >= YES, Flag::UnsafeContent);
    check(a.caption_fits < YES, Flag::CaptionMismatch);
    check(a.clarity < 2.0, Flag::Unclear);
    check(a.hook < 1.5, Flag::WeakHook);
    if let Some(q) = &a.question {
        check(q.key_correct < YES, Flag::AnswerKeyWrong);
        check(q.single_answer < YES, Flag::AmbiguousQuestion);
        check(q.distractors < 1.5, Flag::WeakDistractors);
    }

    // Weighted: accuracy dominates, then clarity, the hook and the question.
    let mut parts = vec![
        (30.0, a.grounded),
        (10.0, a.on_topic),
        (10.0, 1.0 - a.unsafe_content),
        (5.0, a.caption_fits),
        (20.0, a.clarity / 4.0),
        (10.0, a.hook / 4.0),
    ];
    if let Some(q) = &a.question {
        parts.push((10.0, q.key_correct * q.single_answer));
        parts.push((5.0, q.distractors / 4.0));
    }
    let weight: f64 = parts.iter().map(|(w, _)| w).sum();
    let raw: f64 = parts
        .iter()
        .map(|(w, p)| w * p.clamp(0.0, 1.0))
        .sum::<f64>()
        / weight;
    let mut score = (raw * 100.0).round().clamp(0.0, 100.0) as u8;
    // A wrong answer key or unsafe content caps the score: these must never slip through on averages.
    let blocking = flags.iter().any(|f| {
        matches!(
            f,
            Flag::UnsafeContent | Flag::AnswerKeyWrong | Flag::NotGrounded
        )
    });
    if blocking {
        score = score.min(threshold.saturating_sub(1));
    }

    let difficulty = a.difficulty.round().clamp(1.0, 5.0) as i16;
    DraftReview {
        score,
        needs_attention: blocking || score < threshold,
        flags,
        difficulty,
        tags: pick_tags(&a.tags, writer_tags),
    }
}

/// Keep the tags Jev is sure about, writer's first, then vocabulary tags, best first, at most five.
/// With no confident tag at all, the writer's tags stand (Jev only ever narrows or connects).
fn pick_tags(judged: &BTreeMap<String, f64>, writer_tags: &[String]) -> Vec<String> {
    let mut sure: Vec<(&String, f64, bool)> = judged
        .iter()
        .filter(|(_, p)| **p >= TAG_KEEP)
        .map(|(t, p)| (t, *p, writer_tags.contains(t)))
        .collect();
    if sure.is_empty() {
        return writer_tags.iter().take(MAX_TAGS).cloned().collect();
    }
    sure.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.total_cmp(&a.1)));
    sure.into_iter()
        .take(MAX_TAGS)
        .map(|(t, _, _)| t.clone())
        .collect()
}

/// Lower-cased, single-spaced job title used as the key of role relevance.
pub fn role_key(job_title: &str) -> Option<String> {
    let k = job_title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    (!k.is_empty() && k.len() <= 120).then_some(k)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> DraftAnswers {
        DraftAnswers {
            grounded: 0.95,
            on_topic: 0.97,
            unsafe_content: 0.02,
            caption_fits: 0.9,
            clarity: 3.6,
            hook: 3.1,
            difficulty: 2.4,
            question: Some(QuestionAnswers {
                key_correct: 0.96,
                single_answer: 0.93,
                distractors: 3.0,
            }),
            tags: BTreeMap::new(),
        }
    }

    #[test]
    fn company_key_wins_even_without_a_plan() {
        let plan = AiPlan {
            platform_keys: false,
            ..AiPlan::default()
        };
        assert_eq!(
            resolve_source("ai_claude", true, true, &plan, 1e9),
            Ok(KeySource::Company)
        );
    }

    #[test]
    fn platform_key_needs_plan_and_budget() {
        let mut plan = AiPlan::default();
        assert_eq!(
            resolve_source("ai_claude", false, false, &plan, 0.0),
            Err(Unavailable::NotConfigured)
        );
        assert_eq!(
            resolve_source("ai_claude", false, true, &plan, 0.0),
            Ok(KeySource::Platform)
        );
        plan.allowed = vec!["ai_typesafe".into()];
        assert_eq!(
            resolve_source("ai_claude", false, true, &plan, 0.0),
            Err(Unavailable::NotInPlan)
        );
        assert_eq!(
            resolve_source("ai_typesafe", false, true, &plan, 0.0),
            Ok(KeySource::Platform)
        );
        plan.monthly_budget_usd = Some(10.0);
        plan.markup_pct = 25.0;
        // 8 USD of cost is 10 USD billed with the markup: the budget is spent.
        assert_eq!(
            resolve_source("ai_typesafe", false, true, &plan, 8.0),
            Err(Unavailable::BudgetReached)
        );
        assert_eq!(
            resolve_source("ai_typesafe", false, true, &plan, 7.9),
            Ok(KeySource::Platform)
        );
    }

    #[test]
    fn plan_validation() {
        assert!(AiPlan::default().validate().is_ok());
        let bad = AiPlan {
            markup_pct: 900.0,
            ..AiPlan::default()
        };
        assert!(bad.validate().is_err());
        let neg = AiPlan {
            monthly_budget_usd: Some(-1.0),
            ..AiPlan::default()
        };
        assert!(neg.validate().is_err());
    }

    #[test]
    fn costs_are_unknown_until_priced() {
        assert_eq!(Prices::default().cost(1000, 10, 0), None);
        let jev = Prices::jev().cost(1_000_000, 500, 0).unwrap();
        assert!((jev - 0.042).abs() < 1e-12);
        let calls = Prices {
            per_call: Some(0.5),
            ..Prices::default()
        };
        assert_eq!(calls.cost(0, 0, 3), Some(1.5));
        let cfg = serde_json::json!({ "price_input_per_mtok": "3", "price_output_per_mtok": 15 });
        let p = Prices::from_config(&cfg)
            .cost(1_000_000, 100_000, 0)
            .unwrap();
        assert!((p - 4.5).abs() < 1e-9);
    }

    #[test]
    fn a_good_draft_passes() {
        let r = review(&good(), 70, &[]);
        assert!(r.score >= 85, "score {}", r.score);
        assert!(r.flags.is_empty());
        assert!(!r.needs_attention);
        assert_eq!(r.difficulty, 2);
    }

    #[test]
    fn a_wrong_answer_key_always_needs_attention() {
        let mut a = good();
        a.question.as_mut().unwrap().key_correct = 0.1;
        let r = review(&a, 70, &[]);
        assert!(r.flags.contains(&Flag::AnswerKeyWrong));
        assert!(r.needs_attention);
        assert!(r.score < 70);
    }

    #[test]
    fn unsafe_content_is_flagged_and_capped() {
        let mut a = good();
        a.unsafe_content = 0.8;
        let r = review(&a, 50, &[]);
        assert!(r.flags.contains(&Flag::UnsafeContent));
        assert!(r.score < 50);
    }

    #[test]
    fn tags_prefer_the_writer_then_confident_vocabulary() {
        let mut a = good();
        a.tags = [
            ("fire safety".to_string(), 0.9),
            ("evacuation".to_string(), 0.8),
            ("marketing".to_string(), 0.1),
            ("first aid".to_string(), 0.95),
        ]
        .into_iter()
        .collect();
        let r = review(&a, 70, &["evacuation".to_string(), "marketing".to_string()]);
        assert_eq!(r.tags, vec!["evacuation", "first aid", "fire safety"]);
        // Nothing confident: the writer's tags stand.
        a.tags = [("x".to_string(), 0.2)].into_iter().collect();
        let r = review(&a, 70, &["evacuation".to_string()]);
        assert_eq!(r.tags, vec!["evacuation"]);
    }

    #[test]
    fn role_keys_normalise() {
        assert_eq!(
            role_key("  Shift   Supervisor "),
            Some("shift supervisor".into())
        );
        assert_eq!(role_key("   "), None);
    }
}
