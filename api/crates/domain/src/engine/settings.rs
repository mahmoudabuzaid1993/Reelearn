//! What a company can tune: an intensity preset, the value-model weights, exploration,
//! diversity, practice caps, break reminders and re-engagement reminders.
//!
//! Stored under `tenants.settings.engine`. The four presets are our choices; the research
//! notes give no published values for an enterprise learning feed (see `docs/engine.md`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Intensity {
    /// The daily minute only, ordered by the engine. No practice after it.
    Gentle,
    /// The daily minute, then up to 15 minutes of practice a day.
    #[default]
    Balanced,
    /// Longer practice, more colleague signals, two reminders a day.
    Intense,
    /// No practice caps, strongest engagement weights, three reminders a day.
    Maximum,
    /// Any combination the company set by hand.
    Custom,
}

impl Intensity {
    pub const PRESETS: [Intensity; 4] = [
        Intensity::Gentle,
        Intensity::Balanced,
        Intensity::Intense,
        Intensity::Maximum,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Intensity::Gentle => "gentle",
            Intensity::Balanced => "balanced",
            Intensity::Intense => "intense",
            Intensity::Maximum => "maximum",
            Intensity::Custom => "custom",
        }
    }
}

/// Value-model weights. Every term is scaled to roughly 0..1 before weighting, so a weight
/// of 1 means "as important as a typical signal" and 0 switches the term off.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Weights {
    /// Chance of finishing, compared with reels of the same length.
    pub completion: f32,
    /// Desirable difficulty: questions the person will probably, but not certainly, get right.
    pub learning: f32,
    pub save: f32,
    /// Sending to a colleague, the strongest public signal on Instagram.
    pub share: f32,
    /// Decayed interest in the reel's topic and tags.
    pub interest: f32,
    /// Mandatory topics and approaching deadlines.
    pub urgency: f32,
    pub freshness: f32,
    /// What colleagues in the same location, job and team finished, saved and shared.
    pub social: f32,
    /// Subtracted: chance of leaving in the first three seconds.
    pub skip_penalty: f32,
    /// Subtracted: chance of "not for my job".
    pub not_relevant_penalty: f32,
    /// Subtracted: the person already finished this reel.
    pub repeat_penalty: f32,
}

impl Default for Weights {
    fn default() -> Self {
        EngineSettings::preset(Intensity::Balanced).weights
    }
}

impl Weights {
    pub const MAX: f32 = 5.0;

    /// Negative feedback is the person's right, not a tuning knob: it can be weakened but
    /// never switched off (research notes, "Admin knobs").
    pub fn min_for(name: &str) -> f32 {
        match name {
            "not_relevant_penalty" | "skip_penalty" => 1.0,
            _ => 0.0,
        }
    }

