//! Downloads you can verify later and resume.
//!
//! Every saved file is recorded in `snapintel-manifest.json` in its folder: where it came from,
//! when it was downloaded, its SHA-256 and size, what it is and, when Snapchat gives one, its upload
//! date, which also becomes the file's modification time. The format is the one the Python tool
//! writes, so either can continue a folder the other started.
//!
//! Files are named `<kind>_<media id>.<ext>`, so a later run recognises them: a file whose hash still
//! matches the manifest is skipped without touching the network. A file that no longer matches is
//! never overwritten; the fresh copy is saved beside it as `<name>-<first 8 hex of its hash>.<ext>`.
//!
//! Snapchat keeps two versions of most videos, one of them with its watermark (see `media`). The
//! manager downloads the version it was told to prefer. An unchanged file saved earlier as the other
//! version is replaced by the preferred one, since nothing but the choice of version is lost.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::{Error, Result};
use crate::fetch::Client;
use crate::media::{self, VideoVersion};
use crate::util::{hex, media_id, now_iso, secs_from_iso};

pub const MANIFEST: &str = "snapintel-manifest.json";
const KINDS: &[&str] = &["story", "highlight", "spotlight", "lens", "bitmoji", "snapcode", "profile"];
const EXTENSIONS: &[&str] = &["mp4", "jpg", "png", "webp", "svg"];
const MEDIA_EXTENSIONS: &[&str] = &["mp4", "jpg", "jpeg", "png", "webp"];

/// One file to fetch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadItem {
    pub url: String,
    /// `mp4`, `jpg`, `png`, `webp` or `svg`.
    pub ext: String,
    /// `story`, `highlight`, `spotlight`, `lens`, `bitmoji`, `snapcode` or `profile`.
    pub category: String,
    #[serde(default)]
    pub uploaded_at: Option<String>,
}

impl DownloadItem {
    /// The name the file is saved under, or an error if the item isn't one SnapIntel would create.
    pub fn file_name(&self) -> Result<String> {
        if !KINDS.contains(&self.category.as_str()) || !EXTENSIONS.contains(&self.ext.as_str()) {
            return Err(Error::Other(format!("unsupported download ({} .{})", self.category, self.ext)));
        }
        Ok(format!("{}_{}.{}", self.category, media_id(&self.url), self.ext))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub file: String,
    pub sha256: String,
    pub bytes: u64,
    pub category: String,
    pub username: String,
    pub media_id: String,
    pub source_url: String,
    pub uploaded_at: Option<String>,
    pub downloaded_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ManifestFile {
    snapintel_version: String,
    updated_at: String,
    files: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Downloaded,
    /// Saved by an earlier run and unchanged since.
    Skipped,
    /// The same file was already there but not in the manifest; it is now.
    AlreadyPresent,
    /// The file on disk had been changed, so the download was saved beside it.
    SavedBeside,
    /// An unchanged video saved earlier was the other version; the preferred one took its place.
    Replaced,
    Failed,
    Cancelled,
}

/// Sent after each file.
#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
    pub file: String,
    pub outcome: Outcome,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Summary {
    pub directory: String,
    pub manifest: String,
    pub total: usize,
    pub downloaded: u32,
    pub skipped: u32,
    pub already_present: u32,
    pub saved_beside: u32,
    /// Videos saved earlier as the other version and now replaced by the preferred one.
    pub replaced: u32,
    pub failed: u32,
    pub cancelled: bool,
    /// Bytes fetched in this run.
    pub bytes: u64,
    /// Files that had been changed since they were downloaded. They are left as they are.
    pub altered_on_disk: Vec<String>,
    /// Videos that carry Snapchat's watermark although the version without it was asked for,
    /// because the server has no other version of them.
    pub watermarked: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Default)]
struct State {
    entries: BTreeMap<String, ManifestEntry>,
    summary: Summary,
}

pub struct DownloadManager {
    directory: PathBuf,
    username: String,
    video: VideoVersion,
    state: Mutex<State>,
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn load_manifest(directory: &Path) -> BTreeMap<String, ManifestEntry> {
    let Ok(text) = fs::read_to_string(directory.join(MANIFEST)) else { return BTreeMap::new() };
    match serde_json::from_str::<ManifestFile>(&text) {
        Ok(manifest) => manifest.files.into_iter().map(|entry| (entry.file.clone(), entry)).collect(),
        Err(_) => BTreeMap::new(),
    }
}

fn set_modified(path: &Path, uploaded_at: Option<&str>) {
    let Some(secs) = uploaded_at.and_then(secs_from_iso).and_then(|s| u64::try_from(s).ok()) else { return };
    if let Ok(file) = fs::File::options().write(true).open(path) {
        let _ = file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(secs));
    }
}

impl DownloadManager {
    /// Opens (and creates) a download folder and reads its manifest, if it has one.
    pub fn open(directory: impl Into<PathBuf>, username: &str) -> Result<Self> {
        let directory = directory.into();
        fs::create_dir_all(&directory)?;
        let state = State { entries: load_manifest(&directory), ..Default::default() };
        Ok(Self { directory, username: username.to_owned(), video: VideoVersion::default(), state: Mutex::new(state) })
    }

