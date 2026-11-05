//! Pre-signed object-storage URLs (MinIO / S3). Media never passes through the API.

use rusty_s3::{Bucket, Credentials, S3Action, UrlStyle};
use std::time::Duration;
use url::Url;

#[derive(Clone, Debug)]
pub struct Storage {
    bucket: Bucket,
    /// Bucket addressed through the public endpoint (what phones and browsers can reach).
    public_bucket: Bucket,
    credentials: Credentials,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("invalid storage endpoint: {0}")]
    Endpoint(String),
    #[error("bucket setup failed: {0}")]
    Bucket(String),
}

pub const DOWNLOAD_TTL: Duration = Duration::from_secs(60 * 60);
pub const UPLOAD_TTL: Duration = Duration::from_secs(15 * 60);

impl Storage {
    pub fn new(
        endpoint: &str,
        public_endpoint: &str,
        bucket: &str,
        region: &str,
        access_key: &str,
        secret_key: &str,
    ) -> Result<Storage, StorageError> {
        let make = |ep: &str| -> Result<Bucket, StorageError> {
            let url = Url::parse(ep).map_err(|e| StorageError::Endpoint(e.to_string()))?;
            Bucket::new(url, UrlStyle::Path, bucket.to_string(), region.to_string())
                .map_err(|e| StorageError::Endpoint(e.to_string()))
        };
        Ok(Storage {
            bucket: make(endpoint)?,
            public_bucket: make(public_endpoint)?,
            credentials: Credentials::new(access_key, secret_key),
        })
    }

    pub fn bucket_name(&self) -> &str {
        self.bucket.name()
    }

    pub fn presign_put(&self, key: &str, _mime: &str) -> String {
        self.public_bucket
            .put_object(Some(&self.credentials), key)
            .sign(UPLOAD_TTL)
            .to_string()
    }

    pub fn presign_get(&self, key: &str) -> String {
        self.public_bucket
            .get_object(Some(&self.credentials), key)
            .sign(DOWNLOAD_TTL)
            .to_string()
    }

    /// Internal (worker-side) signed PUT for writing generated media.
    pub fn presign_put_internal(&self, key: &str) -> String {
        self.bucket
            .put_object(Some(&self.credentials), key)
            .sign(UPLOAD_TTL)
            .to_string()
    }

    /// Create the bucket if it does not exist (idempotent; 409 means it is already there).
    pub async fn ensure_bucket(&self, http: &reqwest::Client) -> Result<(), StorageError> {
        let url = self
            .bucket
            .create_bucket(&self.credentials)
            .sign(Duration::from_secs(60));
        match http.put(url.as_str()).send().await {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 409 => Ok(()),
            Ok(resp) => Err(StorageError::Bucket(format!("status {}", resp.status()))),
            Err(e) => Err(StorageError::Bucket(e.to_string())),
        }
    }

    /// Upload small generated artefacts (placeholder mp4/vtt, export files) from the worker.
    pub async fn put_bytes(
        &self,
        http: &reqwest::Client,
        key: &str,
        mime: &str,
        body: Vec<u8>,
    ) -> Result<(), StorageError> {
        let url = self.presign_put_internal(key);
        let resp = http
            .put(&url)
            .header("content-type", mime)
            .body(body)
            .send()
            .await
            .map_err(|e| StorageError::Bucket(e.to_string()))?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(StorageError::Bucket(format!(
                "upload failed with status {}",
                resp.status()
            )))
        }
    }
}

/// Content-addressed, never reused keys (CDN caches them for a year).
pub fn reel_media_key(
    tenant: uuid::Uuid,
    reel: uuid::Uuid,
    version: i32,
    language: &str,
    ext: &str,
) -> String {
    format!("reels/{tenant}/{reel}/v{version}/{language}.{ext}")
}

pub fn upload_key(tenant: uuid::Uuid, asset_id: uuid::Uuid, filename: &str) -> String {
    let safe: String = filename
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .take(80)
        .collect();
    format!(
        "uploads/{tenant}/{asset_id}/{}",
        if safe.is_empty() {
            "file".to_string()
        } else {
            safe
        }
    )
}

pub fn export_key(tenant: uuid::Uuid, export_id: uuid::Uuid, ext: &str) -> String {
    format!("exports/{tenant}/{export_id}.{ext}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presigned_urls_point_at_the_public_endpoint() {
        let s = Storage::new(
            "http://minio:9000",
            "http://localhost:9000",
            "reelearn",
            "us-east-1",
            "k",
            "s",
        )
        .expect("storage");
        let url = s.presign_get("reels/a/b.mp4");
        assert!(url.starts_with("http://localhost:9000/reelearn/reels/a/b.mp4?"));
        assert!(url.contains("X-Amz-Signature="));
        assert!(s
            .presign_put_internal("x")
            .starts_with("http://minio:9000/reelearn/x?"));
    }

    #[test]
    fn keys_are_sanitised() {
        let t = uuid::Uuid::nil();
        assert_eq!(
            upload_key(t, t, "../evil file.pdf"),
            format!("uploads/{t}/{t}/..evilfile.pdf")
        );
        assert!(reel_media_key(t, t, 2, "ar", "mp4").ends_with("/v2/ar.mp4"));
    }
}
