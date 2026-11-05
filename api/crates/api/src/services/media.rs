//! Signed media URLs with the `media:{key}` cache (50 min; URLs are signed for 60).

use crate::cache::keys;
use crate::state::AppState;
use std::time::Duration;

pub const MEDIA_CACHE_TTL: Duration = Duration::from_secs(50 * 60);

pub async fn signed_url(state: &AppState, key: Option<&str>) -> Option<String> {
    let key = key?;
    if let Some(url) = state.cache.get_string(&keys::media(key)).await {
        return Some(url);
    }
    let url = state.storage.presign_get(key);
    state
        .cache
        .set_string(&keys::media(key), &url, MEDIA_CACHE_TTL)
        .await;
    Some(url)
}
