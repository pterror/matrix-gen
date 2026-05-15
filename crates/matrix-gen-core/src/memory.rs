use serde::{Deserialize, Serialize};

use crate::agent::AgentId;

pub type Tick = u64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memory {
    pub entries: Vec<MemoryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub tick: Tick,
    pub content: String,
    pub source: MemorySource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum MemorySource {
    Historical,
    SelfAction,
    Observed { from: AgentId },
}

impl Memory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn with_history<I, S>(historical: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let entries = historical
            .into_iter()
            .map(|s| MemoryEntry {
                tick: 0,
                content: s.into(),
                source: MemorySource::Historical,
            })
            .collect();
        Self { entries }
    }

    pub fn push(&mut self, entry: MemoryEntry) {
        self.entries.push(entry);
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_history_marks_entries_as_historical() {
        let m = Memory::with_history(["hello", "goodbye"]);
        assert_eq!(m.entries.len(), 2);
        assert!(matches!(m.entries[0].source, MemorySource::Historical));
        assert_eq!(m.entries[0].tick, 0);
    }

    #[test]
    fn observed_source_roundtrip() {
        let entry = MemoryEntry {
            tick: 42,
            content: "they mentioned ferns".into(),
            source: MemorySource::Observed { from: AgentId(7) },
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: MemoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, back);
    }
}
