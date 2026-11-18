//! Turns the JSON embedded in a profile page into a [`Report`].

use std::collections::HashMap;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::model::*;
use crate::paths::{full, rel, ROOT};
use crate::util::{iso_from_millis, iso_from_secs, media_id, now_iso};

/// `pageMetadata.pageType` of a public profile.
pub const PUBLIC_PAGE: i64 = 18;
/// Snapchat's page carries at most this many highlights, Spotlights or lenses before "show more".
const FIRST_PAGE: usize = 16;
/// What Snapchat calls a Spotlight that has no text on it.
const STOCK_NAME: &str = "Spotlight Snap";

/// Finds the `<script type="application/json">` block Next.js puts in the page and parses it.
pub fn extract_page_json(html: &str) -> Result<Value> {
    let missing = || Error::PageFormat("it has no embedded profile data".into());
    let attribute = html.find("type=\"application/json\"").ok_or_else(missing)?;
    let start = attribute + html[attribute..].find('>').ok_or_else(missing)? + 1;
    let end = start + html[start..].find("</script>").ok_or_else(missing)?;
    serde_json::from_str(&html[start..end]).map_err(|e| Error::PageFormat(format!("its profile data isn't valid JSON ({e})")))
}

pub(crate) fn at<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut node = value;
    for key in path.split('.') {
        node = node.get(key)?;
    }
    (!node.is_null()).then_some(node)
}

fn text(value: &Value, path: &str) -> Option<String> {
    match at(value, path)? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_owned()),
        _ => None,
    }
}

/// Only web addresses are passed on, so nothing else can end up in a link or a download.
fn link(value: &Value, path: &str) -> Option<String> {
    text(value, path).filter(|s| s.starts_with("https://") || s.starts_with("http://"))
}

