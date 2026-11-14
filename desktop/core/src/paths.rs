//! Where each piece of data lives in the JSON that snapchat.com embeds in a profile page.
//!
//! This table is the only place the page's layout is written down. `parse` reads through it and
//! `selfcheck` tests it against the live site, so the two can't drift apart. Paths start at
//! [`ROOT`]; a segment ending in `[]` is a list, and what follows it is read from each element.
//!
//! Snap coordinates (`lat`, `lng`) and the profile's `address` are on the page too. They are not
//! listed here on purpose, so nothing in SnapIntel reads them.

use serde::Serialize;

pub const ROOT: &str = "props.pageProps";

/// Which page layout has the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Any,
    /// Public profiles (`pageType` 18).
    Public,
    /// Profiles that aren't public: a name, a Snapcode and a Bitmoji.
    Private,
}

pub struct Field {
    pub name: &'static str,
    pub scope: Scope,
    pub path: &'static str,
}

const fn any(name: &'static str, path: &'static str) -> Field {
    Field { name, scope: Scope::Any, path }
}
const fn public(name: &'static str, path: &'static str) -> Field {
    Field { name, scope: Scope::Public, path }
}
const fn private(name: &'static str, path: &'static str) -> Field {
    Field { name, scope: Scope::Private, path }
}

pub const FIELDS: &[Field] = &[
    any("page_type", "pageMetadata.pageType"),
    any("page_title", "pageMetadata.pageTitle"),
    any("page_description", "pageMetadata.pageDescription.value"),
    any("canonical_url", "pageLinks.canonicalUrl"),
    any("locale", "locale"),
    // The profile
    public("username", "userProfile.publicProfileInfo.username"),
    public("display_name", "userProfile.publicProfileInfo.title"),
    public("bio", "userProfile.publicProfileInfo.bio"),
    public("badge", "userProfile.publicProfileInfo.badge"),
    public("subscriber_count", "userProfile.publicProfileInfo.subscriberCount"),
    public("website_url", "userProfile.publicProfileInfo.websiteUrl"),
    public("profile_picture_url", "userProfile.publicProfileInfo.profilePictureUrl"),
    public("hero_image_url", "userProfile.publicProfileInfo.squareHeroImageUrl"),
    public("snapcode_url", "userProfile.publicProfileInfo.snapcodeImageUrl"),
    public("category", "userProfile.publicProfileInfo.categoryStringId"),
    public("subcategory", "userProfile.publicProfileInfo.subcategoryStringId"),
    public("created", "userProfile.publicProfileInfo.creationTimestampMs.value"),
    public("updated", "userProfile.publicProfileInfo.lastUpdateTimestampMs.value"),
    public("links", "userProfile.publicProfileInfo.sameAsLinks"),
    public("related", "userProfile.publicProfileInfo.relatedAccountsInfo"),
    public("related.username", "userProfile.publicProfileInfo.relatedAccountsInfo[].publicProfileInfo.username"),
    public("related.display_name", "userProfile.publicProfileInfo.relatedAccountsInfo[].publicProfileInfo.title"),
    public("related.picture_url", "userProfile.publicProfileInfo.relatedAccountsInfo[].publicProfileInfo.profilePictureUrl"),
    public("related.badge", "userProfile.publicProfileInfo.relatedAccountsInfo[].publicProfileInfo.badge"),
    // The current story
    public("story", "story.snapList"),
    // Highlights, and the snap fields every list of snaps shares
    public("highlights", "curatedHighlights"),
    public("highlights.cursor", "curatedHighlightsCursor"),
    public("highlight.id", "curatedHighlights[].highlightId.value"),
    public("highlight.title", "curatedHighlights[].storyTitle.value"),
    public("highlight.thumbnail_url", "curatedHighlights[].thumbnailUrl.value"),
    public("highlight.snaps", "curatedHighlights[].snapList"),
    public("snap.index", "curatedHighlights[].snapList[].snapIndex"),
    public("snap.media_type", "curatedHighlights[].snapList[].snapMediaType"),
    public("snap.media_url", "curatedHighlights[].snapList[].snapUrls.mediaUrl"),
    public("snap.preview_url", "curatedHighlights[].snapList[].snapUrls.mediaPreviewUrl.value"),
    public("snap.timestamp", "curatedHighlights[].snapList[].timestampInSec.value"),
    // Spotlight: the snaps, and a parallel list of details in the same order
    public("spotlights", "spotlightHighlights"),
    public("spotlights.cursor", "spotlightHighlightsCursor"),
    public("spotlight.id", "spotlightHighlights[].storyId.value"),
    public("spotlight.thumbnail_url", "spotlightHighlights[].thumbnailUrl.value"),
    public("spotlight.snaps", "spotlightHighlights[].snapList"),
    public("snap.transcript_url", "spotlightHighlights[].snapList[].audioTranscriptionObjectUrl.value"),
    public("spotlight_details", "spotlightStoryMetadata"),
    public("details.name", "spotlightStoryMetadata[].videoMetadata.name"),
    public("details.uploaded", "spotlightStoryMetadata[].videoMetadata.uploadDateMs"),
    public("details.duration", "spotlightStoryMetadata[].videoMetadata.durationMs"),
    public("details.width", "spotlightStoryMetadata[].videoMetadata.width"),
    public("details.height", "spotlightStoryMetadata[].videoMetadata.height"),
    public("details.views", "spotlightStoryMetadata[].engagementStats.viewCount"),
    public("details.shares", "spotlightStoryMetadata[].engagementStats.shareCount"),
    public("details.comments", "spotlightStoryMetadata[].engagementStats.commentCount"),
    public("details.boosts", "spotlightStoryMetadata[].engagementStats.boostCount"),
    public("details.recommends", "spotlightStoryMetadata[].engagementStats.recommendCount"),
    public("details.hashtags", "spotlightStoryMetadata[].hashtags"),
    public("details.description", "spotlightStoryMetadata[].description"),
    public("details.ai_title", "spotlightStoryMetadata[].llmTitle"),
    public("details.ai_summary", "spotlightStoryMetadata[].llmDescription"),
    public("details.keywords", "spotlightStoryMetadata[].textMetadataKeywords"),
    public("details.context_cards", "spotlightStoryMetadata[].contextCards"),
    public("details.page_url", "spotlightStoryMetadata[].oneLinkParams.deepLinkUrl"),
    // Lenses
    public("lenses", "lenses"),
    public("lenses.cursor", "lensCursor"),
    public("lens.name", "lenses[].lensName"),
    public("lens.creator", "lenses[].lensCreatorDisplayName"),
    public("lens.preview_image_url", "lenses[].lensPreviewImageUrl"),
    public("lens.preview_video_url", "lenses[].lensPreviewVideoUrl"),
    public("lens.icon_url", "lenses[].iconUrl"),
    public("lens.unlock_url", "lenses[].unlockUrl"),
    public("lens.official", "lenses[].isOfficialSnapLens"),
    public("lens.updated", "lenses[].lastUpdatedEpoch"),
    // Profiles that aren't public
    private("private.username", "userProfile.userInfo.username"),
    private("private.display_name", "userProfile.userInfo.displayName"),
    private("private.snapcode_url", "userProfile.userInfo.snapcodeImageUrl"),
    private("private.avatar_url", "userProfile.userInfo.bitmoji3d.avatarImage.url"),
    private("private.background_url", "userProfile.userInfo.bitmoji3d.backgroundImage.url"),
];

