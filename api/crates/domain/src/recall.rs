//! The recall engine: SM-2 with fixed first intervals.
//!
//! Learners see each question rarely, so the first four successful repetitions use
//! fixed intervals (1, 3, 10, 30 days) regardless of ease. From the fifth repetition
//! on, `interval = round(previous_interval * ease)`. The ease factor only moves once a
//! question has been answered correctly at least twice (reps >= 2 before the answer).
//!
//! A wrong answer re-schedules at 3 days, counts a lapse, resets the consecutive-correct
//! counter and (if ease is allowed to move) lowers ease by 0.2, never below 1.3.

use chrono::NaiveDate;

/// The first intervals in days, indexed by the number of consecutive correct answers.
pub const FIRST_INTERVALS: [u16; 4] = [1, 3, 10, 30];
/// Days until a question comes back after a wrong answer.
pub const LAPSE_INTERVAL: u16 = 3;
pub const MIN_EASE: f32 = 1.3;
pub const MAX_EASE: f32 = 3.0;
pub const DEFAULT_EASE: f32 = 2.5;
pub const EASE_GAIN: f32 = 0.1;
pub const EASE_PENALTY: f32 = 0.2;
/// A question counts as "remembered" once it has been answered correctly twice in a row.
pub const REMEMBERED_REPS: i16 = 2;

/// Spaced-repetition state of one (user, question) pair. Mirrors `memory_states`.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryState {
    pub due_on: NaiveDate,
    pub interval_days: i16,
    pub ease: f32,
    /// Consecutive correct answers.
    pub reps: i16,
    pub lapses: i16,
    pub last_result: Option<bool>,
}

impl MemoryState {
    /// State for a question that has never been reviewed; due today so a first
    /// (correct) answer schedules the 1-day interval.
    pub fn fresh(today: NaiveDate) -> Self {
        MemoryState {
            due_on: today,
            interval_days: 0,
            ease: DEFAULT_EASE,
            reps: 0,
            lapses: 0,
            last_result: None,
        }
    }

    pub fn is_remembered(&self) -> bool {
        self.reps >= REMEMBERED_REPS && self.last_result == Some(true)
    }
}

/// Outcome of applying one answer to a memory state.
#[derive(Debug, Clone, PartialEq)]
pub struct Review {
    pub state: MemoryState,
    pub was_correct: bool,
}

/// Apply an answer given on `today` and return the next state.
pub fn review(prev: &MemoryState, correct: bool, today: NaiveDate) -> Review {
    let ease_may_move = prev.reps >= REMEMBERED_REPS;
    let mut next = prev.clone();
    next.last_result = Some(correct);

    if correct {
        let reps_after = prev.reps.saturating_add(1);
        next.reps = reps_after;
        let idx = usize::try_from(reps_after.saturating_sub(1)).unwrap_or(0);
        next.interval_days = match FIRST_INTERVALS.get(idx) {
            Some(fixed) => i16::try_from(*fixed).unwrap_or(i16::MAX),
            None => grow_interval(prev.interval_days, prev.ease),
        };
        if ease_may_move {
            next.ease = (prev.ease + EASE_GAIN).min(MAX_EASE);
        }
    } else {
        next.reps = 0;
        next.lapses = prev.lapses.saturating_add(1);
        next.interval_days = i16::try_from(LAPSE_INTERVAL).unwrap_or(3);
        if ease_may_move {
            next.ease = (prev.ease - EASE_PENALTY).max(MIN_EASE);
        }
    }

    next.due_on = today + chrono::Duration::days(i64::from(next.interval_days));
    Review {
        state: next,
        was_correct: correct,
    }
}

fn grow_interval(previous: i16, ease: f32) -> i16 {
    let base = f32::from(previous.max(1));
    let grown = (base * ease).round();
    // Clamp into the i16 column; a year-long interval is the practical ceiling anyway.
    let capped = grown.clamp(1.0, 365.0);
    // Safe: the value is within [1, 365].
    capped as i16
}

