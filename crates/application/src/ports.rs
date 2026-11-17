//! Ports: what the use cases need from the outside world.

use std::fmt;
use std::path::Path;

use domain::{
    Collection, CollectionName, Cookie, HistoryEntry, KeyValues, PreparedRequest, Response,
    Settings,
};

/// Reading or writing saved data failed; the message says what and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError(pub String);

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StoreError {}

/// Where collections live between runs.
pub trait CollectionRepository: Send + Sync + fmt::Debug {
    /// Every saved collection, and a message for each file that could not
    /// be read (the others still load).
    ///
    /// # Errors
    ///
    /// When the storage itself can't be read.
    fn load_all(&self) -> Result<(Vec<Collection>, Vec<StoreError>), StoreError>;
    /// # Errors
    ///
    /// When the collection could not be written.
    fn save(&self, collection: &Collection) -> Result<(), StoreError>;
    /// Deleting a collection that isn't saved is not an error.
    ///
    /// # Errors
    ///
    /// When the collection could not be deleted.
    fn delete(&self, name: &CollectionName) -> Result<(), StoreError>;
}

/// Collection files the user picks: ours, older versions', Postman's.
pub trait CollectionArchive: Send + Sync + fmt::Debug {
    /// # Errors
    ///
    /// When the file can't be read or is no collection format we know.
    fn read(&self, path: &Path) -> Result<Collection, StoreError>;
    /// # Errors
    ///
    /// When the file could not be written.
    fn write(&self, path: &Path, collection: &Collection) -> Result<(), StoreError>;
}

pub trait SettingsRepository: Send + Sync + fmt::Debug {
    /// The saved settings, or the defaults when there are none.
    ///
    /// # Errors
    ///
    /// When saved settings exist but can't be read.
    fn load(&self) -> Result<Settings, StoreError>;
    /// # Errors
    ///
    /// When the settings could not be written.
    fn save(&self, settings: &Settings) -> Result<(), StoreError>;
}

/// Why no response came back.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HttpError {
    /// The address, once resolved, is not a valid URL.
    InvalidUrl(String),
    Timeout,
    /// DNS, refused connection, proxy…
    Connect(String),
    /// The server's certificate or the TLS handshake.
    Tls(String),
    Other(String),
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(e) => write!(f, "Invalid address: {e}"),
            Self::Timeout => f.write_str("No answer in time (see the timeout in Settings)."),
            Self::Connect(e) => write!(f, "Could not connect: {e}"),
            Self::Tls(e) => write!(
                f,
                "TLS error: {e} (a self-signed certificate? Settings can turn the check off)"
            ),
            Self::Other(e) => f.write_str(e),
        }
    }
}

impl std::error::Error for HttpError {}

/// Sends requests. Blocking: callers run it off the UI thread.
pub trait HttpClient: Send + Sync + fmt::Debug {
    /// Any status is a response, 4xx and 5xx included.
    ///
    /// # Errors
    ///
    /// When no response came back.
    fn send(&self, request: &PreparedRequest, settings: &Settings) -> Result<Response, HttpError>;

    /// The cookies kept for the next requests.
    fn cookies(&self) -> Vec<Cookie> {
        Vec::new()
    }

    /// Forgets every kept cookie.
    fn clear_cookies(&self) {}
}

/// The global variables, seen by every collection.
pub trait GlobalsRepository: Send + Sync + fmt::Debug {
    /// None saved yet is no error: an empty list.
    ///
    /// # Errors
    ///
    /// When saved globals exist but can't be read.
    fn load(&self) -> Result<KeyValues, StoreError>;
    /// # Errors
    ///
    /// When they could not be written.
    fn save(&self, globals: &KeyValues) -> Result<(), StoreError>;
}

/// The requests sent, newest first.
pub trait HistoryRepository: Send + Sync + fmt::Debug {
    /// # Errors
    ///
    /// When a saved history exists but can't be read.
    fn load(&self) -> Result<Vec<HistoryEntry>, StoreError>;
    /// # Errors
    ///
    /// When it could not be written.
    fn save(&self, entries: &[HistoryEntry]) -> Result<(), StoreError>;
}
