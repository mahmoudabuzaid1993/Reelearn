//! Token pair issuance and refresh rotation.

use crate::crypto::{random_token, sha256_hex};
use crate::dto::{Me, TokenPair};
use crate::error::ApiError;
use crate::state::AppState;
use reelearn_db::repos::{auth as auth_repo, users::UserRow};
use reelearn_db::TenantTx;

pub struct Issued {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

/// Issue an access + refresh pair for a user inside an open tenant transaction.
pub async fn issue_pair(
    state: &AppState,
    tx: &mut TenantTx,
    user: &UserRow,
    device_id: Option<&str>,
) -> Result<Issued, ApiError> {
    let tenant_id = tx.tenant_id();
    let (access_token, _claims) = state
        .jwt
        .issue(user.id, tenant_id, user.role)
        .map_err(ApiError::internal)?;
    let refresh_token = random_token();
    auth_repo::insert_refresh_token(
        tx.conn(),
        tenant_id,
        user.id,
        device_id,
        &sha256_hex(refresh_token.as_bytes()),
        state.config.refresh_ttl_days,
    )
    .await?;
    Ok(Issued {
        access_token,
        refresh_token,
        expires_in: state.jwt.ttl_seconds(),
    })
}

pub fn token_pair(issued: Issued, me: Me) -> TokenPair {
    TokenPair {
        access_token: issued.access_token,
        refresh_token: issued.refresh_token,
        expires_in: issued.expires_in,
        user: me,
    }
}
