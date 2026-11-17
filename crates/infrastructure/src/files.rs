//! Collections and settings on disk, and the data folder's place.
//!
//! ```text
//! <data dir>/sending-unicorns/
//!   collections/<name>.json   one file per collection (formats.rs)
//!   settings.json
//! ```

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use application::ports::{
    CollectionArchive, CollectionRepository, GlobalsRepository, HistoryRepository,
    SettingsRepository, StoreError,
};
use domain::{Collection, CollectionName, HistoryEntry, KeyValues, Settings};
use serde::{Deserialize, Serialize};

use crate::formats;

/// `<data dir>/sending-unicorns`; `SU_DATA_DIR` points elsewhere (tests,
/// a portable install).
pub fn data_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("SU_DATA_DIR") {
        return Some(PathBuf::from(dir));
    }
    dirs::data_dir().map(|d| d.join("sending-unicorns"))
}

/// Where sendingUnicorns 0.1–0.2 kept its collections.
pub fn legacy_collections_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join(".sending_unicorns").join("collections"))
}

/// The first run of this version: copies the old version's collections
/// (read as they are; rewritten in the new format when next saved). The
/// old folder is left alone. How many files were copied.
///
/// # Errors
///
/// When a file can't be copied.
pub fn migrate_legacy(legacy: &Path, collections: &Path) -> io::Result<usize> {
    if collections.exists() || !legacy.is_dir() {
        return Ok(0);
    }
    fs::create_dir_all(collections)?;
    let mut copied = 0;
    for entry in fs::read_dir(legacy)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "json")
            && let Some(name) = path.file_name()
        {
            fs::copy(&path, collections.join(name))?;
            copied += 1;
        }
    }
    Ok(copied)
}

/// Writes through a temporary file then renames it: a crash never leaves
/// half a file.
fn write_atomically(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

fn store_error(what: &str, path: &Path, e: impl std::fmt::Display) -> StoreError {
    StoreError(format!("{what} {}: {e}", path.display()))
}

/// One JSON file per collection, named after it.
#[derive(Debug, Clone)]
pub struct JsonCollectionRepository {
    dir: PathBuf,
}

impl JsonCollectionRepository {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path(&self, name: &CollectionName) -> PathBuf {
        self.dir.join(format!("{name}.json"))
    }
}

impl CollectionRepository for JsonCollectionRepository {
    fn load_all(&self) -> Result<(Vec<Collection>, Vec<StoreError>), StoreError> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok((Vec::new(), Vec::new())),
            Err(e) => return Err(store_error("could not list", &self.dir, e)),
        };
        let mut collections: Vec<Collection> = Vec::new();
        let mut problems = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let parsed = fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| formats::parse(&text, stem));
            match parsed {
                // The file's name wins: it is where the collection is saved.
                Ok(mut c) => {
                    if let Ok(name) = CollectionName::parse(stem) {
                        c.name = name;
                    }
                    if collections.iter().any(|other| other.name == c.name) {
                        problems.push(store_error(
                            "skipped",
                            &path,
                            "same name as another collection",
                        ));
                    } else {
                        collections.push(c);
                    }
                }
                Err(e) => problems.push(store_error("could not read", &path, e)),
            }
        }
        Ok((collections, problems))
    }

    fn save(&self, collection: &Collection) -> Result<(), StoreError> {
        let path = self.path(&collection.name);
        let text =
            formats::write(collection).map_err(|e| store_error("could not write", &path, e))?;
        write_atomically(&path, &text).map_err(|e| store_error("could not write", &path, e))
    }

    fn delete(&self, name: &CollectionName) -> Result<(), StoreError> {
        let path = self.path(name);
        match fs::remove_file(&path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => {
                Err(store_error("could not delete", &path, e))
            }
            _ => Ok(()),
        }
    }
}

/// Files the user picks to import or export.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonCollectionArchive;

