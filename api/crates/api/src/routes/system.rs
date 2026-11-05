use crate::dto::Health;
use crate::routes::Json;
use crate::state::AppState;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

/// Served outside `/v1`: the static contract and a Swagger UI page.
pub fn docs_routes() -> Router<AppState> {
    Router::new()
        .route("/openapi.yaml", get(openapi))
        .route("/docs", get(docs))
}

async fn health(State(state): State<AppState>) -> Json<Health> {
    let postgres = state.db.ping().await;
    let redis = state.cache.ping().await;
    Json(Health {
        status: if postgres && redis { "ok" } else { "degraded" },
        postgres,
        redis,
        version: state.config.version.clone(),
    })
}

async fn openapi(State(state): State<AppState>) -> Response {
    match state.openapi.as_ref() {
        Some(yaml) => ([(header::CONTENT_TYPE, "application/yaml")], yaml.clone()).into_response(),
        None => crate::error::ApiError::NotFound(
            "openapi.yaml is not available on this server (set OPENAPI_PATH)".into(),
        )
        .into_response(),
    }
}

async fn docs() -> impl IntoResponse {
    (
        StatusCode::OK,
        Html(
            r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Reelearn API</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.17.14/swagger-ui.css">
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5.17.14/swagger-ui-bundle.js" crossorigin></script>
  <script>
    window.ui = SwaggerUIBundle({ url: "/openapi.yaml", dom_id: "#swagger-ui", persistAuthorization: true });
  </script>
</body>
</html>"##,
        ),
    )
}
