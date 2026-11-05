//! Router assembly and the middleware stack.

use crate::etag::etag_layer;
use crate::request_id::scope_request_id;
use crate::routes;
use crate::state::AppState;
use axum::http::{header, HeaderName, Method, StatusCode};
use axum::middleware;
use axum::Router;
use std::time::Duration;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub fn build_router(state: AppState) -> Router {
    let cors = if state.config.cors_origins.is_empty() {
        CorsLayer::new().allow_origin(AllowOrigin::mirror_request())
    } else {
        let origins: Vec<_> = state
            .config
            .cors_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        CorsLayer::new().allow_origin(AllowOrigin::list(origins))
    }
    .allow_methods([
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
        Method::OPTIONS,
    ])
    .allow_headers([
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::IF_NONE_MATCH,
        HeaderName::from_static("idempotency-key"),
        HeaderName::from_static("x-device-id"),
    ])
    .expose_headers([
        header::ETAG,
        HeaderName::from_static("x-request-id"),
        HeaderName::from_static("idempotent-replayed"),
    ])
    .max_age(Duration::from_secs(3600));

    let request_id = HeaderName::from_static("x-request-id");

    // Everything except SSE gets the request timeout.
    let timed = routes::router(state.clone()).layer(TimeoutLayer::with_status_code(
        StatusCode::REQUEST_TIMEOUT,
        REQUEST_TIMEOUT,
    ));
    let sse = routes::studio::sse_routes().route_layer(middleware::from_fn_with_state(
        state.clone(),
        crate::auth::roles::require_admin,
    ));
    let v1 = timed.merge(sse);

    Router::new()
        .nest("/v1", v1)
        .merge(routes::system::docs_routes())
        .layer(middleware::from_fn(etag_layer))
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(middleware::from_fn(scope_request_id))
        .layer(TraceLayer::new_for_http())
        .layer(PropagateRequestIdLayer::new(request_id.clone()))
        .layer(SetRequestIdLayer::new(request_id, MakeRequestUuid))
        .with_state(state)
}
