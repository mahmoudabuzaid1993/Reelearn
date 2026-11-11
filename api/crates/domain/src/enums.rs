//! Enums mirroring the Postgres enum types in `db/migrations/0001_init.sql`.
//!
//! Serde names are the snake_case wire names from `openapi/openapi.yaml`; with the
//! `sqlx` feature the same values map onto the database enum types.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, thiserror::Error)]
#[error("unknown {kind} value: {value}")]
pub struct ParseEnumError {
    pub kind: &'static str,
    pub value: String,
}

macro_rules! wire_enum {
    ($(#[$meta:meta])* $name:ident, $pg:literal, { $($variant:ident => $wire:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
        #[cfg_attr(feature = "sqlx", sqlx(type_name = $pg, rename_all = "snake_case"))]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn as_str(&self) -> &'static str {
                match self {
                    $($name::$variant => $wire),+
                }
            }

            /// The Postgres enum type name, used for explicit casts in SQL.
            pub const PG_TYPE: &'static str = $pg;
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = ParseEnumError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($wire => Ok($name::$variant),)+
                    other => Err(ParseEnumError { kind: stringify!($name), value: other.to_string() }),
                }
            }
        }
    };
}

wire_enum!(Region, "region_t", { Ksa => "ksa", Uae => "uae", Eu => "eu" });

wire_enum!(Role, "user_role_t", {
    Admin => "admin", Manager => "manager", Sme => "sme", Learner => "learner",
});

impl Role {
    /// Privilege rank used by route gating: admin > manager > sme > learner.
    pub fn rank(&self) -> u8 {
        match self {
            Role::Admin => 3,
            Role::Manager => 2,
            Role::Sme => 1,
            Role::Learner => 0,
        }
    }

    pub fn at_least(&self, min: Role) -> bool {
        self.rank() >= min.rank()
    }

    /// Which approval role a reviewer acts as when signing off a reel.
    /// Admins are the L&D reviewer; SMEs and managers sign as subject-matter experts.
    pub fn approval_role(&self) -> Option<ApprovalRole> {
        match self {
            Role::Admin => Some(ApprovalRole::Ld),
            Role::Sme | Role::Manager => Some(ApprovalRole::Sme),
            Role::Learner => None,
        }
    }
}

wire_enum!(UserStatus, "user_status_t", {
    Invited => "invited", Active => "active", Paused => "paused", Deactivated => "deactivated",
});

wire_enum!(Channel, "channel_t", {
    App => "app", Whatsapp => "whatsapp", Both => "both", Teams => "teams",
});

impl Channel {
    pub fn includes_whatsapp(&self) -> bool {
        matches!(self, Channel::Whatsapp | Channel::Both)
    }
}

wire_enum!(DeliverySlot, "delivery_slot_t", {
    ShiftStart => "shift_start", Break => "break", ShiftEnd => "shift_end",
});

wire_enum!(SourceKind, "source_kind_t", {
    Pdf => "pdf", Pptx => "pptx", Image => "image", Url => "url", Youtube => "youtube",
    TopicPrompt => "topic_prompt", PodcastAudio => "podcast_audio", Video => "video",
    SmeRecording => "sme_recording",
});

impl SourceKind {
    /// Podcasts and talks go through transcription and moment finding rather than text extraction.
    pub fn is_audio_visual(&self) -> bool {
        matches!(self, SourceKind::PodcastAudio | SourceKind::Video)
    }
}

wire_enum!(ExtractionStatus, "extraction_status_t", {
    Pending => "pending", Running => "running", Done => "done", Failed => "failed",
});

wire_enum!(JobStatus, "job_status_t", {
    Queued => "queued", Running => "running", Paused => "paused", Done => "done",
    Failed => "failed", Cancelled => "cancelled",
});

impl JobStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobStatus::Done | JobStatus::Failed | JobStatus::Cancelled
        )
    }
}

wire_enum!(GenStep, "gen_step_t", {
    Extract => "extract", Script => "script", Render => "render", Voice => "voice",
    Caption => "caption", Question => "question", Translate => "translate",
    Transcribe => "transcribe", FindMoments => "find_moments", Cut => "cut",
});

impl GenStep {
    /// The durable job kind (`jobs.kind`) that runs this step.
    pub fn job_kind(&self) -> &'static str {
        match self {
            GenStep::Extract => "extract_source",
            GenStep::Script => "script_reels",
            GenStep::Render => "render_reel",
            GenStep::Voice => "voice_reel",
            GenStep::Caption => "caption_reel",
            GenStep::Question => "write_question",
            GenStep::Translate => "translate_reel",
            GenStep::Transcribe => "transcribe_podcast",
            GenStep::FindMoments => "find_moments",
            GenStep::Cut => "cut_moment",
        }
    }

    /// The ordered steps of a full "source to reels" generation.
    pub fn full_pipeline(extra_languages: bool) -> Vec<GenStep> {
        let mut steps = vec![
            GenStep::Extract,
            GenStep::Script,
            GenStep::Question,
            GenStep::Render,
            GenStep::Voice,
            GenStep::Caption,
        ];
        if extra_languages {
            steps.push(GenStep::Translate);
        }
        steps
    }
}

wire_enum!(ReelKind, "reel_kind_t", {
    AiAvatar => "ai_avatar", Motion => "motion", Broll => "broll",
    PodcastCut => "podcast_cut", SmeRecorded => "sme_recorded",
});

