//! In-memory ports for the use case tests (R-ARCH-01).

use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use domain::{
    Collection, CollectionName, HistoryEntry, KeyValues, PreparedRequest, Response, Settings,
};

use crate::ports::{
    CollectionArchive, CollectionRepository, GlobalsRepository, HistoryRepository, HttpClient,
    HttpError, SettingsRepository, StoreError,
};

#[derive(Debug, Default)]
pub struct MemoryGlobals(Mutex<KeyValues>);

impl MemoryGlobals {
    pub fn saved(&self) -> KeyValues {
        self.0.lock().unwrap().clone()
    }
}

impl GlobalsRepository for MemoryGlobals {
    fn load(&self) -> Result<KeyValues, StoreError> {
        Ok(self.saved())
    }

    fn save(&self, globals: &KeyValues) -> Result<(), StoreError> {
        *self.0.lock().unwrap() = globals.clone();
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct MemoryHistory(Mutex<Vec<HistoryEntry>>);

impl MemoryHistory {
    pub fn saved(&self) -> Vec<HistoryEntry> {
        self.0.lock().unwrap().clone()
    }
}

impl HistoryRepository for MemoryHistory {
    fn load(&self) -> Result<Vec<HistoryEntry>, StoreError> {
        Ok(self.saved())
    }

    fn save(&self, entries: &[HistoryEntry]) -> Result<(), StoreError> {
        *self.0.lock().unwrap() = entries.to_vec();
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct MemoryRepository {
    saved: Mutex<Vec<Collection>>,
    failing: bool,
}

impl MemoryRepository {
    pub fn failing() -> Self {
        Self {
            failing: true,
            ..Self::default()
        }
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<_> = self
            .saved
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.name.to_string())
            .collect();
        names.sort();
        names
    }
}

impl CollectionRepository for MemoryRepository {
    fn load_all(&self) -> Result<(Vec<Collection>, Vec<StoreError>), StoreError> {
        Ok((self.saved.lock().unwrap().clone(), Vec::new()))
    }

    fn save(&self, collection: &Collection) -> Result<(), StoreError> {
        if self.failing {
            return Err(StoreError("disk full".into()));
        }
        let mut saved = self.saved.lock().unwrap();
        saved.retain(|c| c.name != collection.name);
        saved.push(collection.clone());
        Ok(())
    }

    fn delete(&self, name: &CollectionName) -> Result<(), StoreError> {
        self.saved.lock().unwrap().retain(|c| &c.name != name);
        Ok(())
    }
}

#[derive(Debug)]
pub struct FakeArchive {
    content: Collection,
    written: Mutex<Vec<String>>,
}

impl FakeArchive {
    pub fn with(content: Collection) -> Self {
        Self {
            content,
            written: Mutex::default(),
        }
    }

    pub fn written(&self) -> Vec<String> {
        self.written.lock().unwrap().clone()
    }
}

impl CollectionArchive for FakeArchive {
    fn read(&self, _path: &Path) -> Result<Collection, StoreError> {
        Ok(self.content.clone())
    }

    fn write(&self, _path: &Path, collection: &Collection) -> Result<(), StoreError> {
        self.written
            .lock()
            .unwrap()
            .push(collection.name.to_string());
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct FakeClient {
    sent: Mutex<Vec<String>>,
}

impl FakeClient {
    pub fn sent(&self) -> Vec<String> {
        self.sent.lock().unwrap().clone()
    }
}

impl HttpClient for FakeClient {
    fn send(&self, request: &PreparedRequest, _settings: &Settings) -> Result<Response, HttpError> {
        self.sent.lock().unwrap().push(request.url.clone());
        Ok(Response {
            status: 200,
            reason: "OK".into(),
            headers: Vec::new(),
            body: Vec::new(),
            elapsed: Duration::from_millis(1),
        })
    }
}

#[derive(Debug, Default)]
pub struct MemorySettings {
    saved: Mutex<Option<Settings>>,
    broken: bool,
}

impl MemorySettings {
    pub fn broken() -> Self {
        Self {
            broken: true,
            ..Self::default()
        }
    }
}

impl SettingsRepository for MemorySettings {
    fn load(&self) -> Result<Settings, StoreError> {
        if self.broken {
            return Err(StoreError("settings.toml: invalid".into()));
        }
        Ok(self.saved.lock().unwrap().clone().unwrap_or_default())
    }

    fn save(&self, settings: &Settings) -> Result<(), StoreError> {
        *self.saved.lock().unwrap() = Some(settings.clone());
        Ok(())
    }
}
