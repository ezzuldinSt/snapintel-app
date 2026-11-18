//! Offline tests on the page snapshots in `tests/fixtures` (shared with the Python tool) and on a
//! tiny local web server. Nothing here talks to snapchat.com.

use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use serde_json::Value;
use snapintel_core::downloads::{self, DownloadItem, DownloadManager, Outcome, MANIFEST};
use snapintel_core::media::VideoVersion;
use snapintel_core::model::MediaType;
use snapintel_core::selfcheck::{self, Status};
use snapintel_core::{export, parse, Client, ClientOptions, Error, Report};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn fixture(name: &str) -> String {
    let path = format!("{}/../../tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn page(name: &str) -> Value {
    parse::extract_page_json(&fixture(name)).unwrap()
}

fn report(name: &str) -> Report {
    parse::parse_report(&page(name), "spectacles", "https://www.snapchat.com/add/spectacles").unwrap()
}

fn props(page: &Value) -> &Value {
    &page["props"]["pageProps"]
}

// --- Parsing -------------------------------------------------------------------------------

#[test]
fn public_profile() {
    let page = page("public_profile.html");
    let raw = props(&page);
    let info = &raw["userProfile"]["publicProfileInfo"];
    let report = report("public_profile.html");

    assert!(report.profile.public);
    assert_eq!(report.profile.username.as_deref(), info["username"].as_str());
    // The page links the profile picture shrunk to 90 pixels; the report has it at its original size.
    assert!(info["profilePictureUrl"].as_str().unwrap().ends_with("._RS0,90_FMpng"));
    assert!(report.profile.profile_picture_url.as_deref().unwrap().ends_with("._FMpng"));
    assert_eq!(report.profile.display_name.as_deref(), info["title"].as_str());
    assert_eq!(report.profile.badge.as_deref(), Some("creator"));
    assert_eq!(report.profile.subscriber_count, info["subscriberCount"].as_str().and_then(|s| s.parse().ok()));
    assert!(report.profile.created_at.as_deref().is_some_and(|d| d.ends_with('Z')));
    assert!(report.profile.snapcode_url.as_deref().is_some_and(|u| u.starts_with("https://")));
    assert_eq!(report.profile.category.as_deref(), Some("Product Brand"));
    assert!(report.canonical_url.is_some());

    let highlights = raw["curatedHighlights"].as_array().unwrap();
    assert_eq!(report.highlights.len(), highlights.len());
    for (parsed, source) in report.highlights.iter().zip(highlights) {
        assert_eq!(parsed.title.as_deref(), source["storyTitle"]["value"].as_str());
        assert_eq!(parsed.snaps.len(), source["snapList"].as_array().unwrap().len());
        for snap in &parsed.snaps {
            assert!(snap.media_url.starts_with("https://"));
            assert!(snap.id.len() >= 8);
            assert!(snap.uploaded_at.as_deref().is_some_and(|d| d.ends_with('Z')));
        }
    }

    let details = raw["spotlightStoryMetadata"].as_array().unwrap();
    assert_eq!(report.spotlights.len(), raw["spotlightHighlights"].as_array().unwrap().len());
    for (parsed, source) in report.spotlights.iter().zip(details) {
        let milliseconds: u64 = source["videoMetadata"]["durationMs"].as_str().unwrap().parse().unwrap();
        assert_eq!(parsed.duration_seconds, Some(milliseconds / 1000));
        assert_eq!(parsed.description.as_deref(), source["description"].as_str().map(str::trim).filter(|s| !s.is_empty()));
        assert_eq!(parsed.snaps[0].media_type, MediaType::Video);
        // Snapchat sends -1 for a count it hides.
        let views: i64 = source["engagementStats"]["viewCount"].as_str().unwrap().parse().unwrap();
        assert_eq!(parsed.views, u64::try_from(views).ok());
        assert!(parsed.page_url.as_deref().is_some_and(|u| u.contains("/spotlight/")));
    }

    let lenses = raw["lenses"].as_array().unwrap();
    assert_eq!(report.lenses.len(), lenses.len());
    assert_eq!(report.lenses[0].name.as_deref(), lenses[0]["lensName"].as_str());
    assert!(report.lenses[0].unlock_url.is_some());

    assert_eq!(report.stats.highlights as usize, report.highlights.len());
    assert_eq!(report.stats.highlight_snaps as usize, report.highlights.iter().map(|h| h.snaps.len()).sum::<usize>());
    assert_eq!(report.stats.spotlight_duration_seconds, report.spotlights.iter().filter_map(|s| s.duration_seconds).sum::<u64>());
}

#[test]
fn profile_that_is_not_public() {
    let report = report("nonpublic_profile.html");
    assert!(!report.profile.public);
    assert_eq!(report.profile.username.as_deref(), Some("bitmoji"));
    assert_eq!(report.profile.display_name.as_deref(), Some("Bitmoji Temp"));
    assert!(report.profile.snapcode_url.is_some());
    assert!(report.highlights.is_empty() && report.spotlights.is_empty() && report.lenses.is_empty());
}

#[test]
fn unknown_username() {
    let error = parse::parse_report(&page("not_found.html"), "nobody", "https://www.snapchat.com/add/nobody").unwrap_err();
    assert!(matches!(error, Error::NotFound(name) if name == "nobody"));
}

#[test]
fn location_is_never_read() {
    // Put coordinates and an address into the page; none of it may reach the report.
    let mut page = page("public_profile.html");
    let raw = &mut page["props"]["pageProps"];
    raw["userProfile"]["publicProfileInfo"]["address"] = Value::from("1 Example Street, Exampleville");
    for highlight in raw["curatedHighlights"].as_array_mut().unwrap() {
        for snap in highlight["snapList"].as_array_mut().unwrap() {
            snap["lat"] = Value::from(48.8584);
            snap["lng"] = Value::from(2.2945);
        }
    }
    let report = parse::parse_report(&page, "spectacles", "https://www.snapchat.com/add/spectacles").unwrap();
    let json = export::to_json(&report).unwrap();
    let csv = export::to_csv(&report);
    for text in [&json, &csv] {
        assert!(!text.contains("Exampleville") && !text.contains("48.8584") && !text.contains("2.2945"));
    }
    assert!(!json.contains("\"lat\"") && !json.contains("\"lng\"") && !json.contains("\"address\""));
}

#[test]
fn spotlight_extras_and_comments_without_commenters() {
    // Give the first Spotlight known signals and two comments, written by a commenter whose
    // name, ID and avatar must not reach the report.
    let mut page = page("public_profile.html");
    let raw = &mut page["props"]["pageProps"];
    let snap_id = raw["spotlightHighlights"][0]["snapList"][0]["snapId"]["value"].as_str().unwrap().to_owned();
    let details = &mut raw["spotlightStoryMetadata"][0];
    details["videoMetadata"]["embeddedTextCaption"] = Value::from("TEXT ON THE VIDEO");
    details["s2iTags"] = serde_json::json!(["Technology"]);
    details["llmKeywords"] = serde_json::json!(["smart glasses demo"]);
    details["cuSignals"] = serde_json::json!([
        {"signal": {"$case": "textMetadata", "textMetadata": {"description": "A person demonstrates glasses.", "keywords": []}}},
        {"signal": {"$case": "detectedLanguage", "detectedLanguage": {"languageScores": [{"languageCode": "unk", "score": 0.99}]}}},
        {"signal": {"$case": "detectedLanguage", "detectedLanguage": {"languageScores": [{"languageCode": "fr", "score": 0.4}, {"languageCode": "en", "score": 0.9}]}}},
    ]);
    let comment = |text: &str, reactions: &str, replies: &str| {
        serde_json::json!({
            "snapId": snap_id, "replyText": text, "replyTimestampMs": "1787075948348", "threadedReplyCount": replies,
            "reactCounts": [{"reactTypeId": "1", "reactCount": reactions}, {"reactTypeId": "2", "reactCount": "1"}],
            "replyPosterDisplayName": "Casey Commenter", "replyPosterId": {"highBits": "424242424242", "lowBits": "17"},
            "replyPosterBitmojiAvatarId": "avatar-777-secret", "replyPosterProfileLogoUrl": "https://example.com/casey-face.png",
        })
    };
    let groups = serde_json::json!([[comment("Love this", "4", "3"), comment("  ", "0", "0"), comment("Where can I get one?", "0", "0")], [{"snapId": "someone-else", "replyText": "not ours"}]]);
    raw["encodedSpotlightComments"] = Value::from(groups.to_string());

    let report = parse::parse_report(&page, "spectacles", "https://www.snapchat.com/add/spectacles").unwrap();
    let first = &report.spotlights[0];
    assert_eq!(first.on_screen_text.as_deref(), Some("TEXT ON THE VIDEO"));
    assert_eq!(first.ai_caption.as_deref(), Some("A person demonstrates glasses."));
    assert_eq!(first.language.as_deref(), Some("en"));
    assert_eq!((first.categories.as_slice(), first.search_phrases.as_slice()), (&["Technology".to_owned()][..], &["smart glasses demo".to_owned()][..]));
    assert_eq!(first.top_comments.len(), 2);
    assert_eq!(first.top_comments[0].text, "Love this");
    assert_eq!((first.top_comments[0].reactions, first.top_comments[0].replies), (Some(5), Some(3)));
    assert_eq!(first.top_comments[0].posted_at.as_deref(), Some("2026-08-18T17:59:08Z"));
    assert!(report.spotlights[1..].iter().all(|s| s.top_comments.is_empty()));

    let json = export::to_json(&report).unwrap();
    let csv = export::to_csv(&report);
    for text in [&json, &csv] {
        for secret in ["Casey", "424242424242", "avatar-777-secret", "casey-face"] {
            assert!(!text.contains(secret), "{secret} leaked");
        }
    }
    assert!(csv.contains("TEXT ON THE VIDEO") && csv.contains(",en,"));
}

// --- Export --------------------------------------------------------------------------------

#[test]
fn exports() {
    let report = report("public_profile.html");
    let json = export::to_json(&report).unwrap();
    assert_eq!(serde_json::from_str::<Report>(&json).unwrap(), report);

    let csv = export::to_csv(&report);
    let rows = report.story.len() + report.highlights.iter().map(|h| h.snaps.len()).sum::<usize>() + report.spotlights.len() + report.lenses.len();
    assert!(csv.starts_with("\u{feff}section,group,title,id,type,uploaded_at,"));
    assert_eq!(csv.matches("\r\n").count(), rows + 1);
    assert!(csv.contains("\r\nspotlight,") && csv.contains("\r\nlens,") && csv.contains("\r\nhighlight,"));
}

// --- Self-check ----------------------------------------------------------------------------

#[test]
fn snapshots_match_the_paths() {
    for (name, layout) in [("public_profile.html", "public"), ("nonpublic_profile.html", "private")] {
        let (found, fields) = selfcheck::check_page(&page(name));
        assert_eq!(found, Some(layout));
        let missing: Vec<_> = fields.iter().filter(|f| f.status == Status::Missing).map(|f| format!("{}: {}", f.name, f.detail)).collect();
        assert!(missing.is_empty(), "{name}: {missing:#?}");
        assert!(fields.iter().filter(|f| f.status == Status::Ok).count() >= 5);
    }
}

#[test]
fn a_renamed_field_is_reported() {
    let mut page = page("public_profile.html");
    let info = page["props"]["pageProps"]["userProfile"]["publicProfileInfo"].as_object_mut().unwrap();
    let bio = info.remove("bio").unwrap();
    info.insert("biography".into(), bio);
    let (_, fields) = selfcheck::check_page(&page);
    let field = fields.iter().find(|f| f.name == "bio").unwrap();
    assert_eq!(field.status, Status::Missing);
    assert!(field.detail.contains("biography"), "{}", field.detail);
    let report = selfcheck::summarize(vec![selfcheck::AccountResult { account: "spectacles".into(), layout: Some("public"), error: None, fields }]);
    assert!(!report.healthy);
    assert_eq!(report.missing, 1);
}

// --- A local web server for everything that fetches ------------------------------------------

struct Server {
    base: String,
    requests: Arc<Mutex<Vec<String>>>,
}

async fn serve(routes: HashMap<String, (u16, Vec<u8>)>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = requests.clone();
    let routes = Arc::new(routes);
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else { break };
            let routes = routes.clone();
            let seen = seen.clone();
            tokio::spawn(async move {
                let mut request = Vec::new();
                let mut buffer = [0u8; 2048];
                while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                    match socket.read(&mut buffer).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => request.extend_from_slice(&buffer[..n]),
                    }
                }
                let line = String::from_utf8_lossy(&request);
                let target = line.split_whitespace().nth(1).unwrap_or("/").to_owned();
                let path = target.split('?').next().unwrap_or("/").to_owned();
                seen.lock().unwrap().push(path.clone());
                let (status, body) = routes.get(&path).cloned().unwrap_or((404, b"not found".to_vec()));
                let head = format!("HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = socket.write_all(head.as_bytes()).await;
                let _ = socket.write_all(&body).await;
                let _ = socket.shutdown().await;
            });
        }
    });
    Server { base, requests }
}

