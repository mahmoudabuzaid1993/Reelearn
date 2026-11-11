//! Reelearn domain: pure types and rules, no I/O.
//!
//! Everything here is deterministic and unit-tested. The `db` crate persists
//! these types, the `api` crate maps them to HTTP, the `worker` runs them in jobs.

pub mod ai;
pub mod cursor;
pub mod engine;
pub mod enums;
pub mod ids;
pub mod phone;
pub mod plan;
pub mod pricing;
pub mod recall;
pub mod streak;

pub use enums::*;
pub use ids::*;
