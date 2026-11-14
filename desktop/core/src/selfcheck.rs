//! Does Snapchat's profile page still have the layout SnapIntel reads?
//!
//! Every field is read through a path in [`crate::paths`]. When Snapchat renames or moves something,
//! those fields quietly come back empty. This loads reference profiles and reports, for every path,
//! whether it still resolves; for one that doesn't, it says where the path stops and which keys are
//! there instead.

use serde::Serialize;
use serde_json::Value;

use crate::fetch::Client;
use crate::parse::{at, PUBLIC_PAGE};
use crate::paths::{full, Scope, FIELDS, ROOT};
use crate::util::now_iso;

/// Two of Snap's own accounts: Spectacles has a public profile with highlights, Spotlights and
/// lenses; Bitmoji has the other layout.
pub const REFERENCE_ACCOUNTS: [&str; 2] = ["spectacles", "bitmoji"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    /// The page has the key, but this account has nothing there.
    Empty,
    /// The field belongs to a list that is empty on this account.
    Skipped,
    /// The path no longer leads anywhere: Snapchat changed its page.
    Missing,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FieldResult {
    pub name: &'static str,
    pub path: &'static str,
    pub status: Status,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AccountResult {
    pub account: String,
    /// `public`, `private`, or `None` when the page couldn't be read.
    pub layout: Option<&'static str>,
    pub error: Option<String>,
    pub fields: Vec<FieldResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SelfCheckReport {
    pub checked_at: String,
    pub healthy: bool,
    pub ok: usize,
    pub empty: usize,
    pub skipped: usize,
    pub missing: usize,
    /// Fields that were empty or skipped on every account, so nothing confirmed them.
    pub unconfirmed: Vec<&'static str>,
    pub accounts: Vec<AccountResult>,
}

fn describe(value: &Value) -> String {
    match value {
        Value::Array(list) => format!("list of {}", list.len()),
        Value::Object(map) => format!("object with {} keys", map.len()),
        Value::String(text) => {
            let short: String = text.chars().take(48).collect();
            format!("\"{}{}\"", short.replace('\n', " "), if text.chars().count() > 48 { "…" } else { "" })
        }
        other => other.to_string(),
    }
}

/// Walks one path. A segment ending in `[]` steps into the first element of a list.
fn resolve(root: &Value, path: &str) -> (Status, String) {
    let mut node = root;
    let mut walked: Vec<&str> = Vec::new();
    let place = |walked: &[&str]| if walked.is_empty() { "the page".to_owned() } else { walked.join(".") };
    for segment in path.split('.') {
        let (key, into_list) = segment.strip_suffix("[]").map_or((segment, false), |key| (key, true));
        match node {
            Value::Object(map) => match map.get(key) {
                Some(next) => node = next,
                None => {
                    let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
                    keys.sort_unstable();
                    let shown = keys.iter().take(14).copied().collect::<Vec<_>>().join(", ");
                    let more = if keys.len() > 14 { ", …" } else { "" };
                    return (Status::Missing, format!("no \"{key}\" in {}; it has: {shown}{more}", place(&walked)));
                }
            },
            Value::Null => return (Status::Empty, format!("{} is empty on this account", place(&walked))),
            other => return (Status::Missing, format!("{} is {}, not an object", place(&walked), describe(other))),
        }
        walked.push(key);
        if into_list {
            match node {
                Value::Array(list) => match list.first() {
                    Some(first) => node = first,
                    None => return (Status::Skipped, format!("{} is an empty list on this account", place(&walked))),
                },
                Value::Null => return (Status::Empty, format!("{} is empty on this account", place(&walked))),
                other => return (Status::Missing, format!("{} is {}, not a list", place(&walked), describe(other))),
            }
        }
    }
    match node {
        Value::Null => (Status::Empty, "empty on this account".into()),
        Value::String(text) if text.is_empty() => (Status::Empty, "empty on this account".into()),
        Value::Array(list) if list.is_empty() => (Status::Empty, "an empty list on this account".into()),
        found => (Status::Ok, describe(found)),
    }
}

/// Checks every field that applies to the page's layout. Returns the layout and the results.
pub fn check_page(page: &Value) -> (Option<&'static str>, Vec<FieldResult>) {
    let Some(root) = at(page, ROOT) else {
        return (None, Vec::new());
    };
    let page_type = at(root, full("page_type")).and_then(|v| v.as_i64());
    let scope = match page_type {
        Some(PUBLIC_PAGE) => Scope::Public,
        Some(_) => Scope::Private,
        None => return (None, Vec::new()),
    };
    let fields = FIELDS
        .iter()
        .filter(|field| field.scope == Scope::Any || field.scope == scope)
        .map(|field| {
            let (status, detail) = resolve(root, field.path);
            FieldResult { name: field.name, path: field.path, status, detail }
        })
        .collect();
    (Some(if scope == Scope::Public { "public" } else { "private" }), fields)
}

/// Puts per-account results together.
pub fn summarize(accounts: Vec<AccountResult>) -> SelfCheckReport {
    let count = |status: Status| accounts.iter().flat_map(|a| &a.fields).filter(|f| f.status == status).count();
    let missing = count(Status::Missing);
    let unreadable = accounts.iter().any(|a| a.layout.is_none());
    let unconfirmed = FIELDS
        .iter()
        .map(|field| field.name)
        .filter(|name| {
            let results: Vec<&FieldResult> = accounts.iter().flat_map(|a| &a.fields).filter(|f| f.name == *name).collect();
            !results.is_empty() && results.iter().all(|f| matches!(f.status, Status::Empty | Status::Skipped))
        })
        .collect();
    SelfCheckReport {
        checked_at: now_iso(),
        healthy: missing == 0 && !unreadable,
        ok: count(Status::Ok),
        empty: count(Status::Empty),
        skipped: count(Status::Skipped),
        missing,
        unconfirmed,
        accounts,
    }
}

/// Loads each account's page from Snapchat and checks it.
pub async fn run(client: &Client, accounts: &[&str]) -> SelfCheckReport {
    let mut results = Vec::new();
    for account in accounts {
        let result = match client.page(account).await {
            Ok((page, _)) => {
                let (layout, fields) = check_page(&page);
                let error = layout.is_none().then(|| "the page has no profile metadata".to_owned());
                AccountResult { account: (*account).to_owned(), layout, error, fields }
            }
            Err(error) => AccountResult { account: (*account).to_owned(), layout: None, error: Some(error.to_string()), fields: Vec::new() },
        };
        results.push(result);
    }
    summarize(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolution() {
        let root = json!({"a": {"b": "x", "empty": "", "none": null, "list": [], "items": [{"c": 1}]}});
        assert_eq!(resolve(&root, "a.b").0, Status::Ok);
        assert_eq!(resolve(&root, "a.empty").0, Status::Empty);
        assert_eq!(resolve(&root, "a.none").0, Status::Empty);
        assert_eq!(resolve(&root, "a.none.deeper").0, Status::Empty);
        assert_eq!(resolve(&root, "a.list[].c").0, Status::Skipped);
        assert_eq!(resolve(&root, "a.items[].c").0, Status::Ok);
        let (status, detail) = resolve(&root, "a.renamed");
        assert_eq!(status, Status::Missing);
        assert!(detail.contains("no \"renamed\" in a") && detail.contains("items"), "{detail}");
        assert_eq!(resolve(&root, "a.items[].gone").0, Status::Missing);
    }
}
