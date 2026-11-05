//! Route-group gating: `/admin/*` admin only, `/manager/*` manager+admin,
//! `/sme/*` sme+manager+admin, `/me/*` any signed-in user.

use crate::auth::AuthUser;
use crate::error::ApiError;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use reelearn_domain::Role;

pub async fn require_admin(user: AuthUser, req: Request, next: Next) -> Result<Response, ApiError> {
    user.require(Role::Admin)?;
    Ok(next.run(req).await)
}

pub async fn require_manager(
    user: AuthUser,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    user.require(Role::Manager)?;
    Ok(next.run(req).await)
}

pub async fn require_sme(user: AuthUser, req: Request, next: Next) -> Result<Response, ApiError> {
    user.require(Role::Sme)?;
    Ok(next.run(req).await)
}

pub async fn require_signed_in(
    _user: AuthUser,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    Ok(next.run(req).await)
}