fn int(value: &Value, path: &str) -> Option<i64> {
    match at(value, path)? {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// A count, or `None` when Snapchat hides it (it sends -1).
fn count(value: &Value, path: &str) -> Option<u64> {
    int(value, path).and_then(|n| u64::try_from(n).ok())
}

fn list<'a>(value: &'a Value, path: &str) -> &'a [Value] {
    at(value, path).and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn strings(value: &Value, path: &str) -> Vec<String> {
    list(value, path).iter().filter_map(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect()
}

fn badge_name(code: Option<i64>) -> Option<String> {
    match code {
        Some(1) => Some("creator".into()),
        Some(3) => Some("public_figure".into()),
        _ => None,
    }
}

/// `public-profile-category-v3-product-brand` to `Product Brand`.
fn category_name(id: &str) -> Option<String> {
    let mut rest = id;
    for prefix in ["public-profile-subcategory-", "public-profile-category-"] {
        if let Some(stripped) = rest.strip_prefix(prefix) {
            rest = stripped;
            break;
        }
    }
    if let Some((version, after)) = rest.split_once('-') {
        if version.len() > 1 && version.starts_with('v') && version[1..].chars().all(|c| c.is_ascii_digit()) {
            rest = after;
        }
    }
    let words: Vec<String> = rest
        .split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
        })
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

fn snap(value: &Value, position: usize) -> Option<Snap> {
    let media_url = link(value, rel("snap.media_url"))?;
    Some(Snap {
        id: media_id(&media_url),
        index: int(value, rel("snap.index")).and_then(|n| u32::try_from(n).ok()).unwrap_or(position as u32),
        media_type: if int(value, rel("snap.media_type")) == Some(0) { MediaType::Image } else { MediaType::Video },
        preview_url: link(value, rel("snap.preview_url")),
        uploaded_at: int(value, rel("snap.timestamp")).and_then(iso_from_secs),
        transcript_url: link(value, rel("snap.transcript_url")),
        media_url,
    })
}

fn snaps(values: &[Value]) -> Vec<Snap> {
    values.iter().enumerate().filter_map(|(i, v)| snap(v, i)).collect()
}

fn highlight(value: &Value) -> Highlight {
    Highlight {
        id: text(value, rel("highlight.id")),
        title: text(value, rel("highlight.title")),
        thumbnail_url: link(value, rel("highlight.thumbnail_url")),
        snaps: snaps(list(value, rel("highlight.snaps"))),
    }
}

fn sound(details: &Value) -> Option<Sound> {
    // contextType 2 is the sound a Spotlight uses; 3 is its author.
    let card = list(details, rel("details.context_cards")).iter().find(|c| int(c, "contextType") == Some(2))?;
    let sound = Sound { title: text(card, "title"), artist: text(card, "subtitle") };
    (sound.title.is_some() || sound.artist.is_some()).then_some(sound)
}

/// The top public comments under each Spotlight, by snap ID.
///
/// The page carries them as JSON inside a string. Each comment also names who wrote it (a display
/// name, IDs, an avatar); those fields are not read, so they can't reach a report or an export.
fn comments_by_snap(root: &Value) -> HashMap<String, Vec<Comment>> {
    let mut by_snap: HashMap<String, Vec<Comment>> = HashMap::new();
    let Some(encoded) = text(root, full("comments")) else { return by_snap };
    let Ok(Value::Array(groups)) = serde_json::from_str::<Value>(&encoded) else { return by_snap };
    for comment in groups.iter().filter_map(Value::as_array).flatten() {
        let (Some(snap), Some(words)) = (text(comment, "snapId"), text(comment, "replyText")) else { continue };
        by_snap.entry(snap).or_default().push(Comment {
            text: words,
            posted_at: int(comment, "replyTimestampMs").and_then(iso_from_millis),
            reactions: sum_known(list(comment, "reactCounts").iter().map(|react| count(react, "reactCount"))),
            replies: count(comment, "threadedReplyCount"),
        });
    }
    by_snap
}

fn spotlight(value: &Value, details: Option<&Value>) -> Spotlight {
    let mut item = Spotlight {
        id: text(value, rel("spotlight.id")),
        thumbnail_url: link(value, rel("spotlight.thumbnail_url")),
        snaps: snaps(list(value, rel("spotlight.snaps"))),
        ..Default::default()
    };
    if let Some(d) = details {
        item.name = text(d, rel("details.name"));
        item.description = text(d, rel("details.description"));
        item.ai_title = text(d, rel("details.ai_title"));
        item.ai_summary = text(d, rel("details.ai_summary"));
        item.keywords = strings(d, rel("details.keywords"));
        item.hashtags = strings(d, rel("details.hashtags"));
        item.duration_seconds = count(d, rel("details.duration")).map(|ms| ms / 1000);
        item.uploaded_at = int(d, rel("details.uploaded")).and_then(iso_from_millis);
        item.width = count(d, rel("details.width")).filter(|n| *n > 0);
        item.height = count(d, rel("details.height")).filter(|n| *n > 0);
        item.views = count(d, rel("details.views"));
        item.shares = count(d, rel("details.shares"));
        item.comments = count(d, rel("details.comments"));
        item.boosts = count(d, rel("details.boosts"));
        item.recommends = count(d, rel("details.recommends"));
        item.sound = sound(d);
        item.page_url = link(d, rel("details.page_url"));
        // Videos without text on them carry a stock name instead.
        item.on_screen_text = text(d, rel("details.on_screen_text")).or_else(|| item.name.clone().filter(|name| name != STOCK_NAME));
        item.categories = strings(d, rel("details.categories"));
        item.search_phrases = strings(d, rel("details.search_phrases"));
        let mut best_language = 0.0;
        for signal in list(d, rel("details.signals")) {
            if item.ai_caption.is_none() {
                item.ai_caption = text(signal, "signal.textMetadata.description");
            }
            for score in list(signal, "signal.detectedLanguage.languageScores") {
                let confidence = at(score, "score").and_then(Value::as_f64).unwrap_or(0.0);
                if let Some(code) = text(score, "languageCode").filter(|code| code != "unk" && confidence > best_language) {
                    best_language = confidence;
                    item.language = Some(code);
                }
            }
        }
    }
    if item.uploaded_at.is_none() {
        item.uploaded_at = item.snaps.first().and_then(|s| s.uploaded_at.clone());
    }
    item
}

fn lens(value: &Value) -> Lens {
    Lens {
        name: text(value, rel("lens.name")),
        creator: text(value, rel("lens.creator")),
        preview_image_url: link(value, rel("lens.preview_image_url")),
        preview_video_url: link(value, rel("lens.preview_video_url")),
        icon_url: link(value, rel("lens.icon_url")),
        unlock_url: link(value, rel("lens.unlock_url")),
        official: at(value, rel("lens.official")).and_then(Value::as_bool).unwrap_or(false),
        updated_at: int(value, rel("lens.updated")).filter(|n| *n > 0).and_then(iso_from_secs),
    }
}

fn related(value: &Value) -> Option<RelatedAccount> {
    Some(RelatedAccount {
        username: text(value, rel("related.username"))?,
        display_name: text(value, rel("related.display_name")),
        profile_picture_url: link(value, rel("related.picture_url")),
        badge: badge_name(int(value, rel("related.badge"))),
    })
}

fn has_more(root: &Value, cursor: &str, shown: usize) -> bool {
    shown >= FIRST_PAGE && text(root, full(cursor)).is_some()
}

fn sum_known(values: impl Iterator<Item = Option<u64>>) -> Option<u64> {
    values.flatten().fold(None, |total, n| Some(total.unwrap_or(0) + n))
}

/// The page links a profile picture shrunk to 90 pixels: its address ends in a size and a format,
/// such as `._RS0,90_FMjpeg`. Without the size, the same address gives the picture as it was uploaded.
fn full_size(url: &str) -> String {
    if let (true, Some(size)) = (url.contains("/aps/bolt/"), url.rfind("._RS")) {
        if let Some(format) = url[size..].find("_FM") {
            return format!("{}.{}", &url[..size], &url[size + format..]);
        }
    }
    url.to_owned()
}

fn fill_public(root: &Value, report: &mut Report) {
    let profile = &mut report.profile;
    profile.username = text(root, full("username"));
    profile.display_name = text(root, full("display_name"));
    profile.bio = text(root, full("bio"));
    profile.website_url = link(root, full("website_url"));
    profile.badge_code = int(root, full("badge"));
    profile.badge = badge_name(profile.badge_code);
    // Snapchat sends 0 for profiles that don't show their subscriber count.
    profile.subscriber_count = count(root, full("subscriber_count")).filter(|n| *n > 0);
    profile.category = text(root, full("category")).and_then(|id| category_name(&id));
    profile.subcategory = text(root, full("subcategory")).and_then(|id| category_name(&id));
    profile.created_at = int(root, full("created")).filter(|n| *n > 0).and_then(iso_from_millis);
    profile.updated_at = int(root, full("updated")).filter(|n| *n > 0).and_then(iso_from_millis);
    profile.profile_picture_url = link(root, full("profile_picture_url")).map(|url| full_size(&url));
    profile.hero_image_url = link(root, full("hero_image_url"));
    profile.snapcode_url = link(root, full("snapcode_url"));
    profile.links = strings(root, full("links")).into_iter().filter(|s| s.starts_with("https://") || s.starts_with("http://")).collect();

    report.story = snaps(list(root, full("story")));
    report.highlights = list(root, full("highlights")).iter().map(highlight).collect();
    let details = list(root, full("spotlight_details"));
    let sources = list(root, full("spotlights"));
    report.spotlights = sources.iter().enumerate().map(|(i, v)| spotlight(v, details.get(i))).collect();
    let mut comments = comments_by_snap(root);
    for (item, source) in report.spotlights.iter_mut().zip(sources) {
        // Comments are keyed by the Spotlight's snap ID, which the page gives in two places.
        let first_snap = list(source, rel("spotlight.snaps")).first().and_then(|snap| text(snap, rel("snap.snap_id")));
        if let Some(found) = [item.id.clone(), first_snap].into_iter().flatten().find_map(|id| comments.remove(&id)) {
            item.top_comments = found;
        }
    }
    report.lenses = list(root, full("lenses")).iter().map(lens).collect();
    report.related_accounts = list(root, full("related")).iter().filter_map(related).collect();
    report.more_available = MoreAvailable {
        highlights: has_more(root, "highlights.cursor", report.highlights.len()),
        spotlights: has_more(root, "spotlights.cursor", report.spotlights.len()),
        lenses: has_more(root, "lenses.cursor", report.lenses.len()),
    };
    report.stats = Stats {
        story_snaps: report.story.len() as u64,
        highlights: report.highlights.len() as u64,
        highlight_snaps: report.highlights.iter().map(|h| h.snaps.len() as u64).sum(),
        spotlights: report.spotlights.len() as u64,
        spotlight_duration_seconds: report.spotlights.iter().filter_map(|s| s.duration_seconds).sum(),
        spotlight_views: sum_known(report.spotlights.iter().map(|s| s.views)),
        spotlight_comments: sum_known(report.spotlights.iter().map(|s| s.comments)),
        spotlight_shares: sum_known(report.spotlights.iter().map(|s| s.shares)),
        lenses: report.lenses.len() as u64,
    };
}

fn fill_private(root: &Value, report: &mut Report) {
    let profile = &mut report.profile;
    profile.username = text(root, full("private.username"));
    profile.display_name = text(root, full("private.display_name"));
    profile.snapcode_url = link(root, full("private.snapcode_url"));
    profile.avatar_url = link(root, full("private.avatar_url"));
    profile.background_url = link(root, full("private.background_url"));
}

/// Builds the report for a page. `requested` is what was looked up and `url` where it was fetched from.
pub fn parse_report(page: &Value, requested: &str, url: &str) -> Result<Report> {
    let root = at(page, ROOT).ok_or_else(|| Error::PageFormat("its data has no profile section".into()))?;
    let page_title = text(root, full("page_title"));
    let page_type = int(root, full("page_type"));
    // The page Snapchat serves for a name nobody has carries no metadata at all.
    if page_title.is_none() && page_type.is_none() {
        return Err(Error::NotFound(requested.to_owned()));
    }
    let mut report = Report {
        snapintel_version: crate::VERSION.to_owned(),
        retrieved_at: now_iso(),
        requested_username: requested.to_owned(),
        url: url.to_owned(),
        canonical_url: link(root, full("canonical_url")),
        locale: text(root, full("locale")),
        profile: Profile { public: page_type == Some(PUBLIC_PAGE), page_title, page_description: text(root, full("page_description")), ..Default::default() },
        ..Default::default()
    };
    if report.profile.public {
        fill_public(root, &mut report);
    } else {
        fill_private(root, &mut report);
    }
    if report.profile.username.is_none() {
        return Err(Error::PageFormat("the profile's username isn't where it used to be".into()));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_names() {
        assert_eq!(category_name("public-profile-category-v3-product-brand").as_deref(), Some("Product Brand"));
        assert_eq!(category_name("public-profile-subcategory-v3-product").as_deref(), Some("Product"));
        assert_eq!(category_name("something_else").as_deref(), Some("Something Else"));
        assert_eq!(category_name(""), None);
    }

    #[test]
    fn page_json_is_found() {
        let html = r#"<html><script src="x.js"></script><script id="__NEXT_DATA__" type="application/json">{"a":{"b":1}}</script></html>"#;
        assert_eq!(extract_page_json(html).unwrap()["a"]["b"], 1);
        assert!(matches!(extract_page_json("<html></html>"), Err(Error::PageFormat(_))));
    }

    #[test]
    fn hidden_counts_are_none() {
        let value: Value = serde_json::from_str(r#"{"a":"-1","b":"12","c":7,"d":null}"#).unwrap();
        assert_eq!(count(&value, "a"), None);
        assert_eq!(count(&value, "b"), Some(12));
        assert_eq!(count(&value, "c"), Some(7));
        assert_eq!(count(&value, "d"), None);
    }
}
