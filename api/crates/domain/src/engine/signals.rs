//! Signals and interest.
//!
//! Interest is a pointwise, time-decayed sum, in the spirit of Meta's HSTU note that the
//! *intensity* of a preference matters, so repeated engagement is not normalised away:
//! `score(now) = score(t) * 0.5^((now - t) / half_life) + delta`.
//! Deltas are our choices (no published values); negative feedback costs more than a view.

use crate::EngagementKind;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Leaving a reel before this counts as a skip (Meta's Reels card predicts "watch < 3 s").
pub const SKIP_THRESHOLD_MS: i32 = 3_000;

/// Upper edges (exclusive, in ms) of the duration bands used to normalise completion.
/// Band 4 is everything from 70 s up.
pub const DURATION_BANDS: [i32; 4] = [20_000, 35_000, 50_000, 70_000];

/// Interest scores stay inside this range so one obsession cannot drown everything else.
pub const INTEREST_MIN: f32 = -20.0;
pub const INTEREST_MAX: f32 = 50.0;

pub const ANSWER_CORRECT_DELTA: f32 = 0.4;
pub const ANSWER_WRONG_DELTA: f32 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preference {
    Follow,
    Mute,
}

impl Preference {
    pub fn as_str(&self) -> &'static str {
        match self {
            Preference::Follow => "follow",
            Preference::Mute => "mute",
        }
    }

    pub fn parse(s: &str) -> Option<Preference> {
        match s {
            "follow" => Some(Preference::Follow),
            "mute" => Some(Preference::Mute),
            _ => None,
        }
    }
}

pub fn band_of(duration_ms: i32) -> usize {
    DURATION_BANDS
        .iter()
        .position(|edge| duration_ms < *edge)
        .unwrap_or(DURATION_BANDS.len())
}

/// How much one event moves interest in the reel's topic (tags move by half of it).
/// `watched_fraction` scales a completion: finishing a long reel says more than a short one.
pub fn interest_delta(kind: EngagementKind) -> f32 {
    match kind {
        EngagementKind::Impression => 0.0,
        EngagementKind::Start => 0.05,
        EngagementKind::Skip => -0.6,
        EngagementKind::Complete => 1.0,
        EngagementKind::Rewatch => 1.5,
        EngagementKind::Save => 2.0,
        EngagementKind::Unsave => -1.0,
        EngagementKind::Share => 3.0,
        EngagementKind::NotRelevant => -4.0,
        EngagementKind::MoreLikeThis => 3.0,
    }
}

/// Whether the event changes a person's interest scores at all.
pub fn moves_interest(kind: EngagementKind) -> bool {
    interest_delta(kind) != 0.0
}

/// Decay `score`, last written at `last`, to `now`.
pub fn decayed(score: f32, last: DateTime<Utc>, now: DateTime<Utc>, half_life_days: f32) -> f32 {
    let hl = if half_life_days.is_finite() && half_life_days > 0.0 {
        half_life_days
    } else {
        14.0
    };
    let days = ((now - last).num_seconds().max(0) as f32) / 86_400.0;
    score * 0.5_f32.powf(days / hl)
}

/// The new stored score after an event: decay to now, add, clamp.
pub fn add_interest(
    score: f32,
    last: DateTime<Utc>,
    now: DateTime<Utc>,
    half_life_days: f32,
    delta: f32,
) -> f32 {
    (decayed(score, last, now, half_life_days) + delta).clamp(INTEREST_MIN, INTEREST_MAX)
}

/// Turn the raw watch of a reel into the signal it carries.
pub fn classify_watch(
    watched_ms: i32,
    duration_ms: i32,
    completed: bool,
) -> Option<EngagementKind> {
    if completed || (duration_ms > 0 && watched_ms * 10 >= duration_ms * 9) {
        Some(EngagementKind::Complete)
    } else if watched_ms < SKIP_THRESHOLD_MS {
        Some(EngagementKind::Skip)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn bands_split_by_length() {
        assert_eq!(band_of(15_000), 0);
        assert_eq!(band_of(20_000), 1);
        assert_eq!(band_of(34_999), 1);
        assert_eq!(band_of(45_000), 2);
        assert_eq!(band_of(60_000), 3);
        assert_eq!(band_of(90_000), 4);
    }

    #[test]
    fn interest_halves_after_one_half_life() {
        let t0 = Utc::now();
        let later = t0 + Duration::days(14);
        let v = decayed(10.0, t0, later, 14.0);
        assert!((v - 5.0).abs() < 0.01, "{v}");
        assert_eq!(decayed(10.0, t0, t0, 14.0), 10.0);
    }

    #[test]
    fn repeated_engagement_accumulates_instead_of_normalising() {
        let t0 = Utc::now();
        let mut s = 0.0;
        for i in 0..10 {
            s = add_interest(s, t0, t0 + Duration::minutes(i), 14.0, 1.0);
        }
        assert!(s > 9.9, "ten completions should count ten times: {s}");
    }

    #[test]
    fn negative_feedback_outweighs_a_view_and_scores_are_bounded() {
        assert!(
            interest_delta(EngagementKind::NotRelevant) < -interest_delta(EngagementKind::Complete)
        );
        let t0 = Utc::now();
        let mut s = 0.0;
        for _ in 0..100 {
            s = add_interest(s, t0, t0, 14.0, interest_delta(EngagementKind::Share));
        }
        assert_eq!(s, INTEREST_MAX);
    }

    #[test]
    fn a_watch_becomes_a_skip_or_a_completion() {
        assert_eq!(
            classify_watch(1_200, 30_000, false),
            Some(EngagementKind::Skip)
        );
        assert_eq!(
            classify_watch(27_500, 30_000, false),
            Some(EngagementKind::Complete)
        );
        assert_eq!(
            classify_watch(10_000, 30_000, true),
            Some(EngagementKind::Complete)
        );
        assert_eq!(classify_watch(10_000, 30_000, false), None);
    }
}
