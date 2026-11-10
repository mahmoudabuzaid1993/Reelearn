//! Predictions, the value model and the re-rank.
//!
//! The catalogue of one company is small (hundreds to a few thousand reels), so the
//! retrieval and early-ranking stages of a billion-item system collapse into one SQL query;
//! what remains is the late-stage value model and the re-rank, which run here in memory.
//!
//! Predictions are smoothed rates (a Beta prior per signal, pulled toward the company's
//! baseline for reels of the same length) adjusted by the person's interest and ability.
//! The score is a weighted sum with negative terms subtracted:
//!
//! ```text
//! score = w_completion·lift + w_learning·challenge + w_save·10·P(save) + w_share·20·P(share)
//!       + w_interest·tanh(interest/5) + w_urgency·urgency + w_freshness·e^(−age/10d)
//!       + w_social·cohort − w_skip·2·P(skip<3s) − w_not_relevant·10·P(not for me)
//!       − w_repeat·[seen] + shared_boost
//! ```
//!
//! The scale factors put every term on roughly 0..1 for a typical reel, so the weights read
//! as relative importance. They are our choices; see `docs/engine.md`.

use super::rng::SplitMix64;
use super::settings::EngineSettings;
use super::signals::{band_of, Preference, DURATION_BANDS};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Bands: 0 under 20 s, 1 under 35 s, 2 under 50 s, 3 under 70 s, 4 from 70 s.
pub const BAND_COUNT: usize = DURATION_BANDS.len() + 1;

/// Below this many impressions a reel counts as under-explored.
pub const EXPLORE_MIN_IMPRESSIONS: u32 = 30;
/// Weight of the upper-confidence bonus when an exploration slot is filled.
pub const EXPLORE_BONUS: f32 = 1.0;
/// Added to reels a colleague sent, so they lead the next batch.
pub const SHARED_BOOST: f32 = 3.0;
/// Shared reels allowed at the head of one batch.
pub const MAX_SHARED_PER_BATCH: usize = 2;

const PRIOR_COMPLETION: f32 = 0.7;
const PRIOR_SKIP: f32 = 0.12;
const PRIOR_SAVE: f32 = 0.05;
const PRIOR_SHARE: f32 = 0.02;
const PRIOR_NOT_RELEVANT: f32 = 0.02;
const PRIOR_CORRECT: f32 = 0.7;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemStats {
    pub impressions: u32,
    pub starts: u32,
    pub skips: u32,
    pub completions: u32,
    pub rewatches: u32,
    pub saves: u32,
    pub shares: u32,
    pub not_relevant: u32,
    pub more_like_this: u32,
    pub answers: u32,
    pub answers_correct: u32,
}

/// The company's completion and skip behaviour for reels of one length band.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BandBaseline {
    pub starts: u32,
    pub completions: u32,
    pub skips: u32,
}

impl BandBaseline {
    pub fn completion_rate(&self) -> f32 {
        smooth(self.completions, self.starts, PRIOR_COMPLETION, 50.0)
    }

    pub fn skip_rate(&self) -> f32 {
        smooth(self.skips, self.starts, PRIOR_SKIP, 50.0)
    }
}

/// Beta-smoothed rate: `(k + prior·m) / (n + m)`.
pub fn smooth(k: u32, n: u32, prior: f32, m: f32) -> f32 {
    (k as f32 + prior * m) / (n as f32 + m)
}

