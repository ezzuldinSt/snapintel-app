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

/// One line of the CSV. Fields a kind of item doesn't have stay empty.
#[derive(Default)]
struct Row {
    section: &'static str,
    group: String,
    title: String,
    id: String,
    kind: &'static str,
    uploaded_at: String,
    duration_seconds: String,
    views: String,
    comments: String,
    shares: String,
    hashtags: String,
    language: String,
    on_screen_text: String,
    url: String,
    page_url: String,
}

const COLUMNS: [&str; 15] = [
    "section",
    "group",
    "title",
    "id",
    "type",
    "uploaded_at",
    "duration_seconds",
    "views",
    "comments",
    "shares",
    "hashtags",
    "language",
    "on_screen_text",
    "url",
    "page_url",
];

impl Row {
    fn cells(&self) -> [&str; 15] {
        [
            self.section,
            &self.group,
            &self.title,
            &self.id,
            self.kind,
            &self.uploaded_at,
            &self.duration_seconds,
            &self.views,
            &self.comments,
            &self.shares,
            &self.hashtags,
            &self.language,
            &self.on_screen_text,
            &self.url,
            &self.page_url,
        ]
    }

    fn snap(section: &'static str, group: &str, snap: &Snap) -> Self {
        Self {
            section,
            group: group.to_owned(),
            id: snap.id.clone(),
            kind: media_type(snap.media_type),
            uploaded_at: snap.uploaded_at.clone().unwrap_or_default(),
            url: snap.media_url.clone(),
            ..Default::default()
        }
    }
}

/// One row per story snap, highlight snap, Spotlight and lens. Starts with a byte-order mark so
/// spreadsheet programs read Arabic and other scripts correctly.
pub fn to_csv(report: &Report) -> String {
    let mut rows: Vec<Row> = report.story.iter().map(|snap| Row::snap("story", "", snap)).collect();
    for highlight in &report.highlights {
        let title = highlight.title.as_deref().unwrap_or_default();
        rows.extend(highlight.snaps.iter().map(|snap| Row::snap("highlight", title, snap)));
    }
    for spotlight in &report.spotlights {
        let first = spotlight.snaps.first();
        rows.push(Row {
            section: "spotlight",
            title: spotlight.description.clone().or_else(|| spotlight.ai_title.clone()).or_else(|| spotlight.name.clone()).unwrap_or_default(),
            id: first.map(|s| s.id.clone()).or_else(|| spotlight.id.clone()).unwrap_or_default(),
            kind: first.map_or("video", |s| media_type(s.media_type)),
            uploaded_at: spotlight.uploaded_at.clone().unwrap_or_default(),
            duration_seconds: number(spotlight.duration_seconds),
            views: number(spotlight.views),
            comments: number(spotlight.comments),
            shares: number(spotlight.shares),
            hashtags: spotlight.hashtags.join(" "),
            language: spotlight.language.clone().unwrap_or_default(),
            on_screen_text: spotlight.on_screen_text.clone().unwrap_or_default(),
            url: first.map(|s| s.media_url.clone()).unwrap_or_default(),
            page_url: spotlight.page_url.clone().unwrap_or_default(),
            ..Default::default()
        });
    }
    for lens in &report.lenses {
        rows.push(Row {
            section: "lens",
            group: lens.creator.clone().unwrap_or_default(),
            title: lens.name.clone().unwrap_or_default(),
            kind: "video",
            uploaded_at: lens.updated_at.clone().unwrap_or_default(),
            url: lens.preview_video_url.clone().unwrap_or_default(),
            page_url: lens.unlock_url.clone().unwrap_or_default(),
            ..Default::default()
        });
    }
    let mut out = String::from("\u{feff}");
    out.push_str(&COLUMNS.join(","));
    out.push_str("\r\n");
    for row in rows {
        out.push_str(&row.cells().iter().map(|value| cell(value)).collect::<Vec<_>>().join(","));
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