fn client(server: &Server) -> Client {
    Client::new(ClientOptions { timeout: Duration::from_secs(10), locale: Some("en-US".into()), base_url: server.base.clone() }).unwrap()
}

#[tokio::test]
async fn lookup_over_http() {
    let server = serve(HashMap::from([
        ("/add/spectacles".to_owned(), (200, fixture("public_profile.html").into_bytes())),
        ("/add/nobody".to_owned(), (404, fixture("not_found.html").into_bytes())),
        ("/add/broken".to_owned(), (200, b"<html>no data here</html>".to_vec())),
    ]))
    .await;
    let client = client(&server);

    let report = client.lookup(" https://www.snapchat.com/add/Spectacles?share_id=x ").await.unwrap();
    assert_eq!(report.requested_username, "spectacles");
    assert!(report.url.ends_with("/add/spectacles?locale=en-US"));
    assert!(!report.highlights.is_empty());

    assert!(matches!(client.lookup("nobody").await, Err(Error::NotFound(_))));
    assert!(matches!(client.lookup("broken").await, Err(Error::PageFormat(_))));
    assert!(matches!(client.lookup("not a name").await, Err(Error::InvalidUsername(_))));
    assert!(matches!(client.get("http://example.com/elsewhere.mp4").await, Err(Error::BadUrl(_))));
}

