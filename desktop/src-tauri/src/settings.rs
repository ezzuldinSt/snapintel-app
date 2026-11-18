//! Preferences, kept as JSON in the app's configuration folder.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use snapintel_core::media::VideoVersion;
use tauri::{AppHandle, Manager};

pub const THEMES: [&str; 3] = ["system", "light", "dark"];
const MAX_RECENT: usize = 10;

/// A profile that was looked up, for the start screen's shortcuts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recent {
    pub username: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub picture_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Where downloads go. `None` means "SnapIntel" in the user's Downloads folder.
    pub download_dir: Option<String>,
    /// Give each account its own folder inside `download_dir`.
    pub folder_per_account: bool,
    pub concurrency: u8,
    /// Which version of a video to download when Snapchat has two: without its watermark, or the largest.
    pub video_version: VideoVersion,
    pub timeout_secs: u64,
    /// The language Snapchat writes page titles in, such as `en-US`. Empty: let Snapchat choose.
    pub locale: String,
    pub theme: String,
    pub remember_recent: bool,
    pub recent: Vec<Recent>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_dir: None,
            folder_per_account: true,
            concurrency: 6,
            video_version: VideoVersion::Clean,
            timeout_secs: 30,
            locale: "en-US".into(),
            theme: "system".into(),
            remember_recent: true,
            recent: Vec::new(),
        }
    }
}

fn valid_locale(locale: &str) -> bool {
    locale.is_empty() || (locale.len() <= 12 && locale.split('-').all(|part| (2..=8).contains(&part.len()) && part.chars().all(|c| c.is_ascii_alphanumeric())))
}

impl Settings {
    /// Brings every value into its allowed range, whatever the file or the interface sent.
    pub fn sanitized(mut self) -> Self {
        self.concurrency = self.concurrency.clamp(1, 16);
        self.timeout_secs = self.timeout_secs.clamp(5, 120);
        if !THEMES.contains(&self.theme.as_str()) {
            self.theme = "system".into();
        }
        if !valid_locale(&self.locale) {
            self.locale = "en-US".into();
        }
        self.download_dir = self.download_dir.map(|dir| dir.trim().to_owned()).filter(|dir| !dir.is_empty());
        if !self.remember_recent {
            self.recent.clear();
        }
        self.recent.retain(|entry| snapintel_core::normalize_username(&entry.username).is_ok_and(|name| name == entry.username));
        self.recent.truncate(MAX_RECENT);
        self
    }

    pub fn remember(&mut self, entry: Recent) {
        if !self.remember_recent {
            return;
        }
        self.recent.retain(|other| other.username != entry.username);
        self.recent.insert(0, entry);
        self.recent.truncate(MAX_RECENT);
    }
}

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Settings {
    file(app).and_then(|path| fs::read_to_string(path).ok()).and_then(|text| serde_json::from_str::<Settings>(&text).ok()).unwrap_or_default().sanitized()
}

pub fn save(app: &AppHandle, settings: &Settings) -> std::io::Result<()> {
    let Some(path) = file(app) else { return Ok(()) };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
    let partial = path.with_extension("json.part");
    fs::write(&partial, text)?;
    fs::rename(partial, path)
}

/// "SnapIntel" inside the user's Downloads folder.
pub fn default_download_dir(app: &AppHandle) -> PathBuf {
    app.path().download_dir().or_else(|_| app.path().home_dir()).unwrap_or_else(|_| std::env::temp_dir()).join("SnapIntel")
}

pub fn download_root(app: &AppHandle, settings: &Settings) -> PathBuf {
    settings.download_dir.as_ref().map(PathBuf::from).unwrap_or_else(|| default_download_dir(app))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_clamped() {
        let settings =
            Settings { concurrency: 200, timeout_secs: 0, theme: "neon".into(), locale: "../x".into(), download_dir: Some("  ".into()), ..Default::default() }
                .sanitized();
        assert_eq!((settings.concurrency, settings.timeout_secs), (16, 5));
        assert_eq!((settings.theme.as_str(), settings.locale.as_str()), ("system", "en-US"));
        assert_eq!(settings.download_dir, None);
    }

    #[test]
    fn recent_lookups() {
        let entry = |name: &str| Recent { username: name.into(), display_name: None, picture_url: None };
        let mut settings = Settings::default();
        for name in ["a1", "b2", "a1"] {
            settings.remember(entry(name));
        }
        assert_eq!(settings.recent.iter().map(|r| r.username.as_str()).collect::<Vec<_>>(), ["a1", "b2"]);
        settings.remember_recent = false;
        assert!(settings.sanitized().recent.is_empty());
    }
}