fn logit(p: f32) -> f32 {
    let p = p.clamp(0.01, 0.99);
    (p / (1.0 - p)).ln()
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

#[derive(Debug, Clone, PartialEq)]
pub struct RankCandidate {
    pub reel_id: Uuid,
    pub topic_id: Uuid,
    pub tags: Vec<String>,
    pub duration_ms: i32,
    pub published_at: DateTime<Utc>,
    /// The topic or assignment is mandatory for this person.
    pub mandatory: bool,
    /// Reachable through an assignment (as opposed to the open catalogue).
    pub assigned: bool,
    /// Last day of the assignment, when it has one.
    pub deadline: Option<NaiveDate>,
    /// 1 (easy) to 5 (hard), from the studio pipeline.
    pub difficulty: Option<i16>,
    pub has_question: bool,
    pub stats: ItemStats,
    /// 0..1: how much the person's location, job and team engaged with it this week.
    pub cohort_popularity: f32,
    /// The person already finished this reel once.
    pub completed_before: bool,
    /// Name of the colleague who sent it, when it arrived as a share.
    pub shared_by: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserProfile {
    /// Topic interest, already decayed to now.
    pub topic_interest: HashMap<Uuid, f32>,
    pub topic_preference: HashMap<Uuid, Preference>,
    /// Tag interest, already decayed to now.
    pub tag_interest: HashMap<String, f32>,
    pub answers: u32,
    pub answers_correct: u32,
}

impl UserProfile {
    /// Topic interest plus half the mean interest of the reel's known tags.
    pub fn interest_in(&self, c: &RankCandidate) -> f32 {
        let topic = self.topic_interest.get(&c.topic_id).copied().unwrap_or(0.0);
        let tags: Vec<f32> = c
            .tags
            .iter()
            .filter_map(|t| self.tag_interest.get(t).copied())
            .collect();
        let tag = if tags.is_empty() {
            0.0
        } else {
            tags.iter().sum::<f32>() / tags.len() as f32
        };
        topic + 0.5 * tag
    }

    /// The person's answer rate, pulled toward 70 % until there is evidence.
    pub fn ability(&self) -> f32 {
        smooth(self.answers_correct, self.answers, PRIOR_CORRECT, 10.0)
    }

    pub fn preference(&self, topic_id: Uuid) -> Option<Preference> {
        self.topic_preference.get(&topic_id).copied()
    }
}

#[derive(Debug, Clone)]
pub struct RankContext<'a> {
    pub settings: &'a EngineSettings,
    pub now: DateTime<Utc>,
    pub today: NaiveDate,
    pub bands: [BandBaseline; BAND_COUNT],
    /// Mixed into the exploration draw: person, day and session.
    pub seed: [u128; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Predictions {
    pub p_complete: f32,
    /// P(complete) divided by the band's completion rate: 1 is "as finished as its peers".
    pub completion_lift: f32,
    pub p_skip: f32,
    pub p_save: f32,
    pub p_share: f32,
    pub p_not_relevant: f32,
    pub p_correct: Option<f32>,
    pub interest: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub completion: f32,
    pub learning: f32,
    pub save: f32,
    pub share: f32,
    pub interest: f32,
    pub urgency: f32,
    pub freshness: f32,
    pub social: f32,
    pub skip: f32,
    pub not_relevant: f32,
    pub repeat: f32,
    pub shared: f32,
    pub exploration: f32,
}

impl ScoreBreakdown {
    pub fn total(&self) -> f32 {
        self.completion
            + self.learning
            + self.save
            + self.share
            + self.interest
            + self.urgency
            + self.freshness
            + self.social
            + self.shared
            + self.exploration
            - self.skip
            - self.not_relevant
            - self.repeat
    }
}

/// Why the reel is in the stack; the phone turns it into "From Food safety", "Sent by Sara"…
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RankReason {
    Shared,
    Mandatory,
    Followed,
    Interest,
    Popular,
    New,
    Explore,
    Assigned,
    Catalog,
}

impl RankReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            RankReason::Shared => "shared",
            RankReason::Mandatory => "mandatory",
            RankReason::Followed => "followed",
            RankReason::Interest => "interest",
            RankReason::Popular => "popular",
            RankReason::New => "new",
            RankReason::Explore => "explore",
            RankReason::Assigned => "assigned",
            RankReason::Catalog => "catalog",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub reel_id: Uuid,
    pub topic_id: Uuid,
    pub score: f32,
    pub reason: RankReason,
    pub breakdown: ScoreBreakdown,
    pub predictions: Predictions,
}

/// Per-signal predictions for one person and one reel.
pub fn predict(c: &RankCandidate, user: &UserProfile, ctx: &RankContext<'_>) -> Predictions {
    let band = ctx.bands[band_of(c.duration_ms).min(BAND_COUNT - 1)];
    let band_completion = band.completion_rate().max(0.05);
    let band_skip = band.skip_rate();
    let s = &c.stats;
    let interest = user.interest_in(c);
    // Positive interest raises completion/save/share and lowers skips; negative does the opposite.
    let lean = (interest / 4.0).tanh();

    let item_completion = smooth(s.completions, s.starts, band_completion, 10.0);
    let p_complete = (item_completion * (1.0 + 0.2 * lean)).clamp(0.0, 1.0);
    let item_skip = smooth(s.skips, s.starts, band_skip, 10.0);
    let p_skip = (item_skip * (1.0 - 0.3 * lean.max(0.0))
        + 0.3 * (-lean).max(0.0) * (1.0 - item_skip))
        .clamp(0.0, 1.0);
    let p_save = (smooth(s.saves, s.starts, PRIOR_SAVE, 20.0) * (1.0 + 0.5 * lean)).clamp(0.0, 1.0);
    let p_share =
        (smooth(s.shares, s.starts, PRIOR_SHARE, 20.0) * (1.0 + 0.5 * lean)).clamp(0.0, 1.0);
    let p_not_relevant = match user.preference(c.topic_id) {
        Some(Preference::Mute) => 1.0,
        _ => (smooth(s.not_relevant, s.starts, PRIOR_NOT_RELEVANT, 20.0)
            * (1.0 - 0.5 * lean.max(0.0))
            + 0.2 * (-lean).max(0.0))
        .clamp(0.0, 1.0),
    };
    let p_correct = c.has_question.then(|| {
        // Difficulty sets the prior until 8 answers exist; ability shifts it in logit space.
        let prior = match c.difficulty {
            Some(d) => (0.9 - 0.08 * f32::from(d.clamp(1, 5))).clamp(0.3, 0.95),
            None => PRIOR_CORRECT,
        };
        let item = smooth(s.answers_correct, s.answers, prior, 8.0);
        sigmoid(logit(item) + logit(user.ability()) - logit(PRIOR_CORRECT))
    });
    Predictions {
        p_complete,
        completion_lift: p_complete / band_completion,
        p_skip,
        p_save,
        p_share,
        p_not_relevant,
        p_correct,
        interest,
    }
}

/// The value model for one reel.
pub fn score(
    c: &RankCandidate,
    user: &UserProfile,
    ctx: &RankContext<'_>,
) -> (ScoreBreakdown, Predictions) {
    let p = predict(c, user, ctx);
    let w = &ctx.settings.weights;
    let target = ctx.settings.target_correct_rate;
    let challenge = p
        .p_correct
        .map(|pc| (1.0 - ((pc - target) / 0.3).powi(2)).max(0.0))
        .unwrap_or(0.0);
    let urgency = if c.mandatory {
        let soon = c
            .deadline
            .map(|d| {
                let days = (d - ctx.today).num_days();
                if (0..7).contains(&days) {
                    (7 - days) as f32 / 7.0
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0);
        1.0 + soon
    } else if c.assigned {
        0.3
    } else {
        0.0
    };
    let age_days = ((ctx.now - c.published_at).num_seconds().max(0) as f32) / 86_400.0;
    let followed = matches!(user.preference(c.topic_id), Some(Preference::Follow));
    let b = ScoreBreakdown {
        completion: w.completion * (p.completion_lift.clamp(0.0, 2.0) / 2.0),
        learning: w.learning * challenge,
        save: w.save * (10.0 * p.p_save).min(1.5),
        share: w.share * (20.0 * p.p_share).min(1.5),
        interest: w.interest * ((p.interest / 5.0).tanh() + if followed { 0.5 } else { 0.0 }),
        urgency: w.urgency * urgency,
        freshness: w.freshness * (-age_days / 10.0).exp(),
        social: if ctx.settings.social_proof {
            w.social * c.cohort_popularity.clamp(0.0, 1.0)
        } else {
            0.0
        },
        skip: w.skip_penalty * (2.0 * p.p_skip).min(2.0),
        not_relevant: w.not_relevant_penalty * (10.0 * p.p_not_relevant).min(3.0),
        repeat: if c.completed_before {
            w.repeat_penalty
        } else {
            0.0
        },
        shared: if c.shared_by.is_some() {
            SHARED_BOOST
        } else {
            0.0
        },
        exploration: 0.0,
    };
    (b, p)
}

fn reason_for(
    c: &RankCandidate,
    p: &Predictions,
    user: &UserProfile,
    ctx: &RankContext<'_>,
) -> RankReason {
    let age_days = (ctx.now - c.published_at).num_days();
    if c.shared_by.is_some() {
        RankReason::Shared
    } else if c.mandatory {
        RankReason::Mandatory
    } else if matches!(user.preference(c.topic_id), Some(Preference::Follow)) {
        RankReason::Followed
    } else if p.interest >= 2.0 {
        RankReason::Interest
    } else if ctx.settings.social_proof && c.cohort_popularity >= 0.5 {
        RankReason::Popular
    } else if age_days <= 3 {
        RankReason::New
    } else if c.assigned {
        RankReason::Assigned
    } else {
        RankReason::Catalog
    }
}

/// Score every candidate, then fill `limit` slots: colleague shares first, then the best
/// remaining reel that keeps the topic run short, with a share of slots given to
/// under-explored reels by an upper-confidence bonus. Muted topics are dropped unless mandatory.
pub fn rank(
    candidates: &[RankCandidate],
    user: &UserProfile,
    ctx: &RankContext<'_>,
    limit: usize,
) -> Vec<Ranked> {
    let mut seen = HashSet::new();
    let mut pool: Vec<(Ranked, u32, bool)> = candidates
        .iter()
        .filter(|c| seen.insert(c.reel_id))
        .filter(|c| c.mandatory || !matches!(user.preference(c.topic_id), Some(Preference::Mute)))
        .map(|c| {
            let (b, p) = score(c, user, ctx);
            (
                Ranked {
                    reel_id: c.reel_id,
                    topic_id: c.topic_id,
                    score: b.total(),
                    reason: reason_for(c, &p, user, ctx),
                    breakdown: b,
                    predictions: p,
                },
                c.stats.impressions,
                c.shared_by.is_some(),
            )
        })
        .collect();
    pool.sort_by(|a, b| {
        b.0.score
            .partial_cmp(&a.0.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.reel_id.cmp(&b.0.reel_id))
    });

    let total_impressions: u32 = pool.iter().map(|(_, n, _)| *n).sum();
    let ln_total = ((total_impressions as f32) + std::f32::consts::E).ln();
    let mut rng = SplitMix64::from_parts(&ctx.seed);
    let max_run = usize::from(ctx.settings.max_same_topic_in_row.max(1));
    let mut out: Vec<Ranked> = Vec::with_capacity(limit.min(pool.len()));
    let mut taken = vec![false; pool.len()];

    let run_ok = |out: &[Ranked], topic: Uuid| -> bool {
        out.len() < max_run
            || !out[out.len() - max_run..]
                .iter()
                .all(|r| r.topic_id == topic)
    };

    // 1. Colleague shares lead.
    for (i, (r, _, shared)) in pool.iter().enumerate() {
        if out.len() >= limit
            || out
                .iter()
                .filter(|o| o.reason == RankReason::Shared)
                .count()
                >= MAX_SHARED_PER_BATCH
        {
            break;
        }
        if *shared {
            taken[i] = true;
            out.push(r.clone());
        }
    }

    // 2. Fill the rest.
    while out.len() < limit {
        let explore =
            ctx.settings.exploration_rate > 0.0 && rng.next_f32() < ctx.settings.exploration_rate;
        let mut pick: Option<(usize, f32)> = None;
        if explore {
            for (i, (r, n, _)) in pool.iter().enumerate() {
                if taken[i] || *n >= EXPLORE_MIN_IMPRESSIONS || !run_ok(&out, r.topic_id) {
                    continue;
                }
                let bonus = EXPLORE_BONUS * (ln_total / (*n as f32 + 1.0)).sqrt();
                if pick.map(|(_, best)| r.score + bonus > best).unwrap_or(true) {
                    pick = Some((i, r.score + bonus));
                }
            }
            if let Some((i, with_bonus)) = pick {
                let mut r = pool[i].0.clone();
                r.breakdown.exploration = with_bonus - r.score;
                r.score = with_bonus;
                if !matches!(r.reason, RankReason::Mandatory | RankReason::Shared) {
                    r.reason = RankReason::Explore;
                }
                taken[i] = true;
                out.push(r);
                continue;
            }
        }
        let next = pool
            .iter()
            .enumerate()
            .find(|(i, (r, _, _))| !taken[*i] && run_ok(&out, r.topic_id))
            .or_else(|| pool.iter().enumerate().find(|(i, _)| !taken[*i]))
            .map(|(i, _)| i);
        match next {
            Some(i) => {
                taken[i] = true;
                out.push(pool[i].0.clone());
            }
            None => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::settings::Intensity;
    use chrono::{Duration, TimeZone};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 9, 9, 0, 0)
            .single()
            .expect("ts")
    }

    fn cand(id: u128, topic: u128) -> RankCandidate {
        RankCandidate {
            reel_id: Uuid::from_u128(id),
            topic_id: Uuid::from_u128(topic),
            tags: vec![],
            duration_ms: 30_000,
            published_at: now() - Duration::days(30),
            mandatory: false,
            assigned: true,
            deadline: None,
            difficulty: None,
            has_question: true,
            stats: ItemStats::default(),
            cohort_popularity: 0.0,
            completed_before: false,
            shared_by: None,
        }
    }

    fn ctx(settings: &EngineSettings) -> RankContext<'_> {
        RankContext {
            settings,
            now: now(),
            today: now().date_naive(),
            bands: [BandBaseline::default(); BAND_COUNT],
            seed: [1, 2, 3],
        }
    }

    fn no_explore() -> EngineSettings {
        let mut s = EngineSettings::preset(Intensity::Balanced);
        s.exploration_rate = 0.0;
        s.max_same_topic_in_row = 5;
        s
    }

    fn engaged(starts: u32, completions: u32, skips: u32) -> ItemStats {
        ItemStats {
            impressions: starts + 10,
            starts,
            completions,
            skips,
            ..ItemStats::default()
        }
    }

    #[test]
    fn a_reel_people_skip_ranks_below_one_they_finish() {
        let s = no_explore();
        let mut good = cand(1, 10);
        good.stats = engaged(200, 180, 5);
        let mut skipped = cand(2, 10);
        skipped.stats = engaged(200, 60, 120);
        let out = rank(&[skipped, good], &UserProfile::default(), &ctx(&s), 2);
        assert_eq!(out[0].reel_id, Uuid::from_u128(1));
        assert!(out[0].predictions.p_skip < out[1].predictions.p_skip);
    }

    #[test]
    fn completion_is_judged_against_reels_of_the_same_length() {
        let s = no_explore();
        let mut c = ctx(&s);
        // Short reels are usually finished (90 %), long ones rarely (40 %).
        c.bands[0] = BandBaseline {
            starts: 1000,
            completions: 900,
            skips: 50,
        };
        c.bands[4] = BandBaseline {
            starts: 1000,
            completions: 400,
            skips: 50,
        };
        let mut short = cand(1, 10);
        short.duration_ms = 15_000;
        short.stats = engaged(200, 150, 10); // 75 %: below its peers
        let mut long = cand(2, 11);
        long.duration_ms = 80_000;
        long.stats = engaged(200, 120, 10); // 60 %: well above its peers
        let user = UserProfile::default();
        let ps = predict(&short, &user, &c);
        let pl = predict(&long, &user, &c);
        assert!(
            ps.p_complete > pl.p_complete,
            "raw completion favours the short reel"
        );
        assert!(pl.completion_lift > 1.0 && ps.completion_lift < 1.0);
        let out = rank(&[short, long], &user, &c, 2);
        assert_eq!(out[0].reel_id, Uuid::from_u128(2));
    }

    #[test]
    fn interest_pulls_a_topic_forward_and_mute_removes_it() {
        let s = no_explore();
        let a = cand(1, 10);
        let b = cand(2, 20);
        let mut user = UserProfile::default();
        user.topic_interest.insert(Uuid::from_u128(20), 8.0);
        let out = rank(&[a.clone(), b.clone()], &user, &ctx(&s), 2);
        assert_eq!(out[0].reel_id, Uuid::from_u128(2));
        assert_eq!(out[0].reason, RankReason::Interest);

        user.topic_preference
            .insert(Uuid::from_u128(20), Preference::Mute);
        let out = rank(&[a, b.clone()], &user, &ctx(&s), 2);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].reel_id, Uuid::from_u128(1));

        let mut mandatory = b;
        mandatory.mandatory = true;
        let out = rank(&[mandatory], &user, &ctx(&s), 2);
        assert_eq!(out.len(), 1, "mute never hides a mandatory topic");
    }

    #[test]
    fn colleague_shares_lead_the_batch() {
        let s = no_explore();
        let mut strong = cand(1, 10);
        strong.stats = engaged(500, 480, 2);
        strong.mandatory = true;
        let mut shared = cand(2, 20);
        shared.shared_by = Some("Sara".into());
        let out = rank(&[strong, shared], &UserProfile::default(), &ctx(&s), 2);
        assert_eq!(out[0].reel_id, Uuid::from_u128(2));
        assert_eq!(out[0].reason, RankReason::Shared);
    }

    #[test]
    fn topic_runs_are_capped() {
        let mut s = no_explore();
        s.max_same_topic_in_row = 1;
        let mut topic_a: Vec<RankCandidate> = (1..=3)
            .map(|i| {
                let mut c = cand(i, 10);
                c.stats = engaged(100, 95, 1);
                c
            })
            .collect();
        topic_a.push(cand(9, 20));
        let out = rank(&topic_a, &UserProfile::default(), &ctx(&s), 3);
        let topics: Vec<u128> = out.iter().map(|r| r.topic_id.as_u128()).collect();
        assert_eq!(topics, vec![10, 20, 10]);
    }

    #[test]
    fn exploration_gives_slots_to_unseen_reels_and_is_reproducible() {
        let mut s = no_explore();
        s.exploration_rate = 0.5;
        let mut pool: Vec<RankCandidate> = (1..=6)
            .map(|i| {
                let mut c = cand(i, i);
                c.stats = engaged(400, 380, 5);
                c
            })
            .collect();
        pool.push(cand(99, 99)); // never shown
        let user = UserProfile::default();
        let first = rank(&pool, &user, &ctx(&s), 6);
        let again = rank(&pool, &user, &ctx(&s), 6);
        assert_eq!(first, again, "same seed, same order");
        let explored = first
            .iter()
            .find(|r| r.reel_id == Uuid::from_u128(99))
            .expect("explored");
        assert_eq!(explored.reason, RankReason::Explore);
        assert!(explored.breakdown.exploration > 0.0);
    }

    #[test]
    fn learning_value_peaks_at_the_target_answer_rate() {
        let s = no_explore();
        let c = ctx(&s);
        let user = UserProfile::default();
        let mut at_target = cand(1, 10);
        at_target.stats = ItemStats {
            answers: 200,
            answers_correct: 160,
            ..ItemStats::default()
        };
        let mut too_easy = cand(2, 10);
        too_easy.stats = ItemStats {
            answers: 200,
            answers_correct: 199,
            ..ItemStats::default()
        };
        let mut too_hard = cand(3, 10);
        too_hard.stats = ItemStats {
            answers: 200,
            answers_correct: 60,
            ..ItemStats::default()
        };
        let l = |r: &RankCandidate| score(r, &user, &c).0.learning;
        assert!(l(&at_target) > l(&too_easy));
        assert!(l(&at_target) > l(&too_hard));
        assert_eq!(l(&too_hard), 0.0);
    }

    #[test]
    fn a_zero_weight_switches_its_term_off() {
        let mut s = no_explore();
        s.weights = crate::engine::Weights {
            completion: 0.0,
            learning: 0.0,
            save: 0.0,
            share: 0.0,
            interest: 0.0,
            urgency: 0.0,
            freshness: 0.0,
            social: 0.0,
            skip_penalty: 0.0,
            not_relevant_penalty: 0.0,
            repeat_penalty: 0.0,
        };
        let mut c = cand(1, 10);
        c.stats = engaged(100, 10, 80);
        c.mandatory = true;
        let (b, _) = score(&c, &UserProfile::default(), &ctx(&s));
        assert_eq!(b.total(), 0.0);
    }

    #[test]
    fn a_close_deadline_raises_urgency_and_seen_reels_are_penalised() {
        let s = no_explore();
        let c = ctx(&s);
        let mut due_soon = cand(1, 10);
        due_soon.mandatory = true;
        due_soon.deadline = Some(c.today + Duration::days(1));
        let mut later = due_soon.clone();
        later.deadline = Some(c.today + Duration::days(30));
        let user = UserProfile::default();
        assert!(score(&due_soon, &user, &c).0.urgency > score(&later, &user, &c).0.urgency);
        let mut seen = cand(3, 10);
        seen.completed_before = true;
        assert!(score(&seen, &user, &c).0.total() < score(&cand(4, 10), &user, &c).0.total());
    }
}