fn media(server: &Server, name: &str, category: &str, uploaded_at: Option<&str>) -> DownloadItem {
    DownloadItem { url: format!("{}/media/{name}", server.base), ext: "mp4".into(), category: category.into(), uploaded_at: uploaded_at.map(str::to_owned) }
}

#[tokio::test]
async fn downloads_are_recorded_resumed_and_verified() {
    let server = serve(HashMap::from([
        ("/media/aaaaaaaa1.mp4".to_owned(), (200, b"first video".to_vec())),
        ("/media/bbbbbbbb2.mp4".to_owned(), (200, b"second video".to_vec())),
        ("/media/gone0000.mp4".to_owned(), (404, Vec::new())),
    ]))
    .await;
    let client = client(&server);
    let folder = tempfile::tempdir().unwrap();
    let never = AtomicBool::new(false);
    let items = vec![
        media(&server, "aaaaaaaa1.mp4", "story", Some("2026-08-18T17:59:08Z")),
        media(&server, "bbbbbbbb2.mp4", "spotlight", None),
        media(&server, "aaaaaaaa1.mp4", "story", None), // the same file twice is fetched once
        media(&server, "gone0000.mp4", "lens", None),
    ];

    // First run: two saved, one failed, each reported.
    let outcomes = Mutex::new(Vec::new());
    let manager = DownloadManager::open(folder.path(), "tester").unwrap();
    let summary = manager.run(&client, items.clone(), 4, &never, |p| outcomes.lock().unwrap().push((p.file, p.outcome))).await.unwrap();
    assert_eq!((summary.total, summary.downloaded, summary.failed, summary.skipped), (3, 2, 1, 0));
    assert_eq!(summary.errors.len(), 1);
    assert_eq!(outcomes.lock().unwrap().len(), 3);
    let story = folder.path().join("story_aaaaaaaa1.mp4");
    assert_eq!(fs::read(&story).unwrap(), b"first video");
    assert!(folder.path().join("spotlight_bbbbbbbb2.mp4").exists());
    assert!(!folder.path().join("lens_gone0000.mp4").exists());
    assert!(fs::read_dir(folder.path()).unwrap().flatten().all(|f| !f.file_name().to_string_lossy().ends_with(".part")));

    // The file's time is the upload time, and the manifest has what's needed to verify it later.
    let modified = fs::metadata(&story).unwrap().modified().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    assert_eq!(modified, 1787075948);
    let manifest: Value = serde_json::from_str(&fs::read_to_string(folder.path().join(MANIFEST)).unwrap()).unwrap();
    let entry = manifest["files"].as_array().unwrap().iter().find(|f| f["file"] == "story_aaaaaaaa1.mp4").unwrap();
    assert_eq!(entry["sha256"], downloads::sha256_file(&story).unwrap().as_str());
    assert_eq!((entry["bytes"].as_u64(), entry["username"].as_str(), entry["category"].as_str()), (Some(11), Some("tester"), Some("story")));
    assert_eq!(entry["media_id"], "aaaaaaaa1");
    assert!(entry["source_url"].as_str().unwrap().ends_with("/media/aaaaaaaa1.mp4"));
    assert!(downloads::verify(folder.path()).unwrap().intact());

    // Second run: nothing already saved is fetched again.
    let before = server.requests.lock().unwrap().len();
    let manager = DownloadManager::open(folder.path(), "tester").unwrap();
    let summary = manager.run(&client, items[..2].to_vec(), 4, &never, |_| {}).await.unwrap();
    assert_eq!((summary.downloaded, summary.skipped), (0, 2));
    assert_eq!(server.requests.lock().unwrap().len(), before);

    // A file edited after download is reported, left alone, and the fresh copy saved beside it.
    fs::write(&story, b"edited afterwards").unwrap();
    let check = downloads::verify(folder.path()).unwrap();
    assert_eq!((check.altered.as_slice(), check.ok), (&["story_aaaaaaaa1.mp4".to_owned()][..], 1));
    let manager = DownloadManager::open(folder.path(), "tester").unwrap();
    let outcomes = Mutex::new(Vec::new());
    let summary = manager.run(&client, items[..1].to_vec(), 1, &never, |p| outcomes.lock().unwrap().push((p.file, p.outcome))).await.unwrap();
    assert_eq!(summary.altered_on_disk, ["story_aaaaaaaa1.mp4"]);
    assert_eq!(fs::read(&story).unwrap(), b"edited afterwards");
    let (beside, outcome) = outcomes.lock().unwrap()[0].clone();
    assert_eq!(outcome, Outcome::SavedBeside);
    assert!(beside.starts_with("story_aaaaaaaa1-") && beside.ends_with(".mp4"), "{beside}");
    assert_eq!(fs::read(folder.path().join(&beside)).unwrap(), b"first video");

    // A deleted file is reported missing and fetched again.
    fs::remove_file(folder.path().join("spotlight_bbbbbbbb2.mp4")).unwrap();
    assert_eq!(downloads::verify(folder.path()).unwrap().missing, ["spotlight_bbbbbbbb2.mp4"]);
    let manager = DownloadManager::open(folder.path(), "tester").unwrap();
    assert_eq!(manager.run(&client, items[1..2].to_vec(), 1, &never, |_| {}).await.unwrap().downloaded, 1);

    // A media file nobody recorded is pointed out.
    fs::write(folder.path().join("stray.mp4"), b"?").unwrap();
    assert_eq!(downloads::verify(folder.path()).unwrap().untracked, ["stray.mp4"]);
}

