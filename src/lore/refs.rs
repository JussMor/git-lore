use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{AtomState, LoreAtom, Workspace};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AcceptedLoreRecord {
    pub atom: LoreAtom,
    pub accepted_unix_seconds: u64,
    pub source_commit: Option<String>,
}

impl AcceptedLoreRecord {
    pub fn new(atom: LoreAtom, source_commit: Option<String>) -> Self {
        Self {
            atom,
            accepted_unix_seconds: now_unix_seconds(),
            source_commit,
        }
    }
}

impl Workspace {
    pub fn write_accepted_atom(&self, atom: &LoreAtom, source_commit: Option<&str>) -> Result<()> {
        let mut accepted_atom = atom.clone();
        accepted_atom.state = AtomState::Accepted;

        let record = AcceptedLoreRecord::new(
            accepted_atom.clone(),
            source_commit.map(str::to_owned),
        );
        let path = self
            .refs_lore_accepted_dir()
            .join(format!("{}.json", accepted_atom.id));

        self.write_json(&path, &record)
    }

    pub fn load_accepted_atoms(&self) -> Result<Vec<AcceptedLoreRecord>> {
        self.ensure_layout()?;

        let mut records = Vec::new();
        for entry in std::fs::read_dir(self.refs_lore_accepted_dir())? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }

            let record: AcceptedLoreRecord = self.read_json(&path)?;
            records.push(record);
        }

        records.sort_by(|left, right| {
            left.accepted_unix_seconds
                .cmp(&right.accepted_unix_seconds)
                .then(left.atom.id.cmp(&right.atom.id))
        });
        Ok(records)
    }
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}