wire_enum!(ReelStatus, "reel_status_t", {
    Draft => "draft", InReview => "in_review", Approved => "approved",
    Published => "published", Archived => "archived", Rejected => "rejected",
});

wire_enum!(ApprovalRole, "approval_role_t", { Ld => "ld", Sme => "sme" });

wire_enum!(Decision, "decision_t", {
    Approved => "approved", ChangesRequested => "changes_requested", Rejected => "rejected",
});

wire_enum!(PlanItemKind, "plan_item_kind_t", { Reel => "reel", QuestionOnly => "question_only" });

wire_enum!(PlanItemReason, "plan_item_reason_t", { New => "new", Review => "review" });

wire_enum!(
    /// A signal from the phone or the ranker. See `engine::signals` for what each one is worth.
    EngagementKind, "engagement_kind_t", {
    Impression => "impression", Start => "start", Skip => "skip", Complete => "complete",
    Rewatch => "rewatch", Save => "save", Unsave => "unsave", Share => "share",
    NotRelevant => "not_relevant", MoreLikeThis => "more_like_this",
});

wire_enum!(PlanItemState, "plan_item_state_t", {
    Pending => "pending", Watched => "watched", Answered => "answered",
});

wire_enum!(DeliveryStatus, "delivery_status_t", {
    Scheduled => "scheduled", Sent => "sent", Delivered => "delivered", Read => "read",
    Failed => "failed", Cancelled => "cancelled",
});

wire_enum!(IntegrationKind, "integration_kind_t", {
    HrisJisr => "hris_jisr", HrisBayzat => "hris_bayzat", SsoEntra => "sso_entra",
    Whatsapp => "whatsapp", Teams => "teams", AiClaude => "ai_claude", AiMuse => "ai_muse",
    AiHiggsfield => "ai_higgsfield", AiVeo => "ai_veo", Voice => "voice", Asr => "asr",
    Storage => "storage", AiTypesafe => "ai_typesafe",
});

impl IntegrationKind {
    /// Config keys that are secrets: write-only, encrypted at rest, never echoed back.
    pub fn secret_keys(&self) -> &'static [&'static str] {
        &[
            "api_key",
            "access_token",
            "token",
            "secret",
            "client_secret",
            "app_secret",
            "password",
            "secret_key",
            "private_key",
        ]
    }

    /// Keys that must be present for a connection test to pass.
    pub fn required_keys(&self) -> &'static [&'static str] {
        match self {
            IntegrationKind::Whatsapp => &["phone_number_id", "access_token"],
            IntegrationKind::AiClaude
            | IntegrationKind::AiMuse
            | IntegrationKind::AiTypesafe
            | IntegrationKind::AiHiggsfield
            | IntegrationKind::AiVeo
            | IntegrationKind::Voice
            | IntegrationKind::Asr => &["api_key"],
            IntegrationKind::SsoEntra => &["tenant_id", "client_id", "client_secret"],
            IntegrationKind::HrisJisr | IntegrationKind::HrisBayzat => &["api_key"],
            IntegrationKind::Teams => &["tenant_id", "client_id", "client_secret"],
            IntegrationKind::Storage => &["bucket", "access_key", "secret_key"],
        }
    }
}

wire_enum!(ExportKind, "export_kind_t", { HrdfEvidence => "hrdf_evidence", PersonalData => "personal_data" });

/// Generation style requested by the studio; maps onto the reel kind that is produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStyle {
    AiAvatar,
    Motion,
    Broll,
}

impl GenerationStyle {
    pub fn reel_kind(&self) -> ReelKind {
        match self {
            GenerationStyle::AiAvatar => ReelKind::AiAvatar,
            GenerationStyle::Motion => ReelKind::Motion,
            GenerationStyle::Broll => ReelKind::Broll,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_round_trip_through_wire_names() {
        for r in Role::ALL {
            assert_eq!(Role::from_str(r.as_str()).ok(), Some(*r));
            let json = serde_json::to_string(r).expect("serialize");
            assert_eq!(json, format!("\"{}\"", r.as_str()));
        }
        for s in ReelStatus::ALL {
            assert_eq!(ReelStatus::from_str(s.as_str()).ok(), Some(*s));
        }
        assert!(GenStep::from_str("find_moments").is_ok());
        assert!(Role::from_str("root").is_err());
    }

    #[test]
    fn role_ranks_gate_routes() {
        assert!(Role::Admin.at_least(Role::Manager));
        assert!(Role::Manager.at_least(Role::Sme));
        assert!(!Role::Sme.at_least(Role::Manager));
        assert!(!Role::Learner.at_least(Role::Sme));
        assert_eq!(Role::Admin.approval_role(), Some(ApprovalRole::Ld));
        assert_eq!(Role::Sme.approval_role(), Some(ApprovalRole::Sme));
        assert_eq!(Role::Learner.approval_role(), None);
    }

    #[test]
    fn pipeline_steps_have_job_kinds() {
        let steps = GenStep::full_pipeline(true);
        assert_eq!(steps.len(), 7);
        assert_eq!(steps[0].job_kind(), "extract_source");
        assert_eq!(steps.last().map(|s| s.job_kind()), Some("translate_reel"));
        assert_eq!(GenStep::full_pipeline(false).len(), 6);
    }
}
