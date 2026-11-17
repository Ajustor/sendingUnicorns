//! The requests sent, newest first, to find and send again.

use std::collections::VecDeque;

use crate::request::Request;

/// One send: the request as it was, and how it went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    /// Unix seconds.
    pub at: u64,
    pub request: Request,
    /// `None` when no response came.
    pub status: Option<u16>,
    pub elapsed_ms: u64,
}

/// At most [`History::CAPACITY`] entries; the oldest go first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History(VecDeque<HistoryEntry>);

impl History {
    pub const CAPACITY: usize = 300;

    /// From saved entries, newest first.
    pub fn from_entries(entries: Vec<HistoryEntry>) -> Self {
        let mut history = Self(entries.into());
        history.0.truncate(Self::CAPACITY);
        history
    }

    pub fn record(&mut self, entry: HistoryEntry) {
        self.0.push_front(entry);
        self.0.truncate(Self::CAPACITY);
    }

    pub fn entries(&self) -> impl Iterator<Item = &HistoryEntry> {
        self.0.iter()
    }

    pub fn get(&self, index: usize) -> Option<&HistoryEntry> {
        self.0.get(index)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(at: u64) -> HistoryEntry {
        HistoryEntry {
            at,
            request: Request::new("r"),
            status: Some(200),
            elapsed_ms: 1,
        }
    }

    #[test]
    fn newest_first_and_capped() {
        let mut h = History::default();
        for at in 0..(History::CAPACITY as u64 + 5) {
            h.record(entry(at));
        }
        assert_eq!(h.len(), History::CAPACITY);
        assert_eq!(h.get(0).unwrap().at, History::CAPACITY as u64 + 4);
        assert_eq!(h.entries().last().unwrap().at, 5);
    }
}
