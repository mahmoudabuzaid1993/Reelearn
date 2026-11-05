//! Keyset cursors for list endpoints: an opaque base64url token carrying the sort
//! key (a timestamp) and the row id of the last item on the page.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub ts: DateTime<Utc>,
    pub id: Uuid,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("cursor is not valid")]
pub struct CursorError;

impl Cursor {
    pub fn encode(&self) -> String {
        let raw = format!(
            "{}|{}",
            self.ts.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
            self.id
        );
        URL_SAFE_NO_PAD.encode(raw.as_bytes())
    }

    pub fn decode(token: &str) -> Result<Cursor, CursorError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(token.as_bytes())
            .map_err(|_| CursorError)?;
        let raw = String::from_utf8(bytes).map_err(|_| CursorError)?;
        let (ts, id) = raw.split_once('|').ok_or(CursorError)?;
        let ts = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| CursorError)?
            .with_timezone(&Utc);
        let id = Uuid::parse_str(id).map_err(|_| CursorError)?;
        Ok(Cursor { ts, id })
    }
}

/// Clamp a requested page size into the contract's `1..=100` with a default of 25.
pub fn clamp_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(25).clamp(1, 100)
}

/// Split `limit + 1` fetched rows into a page and the cursor of its last row.
pub fn page<T>(
    mut rows: Vec<T>,
    limit: i64,
    key: impl Fn(&T) -> Cursor,
) -> (Vec<T>, Option<String>) {
    let limit = usize::try_from(limit).unwrap_or(25);
    if rows.len() > limit {
        rows.truncate(limit);
        let next = rows.last().map(|r| key(r).encode());
        (rows, next)
    } else {
        (rows, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trips() {
        let c = Cursor {
            ts: Utc::now(),
            id: Uuid::now_v7(),
        };
        let token = c.encode();
        assert!(!token.contains('|'));
        let back = Cursor::decode(&token).expect("decode");
        assert_eq!(back.id, c.id);
        assert_eq!(back.ts.timestamp_micros(), c.ts.timestamp_micros());
    }

    #[test]
    fn bad_tokens_are_rejected() {
        assert_eq!(Cursor::decode("not base64 at all!"), Err(CursorError));
        assert_eq!(
            Cursor::decode(&URL_SAFE_NO_PAD.encode("x|y")),
            Err(CursorError)
        );
    }

    #[test]
    fn limits_are_clamped_and_pages_sliced() {
        assert_eq!(clamp_limit(None), 25);
        assert_eq!(clamp_limit(Some(0)), 1);
        assert_eq!(clamp_limit(Some(500)), 100);
        let rows: Vec<(DateTime<Utc>, Uuid)> =
            (0..4).map(|_| (Utc::now(), Uuid::now_v7())).collect();
        let (page_rows, next) = page(rows.clone(), 3, |r| Cursor { ts: r.0, id: r.1 });
        assert_eq!(page_rows.len(), 3);
        assert_eq!(
            next.map(|t| Cursor::decode(&t).map(|c| c.id)),
            Some(Ok(rows[2].1))
        );
        let (page_rows, next) = page(rows, 10, |r| Cursor { ts: r.0, id: r.1 });
        assert_eq!(page_rows.len(), 4);
        assert!(next.is_none());
    }
}
