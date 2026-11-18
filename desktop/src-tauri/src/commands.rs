//! What the window can ask the program to do. Each function here is called from `ui/app.js`.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::Serialize;
use snapintel_core::downloads::{self, DownloadItem, DownloadManager, Summary, VerifyReport};
use snapintel_core::selfcheck::{self, SelfCheckReport};
use snapintel_core::transcript::{self, Cue};
use snapintel_core::{bitmoji, export, normalize_username, Client, ClientOptions, Report};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::settings::{self, Recent, Settings};
use crate::AppState;

/// An error the interface can show: `kind` picks the wording and actions, `message` is the detail.
#[derive(Debug, Clone, Serialize)]
pub struct UiError {
    pub kind: String,
    pub message: String,
}

impl UiError {
    fn new(kind: &str, message: impl Into<String>) -> Self {
        Self { kind: kind.to_owned(), message: message.into() }
    }
}

impl From<snapintel_core::Error> for UiError {
    fn from(error: snapintel_core::Error) -> Self {
        Self::new(error.kind(), error.to_string())
    }
}

impl From<std::io::Error> for UiError {
    fn from(error: std::io::Error) -> Self {
        Self::new("io", error.to_string())
    }
}

type Answer<T> = Result<T, UiError>;

/// The settings, plus what the interface can't work out for itself.
#[derive(Debug, Clone, Serialize)]
pub struct SettingsView {
    #[serde(flatten)]
    pub settings: Settings,
    pub download_root: String,
    pub default_download_root: String,
    pub version: &'static str,
}

fn view(app: &AppHandle, settings: &Settings) -> SettingsView {
    SettingsView {
        settings: settings.clone(),
        download_root: settings::download_root(app, settings).display().to_string(),
        default_download_root: settings::default_download_dir(app).display().to_string(),
        version: snapintel_core::VERSION,
    }
}

fn current(state: &State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
}

fn client(settings: &Settings) -> Answer<Client> {
    let locale = (!settings.locale.is_empty()).then(|| settings.locale.clone());
    Ok(Client::new(ClientOptions { timeout: Duration::from_secs(settings.timeout_secs), locale, ..Default::default() })?)
}

/// The folder an account's files go to.
fn account_dir(app: &AppHandle, settings: &Settings, username: &str) -> Answer<PathBuf> {
    let username = normalize_username(username)?;
    let root = settings::download_root(app, settings);
    Ok(if settings.folder_per_account { root.join(username) } else { root })
}

pub fn apply_theme(app: &AppHandle, theme: &str) {
    let theme = match theme {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        _ => None,
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_theme(theme);
    }
}

#[tauri::command]
pub async fn lookup(app: AppHandle, state: State<'_, AppState>, username: String) -> Answer<Report> {
    let settings = current(&state);
    let report = client(&settings)?.lookup(&username).await?;
    *state.report.lock().unwrap_or_else(|p| p.into_inner()) = Some(report.clone());
    if settings.remember_recent {
        let entry = Recent {
            username: report.profile.username.clone().unwrap_or_else(|| report.requested_username.clone()),
            display_name: report.profile.display_name.clone(),
            picture_url: report.profile.profile_picture_url.clone().or_else(|| report.profile.avatar_url.clone()),
        };
        let updated = {
            let mut guard = state.settings.lock().unwrap_or_else(|p| p.into_inner());
            guard.remember(entry);
            guard.clone()
        };
        let _ = settings::save(&app, &updated);
    }
    Ok(report)
}

#[tauri::command]
pub fn get_settings(app: AppHandle, state: State<'_, AppState>) -> SettingsView {
    view(&app, &current(&state))
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Answer<SettingsView> {
    let settings = settings.sanitized();
    *state.settings.lock().unwrap_or_else(|p| p.into_inner()) = settings.clone();
    settings::save(&app, &settings)?;
    apply_theme(&app, &settings.theme);
    Ok(view(&app, &settings))
}

/// Asks for a folder. Returns `None` when the dialog is cancelled; the choice isn't saved here.
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Answer<Option<String>> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog().file().set_title("Choose where SnapIntel saves downloads").pick_folder(move |folder| {
        let _ = sender.send(folder);
    });
    let folder = receiver.await.map_err(|_| UiError::new("other", "The folder dialog closed unexpectedly"))?;
    Ok(folder.and_then(|path| path.into_path().ok()).map(|path| path.display().to_string()))
}

