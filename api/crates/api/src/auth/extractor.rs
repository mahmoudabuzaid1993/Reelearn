//! `AuthUser`: validates the Bearer token, checks revocation, exposes tenant/user/role.

use crate::cache::keys;
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use reelearn_domain::Role;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub role: Role,
    pub jti: String,
    pub exp: i64,
}

impl AuthUser {
    pub fn require(&self, min: Role) -> Result<(), ApiError> {
        if self.role.at_least(min) {
            Ok(())
        } else {
            Err(ApiError::Forbidden(format!("this needs the {} role", min)))
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(user) = parts.extensions.get::<AuthUser>() {
            return Ok(user.clone());
        }
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::Unauthorized("missing bearer token".into()))?;
        let token = header
            .strip_prefix("Bearer ")
            .or_else(|| header.strip_prefix("bearer "))
            .ok_or_else(|| ApiError::Unauthorized("missing bearer token".into()))?;
        let claims = state
            .jwt
            .verify(token.trim())
            .map_err(|e| ApiError::Unauthorized(e.to_string()))?;
        if state.cache.exists(&keys::revoked(&claims.jti)).await {
            return Err(ApiError::Unauthorized("token has been revoked".into()));
        }
        let user = AuthUser {
            user_id: claims.sub,
            tenant_id: claims.tenant_id,
            role: claims.role,
            jti: claims.jti,
            exp: claims.exp,
        };
        parts.extensions.insert(user.clone());
        Ok(user)
    }
}
