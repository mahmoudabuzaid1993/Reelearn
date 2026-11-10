//! The practice budget: whether "keep going" is open, how long is left today and in this
//! session, and how often to suggest a pause. Caps of 0 mean "no cap".

use super::settings::EngineSettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    /// Practice time today, across sessions.
    pub today_ms: i64,
    /// Practice time in the current session.
    pub session_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The company has practice switched off.
    Disabled,
    DailyCap,
    SessionCap,
    OutsideWorkHours,
    /// The person paused their reminders and practice.
    Paused,
    /// Nothing left to show.
    Exhausted,
}

impl StopReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            StopReason::Disabled => "disabled",
            StopReason::DailyCap => "daily_cap",
            StopReason::SessionCap => "session_cap",
            StopReason::OutsideWorkHours => "outside_work_hours",
            StopReason::Paused => "paused",
            StopReason::Exhausted => "exhausted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    pub open: bool,
    pub stop: Option<StopReason>,
    pub remaining_today_s: Option<i64>,
    pub remaining_session_s: Option<i64>,
    /// The phone shows a pause card after each this-many seconds of practice.
    pub break_every_s: Option<i64>,
}

pub fn budget(
    settings: &EngineSettings,
    usage: Usage,
    within_work_hours: bool,
    paused: bool,
) -> Budget {
    let remaining = |cap_min: u16, used_ms: i64| -> Option<i64> {
        (cap_min > 0).then(|| (i64::from(cap_min) * 60 - used_ms / 1000).max(0))
    };
    let remaining_today_s = remaining(settings.daily_minutes_cap, usage.today_ms);
    let remaining_session_s = remaining(settings.session_minutes_cap, usage.session_ms);
    let stop = if !settings.continuous_enabled {
        Some(StopReason::Disabled)
    } else if paused {
        Some(StopReason::Paused)
    } else if !within_work_hours && !settings.practice_outside_work_hours {
        Some(StopReason::OutsideWorkHours)
    } else if remaining_today_s == Some(0) {
        Some(StopReason::DailyCap)
    } else if remaining_session_s == Some(0) {
        Some(StopReason::SessionCap)
    } else {
        None
    };
    Budget {
        open: stop.is_none(),
        stop,
        remaining_today_s,
        remaining_session_s,
        break_every_s: (settings.break_reminder_minutes > 0)
            .then(|| i64::from(settings.break_reminder_minutes) * 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Intensity;

    #[test]
    fn gentle_has_no_practice_and_maximum_has_no_caps() {
        let g = budget(
            &EngineSettings::preset(Intensity::Gentle),
            Usage::default(),
            true,
            false,
        );
        assert_eq!(g.stop, Some(StopReason::Disabled));
        let m = budget(
            &EngineSettings::preset(Intensity::Maximum),
            Usage {
                today_ms: 10 * 3_600_000,
                session_ms: 3_600_000,
            },
            false,
            false,
        );
        assert!(m.open);
        assert_eq!(m.remaining_today_s, None);
        assert_eq!(m.break_every_s, None);
    }

    #[test]
    fn caps_close_the_session_in_order() {
        let s = EngineSettings::preset(Intensity::Balanced); // 15 min a day, 10 per session
        let b = budget(
            &s,
            Usage {
                today_ms: 4 * 60_000,
                session_ms: 4 * 60_000,
            },
            true,
            false,
        );
        assert!(b.open);
        assert_eq!(b.remaining_session_s, Some(360));
        assert_eq!(b.remaining_today_s, Some(660));
        assert_eq!(b.break_every_s, Some(600));
        let b = budget(
            &s,
            Usage {
                today_ms: 12 * 60_000,
                session_ms: 10 * 60_000,
            },
            true,
            false,
        );
        assert_eq!(b.stop, Some(StopReason::SessionCap));
        let b = budget(
            &s,
            Usage {
                today_ms: 15 * 60_000,
                session_ms: 0,
            },
            true,
            false,
        );
        assert_eq!(b.stop, Some(StopReason::DailyCap));
    }

    #[test]
    fn work_hours_and_pause_are_respected() {
        let s = EngineSettings::preset(Intensity::Balanced);
        assert_eq!(
            budget(&s, Usage::default(), false, false).stop,
            Some(StopReason::OutsideWorkHours)
        );
        assert_eq!(
            budget(&s, Usage::default(), true, true).stop,
            Some(StopReason::Paused)
        );
        let i = EngineSettings::preset(Intensity::Intense);
        assert!(budget(&i, Usage::default(), false, false).open);
    }
}