/// A field's whole path from [`ROOT`].
pub fn full(name: &str) -> &'static str {
    FIELDS.iter().find(|f| f.name == name).map(|f| f.path).unwrap_or_else(|| panic!("no field named {name} in paths::FIELDS"))
}

/// A field's path inside the list element it belongs to: `snap.media_url` gives `snapUrls.mediaUrl`.
pub fn rel(name: &str) -> &'static str {
    let path = full(name);
    path.rfind("[].").map_or(path, |i| &path[i + 3..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique() {
        for (i, field) in FIELDS.iter().enumerate() {
            assert!(FIELDS[..i].iter().all(|other| other.name != field.name), "{} is listed twice", field.name);
        }
    }

    #[test]
    fn relative_paths() {
        assert_eq!(rel("snap.media_url"), "snapUrls.mediaUrl");
        assert_eq!(rel("highlight.snaps"), "snapList");
        assert_eq!(rel("bio"), "userProfile.publicProfileInfo.bio");
        assert_eq!(full("highlights"), "curatedHighlights");
    }

    #[test]
    fn nothing_reads_location() {
        for field in FIELDS {
            let path = field.path.to_ascii_lowercase();
            assert!(!path.ends_with(".lat") && !path.ends_with(".lng") && !path.contains("address"), "{} reads location data", field.name);
        }
    }
}
