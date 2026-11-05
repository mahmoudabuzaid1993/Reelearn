//! `tenants.settings` jsonb, with the contract's defaults.

use chrono::{DateTime, NaiveDate, Utc};
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
        }
    }
}

impl TenantSettings {
    pub fn from_json(v: &serde_json::Value) -> TenantSettings {
        serde_json::from_value(v.clone()).unwrap_or_default()
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
