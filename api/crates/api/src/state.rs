use crate::auth::jwt::Jwt;
use crate::cache::Cache;
use crate::config::Config;
use crate::crypto::SecretBox;
use crate::s3::Storage;
use reelearn_db::Db;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Db,
    pub cache: Cache,
    pub jwt: Arc<Jwt>,
    pub storage: Arc<Storage>,
    pub secrets: Arc<SecretBox>,
    pub http: reqwest::Client,
    pub openapi: Arc<Option<String>>,
}

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error(transparent)]
    Db(#[from] reelearn_db::DbError),
    #[error(transparent)]
    Jwt(#[from] crate::auth::jwt::JwtError),
    #[error(transparent)]
    Storage(#[from] crate::s3::StorageError),
    #[error(transparent)]
    Crypto(#[from] crate::crypto::CryptoError),
}

impl AppState {
    /// Production path: connect to Postgres (and migrate), Redis and object storage.
    pub async fn connect(config: Config) -> Result<AppState, StateError> {
        let db = Db::connect(&config.database_url, config.database_max_connections)
            .await?
            .with_app_role(config.db_app_role.clone());
        db.migrate().await?;
        let cache = Cache::connect(&config.redis_url, "").await;
        let state = AppState::assemble(config, db, cache)?;
        if let Err(e) = state.storage.ensure_bucket(&state.http).await {
            tracing::warn!(error = %e, "object storage bucket not ready; uploads will fail until it is");
        }
        Ok(state)
    }

    /// Test path: an existing pool (from `#[sqlx::test]`), a key-prefixed Redis namespace.
    pub async fn for_tests(
        config: Config,
        pool: sqlx::PgPool,
        prefix: &str,
    ) -> Result<AppState, StateError> {
        let db = Db::from_pool(pool).with_app_role(config.db_app_role.clone());
        let cache = Cache::connect(&config.redis_url, prefix).await;
        AppState::assemble(config, db, cache)
    }

    fn assemble(config: Config, db: Db, cache: Cache) -> Result<AppState, StateError> {
        let jwt = Jwt::new(&config.jwt, &config.jwt_issuer, config.access_ttl_seconds)?;
        let storage = Storage::new(
            &config.s3_endpoint,
            &config.s3_public_endpoint,
            &config.s3_bucket,
            &config.s3_region,
            &config.s3_access_key,
            &config.s3_secret_key,
        )?;
        let secrets = SecretBox::from_hex(&config.secrets_key_hex)?;
        let openapi = config
            .openapi_candidates()
            .iter()
            .find_map(|p| std::fs::read_to_string(p).ok());
        if openapi.is_none() {
            tracing::warn!(
                "openapi.yaml not found; GET /openapi.yaml will return 404 (set OPENAPI_PATH)"
            );
        }
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .unwrap_or_default();
        Ok(AppState {
            config: Arc::new(config),
            db,
            cache,
            jwt: Arc::new(jwt),
            storage: Arc::new(storage),
            secrets: Arc::new(secrets),
            http,
            openapi: Arc::new(openapi),
        })
    }

    pub fn is_development(&self) -> bool {
        self.config.app_env.is_development()
    }
}