#[tokio::test]
async fn a_cancelled_download_saves_nothing_new() {
    let server = serve(HashMap::from([("/media/cccccccc3.mp4".to_owned(), (200, b"third".to_vec()))])).await;
    let folder = tempfile::tempdir().unwrap();
    let cancelled = AtomicBool::new(true);
    let manager = DownloadManager::open(folder.path(), "tester").unwrap();
    let outcomes = Mutex::new(Vec::new());
    let summary = manager
        .run(&client(&server), vec![media(&server, "cccccccc3.mp4", "story", None)], 2, &cancelled, |p| outcomes.lock().unwrap().push(p.outcome))
        .await
        .unwrap();
    assert!(summary.cancelled && summary.downloaded == 0);
    assert_eq!(outcomes.lock().unwrap().as_slice(), [Outcome::Cancelled]);
    assert!(server.requests.lock().unwrap().is_empty());
    assert!(!cancelled.load(Ordering::Relaxed) || fs::read_dir(folder.path()).unwrap().count() == 0);
    assert!(!downloads::verify(folder.path()).unwrap().has_manifest);
}

/// A video as the page links it: `/d/<id>.<version>.IRZXSOY` with the page's parameters.
fn video(server: &Server, id: &str, version: u32, category: &str) -> DownloadItem {
    DownloadItem { url: format!("{}/d/{id}.{version}.IRZXSOY?mo=abc&uc=46", server.base), ext: "mp4".into(), category: category.into(), uploaded_at: None }
}

