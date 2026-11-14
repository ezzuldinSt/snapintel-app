//! Talks to snapchat.com: the profile page, and the media and caption files it points to.

use std::time::Duration;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::model::Report;
use crate::parse;

pub const SNAPCHAT: &str = "https://www.snapchat.com";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";

#[derive(Debug, Clone)]
pub struct ClientOptions {
    /// How long Snapchat may take to answer, and how long a download may stall.
    pub timeout: Duration,
    /// The language Snapchat writes its page titles and descriptions in, such as `en-US`.
    /// `None` leaves the choice to Snapchat, which goes by the country it sees.
    pub locale: Option<String>,
    /// Only changed by tests, to point at a local server.
    pub base_url: String,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self { timeout: Duration::from_secs(30), locale: Some("en-US".into()), base_url: SNAPCHAT.into() }
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    options: ClientOptions,
}

/// Accepts what people paste: `name`, `@name`, or a profile link such as
/// `https://www.snapchat.com/add/name?share_id=...`. Returns the bare lower-case username.
pub fn normalize_username(input: &str) -> Result<String> {
    let invalid = || Error::InvalidUsername(input.trim().to_owned());
    let mut name = input.trim();
    if let Some((_, after)) = name.split_once("snapchat.com/") {
        name = after;
    }
    name = name.strip_prefix("add/").unwrap_or(name);
    name = name.trim_start_matches('@');
    let end = name.find(['/', '?', '#']).unwrap_or(name.len());
    let name = name[..end].to_ascii_lowercase();
    let valid = (1..=30).contains(&name.len())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        && name.chars().next().is_some_and(|c| c.is_ascii_alphanumeric());
    if valid {
        Ok(name)
    } else {
        Err(invalid())
    }
}

fn request_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout
    } else if error.is_connect() {
        Error::Network("no connection could be made. Check your internet connection.".into())
    } else {
        Error::Network(error.without_url().to_string())
    }
}

impl Client {
    pub fn new(options: ClientOptions) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(options.timeout)
            .read_timeout(options.timeout)
            .build()
            .map_err(|e| Error::Other(format!("couldn't set up the connection: {e}")))?;
        Ok(Self { http, options })
    }

    pub fn options(&self) -> &ClientOptions {
        &self.options
    }

    pub fn profile_url(&self, username: &str) -> String {
        match &self.options.locale {
            Some(locale) if !locale.is_empty() => format!("{}/add/{username}?locale={locale}", self.options.base_url),
            _ => format!("{}/add/{username}", self.options.base_url),
        }
    }

    /// Media and captions are only ever fetched over HTTPS (or from the test server).
    fn check(&self, url: &str) -> Result<()> {
        if url.starts_with("https://") || url.starts_with(&self.options.base_url) {
            Ok(())
        } else {
            Err(Error::BadUrl(url.chars().take(80).collect()))
        }
    }

    pub async fn get(&self, url: &str) -> Result<reqwest::Response> {
        self.check(url)?;
        let response = self.http.get(url).send().await.map_err(request_error)?;
        if response.status().is_success() {
            Ok(response)
        } else {
            Err(Error::Http(response.status().as_u16()))
        }
    }

    pub async fn text(&self, url: &str) -> Result<String> {
        self.get(url).await?.text().await.map_err(request_error)
    }

    pub async fn bytes(&self, url: &str) -> Result<Vec<u8>> {
        Ok(self.get(url).await?.bytes().await.map_err(request_error)?.to_vec())
    }

    /// The JSON embedded in a profile's page, and the address it was fetched from.
    pub async fn page(&self, username: &str) -> Result<(Value, String)> {
        let url = self.profile_url(username);
        let response = self.http.get(&url).timeout(self.options.timeout).send().await.map_err(request_error)?;
        let status = response.status().as_u16();
        let html = response.text().await.map_err(request_error)?;
        match parse::extract_page_json(&html) {
            Ok(page) => Ok((page, url)),
            Err(_) if status == 404 => Err(Error::NotFound(username.to_owned())),
            Err(_) if status >= 400 => Err(Error::Http(status)),
            Err(error) => Err(error),
        }
    }

    /// Looks a profile up. `input` may be a username, an @name or a profile link.
    pub async fn lookup(&self, input: &str) -> Result<Report> {
        let username = normalize_username(input)?;
        let (page, url) = self.page(&username).await?;
        parse::parse_report(&page, &username, &url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usernames() {
        for (input, expected) in [
            ("spectacles", "spectacles"),
            ("  @Spectacles ", "spectacles"),
            ("https://www.snapchat.com/add/team.snap?share_id=abc&locale=en-US", "team.snap"),
            ("https://www.snapchat.com/@nba", "nba"),
            ("snapchat.com/add/some_user-1/", "some_user-1"),
        ] {
            assert_eq!(normalize_username(input).unwrap(), expected, "{input}");
        }
        for input in ["", "   ", "@", "two words", "name!", "../etc", "-dash", &"x".repeat(31)] {
            assert!(matches!(normalize_username(input), Err(Error::InvalidUsername(_))), "{input:?} should be refused");
        }
    }

    #[test]
    fn profile_urls() {
        let client = Client::new(ClientOptions::default()).unwrap();
        assert_eq!(client.profile_url("nba"), "https://www.snapchat.com/add/nba?locale=en-US");
        let client = Client::new(ClientOptions { locale: None, ..Default::default() }).unwrap();
        assert_eq!(client.profile_url("nba"), "https://www.snapchat.com/add/nba");
        assert!(client.check("http://example.com/x.mp4").is_err());
        assert!(client.check("https://cf-st.sc-cdn.net/d/x").is_ok());
    }
}
