//! The request id travels in `x-request-id` (set by tower-http) and in a task-local so
//! `ApiError` can stamp it onto problem+json bodies without threading it through handlers.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

tokio::task_local! {
    static REQUEST_ID: String;
}

pub const HEADER: &str = "x-request-id";

pub fn current_request_id() -> Option<String> {
    REQUEST_ID.try_with(|id| id.clone()).ok()
}

pub async fn scope_request_id(req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get(HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
    REQUEST_ID.scope(id, next.run(req)).await
}