impl CollectionArchive for JsonCollectionArchive {
    fn read(&self, path: &Path) -> Result<Collection, StoreError> {
        let text = fs::read_to_string(path).map_err(|e| store_error("could not read", path, e))?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        // `My API.postman_collection.json`: the name before the first dot.
        let stem = stem.split('.').next().unwrap_or(stem);
        formats::parse(&text, stem).map_err(|e| store_error("could not import", path, e))
    }

    fn write(&self, path: &Path, collection: &Collection) -> Result<(), StoreError> {
        let text =
            formats::write(collection).map_err(|e| store_error("could not export to", path, e))?;
        fs::write(path, text).map_err(|e| store_error("could not export to", path, e))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct SettingsDto {
    timeout_secs: u64,
    verify_tls: bool,
    follow_redirects: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy: Option<String>,
    cookies: bool,
}

impl Default for SettingsDto {
    fn default() -> Self {
        Settings::default().into()
    }
}

impl From<Settings> for SettingsDto {
    fn from(s: Settings) -> Self {
        Self {
            timeout_secs: s.timeout_secs,
            verify_tls: s.verify_tls,
            follow_redirects: s.follow_redirects,
            proxy: s.proxy,
            cookies: s.cookies,
        }
    }
}

#[derive(Debug, Clone)]
pub struct JsonSettingsRepository {
    path: PathBuf,
}

impl JsonSettingsRepository {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl SettingsRepository for JsonSettingsRepository {
    fn load(&self) -> Result<Settings, StoreError> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Settings::default()),
            Err(e) => return Err(store_error("could not read", &self.path, e)),
        };
        let dto: SettingsDto = serde_json::from_str(&text)
            .map_err(|e| store_error("could not read", &self.path, e))?;
        Ok(Settings {
            timeout_secs: dto.timeout_secs,
            verify_tls: dto.verify_tls,
            follow_redirects: dto.follow_redirects,
            proxy: dto.proxy.filter(|p| !p.trim().is_empty()),
            cookies: dto.cookies,
        })
    }

    fn save(&self, settings: &Settings) -> Result<(), StoreError> {
        let text = serde_json::to_string_pretty(&SettingsDto::from(settings.clone()))
            .map_err(|e| store_error("could not write", &self.path, e))?;
        write_atomically(&self.path, &text)
            .map_err(|e| store_error("could not write", &self.path, e))
    }
}

/// Reads a whole file; `None` when there is none yet.
fn read_optional(path: &Path) -> Result<Option<String>, StoreError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(store_error("could not read", path, e)),
    }
}

/// `globals.json`: the variables every collection sees.
#[derive(Debug, Clone)]
pub struct JsonGlobalsRepository {
    path: PathBuf,
}

impl JsonGlobalsRepository {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl GlobalsRepository for JsonGlobalsRepository {
    fn load(&self) -> Result<KeyValues, StoreError> {
        match read_optional(&self.path)? {
            Some(text) => {
                formats::parse_rows(&text).map_err(|e| store_error("could not read", &self.path, e))
            }
            None => Ok(KeyValues::default()),
        }
    }

    fn save(&self, globals: &KeyValues) -> Result<(), StoreError> {
        let text = formats::write_rows(globals)
            .map_err(|e| store_error("could not write", &self.path, e))?;
        write_atomically(&self.path, &text)
            .map_err(|e| store_error("could not write", &self.path, e))
    }
}

/// `history.json`: the requests sent, newest first.
#[derive(Debug, Clone)]
pub struct JsonHistoryRepository {
    path: PathBuf,
}

impl JsonHistoryRepository {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl HistoryRepository for JsonHistoryRepository {
    fn load(&self) -> Result<Vec<HistoryEntry>, StoreError> {
        match read_optional(&self.path)? {
            Some(text) => formats::parse_history(&text)
                .map_err(|e| store_error("could not read", &self.path, e)),
            None => Ok(Vec::new()),
        }
    }

