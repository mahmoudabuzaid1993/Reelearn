//! Process configuration from environment variables (see `api/.env.example`).

use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Test,
    Production,
}

impl AppEnv {
    pub fn is_development(&self) -> bool {
        matches!(self, AppEnv::Development | AppEnv::Test)
    }
}

#[derive(Debug, Clone)]
pub enum JwtKeys {
    Hs256 {
        secret: String,
    },
    Rs256 {
        private_pem: String,
        public_pem: String,
    },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub app_env: AppEnv,
    pub port: u16,
    pub database_url: String,
    pub database_max_connections: u32,
    pub db_app_role: Option<String>,
    pub redis_url: String,
    pub s3_endpoint: String,
    pub s3_public_endpoint: String,
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub jwt: JwtKeys,
    pub jwt_issuer: String,
    pub access_ttl_seconds: i64,
    pub refresh_ttl_days: i64,
    pub otp_ttl_seconds: i64,
    pub otp_max_attempts: i16,
    pub otp_requests_per_hour: i64,
    /// 32-byte key (hex) for AES-GCM encryption of `integrations.config_enc`.
    pub secrets_key_hex: String,
    pub openapi_path: Option<PathBuf>,
    pub whatsapp_app_secret: Option<String>,
    pub whatsapp_verify_token: Option<String>,
    pub provider_webhook_secret: Option<String>,
    pub cors_origins: Vec<String>,
    pub public_base_url: String,
    pub version: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} is required")]
    Missing(&'static str),
    #[error("{0} is not valid: {1}")]
    Invalid(&'static str, String),
}

fn var(name: &'static str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.trim().is_empty())
}

impl Config {
    pub fn from_env() -> Result<Config, ConfigError> {
        let app_env = match var("APP_ENV").as_deref().unwrap_or("development") {
            "production" | "prod" => AppEnv::Production,
            "test" => AppEnv::Test,
            _ => AppEnv::Development,
        };
        let port = var("PORT")
            .unwrap_or_else(|| "8080".into())
            .parse::<u16>()
            .map_err(|e| ConfigError::Invalid("PORT", e.to_string()))?;
        let database_url = var("DATABASE_URL").ok_or(ConfigError::Missing("DATABASE_URL"))?;
        let jwt = match var("JWT_ALGORITHM").as_deref().unwrap_or("HS256") {
            "RS256" => JwtKeys::Rs256 {
                private_pem: read_pem("JWT_PRIVATE_KEY_PEM", "JWT_PRIVATE_KEY_PATH")?,
                public_pem: read_pem("JWT_PUBLIC_KEY_PEM", "JWT_PUBLIC_KEY_PATH")?,
            },
            _ => {
                let secret = var("JWT_SECRET").ok_or(ConfigError::Missing("JWT_SECRET"))?;
                if app_env == AppEnv::Production && secret.len() < 32 {
                    return Err(ConfigError::Invalid(
                        "JWT_SECRET",
                        "must be at least 32 characters in production".into(),
                    ));
                }
                JwtKeys::Hs256 { secret }
            }
        };
        let secrets_key_hex = var("SECRETS_KEY").unwrap_or_else(|| {
            // Development default: deterministic so encrypted integration configs survive restarts.
            "0".repeat(64)
        });
        if hex::decode(&secrets_key_hex)
            .map(|k| k.len() != 32)
            .unwrap_or(true)
        {
            return Err(ConfigError::Invalid(
                "SECRETS_KEY",
                "must be 32 bytes as 64 hex characters".into(),
            ));
        }
        if app_env == AppEnv::Production && secrets_key_hex.chars().all(|c| c == '0') {
            return Err(ConfigError::Invalid(
                "SECRETS_KEY",
                "must be set in production".into(),
            ));
        }
        let s3_endpoint = var("S3_ENDPOINT").unwrap_or_else(|| "http://localhost:9000".into());
        Ok(Config {
            app_env,
            port,
            database_url,
            database_max_connections: var("DATABASE_MAX_CONNECTIONS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(16),
            db_app_role: match var("DB_APP_ROLE") {
                Some(r) if r == "none" => None,
                Some(r) => Some(r),
                None => Some(reelearn_db::pool::DEFAULT_APP_ROLE.to_string()),
            },
            redis_url: var("REDIS_URL").unwrap_or_else(|| "redis://localhost:6379".into()),
            s3_public_endpoint: var("S3_PUBLIC_ENDPOINT").unwrap_or_else(|| s3_endpoint.clone()),
            s3_endpoint,
            s3_bucket: var("S3_BUCKET").unwrap_or_else(|| "reelearn".into()),
            s3_region: var("S3_REGION").unwrap_or_else(|| "us-east-1".into()),
            s3_access_key: var("S3_ACCESS_KEY").unwrap_or_else(|| "reelearn".into()),
            s3_secret_key: var("S3_SECRET_KEY").unwrap_or_else(|| "reelearn123".into()),
            jwt,
            jwt_issuer: var("JWT_ISSUER").unwrap_or_else(|| "reelearn".into()),
            access_ttl_seconds: var("ACCESS_TTL_SECONDS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(15 * 60),
            refresh_ttl_days: var("REFRESH_TTL_DAYS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            otp_ttl_seconds: var("OTP_TTL_SECONDS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            otp_max_attempts: var("OTP_MAX_ATTEMPTS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            otp_requests_per_hour: var("OTP_REQUESTS_PER_HOUR")
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            secrets_key_hex,
            openapi_path: var("OPENAPI_PATH").map(PathBuf::from),
            whatsapp_app_secret: var("WHATSAPP_APP_SECRET"),
            whatsapp_verify_token: var("WHATSAPP_VERIFY_TOKEN"),
            provider_webhook_secret: var("PROVIDER_WEBHOOK_SECRET"),
            cors_origins: var("CORS_ORIGINS")
                .map(|v| {
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            public_base_url: var("PUBLIC_BASE_URL")
                .unwrap_or_else(|| format!("http://localhost:{port}")),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    /// Configuration for tests: everything local, deterministic secrets.
    pub fn for_tests(database_url: &str) -> Config {
        Config {
            app_env: AppEnv::Test,
            port: 0,
            database_url: database_url.to_string(),
            database_max_connections: 4,
            db_app_role: Some(reelearn_db::pool::DEFAULT_APP_ROLE.to_string()),
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into()),
            s3_endpoint: env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".into()),
            s3_public_endpoint: env::var("S3_PUBLIC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:9000".into()),
            s3_bucket: "reelearn-test".into(),
            s3_region: "us-east-1".into(),
            s3_access_key: "reelearn".into(),
            s3_secret_key: "reelearn123".into(),
            jwt: JwtKeys::Hs256 {
                secret: "test-secret-test-secret-test-secret-1234".into(),
            },
            jwt_issuer: "reelearn-test".into(),
            access_ttl_seconds: 900,
            refresh_ttl_days: 30,
            otp_ttl_seconds: 300,
            otp_max_attempts: 5,
            otp_requests_per_hour: 3,
            secrets_key_hex: "1".repeat(64),
            openapi_path: None,
            whatsapp_app_secret: None,
            whatsapp_verify_token: Some("verify-me".into()),
            provider_webhook_secret: None,
            cors_origins: vec![],
            public_base_url: "http://localhost:8080".into(),
            version: "test".into(),
        }
    }

    /// Candidate locations for the static OpenAPI document.
    pub fn openapi_candidates(&self) -> Vec<PathBuf> {
        let mut v = Vec::new();
        if let Some(p) = &self.openapi_path {
            v.push(p.clone());
        }
        v.push(PathBuf::from("../openapi/openapi.yaml"));
        v.push(PathBuf::from("openapi/openapi.yaml"));
        v.push(PathBuf::from("/app/openapi/openapi.yaml"));
        v.push(PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../openapi/openapi.yaml"
        )));
        v
    }
}

fn read_pem(inline_var: &'static str, path_var: &'static str) -> Result<String, ConfigError> {
    if let Some(v) = var(inline_var) {
        return Ok(v.replace("\\n", "\n"));
    }
    if let Some(p) = var(path_var) {
        return std::fs::read_to_string(&p)
            .map_err(|e| ConfigError::Invalid(path_var, e.to_string()));
    }
    Err(ConfigError::Missing(inline_var))
}