#[tauri::command]
pub async fn download(app: AppHandle, state: State<'_, AppState>, username: String, items: Vec<DownloadItem>) -> Answer<Summary> {
    if state.downloading.swap(true, Ordering::SeqCst) {
        return Err(UiError::new("busy", "A download is already running"));
    }
    let result = async {
        let settings = current(&state);
        let directory = account_dir(&app, &settings, &username)?;
        let manager = DownloadManager::open(&directory, &normalize_username(&username)?)?.prefer(settings.video_version);
        state.cancel.store(false, Ordering::SeqCst);
        let progress = app.clone();
        let summary = manager
            .run(&client(&settings)?, items, settings.concurrency as usize, &state.cancel, move |update| {
                let _ = progress.emit("download-progress", update);
            })
            .await?;
        Ok(summary)
    }
    .await;
    state.downloading.store(false, Ordering::SeqCst);
    result
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn verify_downloads(app: AppHandle, state: State<'_, AppState>, username: String) -> Answer<VerifyReport> {
    let directory = account_dir(&app, &current(&state), &username)?;
    tauri::async_runtime::spawn_blocking(move || downloads::verify(&directory)).await.map_err(|e| UiError::new("other", e.to_string()))?.map_err(UiError::from)
}

/// Saves an account's Snapcode into its download folder and returns the file's path.
#[tauri::command]
pub async fn save_snapcode(app: AppHandle, state: State<'_, AppState>, username: String, format: String) -> Answer<String> {
    let username = normalize_username(&username)?;
    let (kind, ext) = if format.eq_ignore_ascii_case("png") { ("PNG", "png") } else { ("SVG", "svg") };
    let settings = current(&state);
    let url = format!("https://gcp.api.snapchat.com/web/deeplink/snapcode?username={username}&type={kind}&bitmoji=enable");
    let content = client(&settings)?.bytes(&url).await?;
    let manager = DownloadManager::open(account_dir(&app, &settings, &username)?, &username)?;
    let item = DownloadItem { url, ext: ext.into(), category: "snapcode".into(), uploaded_at: None };
    let name = manager.store(&format!("snapcode_{username}.{ext}"), &content, &item)?;
    manager.save()?;
    Ok(manager.directory().join(name).display().to_string())
}

/// Writes the last report as JSON or CSV where the user chooses. Returns the path, or `None` if cancelled.
#[tauri::command]
pub async fn export_report(app: AppHandle, state: State<'_, AppState>, format: String) -> Answer<Option<String>> {
    let report = state.report.lock().unwrap_or_else(|p| p.into_inner()).clone().ok_or_else(|| UiError::new("other", "Look a profile up first"))?;
    let csv = format.eq_ignore_ascii_case("csv");
    let (label, ext) = if csv { ("CSV spreadsheet", "csv") } else { ("JSON", "json") };
    let name = report.profile.username.clone().unwrap_or_else(|| report.requested_username.clone());
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog().file().set_title("Export profile").set_file_name(format!("snapintel-{name}.{ext}")).add_filter(label, &[ext]).save_file(move |path| {
        let _ = sender.send(path);
    });
    let chosen = receiver.await.map_err(|_| UiError::new("other", "The save dialog closed unexpectedly"))?;
    let Some(path) = chosen.and_then(|path| path.into_path().ok()) else { return Ok(None) };
    let text = if csv { export::to_csv(&report) } else { export::to_json(&report)? };
    std::fs::write(&path, text)?;
    state.exports.lock().unwrap_or_else(|p| p.into_inner()).push(path.clone());
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
pub async fn transcript(state: State<'_, AppState>, url: String) -> Answer<Vec<Cue>> {
    Ok(transcript::fetch(&client(&current(&state))?, &url).await?)
}

#[tauri::command]
pub async fn bitmoji_history(state: State<'_, AppState>, avatar_url: String) -> Answer<Vec<bitmoji::Look>> {
    let settings = current(&state);
    Ok(bitmoji::history(&client(&settings)?, &avatar_url, settings.concurrency as usize).await?)
}

#[tauri::command]
pub async fn self_check(state: State<'_, AppState>) -> Answer<SelfCheckReport> {
    Ok(selfcheck::run(&client(&current(&state))?, &selfcheck::REFERENCE_ACCOUNTS).await)
}

/// Opens a web link in the default browser. Anything that isn't http(s) is refused.
#[tauri::command]
pub fn open_link(app: AppHandle, url: String) -> Answer<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(UiError::new("bad_url", "Only web links can be opened"));
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| UiError::new("other", e.to_string()))
}

fn inside(path: &Path, root: &Path) -> bool {
    match (path.canonicalize(), root.canonicalize()) {
        (Ok(path), Ok(root)) => path.starts_with(root),
        _ => false,
    }
}

/// Shows a download folder, or a file SnapIntel saved, in the file manager. Other paths are refused.
#[tauri::command]
pub fn show_in_folder(app: AppHandle, state: State<'_, AppState>, path: String) -> Answer<()> {
    let path = PathBuf::from(path);
    let root = settings::download_root(&app, &current(&state));
    let exported = state.exports.lock().unwrap_or_else(|p| p.into_inner()).iter().any(|saved| saved == &path);
    if !exported && !inside(&path, &root) {
        return Err(UiError::new("other", "That isn't a file or folder SnapIntel saved"));
    }
    let result = if path.is_dir() { app.opener().open_path(path.display().to_string(), None::<&str>) } else { app.opener().reveal_item_in_dir(&path) };
    result.map_err(|e| UiError::new("other", e.to_string()))
}

/// The folder an account's downloads go to, whether or not it exists yet.
#[tauri::command]
pub fn account_folder(app: AppHandle, state: State<'_, AppState>, username: String) -> Answer<String> {
    Ok(account_dir(&app, &current(&state), &username)?.display().to_string())
}

/// A username given on the command line (`snapintel nba`), to open straight onto that profile.
#[tauri::command]
pub fn startup_username() -> Option<String> {
    std::env::args().nth(1).and_then(|argument| normalize_username(&argument).ok())
}
