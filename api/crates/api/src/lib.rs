//! Reelearn HTTP API.
//!
//! `app::build_router` assembles the `/v1` routes with the middleware stack
//! (request id, tracing, CORS, compression, timeout, problem+json errors, ETags).
//! Handlers live in `routes::*`, map HTTP to `services::*` and the `db` repositories,
//! and return `Result<_, ApiError>`.

pub mod app;
pub mod auth;
pub mod cache;
pub mod config;
pub mod crypto;
pub mod dto;
pub mod error;
pub mod etag;
pub mod idempotency;
pub mod request_id;
pub mod routes;
pub mod s3;
pub mod seed;
pub mod services;
pub mod state;

pub use app::build_router;
pub use config::Config;
pub use error::ApiError;
pub use state::AppState;

pub type ApiResult<T> = Result<T, ApiError>;
