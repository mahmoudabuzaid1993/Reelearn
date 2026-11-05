use crate::error::DbError;
use sqlx::{PgConnection, Postgres, Transaction};
use uuid::Uuid;

/// A transaction pinned to one tenant.
///
/// On open it runs `SET LOCAL ROLE reelearn_app` (when configured) and
/// `SELECT set_config('app.tenant_id', $1, true)` — the parameterised form of
/// `SET LOCAL app.tenant_id = $1` — so every row-level-security policy
/// (`tenant_id = current_setting('app.tenant_id')::uuid`) is enforced for the
/// rest of the transaction. Both settings end with the transaction.
pub struct TenantTx {
    tx: Transaction<'static, Postgres>,
    tenant_id: Uuid,
}

impl TenantTx {
    pub(crate) async fn open(
        mut tx: Transaction<'static, Postgres>,
        tenant_id: Uuid,
        app_role: Option<&str>,
    ) -> Result<TenantTx, DbError> {
        if let Some(role) = app_role {
            if !role.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(DbError::Invalid(format!(
                    "app role name {role:?} is not a plain identifier"
                )));
            }
            sqlx::query(&format!("SET LOCAL ROLE {role}"))
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
            .bind(tenant_id.to_string())
            .execute(&mut *tx)
            .await?;
        Ok(TenantTx { tx, tenant_id })
    }

    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    /// The connection to hand to repository functions.
    pub fn conn(&mut self) -> &mut PgConnection {
        &mut self.tx
    }

    pub async fn commit(self) -> Result<(), DbError> {
        self.tx.commit().await?;
        Ok(())
    }

    pub async fn rollback(self) -> Result<(), DbError> {
        self.tx.rollback().await?;
        Ok(())
    }
}