    fn save(&self, entries: &[HistoryEntry]) -> Result<(), StoreError> {
        let text = formats::write_history(entries)
            .map_err(|e| store_error("could not write", &self.path, e))?;
        write_atomically(&self.path, &text)
            .map_err(|e| store_error("could not write", &self.path, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Request;

    fn name(s: &str) -> CollectionName {
        CollectionName::parse(s).unwrap()
    }

    #[test]
    fn saves_loads_and_deletes() {
        let dir = tempfile::tempdir().unwrap();
        let repo = JsonCollectionRepository::new(dir.path().join("collections"));
        assert!(repo.load_all().unwrap().0.is_empty());

        let mut c = Collection::new(name("My API"));
        c.items.push(domain::Item::Request(Request::new("ping")));
        repo.save(&c).unwrap();
        fs::write(dir.path().join("collections/broken.json"), "{").unwrap();
        fs::write(dir.path().join("collections/notes.txt"), "x").unwrap();

        let (loaded, problems) = repo.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, name("My API"));
        assert_eq!(loaded[0].requests()[0].name, "ping");
        assert_eq!(problems.len(), 1);

        repo.delete(&name("My API")).unwrap();
        repo.delete(&name("My API")).unwrap();
        assert!(!dir.path().join("collections/My API.json").exists());
    }

    #[test]
    fn migrates_the_old_folder_once() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("old");
        let new = dir.path().join("new");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("api.json"), r#"{"name":"api","requests":[]}"#).unwrap();
        assert_eq!(migrate_legacy(&legacy, &new).unwrap(), 1);
        assert_eq!(migrate_legacy(&legacy, &new).unwrap(), 0);
        let (loaded, _) = JsonCollectionRepository::new(&new).load_all().unwrap();
        assert_eq!(loaded[0].name, name("api"));
        assert!(legacy.join("api.json").exists());
    }

    #[test]
    fn archive_names_postman_files_without_their_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Shop.postman_collection.json");
        fs::write(&path, r#"{"info":{"name":""},"item":[]}"#).unwrap();
        assert_eq!(
            JsonCollectionArchive.read(&path).unwrap().name,
            name("Shop")
        );
        let out = dir.path().join("out.json");
        JsonCollectionArchive
            .write(&out, &Collection::new(name("x")))
            .unwrap();
        assert_eq!(JsonCollectionArchive.read(&out).unwrap().name, name("x"));
    }

    #[test]
    fn globals_and_history_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let globals = JsonGlobalsRepository::new(dir.path().join("globals.json"));
        assert!(globals.load().unwrap().is_empty());
        let rows = KeyValues(vec![domain::KeyValue::new("a", "1")]);
        globals.save(&rows).unwrap();
        assert_eq!(globals.load().unwrap(), rows);

        let history = JsonHistoryRepository::new(dir.path().join("history.json"));
        assert!(history.load().unwrap().is_empty());
        let mut request = Request::new("ping");
        request.url = "http://h/:id".into();
        request.sync_params_from_url();
        let entry = HistoryEntry {
            at: 42,
            request,
            status: None,
            elapsed_ms: 5,
        };
        history.save(std::slice::from_ref(&entry)).unwrap();
        let back = history.load().unwrap();
        assert_eq!(
            (back[0].at, back[0].status, back[0].elapsed_ms),
            (42, None, 5)
        );
        assert_eq!(back[0].request.url, "http://h/:id");
    }

    #[test]
    fn settings_default_then_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let repo = JsonSettingsRepository::new(dir.path().join("settings.json"));
        assert_eq!(repo.load().unwrap(), Settings::default());
        let s = Settings {
            timeout_secs: 5,
            verify_tls: false,
            follow_redirects: false,
            proxy: Some("http://proxy:3128".into()),
            cookies: false,
        };
        repo.save(&s).unwrap();
        assert_eq!(repo.load().unwrap(), s);
        fs::write(dir.path().join("settings.json"), r#"{"timeout_secs": 9}"#).unwrap();
        assert_eq!(repo.load().unwrap().timeout_secs, 9);
        assert!(repo.load().unwrap().verify_tls);
    }
}