#[tokio::test]
async fn videos_are_saved_without_the_watermark() {
    // The server has two versions of one Spotlight (27 carries the watermark), only the watermarked
    // one of another, and one clean highlight video. A photo sits beside them.
    let server = serve(HashMap::from([
        ("/d/spotlight1.27.IRZXSOY".to_owned(), (200, b"spotlight one, with watermark".to_vec())),
        ("/d/spotlight1.1034.IRZXSOY".to_owned(), (200, b"spotlight one, clean".to_vec())),
        ("/d/spotlight2.27.IRZXSOY".to_owned(), (200, b"spotlight two, with watermark".to_vec())),
        ("/d/highlight1.1322.IRZXSOY".to_owned(), (200, b"highlight, clean".to_vec())),
        ("/d/highlight1.27.IRZXSOY".to_owned(), (200, b"highlight, larger, with watermark".to_vec())),
        ("/d/photo0001.400.IRZXSOY".to_owned(), (200, b"photo".to_vec())),
        // Snapchat serves some Spotlights from another server, in a folder with another name.
        ("/y/spotlight3.27.IRZXSOY".to_owned(), (200, b"spotlight three, with watermark".to_vec())),
        ("/y/spotlight3.1034.IRZXSOY".to_owned(), (200, b"spotlight three, clean".to_vec())),
    ]))
    .await;
    let client = client(&server);
    let never = AtomicBool::new(false);
    let photo =
        DownloadItem { url: format!("{}/d/photo0001.400.IRZXSOY?mo=abc", server.base), ext: "jpg".into(), category: "highlight".into(), uploaded_at: None };
    let items = vec![
        video(&server, "spotlight1", 27, "spotlight"),
        video(&server, "spotlight2", 27, "spotlight"),
        video(&server, "highlight1", 1322, "highlight"),
        photo,
    ];
    let source = |folder: &std::path::Path, file: &str| {
        let manifest: Value = serde_json::from_str(&fs::read_to_string(folder.join(MANIFEST)).unwrap()).unwrap();
        manifest["files"].as_array().unwrap().iter().find(|f| f["file"] == file).unwrap()["source_url"].as_str().unwrap().to_owned()
    };

    // Asked for nothing in particular, the manager takes the versions without the watermark.
    let folder = tempfile::tempdir().unwrap();
    let summary = DownloadManager::open(folder.path(), "tester").unwrap().run(&client, items.clone(), 1, &never, |_| {}).await.unwrap();
    assert_eq!((summary.downloaded, summary.failed, summary.replaced), (4, 0, 0));
    assert_eq!(fs::read(folder.path().join("spotlight_spotlight1.mp4")).unwrap(), b"spotlight one, clean");
    assert!(source(folder.path(), "spotlight_spotlight1.mp4").ends_with("/d/spotlight1.1034.IRZXSOY"));
    assert_eq!(fs::read(folder.path().join("highlight_highlight1.mp4")).unwrap(), b"highlight, clean");
    // A video the server only has with the watermark is still saved, and pointed out.
    assert_eq!(fs::read(folder.path().join("spotlight_spotlight2.mp4")).unwrap(), b"spotlight two, with watermark");
    assert_eq!(summary.watermarked, ["spotlight_spotlight2.mp4"]);
    // Videos already linked without the watermark, and photos, cost no extra request.
    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.iter().filter(|path| path.contains("highlight1")).count(), 1);
    assert_eq!(requests.iter().filter(|path| path.contains("photo0001")).count(), 1);
    assert!(!requests.iter().any(|path| path.ends_with("/d/spotlight1.27.IRZXSOY")), "the watermarked version was fetched: {requests:?}");

    // A video in another folder of the server is handled the same way.
    let elsewhere = DownloadItem {
        url: format!("{}/y/spotlight3.27.IRZXSOY?mo=abc&uc=46", server.base),
        ext: "mp4".into(),
        category: "spotlight".into(),
        uploaded_at: None,
    };
    let other = tempfile::tempdir().unwrap();
    let summary = DownloadManager::open(other.path(), "tester").unwrap().run(&client, vec![elsewhere.clone()], 1, &never, |_| {}).await.unwrap();
    assert_eq!((summary.downloaded, summary.watermarked.len()), (1, 0));
    assert_eq!(fs::read(other.path().join("spotlight_spotlight3.mp4")).unwrap(), b"spotlight three, clean");
    assert!(source(other.path(), "spotlight_spotlight3.mp4").ends_with("/y/spotlight3.1034.IRZXSOY"));
    // A copy saved with the watermark by the version that missed such addresses is replaced.
    let old = tempfile::tempdir().unwrap();
    let summary = DownloadManager::open(old.path(), "tester")
        .unwrap()
        .prefer(VideoVersion::Sharpest)
        .run(&client, vec![elsewhere.clone()], 1, &never, |_| {})
        .await
        .unwrap();
    assert_eq!(summary.downloaded, 1);
    assert_eq!(fs::read(old.path().join("spotlight_spotlight3.mp4")).unwrap(), b"spotlight three, with watermark");
    let summary = DownloadManager::open(old.path(), "tester").unwrap().run(&client, vec![elsewhere], 1, &never, |_| {}).await.unwrap();
    assert_eq!(summary.replaced, 1);
    assert_eq!(fs::read(old.path().join("spotlight_spotlight3.mp4")).unwrap(), b"spotlight three, clean");

    // A second run fetches nothing: every file is the version asked for, or the only one there is.
    let summary = DownloadManager::open(folder.path(), "tester").unwrap().run(&client, items.clone(), 2, &never, |_| {}).await.unwrap();
    assert_eq!((summary.downloaded, summary.replaced, summary.skipped), (0, 0, 4));
    assert_eq!(summary.watermarked, ["spotlight_spotlight2.mp4"]);

    // Asked for the largest versions, it takes the watermarked ones where they exist...
    let sharp = tempfile::tempdir().unwrap();
    let summary =
        DownloadManager::open(sharp.path(), "tester").unwrap().prefer(VideoVersion::Sharpest).run(&client, items.clone(), 2, &never, |_| {}).await.unwrap();
    assert_eq!(summary.downloaded, 4);
    assert!(summary.watermarked.is_empty());
    assert_eq!(fs::read(sharp.path().join("spotlight_spotlight1.mp4")).unwrap(), b"spotlight one, with watermark");
    assert_eq!(fs::read(sharp.path().join("highlight_highlight1.mp4")).unwrap(), b"highlight, larger, with watermark");
    assert!(source(sharp.path(), "highlight_highlight1.mp4").ends_with("/d/highlight1.27.IRZXSOY"));

    // ...and a folder saved that way is brought to the clean versions by a later run: unchanged files
    // are replaced, a file that was edited is left alone with the clean copy beside it.
    fs::write(sharp.path().join("highlight_highlight1.mp4"), b"edited afterwards").unwrap();
    let outcomes = Mutex::new(Vec::new());
    let summary = DownloadManager::open(sharp.path(), "tester")
        .unwrap()
        .run(&client, items.clone(), 1, &never, |p| outcomes.lock().unwrap().push((p.file, p.outcome)))
        .await
        .unwrap();
    assert_eq!((summary.replaced, summary.saved_beside, summary.skipped), (1, 1, 2));
    assert_eq!(fs::read(sharp.path().join("spotlight_spotlight1.mp4")).unwrap(), b"spotlight one, clean");
    assert!(outcomes.lock().unwrap().contains(&("spotlight_spotlight1.mp4".to_owned(), Outcome::Replaced)));
    assert_eq!(fs::read(sharp.path().join("highlight_highlight1.mp4")).unwrap(), b"edited afterwards");
    assert_eq!(summary.altered_on_disk, ["highlight_highlight1.mp4"]);
    assert_eq!(summary.watermarked, ["spotlight_spotlight2.mp4"]);
    let check = downloads::verify(sharp.path()).unwrap();
    assert_eq!(check.altered, ["highlight_highlight1.mp4"]);
    assert!(source(sharp.path(), "spotlight_spotlight1.mp4").ends_with("/d/spotlight1.1034.IRZXSOY"));
}

#[tokio::test]
async fn bitmoji_looks_and_captions() {
    let avatar = |v: u32| format!("/3d/render/10226021-99120672960_{v}-s5-v1.webp");
    let server = serve(HashMap::from([
        (avatar(1), (200, b"look A".to_vec())),
        (avatar(2), (200, b"look A".to_vec())),
        (avatar(3), (200, b"look B".to_vec())),
        ("/captions.vtt".to_owned(), (200, b"WEBVTT\n\n00:00.000 --> 00:01.500\nHello\n".to_vec())),
    ]))
    .await;
    let client = client(&server);
    let looks = snapintel_core::bitmoji::history(&client, &format!("{}{}", server.base, avatar(3)), 4).await.unwrap();
    assert_eq!(looks.iter().map(|l| l.version).collect::<Vec<_>>(), [3, 2]);
    assert_ne!(looks[0].md5, looks[1].md5);

    let cues = snapintel_core::transcript::fetch(&client, &format!("{}/captions.vtt", server.base)).await.unwrap();
    assert_eq!((cues.len(), cues[0].text.as_str(), cues[0].end), (1, "Hello", 1.5));
}
