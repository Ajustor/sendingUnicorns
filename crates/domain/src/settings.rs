//! How requests are sent.

/// The user's choices, the same for every request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Gives up after this many seconds (connection and answer together).
    pub timeout_secs: u64,
    /// Refuses servers whose certificate the system does not trust.
    /// Off for local servers with a self-signed certificate.
    pub verify_tls: bool,
    pub follow_redirects: bool,
    /// `http://host:port` for every request; `None` uses the environment's
    /// `HTTPS_PROXY` / `HTTP_PROXY`.
    pub proxy: Option<String>,
    /// Keeps the cookies servers set and sends them back, as a browser.
    pub cookies: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            verify_tls: true,
            follow_redirects: true,
            proxy: None,
            cookies: true,
        }
    }
}
