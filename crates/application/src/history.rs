//! The history of sent requests: loaded at start-up, one entry per send,
//! cleared on demand.

use std::sync::Arc;

use domain::{History, HistoryEntry};

use crate::ports::{HistoryRepository, StoreError};

#[derive(Debug, Clone)]
pub struct LoadHistory {
    repository: Arc<dyn HistoryRepository>,
}

impl LoadHistory {
    pub fn new(repository: Arc<dyn HistoryRepository>) -> Self {
        Self { repository }
    }

    /// The saved history; an empty one, and why, when it can't be read.
    pub fn execute(&self) -> (History, Option<StoreError>) {
        match self.repository.load() {
            Ok(entries) => (History::from_entries(entries), None),
            Err(e) => (History::default(), Some(e)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecordHistory {
    repository: Arc<dyn HistoryRepository>,
}

impl RecordHistory {
    pub fn new(repository: Arc<dyn HistoryRepository>) -> Self {
        Self { repository }
    }

    /// Adds `entry` (kept in memory even when it can't be written).
    ///
    /// # Errors
    ///
    /// When the history could not be written.
    pub fn execute(&self, history: &mut History, entry: HistoryEntry) -> Result<(), StoreError> {
        history.record(entry);
        self.repository
            .save(&history.entries().cloned().collect::<Vec<_>>())
    }

    /// Forgets every entry.
    ///
    /// # Errors
    ///
    /// When the empty history could not be written.
    pub fn clear(&self, history: &mut History) -> Result<(), StoreError> {
        history.clear();
        self.repository.save(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::MemoryHistory;
    use domain::Request;

    #[test]
    fn records_saves_and_clears() {
        let repo = Arc::new(MemoryHistory::default());
        let (mut history, error) = LoadHistory::new(repo.clone()).execute();
        assert!(history.is_empty() && error.is_none());
        let record = RecordHistory::new(repo.clone());
        for at in [1, 2] {
            record
                .execute(
                    &mut history,
                    HistoryEntry {
                        at,
                        request: Request::new("r"),
                        status: Some(200),
                        elapsed_ms: 3,
                    },
                )
                .unwrap();
        }
        assert_eq!(
            repo.saved().iter().map(|e| e.at).collect::<Vec<_>>(),
            [2, 1]
        );
        record.clear(&mut history).unwrap();
        assert!(history.is_empty() && repo.saved().is_empty());
    }
}
