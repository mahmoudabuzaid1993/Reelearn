//! The learning engine: how the next reels are chosen for a person.
//!
//! It follows the published structure of short-video recommenders (Meta's Reels, TikTok,
//! YouTube; see `docs/engine.md` and `docs/research/short-video-recommenders.md`):
//!
//! 1. **Candidates** come from several sources (assigned topics, the open catalogue, colleague
//!    shares, reviews that are due) and are loaded by the `db` crate.
//! 2. **Predictions** per candidate: duration-normalised completion, quick skip, save, share,
//!    "not for my job", and the chance of a correct answer ([`rank::predict`]).
//! 3. **A value model** turns predictions into one score with weights a company tunes
//!    ([`settings::Weights`]); negative predictions are subtracted.
//! 4. **Re-ranking** adds exploration of under-seen reels, topic diversity and interleaved
//!    reviews ([`rank::rank`]).
//!
//! Interest per topic and tag is a time-decayed score that is not normalised away, so ten
//! completions in a topic count more than one ([`signals`]). A session budget enforces the
//! company's caps and break reminders ([`budget`]). Everything here is pure and deterministic.

pub mod budget;
pub mod rank;
pub mod rng;
pub mod settings;
pub mod signals;

pub use budget::{budget, Budget, StopReason, Usage};
pub use rank::{
    predict, rank, score, BandBaseline, ItemStats, Predictions, RankCandidate, RankContext,
    RankReason, Ranked, ScoreBreakdown, UserProfile,
};
pub use settings::{EngineSettings, FieldError, Intensity, Weights};
pub use signals::{band_of, decayed, Preference, DURATION_BANDS, SKIP_THRESHOLD_MS};
