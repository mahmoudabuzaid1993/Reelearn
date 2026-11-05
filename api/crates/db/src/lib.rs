//! Postgres access for Reelearn.
//!
//! - [`Db`] owns the pool and runs migrations from `db/migrations`.
//! - [`TenantTx`] is the only way to touch tenant rows: it opens a transaction,
//!   drops into the `reelearn_app` role and sets `app.tenant_id`, so the FORCE'd
//!   row-level-security policies isolate the tenant even if a query forgets `WHERE tenant_id`.
//! - `repos::*` hold the only SQL in the workspace, one module per aggregate.
//! - [`plans`] materialises the daily plan (the orchestration shared by API and worker).

pub mod error;
pub mod plans;
pub mod pool;
pub mod repos;
pub mod settings;
pub mod tenant_tx;

pub use error::DbError;
pub use pool::{Db, MIGRATOR};
pub use settings::TenantSettings;
pub use tenant_tx::TenantTx;

pub type DbResult<T> = Result<T, DbError>;
