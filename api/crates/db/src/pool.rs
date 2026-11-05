use crate::error::DbError;
use crate::tenant_tx::TenantTx;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Postgres, Transaction};
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

/// Migrations live in the repository's `db/migrations` folder (outside this crate).
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../../db/migrations");

/// Default role tenant transactions drop into. Created by `0002_roles.sql`.
pub const DEFAULT_APP_ROLE: &str = "reelearn_app";

#[derive(Clone)]
pub struct Db {
    pool: PgPool,
    /// Role to `SET LOCAL ROLE` to inside tenant transactions. `None` keeps the
    /// connection role (only sensible when the pool already connects as `reelearn_app`).
    app_role: Option<String>,
}

impl std::fmt::Debug for Db {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Db")
            .field("app_role", &self.app_role)
            .finish()
    }
}

impl Db {
    pub async fn connect(database_url: &str, max_connections: u32) -> Result<Db, DbError> {
        let options = PgConnectOptions::from_str(database_url).map_err(DbError::Sqlx)?;
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(options)
            .await?;
        Ok(Db {
            pool,
            app_role: Some(DEFAULT_APP_ROLE.to_string()),
        })
    }

    /// Wrap an existing pool (used by `#[sqlx::test]`).
    pub fn from_pool(pool: PgPool) -> Db {
        Db {
            pool,
            app_role: Some(DEFAULT_APP_ROLE.to_string()),
        }
    }

    pub fn with_app_role(mut self, role: Option<String>) -> Db {
        self.app_role = role;
        self
    }

    pub fn app_role(&self) -> Option<&str> {
        self.app_role.as_deref()
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn migrate(&self) -> Result<(), DbError> {
        MIGRATOR.run(&self.pool).await?;
        Ok(())
    }

    pub async fn ping(&self) -> bool {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .is_ok()
    }

    /// A tenant-scoped transaction: RLS applies to every statement inside it.
    pub async fn tenant_tx(&self, tenant_id: Uuid) -> Result<TenantTx, DbError> {
        let tx = self.pool.begin().await?;
        TenantTx::open(tx, tenant_id, self.app_role.as_deref()).await
    }

    /// A transaction in the connection role (BYPASSRLS / superuser in dev) for the
    /// few cross-tenant lookups: slug and challenge resolution, the job queue, the scheduler.
    pub async fn admin_tx(&self) -> Result<Transaction<'static, Postgres>, DbError> {
        Ok(self.pool.begin().await?)
    }
}
