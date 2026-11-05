//! HTTP handlers, one module per contract tag. `router()` wires them under `/v1`
//! with role gating per route group.

pub mod auth;
pub mod learn;
pub mod manager;
pub mod me;
pub mod people;
pub mod reels;
pub mod reports;
pub mod settings;
pub mod sme;
pub mod studio;
pub mod system;
pub mod tenants;
pub mod topics;
pub mod webhooks;

use crate::auth::roles;
use crate::dto::PageQuery;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{FromRequest, FromRequestParts, OptionalFromRequest, Request};
use axum::http::request::Parts;
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::Router;
use chrono::NaiveDate;
use reelearn_db::repos::audit;
use reelearn_db::TenantTx;
use reelearn_domain::cursor::{clamp_limit, Cursor};
use serde::de::DeserializeOwned;
use uuid::Uuid;

/// JSON body extractor/response whose rejection is a problem+json 400.
pub struct Json<T>(pub T);

impl<T: DeserializeOwned, S: Send + Sync> FromRequest<S> for Json<T> {
    type Rejection = ApiError;
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(v) = <axum::Json<T> as FromRequest<S>>::from_request(req, state).await?;
        Ok(Json(v))
    }
}

/// `Option<Json<T>>`: an absent or empty body is `None`; a present body must parse.
impl<T: DeserializeOwned, S: Send + Sync> OptionalFromRequest<S> for Json<T> {
    type Rejection = ApiError;
    async fn from_request(req: Request, _state: &S) -> Result<Option<Self>, Self::Rejection> {
        let bytes = axum::body::to_bytes(req.into_body(), 1 << 20)
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        if bytes.iter().all(|b| b.is_ascii_whitespace()) {
            return Ok(None);
        }
        Ok(Some(Json(serde_json::from_slice(&bytes)?)))
    }
}

impl<T: serde::Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

/// Query extractor whose rejection is a problem+json 400.
pub struct Query<T>(pub T);

impl<T: DeserializeOwned, S: Send + Sync> FromRequestParts<S> for Query<T> {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Query(v) =
            axum::extract::Query::<T>::from_request_parts(parts, state).await?;
        Ok(Query(v))
    }
}

/// Path extractor whose rejection is a problem+json 400.
pub struct Path<T>(pub T);

impl<T: DeserializeOwned + Send, S: Send + Sync> FromRequestParts<S> for Path<T> {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Path(v) =
            axum::extract::Path::<T>::from_request_parts(parts, state).await?;
        Ok(Path(v))
    }
}

pub fn page_params(q: &PageQuery) -> Result<(Option<Cursor>, i64), ApiError> {
    let cursor = match q.cursor.as_deref().filter(|c| !c.is_empty()) {
        Some(c) => Some(
            Cursor::decode(c).map_err(|_| ApiError::BadRequest("cursor is not valid".into()))?,
        ),
        None => None,
    };
    Ok((cursor, clamp_limit(q.limit)))
}

/// `range` query (7d | 30d | 90d | 12m) to an inclusive date window ending today.
pub fn range_window(range: Option<&str>, today: NaiveDate) -> (NaiveDate, NaiveDate, String) {
    let r = range.unwrap_or("30d");
    let days = match r {
        "7d" => 7,
        "90d" => 90,
        "12m" => 365,
        _ => 30,
    };
    let label = if matches!(r, "7d" | "30d" | "90d" | "12m") {
        r
    } else {
        "30d"
    };
    (
        today - chrono::Duration::days(days - 1),
        today,
        label.to_string(),
    )
}

pub async fn record_audit(
    tx: &mut TenantTx,
    actor: Uuid,
    action: &str,
    entity: &str,
    entity_id: Option<Uuid>,
    diff: Option<serde_json::Value>,
) -> Result<(), ApiError> {
    let tenant_id = tx.tenant_id();
    audit::record(
        tx.conn(),
        tenant_id,
        Some(actor),
        action,
        entity,
        entity_id,
        diff,
        None,
    )
    .await?;
    Ok(())
}

/// A contract path that is not implemented yet: a 501 problem, never a 404.
pub async fn not_implemented() -> ApiError {
    ApiError::not_implemented("this endpoint")
}

pub fn router(state: AppState) -> Router<AppState> {
    let public = Router::new()
        .merge(system::routes())
        .merge(auth::public_routes())
        .merge(tenants::public_routes())
        .merge(webhooks::routes());

    let signed_in = Router::new()
        .merge(auth::private_routes())
        .merge(me::routes())
        .merge(learn::routes())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            roles::require_signed_in,
        ));

    let manager = manager::routes().route_layer(middleware::from_fn_with_state(
        state.clone(),
        roles::require_manager,
    ));
    let sme = sme::routes().route_layer(middleware::from_fn_with_state(
        state.clone(),
        roles::require_sme,
    ));
    // Reviewer routes under /admin that SMEs and managers need (sign-off on reels they review).
    let reviewer = reels::reviewer_routes().route_layer(middleware::from_fn_with_state(
        state.clone(),
        roles::require_sme,
    ));

    let admin = Router::new()
        .merge(tenants::admin_routes())
        .merge(people::routes())
        .merge(topics::routes())
        .merge(studio::routes())
        .merge(reels::admin_routes())
        .merge(reports::routes())
        .merge(settings::routes())
        .route_layer(middleware::from_fn_with_state(state, roles::require_admin));

    Router::new()
        .merge(public)
        .merge(signed_in)
        .merge(manager)
        .merge(sme)
        .merge(reviewer)
        .merge(admin)
}
