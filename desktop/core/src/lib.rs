//! SnapIntel's core: everything except the window.
//!
//! It reads the page snapchat.com serves for a public profile (`fetch`, `parse`, `paths`), turns it
//! into a typed [`model::Report`], downloads media with a manifest that can be verified later
//! (`downloads`), exports reports (`export`) and checks that Snapchat's page still has the layout
//! this code expects (`selfcheck`).
//!
//! Location data is deliberately not read: the page carries coordinates on some snaps and an
//! address on some profiles, and neither is parsed, stored or exported.

pub mod bitmoji;
pub mod downloads;
pub mod error;
pub mod export;
pub mod fetch;
pub mod model;
pub mod parse;
pub mod paths;
pub mod selfcheck;
pub mod transcript;
mod util;

pub use error::{Error, Result};
pub use fetch::{normalize_username, Client, ClientOptions};
pub use model::Report;

/// The version written into reports and manifests.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
