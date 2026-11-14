//! Captions. Snapchat transcribes many Spotlight videos and links a WebVTT file from the page.

use serde::Serialize;

use crate::error::Result;
use crate::fetch::Client;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cue {
    /// Seconds from the start of the video.
    pub start: f64,
    pub end: f64,
    pub text: String,
}

fn seconds(stamp: &str) -> Option<f64> {
    // `00:01:02.500` or `01:02.500`
    let mut total = 0.0;
    for part in stamp.trim().split(':') {
        total = total * 60.0 + part.replace(',', ".").parse::<f64>().ok()?;
    }
    Some(total)
}

fn without_tags(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_tag = false;
    for c in line.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " ").trim().to_owned()
}

/// Reads the cues out of a WebVTT file. Anything it doesn't understand is skipped.
pub fn parse_vtt(vtt: &str) -> Vec<Cue> {
    let mut cues: Vec<Cue> = Vec::new();
    let normalized = vtt.replace("\r\n", "\n").replace('\r', "\n");
    for block in normalized.split("\n\n") {
        let mut lines = block.lines().skip_while(|line| !line.contains("-->"));
        let Some(timing) = lines.next() else { continue };
        let Some((start, rest)) = timing.split_once("-->") else { continue };
        let end = rest.split_whitespace().next().unwrap_or("");
        let (Some(start), Some(end)) = (seconds(start), seconds(end)) else { continue };
        let text = lines.map(without_tags).filter(|line| !line.is_empty()).collect::<Vec<_>>().join(" ");
        if text.is_empty() {
            continue;
        }
        // Rolling captions repeat the previous line; keep each sentence once.
        if cues.last().is_some_and(|previous| previous.text == text) {
            if let Some(previous) = cues.last_mut() {
                previous.end = end;
            }
            continue;
        }
        cues.push(Cue { start, end, text });
    }
    cues
}

pub async fn fetch(client: &Client, url: &str) -> Result<Vec<Cue>> {
    Ok(parse_vtt(&client.text(url).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cues() {
        let vtt = "WEBVTT\n\n1\n00:00:00.000 --> 00:00:02.500 align:start\nHello <b>there</b>\n\n00:00:02.500 --> 00:00:04.000\nHello there\n\nNOTE a comment\n\n01:05.250 --> 01:07.000\nSecond &amp; last\nline\n";
        let cues = parse_vtt(vtt);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0], Cue { start: 0.0, end: 4.0, text: "Hello there".into() });
        assert_eq!(cues[1], Cue { start: 65.25, end: 67.0, text: "Second & last line".into() });
        assert!(parse_vtt("not a caption file").is_empty());
    }
}