    /// Which version of a video to download when Snapchat has two. Without this, the one with no watermark.
    pub fn prefer(mut self, video: VideoVersion) -> Self {
        self.video = video;
        self
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Fetches the items, at most `concurrency` at a time, calling `on_progress` after each one.
    /// Setting `cancel` stops it; files already saved stay, and the manifest is still written.
    pub async fn run<F>(&self, client: &Client, items: Vec<DownloadItem>, concurrency: usize, cancel: &AtomicBool, on_progress: F) -> Result<Summary>
    where
        F: Fn(Progress) + Send + Sync,
    {
        // The same media can appear twice (a Spotlight also pinned as a highlight, say): fetch it once.
        let mut names = HashSet::new();
        let mut urls = HashSet::new();
        let mut queue = Vec::new();
        for item in items {
            let name = item.file_name()?;
            if urls.insert(item.url.clone()) && names.insert(name.clone()) {
                queue.push((name, item));
            }
        }
        let total = queue.len();
        let done = AtomicUsize::new(0);
        let on_progress = &on_progress;
        let done = &done;
        futures_util::stream::iter(queue)
            .for_each_concurrent(concurrency.clamp(1, 32), |(name, item)| async move {
                let (file, outcome, bytes) = if cancel.load(Ordering::Relaxed) {
                    (name, Outcome::Cancelled, 0)
                } else {
                    match self.fetch_one(client, &name, &item, cancel).await {
                        Ok(result) => result,
                        Err(error) => {
                            let mut state = self.lock();
                            state.summary.failed += 1;
                            state.summary.errors.push(format!("{name}: {error}"));
                            (name, Outcome::Failed, 0)
                        }
                    }
                };
                on_progress(Progress { done: done.fetch_add(1, Ordering::Relaxed) + 1, total, file, outcome, bytes });
            })
            .await;
        self.save()?;
        let mut state = self.lock();
        state.summary.total = total;
        state.summary.cancelled = cancel.load(Ordering::Relaxed);
        state.summary.directory = self.directory.display().to_string();
        state.summary.manifest = self.directory.join(MANIFEST).display().to_string();
        Ok(state.summary.clone())
    }

    async fn fetch_one(&self, client: &Client, name: &str, item: &DownloadItem, cancel: &AtomicBool) -> Result<(String, Outcome, u64)> {
        let path = self.directory.join(name);
        let is_video = item.ext == "mp4";
        let recorded = self.lock().entries.get(name).map(|entry| (entry.sha256.clone(), entry.source_url.clone()));
        // Set when an unchanged file is on disk that was saved as the other version of the video.
        let mut other_version = None;
        if let Some((recorded, source)) = recorded {
            if path.exists() {
                let existing = path.clone();
                let on_disk = tokio::task::spawn_blocking(move || sha256_file(&existing)).await.map_err(|e| Error::Other(e.to_string()))??;
                if on_disk == recorded {
                    if !is_video || media::satisfies(&source, self.video) {
                        self.lock().summary.skipped += 1;
                        return Ok((name.to_owned(), Outcome::Skipped, 0));
                    }
                    other_version = Some(source);
                }
            }
        }

        let choice = if is_video { media::choose(client, &item.url, self.video).await? } else { media::Choice { url: item.url.clone(), watermarked: false } };
        if choice.watermarked && self.video == VideoVersion::Clean {
            self.lock().summary.watermarked.push(name.to_owned());
        }
        if let Some(source) = &other_version {
            // The server has nothing but the version already saved.
            if media::version_of(source) == media::version_of(&choice.url) {
                self.lock().summary.skipped += 1;
                return Ok((name.to_owned(), Outcome::Skipped, 0));
            }
        }
        let item = &DownloadItem { url: choice.url, ..item.clone() };

        let response = client.get(&item.url).await?;
        let partial = self.directory.join(format!("{name}.part"));
        let mut file = tokio::fs::File::create(&partial).await?;
        let mut hasher = Sha256::new();
        let mut size = 0u64;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::Relaxed) {
                drop(file);
                let _ = tokio::fs::remove_file(&partial).await;
                return Ok((name.to_owned(), Outcome::Cancelled, 0));
            }
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    drop(file);
                    let _ = tokio::fs::remove_file(&partial).await;
                    return Err(if error.is_timeout() { Error::Timeout } else { Error::Network(error.without_url().to_string()) });
                }
            };
            hasher.update(&chunk);
            size += chunk.len() as u64;
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        drop(file);
        let digest = hex(&hasher.finalize());
        let (saved_as, outcome) = self.place(name, &partial, &digest, size, item, other_version.is_some())?;
        Ok((saved_as, outcome, size))
    }

    /// Moves a finished download into place and records it, without ever replacing a file that was
    /// changed. With `replace`, it takes the place of the file saved earlier, provided that file is
    /// still exactly what was downloaded then.
    fn place(&self, name: &str, partial: &Path, digest: &str, size: u64, item: &DownloadItem, replace: bool) -> Result<(String, Outcome)> {
        let mut state = self.lock();
        let mut name = name.to_owned();
        let mut path = self.directory.join(&name);
        let mut outcome = Outcome::Downloaded;
        if path.exists() {
            let existing = sha256_file(&path)?;
            if existing == digest {
                fs::remove_file(partial)?;
                state.summary.already_present += 1;
                self.record(&mut state, &name, digest, size, item, &path);
                return Ok((name, Outcome::AlreadyPresent));
            }
            if replace && state.entries.get(&name).is_some_and(|entry| entry.sha256 == existing) {
                fs::rename(partial, &path)?;
                state.summary.replaced += 1;
                state.summary.bytes += size;
                self.record(&mut state, &name, digest, size, item, &path);
                return Ok((name, Outcome::Replaced));
            }
            if state.entries.get(&name).is_some_and(|entry| entry.sha256 != existing) {
                state.summary.altered_on_disk.push(name.clone());
            }
            let (stem, ext) = name.rsplit_once('.').unwrap_or((&name, ""));
            name = format!("{stem}-{}.{ext}", &digest[..8]);
            path = self.directory.join(&name);
            outcome = Outcome::SavedBeside;
            state.summary.saved_beside += 1;
            if path.exists() && sha256_file(&path)? == digest {
                fs::remove_file(partial)?;
                self.record(&mut state, &name, digest, size, item, &path);
                return Ok((name, outcome));
            }
        }
        fs::rename(partial, &path)?;
        state.summary.downloaded += 1;
        state.summary.bytes += size;
        self.record(&mut state, &name, digest, size, item, &path);
        Ok((name, outcome))
    }

    /// Saves bytes that are already in memory (a Snapcode, say) under `name`, with the same rules.
    pub fn store(&self, name: &str, content: &[u8], item: &DownloadItem) -> Result<String> {
        let partial = self.directory.join(format!("{name}.part"));
        fs::write(&partial, content)?;
        let digest = hex(&Sha256::digest(content));
        let (saved_as, _) = self.place(name, &partial, &digest, content.len() as u64, item, false)?;
        Ok(saved_as)
    }

    fn record(&self, state: &mut State, name: &str, digest: &str, size: u64, item: &DownloadItem, path: &Path) {
        set_modified(path, item.uploaded_at.as_deref());
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        state.entries.insert(
            name.to_owned(),
            ManifestEntry {
                file: name.to_owned(),
                sha256: digest.to_owned(),
                bytes: size,
                category: item.category.clone(),
                username: self.username.clone(),
                media_id: stem.split_once('_').map_or(stem, |(_, id)| id).to_owned(),
                source_url: item.url.clone(),
                uploaded_at: item.uploaded_at.clone(),
                downloaded_at: now_iso(),
            },
        );
    }

    /// Writes the manifest. `run` does this itself.
    pub fn save(&self) -> Result<()> {
        let state = self.lock();
        if state.entries.is_empty() {
            return Ok(());
        }
        let manifest = ManifestFile { snapintel_version: crate::VERSION.to_owned(), updated_at: now_iso(), files: state.entries.values().cloned().collect() };
        let mut text = serde_json::to_string_pretty(&manifest).map_err(|e| Error::Other(e.to_string()))?;
        text.push('\n');
        let path = self.directory.join(MANIFEST);
        let partial = self.directory.join(format!("{MANIFEST}.part"));
        fs::write(&partial, text)?;
        fs::rename(&partial, &path)?;
        Ok(())
    }
}

