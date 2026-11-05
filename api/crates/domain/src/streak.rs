//! Streak rules.
//!
//! - A streak counts consecutive learning days.
//! - Friday never breaks a streak, and neither do the tenant's configured weekend days.
//! - A missed working day consumes a freeze if one is available; otherwise the streak
//!   restarts at 1 on the next active day.
//! - Freezes replenish to `freezes_per_week` at the start of each ISO week.

use chrono::{Datelike, NaiveDate, Weekday};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreakState {
    pub streak_current: i32,
    pub streak_longest: i32,
    pub last_active_on: Option<NaiveDate>,
    pub freezes_available: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreakOutcome {
    pub state: StreakState,
    pub freezes_used: i16,
    /// True when the streak restarted because a working day was missed without a freeze.
    pub streak_broken: bool,
}

/// Parse tenant `settings.weekend_days` entries like "fri", "sat", "sun".
pub fn parse_weekend_days(days: &[String]) -> Vec<Weekday> {
    days.iter()
        .filter_map(|d| match d.to_ascii_lowercase().as_str() {
            "mon" | "monday" => Some(Weekday::Mon),
            "tue" | "tuesday" => Some(Weekday::Tue),
            "wed" | "wednesday" => Some(Weekday::Wed),
            "thu" | "thursday" => Some(Weekday::Thu),
            "fri" | "friday" => Some(Weekday::Fri),
            "sat" | "saturday" => Some(Weekday::Sat),
            "sun" | "sunday" => Some(Weekday::Sun),
            _ => None,
        })
        .collect()
}

fn is_rest_day(day: NaiveDate, weekend: &[Weekday]) -> bool {
    day.weekday() == Weekday::Fri || weekend.contains(&day.weekday())
}

/// Working days strictly between `from` and `to` that were missed.
fn missed_working_days(from: NaiveDate, to: NaiveDate, weekend: &[Weekday]) -> i64 {
    let mut d = from + chrono::Duration::days(1);
    let mut missed = 0;
    while d < to {
        if !is_rest_day(d, weekend) {
            missed += 1;
        }
        d += chrono::Duration::days(1);
    }
    missed
}

/// Replenish freezes when a new ISO week has started since the last activity.
pub fn replenish_freezes(state: &StreakState, today: NaiveDate, freezes_per_week: i16) -> i16 {
    match state.last_active_on {
        Some(last) if last.iso_week() == today.iso_week() => state.freezes_available,
        _ => state.freezes_available.max(freezes_per_week),
    }
}

/// Record learning activity on `today` and return the new streak state.
pub fn on_activity(
    state: &StreakState,
    today: NaiveDate,
    weekend: &[Weekday],
    freezes_per_week: i16,
) -> StreakOutcome {
    let mut next = state.clone();
    next.freezes_available = replenish_freezes(state, today, freezes_per_week);

    let (freezes_used, broken) = match state.last_active_on {
        Some(last) if last >= today => {
            // Already counted today (or clock skew): nothing changes.
            return StreakOutcome {
                state: next,
                freezes_used: 0,
                streak_broken: false,
            };
        }
        Some(last) => {
            let missed = missed_working_days(last, today, weekend);
            if missed == 0 {
                next.streak_current = state.streak_current.max(0) + 1;
                (0, false)
            } else {
                let available = i64::from(next.freezes_available.max(0));
                if missed <= available {
                    let used = i16::try_from(missed).unwrap_or(i16::MAX);
                    next.freezes_available -= used;
                    next.streak_current = state.streak_current.max(0) + 1;
                    (used, false)
                } else {
                    next.streak_current = 1;
                    (0, true)
                }
            }
        }
        None => {
            next.streak_current = 1;
            (0, false)
        }
    };

    next.last_active_on = Some(today);
    next.streak_longest = next.streak_longest.max(next.streak_current);
    StreakOutcome {
        state: next,
        freezes_used,
        streak_broken: broken,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("date")
    }

    fn state(streak: i32, last: Option<NaiveDate>, freezes: i16) -> StreakState {
        StreakState {
            streak_current: streak,
            streak_longest: streak,
            last_active_on: last,
            freezes_available: freezes,
        }
    }

    const KSA_WEEKEND: [Weekday; 2] = [Weekday::Fri, Weekday::Sat];

    #[test]
    fn first_activity_starts_a_streak() {
        let out = on_activity(&state(0, None, 1), d(2026, 10, 19), &KSA_WEEKEND, 1);
        assert_eq!(out.state.streak_current, 1);
        assert_eq!(out.state.streak_longest, 1);
        assert_eq!(out.state.last_active_on, Some(d(2026, 10, 19)));
    }

    #[test]
    fn consecutive_days_extend_the_streak() {
        // 2026-10-19 is a Monday.
        let out = on_activity(
            &state(3, Some(d(2026, 10, 19)), 1),
            d(2026, 10, 20),
            &KSA_WEEKEND,
            1,
        );
        assert_eq!(out.state.streak_current, 4);
        assert_eq!(out.freezes_used, 0);
    }

    #[test]
    fn same_day_twice_does_not_double_count() {
        let out = on_activity(
            &state(3, Some(d(2026, 10, 19)), 1),
            d(2026, 10, 19),
            &KSA_WEEKEND,
            1,
        );
        assert_eq!(out.state.streak_current, 3);
    }

    #[test]
    fn friday_never_breaks_a_streak_even_without_weekend_config() {
        // Thursday 2026-10-22 -> Saturday 2026-10-24 skipping Friday, no weekend days set.
        let out = on_activity(&state(5, Some(d(2026, 10, 22)), 0), d(2026, 10, 24), &[], 1);
        assert_eq!(out.state.streak_current, 6);
        assert!(!out.streak_broken);
    }

    #[test]
    fn tenant_weekend_days_are_rest_days() {
        // Thursday -> Sunday with Fri+Sat weekend: nothing missed.
        let out = on_activity(
            &state(5, Some(d(2026, 10, 22)), 0),
            d(2026, 10, 25),
            &KSA_WEEKEND,
            1,
        );
        assert_eq!(out.state.streak_current, 6);
        // Without Saturday as a weekend day, Saturday counts as missed and there is no freeze.
        let out = on_activity(&state(5, Some(d(2026, 10, 22)), 0), d(2026, 10, 25), &[], 0);
        assert_eq!(out.state.streak_current, 1);
        assert!(out.streak_broken);
    }

    #[test]
    fn a_missed_working_day_consumes_a_freeze() {
        // Monday -> Wednesday, Tuesday missed, one freeze available in the same week.
        let out = on_activity(
            &state(7, Some(d(2026, 10, 19)), 1),
            d(2026, 10, 21),
            &KSA_WEEKEND,
            1,
        );
        assert_eq!(out.state.streak_current, 8);
        assert_eq!(out.freezes_used, 1);
        assert_eq!(out.state.freezes_available, 0);
    }

    #[test]
    fn two_missed_days_with_one_freeze_break_the_streak() {
        let out = on_activity(
            &state(7, Some(d(2026, 10, 19)), 1),
            d(2026, 10, 22),
            &KSA_WEEKEND,
            1,
        );
        assert_eq!(out.state.streak_current, 1);
        assert!(out.streak_broken);
        assert_eq!(
            out.state.freezes_available, 1,
            "freeze is kept when it cannot save the streak"
        );
        assert_eq!(out.state.streak_longest, 7);
    }

    #[test]
    fn freezes_replenish_each_iso_week() {
        // Last active Thursday 2026-10-15 with 0 freezes; next Monday 2026-10-19 is a new week.
        let s = state(2, Some(d(2026, 10, 15)), 0);
        assert_eq!(replenish_freezes(&s, d(2026, 10, 19), 1), 1);
        assert_eq!(replenish_freezes(&s, d(2026, 10, 16), 1), 0);
        // Fri/Sat are rest days, Sunday 18 missed -> uses the replenished freeze on Monday 19.
        let out = on_activity(&s, d(2026, 10, 19), &KSA_WEEKEND, 1);
        assert_eq!(out.state.streak_current, 3);
        assert_eq!(out.freezes_used, 1);
    }

    #[test]
    fn parses_weekend_day_names() {
        let days = parse_weekend_days(&["fri".into(), "Saturday".into(), "nope".into()]);
        assert_eq!(days, vec![Weekday::Fri, Weekday::Sat]);
    }
}
