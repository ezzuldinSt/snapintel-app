//! Writes a report as JSON (everything) or CSV (one row per media item, for spreadsheets).

use crate::error::{Error, Result};
use crate::model::{MediaType, Report, Snap};

pub fn to_json(report: &Report) -> Result<String> {
    let mut text = serde_json::to_string_pretty(report).map_err(|e| Error::Other(e.to_string()))?;
    text.push('\n');
    Ok(text)
}

fn cell(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn number(value: Option<u64>) -> String {
    value.map(|n| n.to_string()).unwrap_or_default()
}

fn media_type(kind: MediaType) -> &'static str {
    match kind {
        MediaType::Image => "image",
        MediaType::Video => "video",
    }
}

const COLUMNS: [&str; 13] =
    ["section", "group", "title", "id", "type", "uploaded_at", "duration_seconds", "views", "comments", "shares", "hashtags", "url", "page_url"];

/// One row per story snap, highlight snap, Spotlight and lens. Starts with a byte-order mark so
/// spreadsheet programs read Arabic and other scripts correctly.
pub fn to_csv(report: &Report) -> String {
    let mut rows: Vec<[String; 13]> = Vec::new();
    let snap_row = |section: &str, group: &str, snap: &Snap| -> [String; 13] {
        [
            section.into(),
            group.into(),
            String::new(),
            snap.id.clone(),
            media_type(snap.media_type).into(),
            snap.uploaded_at.clone().unwrap_or_default(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            snap.media_url.clone(),
            String::new(),
        ]
    };
    for snap in &report.story {
        rows.push(snap_row("story", "", snap));
    }
    for highlight in &report.highlights {
        let title = highlight.title.clone().unwrap_or_default();
        for snap in &highlight.snaps {
            rows.push(snap_row("highlight", &title, snap));
        }
    }
    for spotlight in &report.spotlights {
        let first = spotlight.snaps.first();
        rows.push([
            "spotlight".into(),
            String::new(),
            spotlight.description.clone().or_else(|| spotlight.ai_title.clone()).or_else(|| spotlight.name.clone()).unwrap_or_default(),
            first.map(|s| s.id.clone()).or_else(|| spotlight.id.clone()).unwrap_or_default(),
            first.map_or("video", |s| media_type(s.media_type)).into(),
            spotlight.uploaded_at.clone().unwrap_or_default(),
            number(spotlight.duration_seconds),
            number(spotlight.views),
            number(spotlight.comments),
            number(spotlight.shares),
            spotlight.hashtags.join(" "),
            first.map(|s| s.media_url.clone()).unwrap_or_default(),
            spotlight.page_url.clone().unwrap_or_default(),
        ]);
    }
    for lens in &report.lenses {
        rows.push([
            "lens".into(),
            lens.creator.clone().unwrap_or_default(),
            lens.name.clone().unwrap_or_default(),
            String::new(),
            "video".into(),
            lens.updated_at.clone().unwrap_or_default(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            lens.preview_video_url.clone().unwrap_or_default(),
            lens.unlock_url.clone().unwrap_or_default(),
        ]);
    }
    let mut out = String::from("\u{feff}");
    out.push_str(&COLUMNS.join(","));
    out.push_str("\r\n");
    for row in rows {
        out.push_str(&row.iter().map(|value| cell(value)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_are_quoted_when_needed() {
        assert_eq!(cell("plain"), "plain");
        assert_eq!(cell("a,b"), "\"a,b\"");
        assert_eq!(cell("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(cell("two\nlines"), "\"two\nlines\"");
    }
}