/// The result of checking a folder against its manifest.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct VerifyReport {
    pub directory: String,
    pub has_manifest: bool,
    pub ok: u32,
    pub altered: Vec<String>,
    pub missing: Vec<String>,
    /// Media files in the folder that the manifest doesn't know about.
    pub untracked: Vec<String>,
}

impl VerifyReport {
    pub fn intact(&self) -> bool {
        self.has_manifest && self.altered.is_empty() && self.missing.is_empty()
    }
}

/// Re-hashes every file listed in a folder's manifest.
pub fn verify(directory: &Path) -> Result<VerifyReport> {
    let mut report = VerifyReport { directory: directory.display().to_string(), ..Default::default() };
    let entries = load_manifest(directory);
    if entries.is_empty() {
        return Ok(report);
    }
    report.has_manifest = true;
    for (name, entry) in &entries {
        let path = directory.join(name);
        if !path.exists() {
            report.missing.push(name.clone());
        } else if sha256_file(&path)? != entry.sha256 {
            report.altered.push(name.clone());
        } else {
            report.ok += 1;
        }
    }
    for file in fs::read_dir(directory)?.flatten() {
        let name = file.file_name().to_string_lossy().into_owned();
        let extension = name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).unwrap_or_default();
        if MEDIA_EXTENSIONS.contains(&extension.as_str()) && !entries.contains_key(&name) {
            report.untracked.push(name);
        }
    }
    report.untracked.sort();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names() {
        let item = |category: &str, ext: &str| DownloadItem {
            url: "https://cf-st.sc-cdn.net/d/CBVqD5SrTfygLFpfCgKGH.1322?mo=x".into(),
            ext: ext.into(),
            category: category.into(),
            uploaded_at: None,
        };
        assert_eq!(item("highlight", "mp4").file_name().unwrap(), "highlight_CBVqD5SrTfygLFpfCgKGH.mp4");
        assert!(item("../../evil", "mp4").file_name().is_err());
        assert!(item("story", "exe").file_name().is_err());
    }
}