/// Human line for a review item: "Asked again · day 10".
pub fn review_note(state: &MemoryState) -> String {
    format!("Asked again · day {}", state.interval_days.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
    }

    #[test]
    fn first_four_correct_answers_use_fixed_intervals() {
        let today = d(2026, 10, 1);
        let mut state = MemoryState::fresh(today);
        let mut day = today;
        let mut seen = vec![];
        for _ in 0..4 {
            let r = review(&state, true, day);
            seen.push(r.state.interval_days);
            assert_eq!(
                r.state.due_on,
                day + chrono::Duration::days(i64::from(r.state.interval_days))
            );
            day = r.state.due_on;
            state = r.state;
        }
        assert_eq!(seen, vec![1, 3, 10, 30]);
        assert_eq!(state.reps, 4);
        assert_eq!(state.lapses, 0);
    }

    #[test]
    fn ease_only_moves_after_second_rep() {
        let today = d(2026, 10, 1);
        let s0 = MemoryState::fresh(today);
        let s1 = review(&s0, true, today).state;
        assert_eq!(
            s1.ease, DEFAULT_EASE,
            "first correct answer leaves ease alone"
        );
        let s2 = review(&s1, true, today).state;
        assert_eq!(
            s2.ease, DEFAULT_EASE,
            "second correct answer leaves ease alone"
        );
        let s3 = review(&s2, true, today).state;
        assert!(
            (s3.ease - 2.6).abs() < 1e-5,
            "third correct answer raises ease by 0.1"
        );
    }

    #[test]
    fn ease_is_capped_at_max() {
        let today = d(2026, 10, 1);
        let mut s = MemoryState {
            ease: 2.95,
            reps: 5,
            ..MemoryState::fresh(today)
        };
        s = review(&s, true, today).state;
        assert_eq!(s.ease, MAX_EASE);
    }

    #[test]
    fn fifth_rep_grows_by_ease() {
        let today = d(2026, 10, 1);
        let s = MemoryState {
            interval_days: 30,
            ease: 2.5,
            reps: 4,
            ..MemoryState::fresh(today)
        };
        let r = review(&s, true, today).state;
        assert_eq!(r.interval_days, 75);
        assert_eq!(r.due_on, d(2026, 12, 15));
    }

    #[test]
    fn wrong_answer_comes_back_in_three_days_and_counts_a_lapse() {
        let today = d(2026, 10, 10);
        let s = MemoryState {
            interval_days: 10,
            ease: 2.5,
            reps: 3,
            ..MemoryState::fresh(today)
        };
        let r = review(&s, false, today);
        assert!(!r.was_correct);
        assert_eq!(r.state.interval_days, 3);
        assert_eq!(r.state.due_on, d(2026, 10, 13));
        assert_eq!(r.state.lapses, 1);
        assert_eq!(r.state.reps, 0);
        assert!((r.state.ease - 2.3).abs() < 1e-5);
        assert_eq!(r.state.last_result, Some(false));
    }

    #[test]
    fn wrong_answer_before_second_rep_does_not_touch_ease() {
        let today = d(2026, 10, 10);
        let s = MemoryState {
            reps: 1,
            ..MemoryState::fresh(today)
        };
        let r = review(&s, false, today).state;
        assert_eq!(r.ease, DEFAULT_EASE);
        assert_eq!(r.interval_days, 3);
    }

    #[test]
    fn ease_never_drops_below_min() {
        let today = d(2026, 10, 10);
        let s = MemoryState {
            reps: 6,
            ease: 1.4,
            ..MemoryState::fresh(today)
        };
        let r = review(&s, false, today).state;
        assert_eq!(r.ease, MIN_EASE);
    }

    #[test]
    fn remembered_requires_two_consecutive_correct() {
        let today = d(2026, 10, 1);
        let s0 = MemoryState::fresh(today);
        let s1 = review(&s0, true, today).state;
        assert!(!s1.is_remembered());
        let s2 = review(&s1, true, today).state;
        assert!(s2.is_remembered());
        let s3 = review(&s2, false, today).state;
        assert!(!s3.is_remembered());
    }

    #[test]
    fn review_note_names_the_day() {
        let s = MemoryState {
            interval_days: 10,
            ..MemoryState::fresh(d(2026, 1, 1))
        };
        assert_eq!(review_note(&s), "Asked again · day 10");
    }
}
