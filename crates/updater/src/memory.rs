use std::fs;
use std::path::PathBuf;

/// What the updater remembers across runs, as small text files in `dir`:
/// when it last checked, and which version the user chose to skip.
#[derive(Debug, Clone)]
pub struct UpdateMemory {
    dir: PathBuf,
}

impl UpdateMemory {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn read(&self, name: &str) -> Option<String> {
        let text = fs::read_to_string(self.dir.join(name)).ok()?;
        let text = text.trim();
        (!text.is_empty()).then(|| text.to_owned())
    }

    /// Best effort: a lost value only means one more check or prompt.
    fn write(&self, name: &str, value: &str) {
        let path = self.dir.join(name);
        if let Err(e) = fs::create_dir_all(&self.dir).and_then(|_| fs::write(&path, value)) {
            log::debug!("writing {}: {e}", path.display());
        }
    }

    /// Unix time of the last completed automatic check.
    pub fn last_check(&self) -> Option<u64> {
        self.read("last_update_check")?.parse().ok()
    }

    pub fn record_check(&self, now: u64) {
        self.write("last_update_check", &now.to_string());
    }

    pub fn skipped_version(&self) -> Option<String> {
        self.read("skipped_version")
    }

    pub fn skip_version(&self, version: &str) {
        self.write("skipped_version", version);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_round_trip_and_start_empty() {
        let dir = tempfile::tempdir().unwrap();
        let m = UpdateMemory::new(dir.path().join("nested"));
        assert_eq!(m.last_check(), None);
        assert_eq!(m.skipped_version(), None);
        m.record_check(42);
        m.skip_version("1.2.3");
        assert_eq!(m.last_check(), Some(42));
        assert_eq!(m.skipped_version().as_deref(), Some("1.2.3"));
    }
}
