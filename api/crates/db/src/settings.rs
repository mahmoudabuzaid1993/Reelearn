//! `tenants.settings` jsonb, with the contract's defaults.

use chrono::{DateTime, NaiveDate, Utc};
use reelearn_domain::ai::AiRouting;
use reelearn_domain::engine::EngineSettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeliveryWindow {
    #[serde(default)]
    pub days: Vec<String>,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TenantSettings {
    pub reels_per_day: i32,
    pub max_reviews_per_day: i32,
    pub delivery_windows: Vec<DeliveryWindow>,
    pub prayer_aware: bool,
    pub ramadan_aware: bool,
    pub weekend_days: Vec<String>,
    pub retention_months: i32,
    pub streak_freezes_per_week: i32,
    /// Local hour at which the scheduler builds the day's plans.
    pub plan_build_hour: u32,
    /// Local hour of the default WhatsApp delivery (shift start).
    pub delivery_hour: u32,
    /// The learning engine: intensity, value-model weights, practice caps, reminders.
    pub engine: EngineSettings,
    /// Which AI provider writes and whether TypeSafe Jev checks drafts and matches roles.
    pub ai: AiRouting,
}

impl Default for TenantSettings {
    fn default() -> Self {
        TenantSettings {
            reels_per_day: 2,
            max_reviews_per_day: 1,
            delivery_windows: vec![DeliveryWindow {
                days: vec![
                    "sun".into(),
                    "mon".into(),
                    "tue".into(),
                    "wed".into(),
                    "thu".into(),
                ],
                start: "08:00".into(),
                end: "17:00".into(),
            }],
            prayer_aware: true,
            ramadan_aware: true,
            weekend_days: vec!["fri".into(), "sat".into()],
            retention_months: 36,
            streak_freezes_per_week: 1,
            plan_build_hour: 4,
            delivery_hour: 8,
            engine: EngineSettings::default(),
            ai: AiRouting::default(),
        }
    }
}

impl TenantSettings {
    pub fn from_json(v: &serde_json::Value) -> TenantSettings {
        let mut s: TenantSettings = serde_json::from_value(v.clone()).unwrap_or_default();
        s.engine = s.engine.sanitized();
        s.ai = s.ai.sanitized();
        s
    }

    /// Whether `now` (UTC) falls inside one of the delivery windows, in the tenant's zone.
    /// No windows configured means "always".
    pub fn within_windows(&self, timezone: &str, now: DateTime<Utc>) -> bool {
        if self.delivery_windows.is_empty() {
            return true;
        }
        let local = local_now(timezone, now);
        let day = local.format("%a").to_string().to_lowercase();
        let hm = local.format("%H:%M").to_string();
        self.delivery_windows.iter().any(|w| {
            (w.days.is_empty() || w.days.iter().any(|d| d.eq_ignore_ascii_case(&day)))
                && (w.start.is_empty() || hm.as_str() >= w.start.as_str())
                && (w.end.is_empty() || hm.as_str() < w.end.as_str())
        })
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}))
    }

    /// Merge a partial patch (only the keys present) over the current settings.
    pub fn merge_patch(&self, patch: &serde_json::Value) -> TenantSettings {
        let mut base = self.to_json();
        if let (Some(b), Some(p)) = (base.as_object_mut(), patch.as_object()) {
            for (k, v) in p {
                if !v.is_null() {
                    b.insert(k.clone(), v.clone());
                }
            }
        }
        TenantSettings::from_json(&base)
    }
}

/// Today's date in the tenant's timezone (falls back to UTC on an unknown zone).
pub fn local_today(timezone: &str, now: DateTime<Utc>) -> NaiveDate {
    local_now(timezone, now).date()
}

/// Midnight of `day` in the tenant's zone, as a UTC instant (UTC on an unknown zone).
pub fn local_day_start(timezone: &str, day: NaiveDate) -> DateTime<Utc> {
    use chrono::TimeZone;
    let midnight = day.and_hms_opt(0, 0, 0).unwrap_or_default();
    match timezone.parse::<chrono_tz::Tz>() {
        Ok(tz) => tz
            .from_local_datetime(&midnight)
            .earliest()
            .map(|t| t.with_timezone(&Utc))
            .unwrap_or_else(|| Utc.from_utc_datetime(&midnight)),
        Err(_) => Utc.from_utc_datetime(&midnight),
    }
}

pub fn local_now(timezone: &str, now: DateTime<Utc>) -> chrono::NaiveDateTime {
    match timezone.parse::<chrono_tz::Tz>() {
        Ok(tz) => now.with_timezone(&tz).naive_local(),
        Err(_) => now.naive_utc(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_contract() {
        let s = TenantSettings::from_json(&serde_json::json!({}));
        assert_eq!(s.reels_per_day, 2);
        assert_eq!(s.max_reviews_per_day, 1);
        assert_eq!(s.weekend_days, vec!["fri", "sat"]);
        assert_eq!(s.retention_months, 36);
        assert_eq!(s.engine, EngineSettings::default());
    }

    #[test]
    fn delivery_windows_use_the_tenant_zone() {
        let s = TenantSettings::default(); // sun-thu 08:00-17:00, Riyadh
        let tue_10_riyadh = DateTime::parse_from_rfc3339("2026-11-10T07:00:00Z")
            .expect("ts")
            .with_timezone(&Utc);
        let tue_20_riyadh = DateTime::parse_from_rfc3339("2026-11-10T17:00:00Z")
            .expect("ts")
            .with_timezone(&Utc);
        let fri_10_riyadh = DateTime::parse_from_rfc3339("2026-11-13T07:00:00Z")
            .expect("ts")
            .with_timezone(&Utc);
        assert!(s.within_windows("Asia/Riyadh", tue_10_riyadh));
        assert!(!s.within_windows("Asia/Riyadh", tue_20_riyadh));
        assert!(!s.within_windows("Asia/Riyadh", fri_10_riyadh));
    }

    #[test]
    fn patch_merges_only_present_keys() {
        let s = TenantSettings::default();
        let merged = s.merge_patch(&serde_json::json!({"reels_per_day": 3, "weekend_days": null}));
        assert_eq!(merged.reels_per_day, 3);
        assert_eq!(merged.weekend_days, vec!["fri", "sat"]);
    }

    #[test]
    fn riyadh_date_is_ahead_of_utc_late_at_night() {
        let now = DateTime::parse_from_rfc3339("2026-10-29T22:30:00Z")
            .expect("ts")
            .with_timezone(&Utc);
        assert_eq!(local_today("Asia/Riyadh", now).to_string(), "2026-10-30");
        assert_eq!(
            local_today("Nowhere/Invalid", now).to_string(),
            "2026-10-29"
        );
    }
}
