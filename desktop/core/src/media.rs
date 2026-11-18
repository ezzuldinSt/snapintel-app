//! Which of Snapchat's versions of a video to download.
//!
//! Snapchat's media server keeps several versions of an item under one ID, told apart by a number in
//! the address: `https://cf-st.sc-cdn.net/d/<id>.<version>.<suffix>`. A profile's page links one of
//! them. For videos two kinds exist:
//!
//! - versions 1034 and 1322: 480 x 852, nothing added to the picture. The page links these for
//!   stories and highlights.
//! - version 27: 540 x 960 at a higher bitrate, with Snapchat's logo and the username burned into the
//!   picture. The page links this one for Spotlight, and the server has it for some highlights too.
//!
//! Nothing larger is published: asking the server for every version number from 0 to 3000 of sample
//! stories, highlights and Spotlights (November 2026) found only these and small previews. Photos
//! need no choosing, because the page already links their largest version.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::fetch::Client;

/// Versions of a video with nothing added to the picture.
const CLEAN: [u32; 2] = [1034, 1322];
/// The version with Snapchat's watermark.
const WATERMARKED: u32 = 27;

/// The version of a video a download should take.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoVersion {
    /// Without Snapchat's watermark (480 x 852).
    #[default]
    Clean,
    /// The largest one (540 x 960), which carries the watermark.
    Sharpest,
}

/// A media address taken apart at its version number: `<before>.<version><after>`.
struct Address<'a> {
    before: &'a str,
    version: u32,
    after: &'a str,
}

impl Address<'_> {
    /// The address of another version of the same item. The page's parameters describe the version
    /// it links, so they are left off; the server doesn't need them.
    fn with(&self, version: u32) -> String {
        format!("{}.{version}{}", self.before, self.after)
    }
}

fn parse(url: &str) -> Option<Address<'_>> {
    let path_end = url.find(['?', '#']).unwrap_or(url.len());
    let path = &url[..path_end];
    let name_start = path.rfind("/d/")? + 3;
    let name = &path[name_start..];
    if name.contains('/') {
        return None;
    }
    let (id, rest) = name.split_once('.')?;
    let (digits, after) = match rest.find('.') {
        Some(dot) => rest.split_at(dot),
        None => (rest, ""),
    };
    if id.is_empty() || digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(Address { before: &path[..name_start + id.len()], version: digits.parse().ok()?, after })
}

/// The version number in a media address, when it has one.
pub fn version_of(url: &str) -> Option<u32> {
    parse(url).map(|address| address.version)
}

/// Whether the video at this address carries Snapchat's watermark.
pub fn is_watermarked(url: &str) -> bool {
    version_of(url) == Some(WATERMARKED)
}

/// Whether a video saved from `url` is already the version asked for, as far as the address tells.
/// `false` means another version may exist and is worth asking the server about.
pub fn satisfies(url: &str, wanted: VideoVersion) -> bool {
    match (wanted, version_of(url)) {
        (VideoVersion::Clean, Some(WATERMARKED)) => false,
        (VideoVersion::Sharpest, Some(version)) => !CLEAN.contains(&version),
        _ => true,
    }
}

/// The address to download a video from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub url: String,
    /// The video carries Snapchat's watermark.
    pub watermarked: bool,
}

/// Picks the version of the video the page links at `url`: that link itself when it already is the
/// version wanted, otherwise the wanted one if the server has it. Asking costs one byte per version.
pub async fn choose(client: &Client, url: &str, wanted: VideoVersion) -> Result<Choice> {
    let page_link = Choice { url: url.to_owned(), watermarked: is_watermarked(url) };
    let Some(address) = parse(url) else { return Ok(page_link) };
    let others: &[u32] = match wanted {
        VideoVersion::Clean if address.version == WATERMARKED => &CLEAN,
        VideoVersion::Sharpest if CLEAN.contains(&address.version) => &[WATERMARKED],
        _ => &[],
    };
    let mut best: Option<(u64, String)> = None;
    for version in others {
        let candidate = address.with(*version);
        if let Some(size) = client.video_size(&candidate).await? {
            let larger = match &best {
                Some((largest, _)) => size > *largest,
                None => true,
            };
            if larger {
                best = Some((size, candidate));
            }
        }
    }
    Ok(match best {
        Some((_, found)) => Choice { watermarked: is_watermarked(&found), url: found },
        None => page_link,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        let spotlight = "https://cf-st.sc-cdn.net/d/rDy6t5tIqg6vqmuUUNAD4.27.IRZXSOY?mo=GmUaCzIB%3D%3D&uc=46";
        let address = parse(spotlight).unwrap();
        assert_eq!((address.before, address.version, address.after), ("https://cf-st.sc-cdn.net/d/rDy6t5tIqg6vqmuUUNAD4", 27, ".IRZXSOY"));
        assert_eq!(address.with(1034), "https://cf-st.sc-cdn.net/d/rDy6t5tIqg6vqmuUUNAD4.1034.IRZXSOY");
        // Some highlight links have no suffix.
        assert_eq!(
            parse("https://cf-st.sc-cdn.net/d/ZXSSacNIpSYqxAm21SSGc.400?mo=x").unwrap().with(999),
            "https://cf-st.sc-cdn.net/d/ZXSSacNIpSYqxAm21SSGc.999"
        );
        assert_eq!(version_of("https://cf-st.sc-cdn.net/d/abc.1322.IRZXSOY"), Some(1322));
        // Lens previews and anything else that isn't one of these addresses is left alone.
        for other in [
            "https://community-lens.storage.googleapis.com/preview-media/final/0d5ab4c2.mp4",
            "https://cf-st.sc-cdn.net/d/plain",
            "https://cf-st.sc-cdn.net/d/x.mp4",
            "https://cf-st.sc-cdn.net/aps/bolt/aHR0c._RS0,90_FMjpeg",
            "",
        ] {
            assert!(parse(other).is_none(), "{other}");
        }
    }

    #[test]
    fn what_is_already_right() {
        let clean = "https://cf-st.sc-cdn.net/d/abcdefgh.1034.IRZXSOY";
        let marked = "https://cf-st.sc-cdn.net/d/abcdefgh.27.IRZXSOY";
        let lens = "https://lens-storage.storage.googleapis.com/previewvideo/abcdefgh";
        assert!(is_watermarked(marked) && !is_watermarked(clean) && !is_watermarked(lens));
        assert!(satisfies(clean, VideoVersion::Clean) && !satisfies(marked, VideoVersion::Clean));
        assert!(satisfies(marked, VideoVersion::Sharpest) && !satisfies(clean, VideoVersion::Sharpest));
        assert!(satisfies(lens, VideoVersion::Clean) && satisfies(lens, VideoVersion::Sharpest));
    }
}
