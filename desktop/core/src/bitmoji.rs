//! Earlier looks of a 3D Bitmoji.
//!
//! A profile that isn't public shows a 3D Bitmoji whose image address carries a version number:
//! `…/<id>_<version>-<rest>`. Earlier versions stay online, so counting down from the current one
//! finds the looks the account has had. Many versions render the same image, so they are grouped by
//! content. The idea is from hatless1der's write-up, which the original SnapIntel credits.

use std::collections::BTreeMap;

use futures_util::StreamExt;
use md5::{Digest, Md5};
use serde::Serialize;

use crate::error::Result;
use crate::fetch::Client;
use crate::util::hex;

/// Never ask for more versions than this, whatever the address says.
pub const MAX_VERSIONS: u32 = 300;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Look {
    /// The newest version that shows this look.
    pub version: u32,
    pub url: String,
    pub md5: String,
    pub bytes: u64,
}

/// The address of every version from the current one down to 1, newest first.
/// Empty when the address doesn't have the expected shape.
pub fn version_urls(avatar_url: &str) -> Vec<(u32, String)> {
    let Some((base, after)) = avatar_url.split_once('_') else { return Vec::new() };
    let Some((version, rest)) = after.split_once('-') else { return Vec::new() };
    let Ok(latest) = version.parse::<u32>() else { return Vec::new() };
    (1..=latest.min(MAX_VERSIONS)).rev().map(|v| (v, format!("{base}_{v}-{rest}"))).collect()
}

/// Fetches every version and returns the distinct looks, newest first.
pub async fn history(client: &Client, avatar_url: &str, concurrency: usize) -> Result<Vec<Look>> {
    let fetched: Vec<Option<Look>> = futures_util::stream::iter(version_urls(avatar_url))
        .map(|(version, url)| async move {
            let content = client.bytes(&url).await.ok()?;
            Some(Look { version, md5: hex(&Md5::digest(&content)), bytes: content.len() as u64, url })
        })
        .buffer_unordered(concurrency.clamp(1, 16))
        .collect()
        .await;
    let mut looks: BTreeMap<String, Look> = BTreeMap::new();
    for look in fetched.into_iter().flatten() {
        match looks.get(&look.md5) {
            Some(existing) if existing.version >= look.version => {}
            _ => {
                looks.insert(look.md5.clone(), look);
            }
        }
    }
    let mut looks: Vec<Look> = looks.into_values().collect();
    looks.sort_by_key(|look| std::cmp::Reverse(look.version));
    Ok(looks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        let urls = version_urls("https://cf-st.sc-cdn.net/3d/render/10226021-99120672960_3-s5-v1.webp?ua=2");
        assert_eq!(
            urls,
            vec![
                (3, "https://cf-st.sc-cdn.net/3d/render/10226021-99120672960_3-s5-v1.webp?ua=2".to_owned()),
                (2, "https://cf-st.sc-cdn.net/3d/render/10226021-99120672960_2-s5-v1.webp?ua=2".to_owned()),
                (1, "https://cf-st.sc-cdn.net/3d/render/10226021-99120672960_1-s5-v1.webp?ua=2".to_owned()),
            ]
        );
        assert!(version_urls("https://example.com/no-version.webp").is_empty());
        assert_eq!(version_urls("https://example.com/a_99999-x.webp").len(), MAX_VERSIONS as usize);
    }
}
