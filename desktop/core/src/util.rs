use chrono::{DateTime, NaiveDateTime, Utc};
use sha1::{Digest, Sha1};

const ISO: &str = "%Y-%m-%dT%H:%M:%SZ";

pub fn now_iso() -> String {
    Utc::now().format(ISO).to_string()
}

/// Seconds since 1970 to `2026-08-18T17:59:08Z`.
pub fn iso_from_secs(secs: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp(secs, 0).map(|d| d.format(ISO).to_string())
}

pub fn iso_from_millis(millis: i64) -> Option<String> {
    iso_from_secs(millis.div_euclid(1000))
}

pub fn secs_from_iso(iso: &str) -> Option<i64> {
    NaiveDateTime::parse_from_str(iso, ISO).ok().map(|d| d.and_utc().timestamp())
}

pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn url_path(url: &str) -> &str {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = after_scheme.find('/').map_or("", |i| &after_scheme[i..]);
    let end = path.find(['?', '#']).unwrap_or(path.len());
    &path[..end]
}

/// A stable, file-name-safe ID for a media URL: its last path segment without extension, or a
/// hash of the path when that is too short to be an ID. The Python tool names files the same way,
/// so a folder it filled can be resumed here.
pub fn media_id(url: &str) -> String {
    let path = url_path(url);
    let segment = path.rsplit('/').find(|s| !s.is_empty()).unwrap_or("");
    let stem = segment.split('.').next().unwrap_or("");
    let candidate: String = stem.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
    if candidate.len() >= 8 {
        candidate
    } else {
        hex(&Sha1::digest(path.as_bytes()))[..16].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_ids() {
        assert_eq!(media_id("https://cf-st.sc-cdn.net/d/CBVqD5SrTfygLFpfCgKGH.1322?mo=Glca&uc=75"), "CBVqD5SrTfygLFpfCgKGH");
        assert_eq!(
            media_id("https://community-lens.storage.googleapis.com/preview-media/final/19aca49f-6013-4f32-ab25-07f3f14b6b17.mp4"),
            "19aca49f-6013-4f32-ab25-07f3f14b6b17"
        );
        // Too short to be an ID: a hash of the path instead, the same one Python's hashlib.sha1 gives.
        assert_eq!(media_id("https://example.com/a/b.mp4?x=1"), "f03580b4448fc02e");
    }

    #[test]
    fn times() {
        assert_eq!(iso_from_secs(1787075948).as_deref(), Some("2026-08-18T17:59:08Z"));
        assert_eq!(iso_from_millis(1787075948348).as_deref(), Some("2026-08-18T17:59:08Z"));
        assert_eq!(secs_from_iso("2026-08-18T17:59:08Z"), Some(1787075948));
        assert_eq!(hex(&[0, 255, 16]), "00ff10");
    }
}