    fn named(&self) -> [(&'static str, f32); 11] {
        [
            ("completion", self.completion),
            ("learning", self.learning),
            ("save", self.save),
            ("share", self.share),
            ("interest", self.interest),
            ("urgency", self.urgency),
            ("freshness", self.freshness),
            ("social", self.social),
            ("skip_penalty", self.skip_penalty),
            ("not_relevant_penalty", self.not_relevant_penalty),
            ("repeat_penalty", self.repeat_penalty),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineSettings {
    pub intensity: Intensity,
    pub weights: Weights,
    /// Share of slots given to reels with few impressions (0 to 0.5).
    pub exploration_rate: f32,
    /// Days after which an interest signal counts half (1 to 90).
    pub interest_half_life_days: f32,
    /// Longest run of reels from one topic (1 to 5).
    pub max_same_topic_in_row: u8,
    /// Interleave a review question every N reels in practice (0 = never, up to 10).
    pub review_every: u8,
    /// The answer rate the engine aims for when choosing questions (0.5 to 0.95).
    pub target_correct_rate: f32,
    /// Include published reels outside the person's assigned topics.
    pub open_catalog: bool,
    /// Use colleagues' activity as a signal and show "Popular with your team".
    pub social_proof: bool,
    /// Let people send reels to colleagues.
    pub peer_sharing: bool,
    /// Offer "Keep going" practice after the daily minute.
    pub continuous_enabled: bool,
    /// Practice minutes per person per day (0 = no cap, up to 240).
    pub daily_minutes_cap: u16,
    /// Minutes per practice session (0 = no cap, up to 240).
    pub session_minutes_cap: u16,
    /// Suggest a pause every N minutes of practice (0 = never, up to 120).
    pub break_reminder_minutes: u16,
    /// Allow practice outside the company's delivery windows.
    pub practice_outside_work_hours: bool,
    /// Reminders a person may receive per day (0 to 6).
    pub max_nudges_per_day: u8,
    /// Send a reminder after this many hours away (0 = never, up to 72).
    pub reengage_after_hours: u16,
    /// Start the next reel automatically after a question is answered.
    pub autoplay_next: bool,
    /// Items returned per request (3 to 20).
    pub batch_size: u8,
}

impl Default for EngineSettings {
    fn default() -> Self {
        EngineSettings::preset(Intensity::Balanced)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl EngineSettings {
    pub fn preset(intensity: Intensity) -> EngineSettings {
        match intensity {
            Intensity::Gentle => EngineSettings {
                intensity,
                weights: Weights {
                    completion: 1.0,
                    learning: 1.5,
                    save: 0.5,
                    share: 0.5,
                    interest: 0.8,
                    urgency: 2.0,
                    freshness: 0.3,
                    social: 0.0,
                    skip_penalty: 1.0,
                    not_relevant_penalty: 3.0,
                    repeat_penalty: 1.0,
                },
                exploration_rate: 0.05,
                interest_half_life_days: 21.0,
                max_same_topic_in_row: 1,
                review_every: 0,
                target_correct_rate: 0.85,
                open_catalog: false,
                social_proof: false,
                peer_sharing: true,
                continuous_enabled: false,
                daily_minutes_cap: 0,
                session_minutes_cap: 0,
                break_reminder_minutes: 0,
                practice_outside_work_hours: false,
                max_nudges_per_day: 1,
                reengage_after_hours: 0,
                autoplay_next: false,
                batch_size: 5,
            },
            Intensity::Balanced | Intensity::Custom => EngineSettings {
                intensity: Intensity::Balanced,
                weights: Weights {
                    completion: 1.2,
                    learning: 1.2,
                    save: 0.8,
                    share: 1.0,
                    interest: 1.0,
                    urgency: 1.5,
                    freshness: 0.5,
                    social: 0.5,
                    skip_penalty: 1.2,
                    not_relevant_penalty: 3.0,
                    repeat_penalty: 1.0,
                },
                exploration_rate: 0.1,
                interest_half_life_days: 14.0,
                max_same_topic_in_row: 2,
                review_every: 4,
                target_correct_rate: 0.85,
                open_catalog: true,
                social_proof: true,
                peer_sharing: true,
                continuous_enabled: true,
                daily_minutes_cap: 15,
                session_minutes_cap: 10,
                break_reminder_minutes: 10,
                practice_outside_work_hours: false,
                max_nudges_per_day: 1,
                reengage_after_hours: 24,
                autoplay_next: true,
                batch_size: 6,
            },
            Intensity::Intense => EngineSettings {
                intensity,
                weights: Weights {
                    completion: 1.6,
                    learning: 0.9,
                    save: 1.0,
                    share: 1.4,
                    interest: 1.4,
                    urgency: 1.2,
                    freshness: 0.7,
                    social: 0.8,
                    skip_penalty: 1.5,
                    not_relevant_penalty: 3.0,
                    repeat_penalty: 1.2,
                },
                exploration_rate: 0.12,
                interest_half_life_days: 10.0,
                max_same_topic_in_row: 2,
                review_every: 5,
                target_correct_rate: 0.85,
                open_catalog: true,
                social_proof: true,
                peer_sharing: true,
                continuous_enabled: true,
                daily_minutes_cap: 45,
                session_minutes_cap: 20,
                break_reminder_minutes: 20,
                practice_outside_work_hours: true,
                max_nudges_per_day: 2,
                reengage_after_hours: 8,
                autoplay_next: true,
                batch_size: 8,
            },
            Intensity::Maximum => EngineSettings {
                intensity,
                weights: Weights {
                    completion: 2.0,
                    learning: 0.6,
                    save: 1.2,
                    share: 1.8,
                    interest: 1.8,
                    urgency: 1.0,
                    freshness: 0.8,
                    social: 1.0,
                    skip_penalty: 1.8,
                    not_relevant_penalty: 3.0,
                    repeat_penalty: 1.5,
                },
                exploration_rate: 0.15,
                interest_half_life_days: 7.0,
                max_same_topic_in_row: 3,
                review_every: 6,
                target_correct_rate: 0.85,
                open_catalog: true,
                social_proof: true,
                peer_sharing: true,
                continuous_enabled: true,
                daily_minutes_cap: 0,
                session_minutes_cap: 0,
                break_reminder_minutes: 0,
                practice_outside_work_hours: true,
                max_nudges_per_day: 3,
                reengage_after_hours: 4,
                autoplay_next: true,
                batch_size: 10,
            },
        }
    }

    /// The preset these values match exactly, else `Custom`.
    pub fn matching_intensity(&self) -> Intensity {
        for p in Intensity::PRESETS {
            let mut candidate = EngineSettings::preset(p);
            candidate.intensity = self.intensity;
            if &candidate == self {
                return p;
            }
        }
        Intensity::Custom
    }

    /// Re-label after an edit so the stored intensity always tells the truth.
    pub fn relabelled(mut self) -> EngineSettings {
        self.intensity = self.matching_intensity();
        self
    }

    pub fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut errors = Vec::new();
        let mut check = |ok: bool, field: &str, message: &str| {
            if !ok {
                errors.push(FieldError {
                    field: field.to_string(),
                    message: message.to_string(),
                });
            }
        };
        for (name, w) in self.weights.named() {
            let min = Weights::min_for(name);
            check(
                w.is_finite() && (min..=Weights::MAX).contains(&w),
                &format!("weights.{name}"),
                if min > 0.0 {
                    "must be between 1 and 5: people's own \"not for me\" and quick skips always count"
                } else {
                    "must be between 0 and 5"
                },
            );
        }
        check(
            (0.0..=0.5).contains(&self.exploration_rate),
            "exploration_rate",
            "must be between 0 and 0.5",
        );
        check(
            (1.0..=90.0).contains(&self.interest_half_life_days),
            "interest_half_life_days",
            "must be between 1 and 90",
        );
        check(
            (1..=5).contains(&self.max_same_topic_in_row),
            "max_same_topic_in_row",
            "must be between 1 and 5",
        );
        check(
            self.review_every <= 10,
            "review_every",
            "must be at most 10",
        );
        check(
            (0.5..=0.95).contains(&self.target_correct_rate),
            "target_correct_rate",
            "must be between 0.5 and 0.95",
        );
        check(
            self.daily_minutes_cap <= 240,
            "daily_minutes_cap",
            "must be at most 240",
        );
        check(
            self.session_minutes_cap <= 240,
            "session_minutes_cap",
            "must be at most 240",
        );
        check(
            self.break_reminder_minutes <= 120,
            "break_reminder_minutes",
            "must be at most 120",
        );
        check(
            self.max_nudges_per_day <= 6,
            "max_nudges_per_day",
            "must be at most 6",
        );
        check(
            self.reengage_after_hours <= 72,
            "reengage_after_hours",
            "must be at most 72",
        );
        check(
            (3..=20).contains(&self.batch_size),
            "batch_size",
            "must be between 3 and 20",
        );
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Clamp stored values into range, for settings written by an older version.
    pub fn sanitized(mut self) -> EngineSettings {
        let w = &mut self.weights;
        for v in [
            &mut w.completion,
            &mut w.learning,
            &mut w.save,
            &mut w.share,
            &mut w.interest,
            &mut w.urgency,
            &mut w.freshness,
            &mut w.social,
            &mut w.skip_penalty,
            &mut w.not_relevant_penalty,
            &mut w.repeat_penalty,
        ] {
            *v = if v.is_finite() {
                v.clamp(0.0, Weights::MAX)
            } else {
                1.0
            };
        }
        w.not_relevant_penalty = w
            .not_relevant_penalty
            .max(Weights::min_for("not_relevant_penalty"));
        w.skip_penalty = w.skip_penalty.max(Weights::min_for("skip_penalty"));
        self.exploration_rate = clamp_f(self.exploration_rate, 0.0, 0.5, 0.1);
        self.interest_half_life_days = clamp_f(self.interest_half_life_days, 1.0, 90.0, 14.0);
        self.max_same_topic_in_row = self.max_same_topic_in_row.clamp(1, 5);
        self.review_every = self.review_every.min(10);
        self.target_correct_rate = clamp_f(self.target_correct_rate, 0.5, 0.95, 0.85);
        self.daily_minutes_cap = self.daily_minutes_cap.min(240);
        self.session_minutes_cap = self.session_minutes_cap.min(240);
        self.break_reminder_minutes = self.break_reminder_minutes.min(120);
        self.max_nudges_per_day = self.max_nudges_per_day.min(6);
        self.reengage_after_hours = self.reengage_after_hours.min(72);
        self.batch_size = self.batch_size.clamp(3, 20);
        self
    }
}

fn clamp_f(v: f32, lo: f32, hi: f32, fallback: f32) -> f32 {
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_is_valid_and_recognised() {
        for p in Intensity::PRESETS {
            let s = EngineSettings::preset(p);
            assert!(s.validate().is_ok(), "{p:?}: {:?}", s.validate());
            assert_eq!(s.matching_intensity(), p);
        }
    }

    #[test]
    fn presets_grow_in_intensity() {
        let g = EngineSettings::preset(Intensity::Gentle);
        let b = EngineSettings::preset(Intensity::Balanced);
        let i = EngineSettings::preset(Intensity::Intense);
        let m = EngineSettings::preset(Intensity::Maximum);
        assert!(!g.continuous_enabled && b.continuous_enabled);
        assert!(b.daily_minutes_cap < i.daily_minutes_cap);
        assert_eq!(m.daily_minutes_cap, 0, "maximum has no daily cap");
        assert!(g.max_nudges_per_day <= b.max_nudges_per_day);
        assert!(i.max_nudges_per_day < m.max_nudges_per_day);
        assert!(b.weights.completion < m.weights.completion);
        assert!(b.weights.learning > m.weights.learning);
    }

    #[test]
    fn an_edit_relabels_as_custom() {
        let mut s = EngineSettings::preset(Intensity::Intense);
        s.daily_minutes_cap = 30;
        assert_eq!(s.relabelled().intensity, Intensity::Custom);
        let back = EngineSettings::preset(Intensity::Intense);
        assert_eq!(back.relabelled().intensity, Intensity::Intense);
    }

    #[test]
    fn validation_names_each_bad_field() {
        let mut s = EngineSettings::default();
        s.weights.share = 9.0;
        s.exploration_rate = 0.9;
        s.batch_size = 1;
        let errors = s.validate().expect_err("invalid");
        let fields: Vec<&str> = errors.iter().map(|e| e.field.as_str()).collect();
        assert_eq!(
            fields,
            vec!["weights.share", "exploration_rate", "batch_size"]
        );
    }

    #[test]
    fn negative_feedback_cannot_be_switched_off() {
        let mut s = EngineSettings::default();
        s.weights.not_relevant_penalty = 0.0;
        s.weights.skip_penalty = 0.2;
        let errors = s.validate().expect_err("invalid");
        assert_eq!(errors.len(), 2);
        let clean = s.sanitized();
        assert_eq!(clean.weights.not_relevant_penalty, 1.0);
        assert_eq!(clean.weights.skip_penalty, 1.0);
    }

    #[test]
    fn missing_keys_fall_back_to_balanced_and_bad_values_are_clamped() {
        let s: EngineSettings =
            serde_json::from_value(serde_json::json!({ "daily_minutes_cap": 30 })).expect("json");
        assert_eq!(s.daily_minutes_cap, 30);
        assert_eq!(s.batch_size, 6);
        let mut wild = EngineSettings::default();
        wild.weights.save = f32::NAN;
        wild.batch_size = 99;
        let clean = wild.sanitized();
        assert_eq!(clean.weights.save, 1.0);
        assert_eq!(clean.batch_size, 20);
    }
}
