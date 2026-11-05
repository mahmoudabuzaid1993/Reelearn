//! Redis layer: cache, rate limits, idempotency keys, revocations, leader leases.
//! Keys follow `docs/architecture.md` section 3. A Redis outage degrades to slower
//! (cache misses) rather than wrong, except for rate limiting and idempotency which
//! fall back to the database-side checks where they exist.

use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::time::Duration;

#[derive(Clone)]
pub struct Cache {
    conn: Option<ConnectionManager>,
    prefix: String,
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("connected", &self.conn.is_some())
            .field("prefix", &self.prefix)
            .finish()
    }
}

impl Cache {
    pub async fn connect(url: &str, prefix: &str) -> Cache {
        let conn = match redis::Client::open(url) {
            Ok(client) => {
                match tokio::time::timeout(Duration::from_secs(3), ConnectionManager::new(client))
                    .await
                {
                    Ok(Ok(c)) => Some(c),
                    Ok(Err(e)) => {
                        tracing::warn!(error = %e, "redis unavailable, running without cache");
                        None
                    }
                    Err(_) => {
                        tracing::warn!("redis connection timed out, running without cache");
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "redis url invalid, running without cache");
                None
            }
        };
        Cache {
            conn,
            prefix: prefix.to_string(),
        }
    }

    pub fn disconnected() -> Cache {
        Cache {
            conn: None,
            prefix: String::new(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.conn.is_some()
    }

    fn key(&self, k: &str) -> String {
        if self.prefix.is_empty() {
            k.to_string()
        } else {
            format!("{}:{}", self.prefix, k)
        }
    }

    pub async fn ping(&self) -> bool {
        match self.conn.clone() {
            Some(mut c) => redis::cmd("PING")
                .query_async::<String>(&mut c)
                .await
                .is_ok(),
            None => false,
        }
    }

    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let mut c = self.conn.clone()?;
        let raw: Option<String> = c.get(self.key(key)).await.ok()?;
        raw.and_then(|s| serde_json::from_str(&s).ok())
    }

    pub async fn set_json<T: serde::Serialize>(&self, key: &str, value: &T, ttl: Duration) {
        let Some(mut c) = self.conn.clone() else {
            return;
        };
        if let Ok(s) = serde_json::to_string(value) {
            let _: Result<(), _> = c.set_ex(self.key(key), s, ttl.as_secs().max(1)).await;
        }
    }

    pub async fn get_string(&self, key: &str) -> Option<String> {
        let mut c = self.conn.clone()?;
        c.get(self.key(key)).await.ok().flatten()
    }

    pub async fn set_string(&self, key: &str, value: &str, ttl: Duration) {
        let Some(mut c) = self.conn.clone() else {
            return;
        };
        let _: Result<(), _> = c.set_ex(self.key(key), value, ttl.as_secs().max(1)).await;
    }

    pub async fn del(&self, key: &str) {
        let Some(mut c) = self.conn.clone() else {
            return;
        };
        let _: Result<(), _> = c.del(self.key(key)).await;
    }

    /// Delete every key matching the prefix (small key spaces only: a user's plan days, a reel's languages).
    pub async fn del_prefix(&self, prefix: &str) {
        let Some(mut c) = self.conn.clone() else {
            return;
        };
        let pattern = format!("{}*", self.key(prefix));
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut c)
            .await
            .unwrap_or_default();
        if !keys.is_empty() {
            let _: Result<(), _> = c.del(keys).await;
        }
    }

    pub async fn exists(&self, key: &str) -> bool {
        let Some(mut c) = self.conn.clone() else {
            return false;
        };
        c.exists(self.key(key)).await.unwrap_or(false)
    }

    /// Fixed-window counter. Returns the count after increment, or `None` without Redis.
    pub async fn incr_window(&self, key: &str, window: Duration) -> Option<i64> {
        let mut c = self.conn.clone()?;
        let k = self.key(key);
        let n: i64 = c.incr(&k, 1).await.ok()?;
        if n == 1 {
            let _: Result<(), _> = c
                .expire(&k, i64::try_from(window.as_secs()).unwrap_or(3600))
                .await;
        }
        Some(n)
    }

    pub async fn ttl_seconds(&self, key: &str) -> Option<i64> {
        let mut c = self.conn.clone()?;
        let t: i64 = c.ttl(self.key(key)).await.ok()?;
        (t > 0).then_some(t)
    }

    /// `SET key value NX PX ttl`: true when the lock/lease was acquired.
    pub async fn set_nx(&self, key: &str, value: &str, ttl: Duration) -> Option<bool> {
        let mut c = self.conn.clone()?;
        let result: Option<String> = redis::cmd("SET")
            .arg(self.key(key))
            .arg(value)
            .arg("NX")
            .arg("PX")
            .arg(ttl.as_millis().max(1) as u64)
            .query_async(&mut c)
            .await
            .ok()?;
        Some(result.is_some())
    }
}

pub mod keys {
    use uuid::Uuid;

    pub fn plan(tenant: Uuid, user: Uuid, date: chrono::NaiveDate) -> String {
        format!("plan:{tenant}:{user}:{date}")
    }
    pub fn plan_prefix(tenant: Uuid, user: Uuid) -> String {
        format!("plan:{tenant}:{user}:")
    }
    pub fn reel(reel: Uuid, lang: &str) -> String {
        format!("reel:{reel}:{lang}")
    }
    pub fn reel_prefix(reel: Uuid) -> String {
        format!("reel:{reel}:")
    }
    pub fn media(key: &str) -> String {
        format!("media:{key}")
    }
    pub fn dash(tenant: Uuid, range: &str) -> String {
        format!("dash:{tenant}:{range}")
    }
    pub fn report(tenant: Uuid, kind: &str, range: &str) -> String {
        format!("report:{tenant}:{kind}:{range}")
    }
    pub fn rate_limit(scope: &str, id: &str) -> String {
        format!("rl:{scope}:{id}")
    }
    pub fn idempotency(tenant: Uuid, key: &str) -> String {
        format!("idem:{tenant}:{key}")
    }
    pub fn revoked(jti: &str) -> String {
        format!("revoked:{jti}")
    }
    pub fn scheduler_leader() -> String {
        "sched:leader".to_string()
    }
}
