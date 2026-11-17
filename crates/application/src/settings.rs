//! Reading and changing how requests are sent.

use std::sync::Arc;

use domain::Settings;

use crate::ports::{SettingsRepository, StoreError};

#[derive(Debug, Clone)]
pub struct LoadSettings {
    repository: Arc<dyn SettingsRepository>,
}

impl LoadSettings {
    pub fn new(repository: Arc<dyn SettingsRepository>) -> Self {
        Self { repository }
    }

    /// The saved settings; the defaults, and why, when they can't be read.
    pub fn execute(&self) -> (Settings, Option<StoreError>) {
        match self.repository.load() {
            Ok(settings) => (settings, None),
            Err(e) => (Settings::default(), Some(e)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChangeSettings {
    repository: Arc<dyn SettingsRepository>,
}

impl ChangeSettings {
    pub fn new(repository: Arc<dyn SettingsRepository>) -> Self {
        Self { repository }
    }

    /// A timeout of 0 would never answer: it becomes 1 second.
    ///
    /// # Errors
    ///
    /// When the settings could not be written.
    pub fn execute(&self, mut settings: Settings) -> Result<Settings, StoreError> {
        settings.timeout_secs = settings.timeout_secs.max(1);
        self.repository.save(&settings)?;
        Ok(settings)
    }
}

/// The cookies kept between requests: to list them, or forget them.
#[derive(Debug, Clone)]
pub struct ManageCookies {
    client: Arc<dyn crate::ports::HttpClient>,
}

impl ManageCookies {
    pub fn new(client: Arc<dyn crate::ports::HttpClient>) -> Self {
        Self { client }
    }

    pub fn list(&self) -> Vec<domain::Cookie> {
        self.client.cookies()
    }

    pub fn clear(&self) {
        self.client.clear_cookies();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::MemorySettings;

    #[test]
    fn changes_are_saved_and_read_back() {
        let repo = Arc::new(MemorySettings::default());
        let saved = ChangeSettings::new(repo.clone())
            .execute(Settings {
                timeout_secs: 0,
                verify_tls: false,
                ..Settings::default()
            })
            .unwrap();
        assert_eq!(saved.timeout_secs, 1);
        assert_eq!(LoadSettings::new(repo).execute(), (saved, None));
    }

    #[test]
    fn unreadable_settings_fall_back_to_defaults() {
        let repo = Arc::new(MemorySettings::broken());
        let (settings, error) = LoadSettings::new(repo).execute();
        assert_eq!(settings, Settings::default());
        assert!(error.is_some());
    }
}
