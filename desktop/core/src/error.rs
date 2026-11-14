use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("\"{0}\" doesn't look like a Snapchat username")]
    InvalidUsername(String),
    #[error("Snapchat has no profile named \"{0}\"")]
    NotFound(String),
    #[error("Snapchat didn't answer in time")]
    Timeout,
    #[error("Couldn't reach Snapchat: {0}")]
    Network(String),
    #[error("Snapchat answered with an error (HTTP {0})")]
    Http(u16),
    #[error("Snapchat's page couldn't be read: {0}")]
    PageFormat(String),
    #[error("That link can't be used: {0}")]
    BadUrl(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// A short machine-readable name, so an interface can show the right message and actions.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::InvalidUsername(_) => "invalid_username",
            Error::NotFound(_) => "not_found",
            Error::Timeout => "timeout",
            Error::Network(_) => "network",
            Error::Http(_) => "http",
            Error::PageFormat(_) => "page_format",
            Error::BadUrl(_) => "bad_url",
            Error::Io(_) => "io",
            Error::Other(_) => "other",
        }
    }
}
