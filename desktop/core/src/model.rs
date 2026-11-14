//! What a lookup returns. The same structure is sent to the interface and written by the JSON export.
//!
//! Counts and durations are numbers, dates are ISO 8601 in UTC, and anything Snapchat leaves out or
//! hides (it sends -1 for a hidden count) is `null`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub snapintel_version: String,
    pub retrieved_at: String,
    pub requested_username: String,
    pub url: String,
    /// Snapchat's own address for the profile, which differs from `url` when a username redirects.
    pub canonical_url: Option<String>,
    pub locale: Option<String>,
    pub profile: Profile,
    pub story: Vec<Snap>,
    pub highlights: Vec<Highlight>,
    pub spotlights: Vec<Spotlight>,
    pub lenses: Vec<Lens>,
    pub related_accounts: Vec<RelatedAccount>,
    pub more_available: MoreAvailable,
    pub stats: Stats,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// Public profiles have a story, highlights, Spotlight and lenses; the others only a name and Bitmoji.
    pub public: bool,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub website_url: Option<String>,
    /// `creator` or `public_figure`.
    pub badge: Option<String>,
    pub badge_code: Option<i64>,
    pub subscriber_count: Option<u64>,
    pub category: Option<String>,
    pub subcategory: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub profile_picture_url: Option<String>,
    pub hero_image_url: Option<String>,
    pub snapcode_url: Option<String>,
    /// The 3D Bitmoji of a profile that isn't public.
    pub avatar_url: Option<String>,
    pub background_url: Option<String>,
    /// Other sites the profile says are the same person or organisation.
    pub links: Vec<String>,
    pub page_title: Option<String>,
    pub page_description: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Image,
    #[default]
    Video,
}

impl MediaType {
    pub fn extension(self) -> &'static str {
        match self {
            MediaType::Image => "jpg",
            MediaType::Video => "mp4",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Snap {
    /// Snapchat's media ID, which also names the downloaded file.
    pub id: String,
    pub index: u32,
    pub media_type: MediaType,
    pub media_url: String,
    pub preview_url: Option<String>,
    pub uploaded_at: Option<String>,
    /// Captions for the video (WebVTT), when Snapchat has transcribed it.
    pub transcript_url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Highlight {
    pub id: Option<String>,
    pub title: Option<String>,
    pub thumbnail_url: Option<String>,
    pub snaps: Vec<Snap>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Sound {
    pub title: Option<String>,
    pub artist: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Spotlight {
    pub id: Option<String>,
    pub name: Option<String>,
    /// The caption its author wrote.
    pub description: Option<String>,
    /// A title and summary Snapchat generates for the video.
    pub ai_title: Option<String>,
    pub ai_summary: Option<String>,
    pub keywords: Vec<String>,
    pub hashtags: Vec<String>,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<u64>,
    pub uploaded_at: Option<String>,
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub views: Option<u64>,
    pub shares: Option<u64>,
    pub comments: Option<u64>,
    pub boosts: Option<u64>,
    pub recommends: Option<u64>,
    pub sound: Option<Sound>,
    /// The Spotlight's own page on snapchat.com.
    pub page_url: Option<String>,
    pub snaps: Vec<Snap>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Lens {
    pub name: Option<String>,
    pub creator: Option<String>,
    pub preview_image_url: Option<String>,
    pub preview_video_url: Option<String>,
    pub icon_url: Option<String>,
    /// Opens the lens in Snapchat.
    pub unlock_url: Option<String>,
    pub official: bool,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RelatedAccount {
    pub username: String,
    pub display_name: Option<String>,
    pub profile_picture_url: Option<String>,
    pub badge: Option<String>,
}

/// Snapchat's page carries the first page of each list. These say whether the profile has more.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoreAvailable {
    pub highlights: bool,
    pub spotlights: bool,
    pub lenses: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub story_snaps: u64,
    pub highlights: u64,
    pub highlight_snaps: u64,
    pub spotlights: u64,
    pub spotlight_duration_seconds: u64,
    /// Totals over the Spotlights whose figure Snapchat shows; `null` when it shows none.
    pub spotlight_views: Option<u64>,
    pub spotlight_comments: Option<u64>,
    pub spotlight_shares: Option<u64>,
    pub lenses: u64,
}
