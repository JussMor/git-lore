use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{LoreAtom, Workspace};

pub const PRISM_STALE_TTL_SECONDS: u64 = 24 * 60 * 60;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PrismSignal {
    pub session_id: String,
    pub agent: Option<String>,
    pub scope: Option<String>,
    pub paths: Vec<String>,
    pub assumptions: Vec<String>,
    pub decision: Option<String>,
    pub created_unix_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrismConflict {
    pub session_id: String,
    pub agent: Option<String>,
    pub scope: Option<String>,
    pub decision: Option<String>,
    pub overlapping_paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardLockViolation {
    pub session_id: String,
    pub message: String,
    pub atom_ids: Vec<String>,
}

impl PrismSignal {
    pub fn new(
        session_id: String,
        agent: Option<String>,
        scope: Option<String>,
        mut paths: Vec<String>,
        assumptions: Vec<String>,
        decision: Option<String>,
    ) -> Self {
        paths.sort();
        paths.dedup();

        Self {
            session_id,
            agent,
            scope,
            paths,
            assumptions,
            decision,
            created_unix_seconds: now_unix_seconds(),
        }
    }
}

impl Workspace {
    pub fn write_prism_signal(&self, signal: &PrismSignal) -> Result<()> {
        self.ensure_layout()?;
        let signal_path = self.prism_signal_path(&signal.session_id);
        self.write_json(&signal_path, signal)
    }

    pub fn remove_prism_signal(&self, session_id: &str) -> Result<bool> {
        self.ensure_layout()?;
        let signal_path = self.prism_signal_path(session_id);
        if !signal_path.exists() {
            return Ok(false);
        }

        fs::remove_file(signal_path)?;
        Ok(true)
    }

    pub fn load_prism_signals(&self) -> Result<Vec<PrismSignal>> {
        self.ensure_layout()?;

        let mut signals = Vec::new();
        for entry in fs::read_dir(self.prism_dir())? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("signal") {
                continue;
            }

            let signal: PrismSignal = self.read_json(&path)?;
            signals.push(signal);
        }

        signals.sort_by(|left, right| left.session_id.cmp(&right.session_id));
        Ok(signals)
    }

    pub fn scan_prism_conflicts(&self, current_signal: &PrismSignal) -> Result<Vec<PrismConflict>> {
        let mut conflicts = Vec::new();
        let now = now_unix_seconds();

        for signal in self.load_prism_signals()? {
            if signal.session_id == current_signal.session_id {
                continue;
            }

            if is_stale_signal(&signal, now, PRISM_STALE_TTL_SECONDS) {
                continue;
            }

            let overlapping_paths = current_signal
                .paths
                .iter()
                .flat_map(|current_path| {
                    signal
                        .paths
                        .iter()
                        .filter(move |other_path| patterns_may_overlap(current_path, other_path))
                        .cloned()
                })
                .collect::<Vec<_>>();

            if overlapping_paths.is_empty() {
                continue;
            }

            conflicts.push(PrismConflict {
                session_id: signal.session_id,
                agent: signal.agent,
                scope: signal.scope,
                decision: signal.decision,
                overlapping_paths,
            });
        }

        Ok(conflicts)
    }

    pub fn scan_prism_hard_locks(&self, active_atoms: &[LoreAtom]) -> Result<Vec<HardLockViolation>> {
        let mut violations = Vec::new();
        let now = now_unix_seconds();

        for signal in self.load_prism_signals()? {
            if is_stale_signal(&signal, now, PRISM_STALE_TTL_SECONDS) {
                continue;
            }

            let Some(decision) = signal.decision.as_deref() else {
                continue;
            };

            let overlapping_atoms = active_atoms
                .iter()
                .filter(|atom| atom_overlaps_signal(atom, &signal))
                .filter(|atom| atom.title != decision && atom.body.as_deref() != Some(decision))
                .collect::<Vec<_>>();

            if overlapping_atoms.is_empty() {
                continue;
            }

            violations.push(HardLockViolation {
                session_id: signal.session_id.clone(),
                message: format!(
                    "hard-lock from session {} blocks overlapping active lore",
                    signal.session_id
                ),
                atom_ids: overlapping_atoms
                    .into_iter()
                    .map(|atom| atom.id.clone())
                    .collect(),
            });
        }

        Ok(violations)
    }

    fn prism_signal_path(&self, session_id: &str) -> PathBuf {
        self.prism_dir().join(format!("{session_id}.signal"))
    }

    pub fn count_stale_prism_signals(&self, stale_ttl_seconds: u64) -> Result<usize> {
        let now = now_unix_seconds();
        Ok(self
            .load_prism_signals()?
            .into_iter()
            .filter(|signal| is_stale_signal(signal, now, stale_ttl_seconds))
            .count())
    }

    pub fn prune_stale_prism_signals(&self, stale_ttl_seconds: u64) -> Result<usize> {
        self.ensure_layout()?;
        let now = now_unix_seconds();
        let mut pruned = 0usize;

        for signal in self.load_prism_signals()? {
            if !is_stale_signal(&signal, now, stale_ttl_seconds) {
                continue;
            }

            let path = self.prism_signal_path(&signal.session_id);
            if path.exists() {
                fs::remove_file(&path)?;
                pruned += 1;
            }
        }

        Ok(pruned)
    }
}

fn patterns_may_overlap(left: &str, right: &str) -> bool {
    let left = normalize_pattern(left);
    let right = normalize_pattern(right);

    if left == right {
        return true;
    }

    let left_has_glob = left.chars().any(is_glob_char);
    let right_has_glob = right.chars().any(is_glob_char);

    if !left_has_glob && !right_has_glob {
        return path_prefix_overlap(&left, &right);
    }

    let left_prefix = literal_prefix(&left);
    let right_prefix = literal_prefix(&right);

    if left_prefix.is_empty() || right_prefix.is_empty() {
        return true;
    }

    left_prefix == right_prefix || path_prefix_overlap(&left_prefix, &right_prefix)
}

fn literal_prefix(pattern: &str) -> String {
    let mut segments = Vec::new();
    for segment in pattern.split('/') {
        if segment.chars().any(is_glob_char) {
            break;
        }
        if segment.is_empty() {
            continue;
        }
        segments.push(segment);
    }

    segments.join("/")
}

fn path_prefix_overlap(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }

    let left = left.trim_end_matches('/');
    let right = right.trim_end_matches('/');

    left.starts_with(&(right.to_string() + "/")) || right.starts_with(&(left.to_string() + "/"))
}

fn normalize_pattern(pattern: &str) -> String {
    pattern.replace('\\', "/")
}

fn is_glob_char(character: char) -> bool {
    matches!(character, '*' | '?' | '[' | ']')
}

fn atom_overlaps_signal(atom: &LoreAtom, signal: &PrismSignal) -> bool {
    let atom_path = atom.path.as_ref().map(|path| path.to_string_lossy().replace('\\', "/"));
    let atom_scope = atom.scope.as_deref();

    signal.paths.iter().any(|signal_path| {
        atom_path
            .as_deref()
            .map(|path| patterns_may_overlap(path, signal_path))
            .unwrap_or(false)
            || atom_scope
                .zip(signal.scope.as_deref())
                .map(|(left, right)| left == right)
                .unwrap_or(false)
    })
}

fn is_stale_signal(signal: &PrismSignal, now_unix_seconds: u64, stale_ttl_seconds: u64) -> bool {
    now_unix_seconds.saturating_sub(signal.created_unix_seconds) > stale_ttl_seconds
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use std::path::PathBuf;

    #[test]
    fn overlapping_signals_are_reported() {
        let temp_root = std::env::temp_dir().join(format!("git-lore-prism-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let workspace = Workspace::init(&temp_root).unwrap();

        let existing = PrismSignal::new(
            "session-b".to_string(),
            Some("agent-b".to_string()),
            Some("db".to_string()),
            vec!["src/db/**".to_string()],
            vec!["Database layer is stable".to_string()],
            Some("refactor db".to_string()),
        );
        workspace.write_prism_signal(&existing).unwrap();

        let current = PrismSignal::new(
            "session-a".to_string(),
            Some("agent-a".to_string()),
            Some("api".to_string()),
            vec!["src/db/models.rs".to_string()],
            vec!["Assuming schema compatibility".to_string()],
            None,
        );

        let conflicts = workspace.scan_prism_conflicts(&current).unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].session_id, "session-b");
        assert_eq!(conflicts[0].overlapping_paths, vec!["src/db/**".to_string()]);
    }

    #[test]
    fn non_overlapping_signals_are_ignored() {
        let temp_root = std::env::temp_dir().join(format!("git-lore-prism-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let workspace = Workspace::init(&temp_root).unwrap();

        let existing = PrismSignal::new(
            "session-b".to_string(),
            Some("agent-b".to_string()),
            Some("docs".to_string()),
            vec!["docs/**/*.md".to_string()],
            vec![],
            None,
        );
        workspace.write_prism_signal(&existing).unwrap();

        let current = PrismSignal::new(
            "session-a".to_string(),
            Some("agent-a".to_string()),
            Some("src".to_string()),
            vec!["src/**/*.rs".to_string()],
            vec![],
            None,
        );

        let conflicts = workspace.scan_prism_conflicts(&current).unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn conflicting_decisions_trigger_a_hard_lock() {
        let temp_root = std::env::temp_dir().join(format!("git-lore-prism-hardlock-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let workspace = Workspace::init(&temp_root).unwrap();

        let existing = PrismSignal::new(
            "session-b".to_string(),
            Some("agent-b".to_string()),
            Some("db".to_string()),
            vec!["src/db/**".to_string()],
            vec![],
            Some("Use SQLite".to_string()),
        );
        workspace.write_prism_signal(&existing).unwrap();

        let atom = LoreAtom {
            id: Uuid::new_v4().to_string(),
            kind: super::super::LoreKind::Decision,
            state: super::super::AtomState::Proposed,
            title: "Use Postgres".to_string(),
            body: None,
            scope: Some("db".to_string()),
            path: Some(PathBuf::from("src/db/models.rs")),
            validation_script: None,
            created_unix_seconds: 0,
        };

        let violations = workspace.scan_prism_hard_locks(&[atom]).unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].session_id, "session-b");
    }

    #[test]
    fn stale_signals_are_ignored_for_conflicts_and_hard_locks() {
        let temp_root = std::env::temp_dir().join(format!("git-lore-prism-stale-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let workspace = Workspace::init(&temp_root).unwrap();

        let mut stale = PrismSignal::new(
            "stale-session".to_string(),
            Some("agent-stale".to_string()),
            Some("db".to_string()),
            vec!["src/db/**".to_string()],
            vec![],
            Some("Use SQLite".to_string()),
        );
        stale.created_unix_seconds = now_unix_seconds().saturating_sub(PRISM_STALE_TTL_SECONDS + 10);
        workspace.write_prism_signal(&stale).unwrap();

        let current = PrismSignal::new(
            "current-session".to_string(),
            Some("agent-current".to_string()),
            Some("db".to_string()),
            vec!["src/db/models.rs".to_string()],
            vec![],
            None,
        );

        let conflicts = workspace.scan_prism_conflicts(&current).unwrap();
        assert!(conflicts.is_empty());

        let atom = LoreAtom {
            id: Uuid::new_v4().to_string(),
            kind: super::super::LoreKind::Decision,
            state: super::super::AtomState::Proposed,
            title: "Use Postgres".to_string(),
            body: None,
            scope: Some("db".to_string()),
            path: Some(PathBuf::from("src/db/models.rs")),
            validation_script: None,
            created_unix_seconds: 0,
        };
        let hard_locks = workspace.scan_prism_hard_locks(&[atom]).unwrap();
        assert!(hard_locks.is_empty());
    }

    #[test]
    fn stale_signal_pruning_removes_expired_entries() {
        let temp_root = std::env::temp_dir().join(format!("git-lore-prism-prune-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let workspace = Workspace::init(&temp_root).unwrap();

        let mut stale = PrismSignal::new(
            "stale-session".to_string(),
            Some("agent-stale".to_string()),
            None,
            vec!["src/**".to_string()],
            vec![],
            None,
        );
        stale.created_unix_seconds = now_unix_seconds().saturating_sub(PRISM_STALE_TTL_SECONDS + 10);
        workspace.write_prism_signal(&stale).unwrap();

        let fresh = PrismSignal::new(
            "fresh-session".to_string(),
            Some("agent-fresh".to_string()),
            None,
            vec!["src/**".to_string()],
            vec![],
            None,
        );
        workspace.write_prism_signal(&fresh).unwrap();

        let stale_before = workspace.count_stale_prism_signals(PRISM_STALE_TTL_SECONDS).unwrap();
        assert_eq!(stale_before, 1);

        let removed = workspace.prune_stale_prism_signals(PRISM_STALE_TTL_SECONDS).unwrap();
        assert_eq!(removed, 1);

        let remaining = workspace.load_prism_signals().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].session_id, "fresh-session");
    }

    #[test]
    fn removing_prism_signal_deletes_specific_session_file() {
        let temp_root = std::env::temp_dir().join(format!("git-lore-prism-remove-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let workspace = Workspace::init(&temp_root).unwrap();

        let signal = PrismSignal::new(
            "session-a".to_string(),
            Some("agent-a".to_string()),
            None,
            vec!["src/**".to_string()],
            vec![],
            Some("ship ui".to_string()),
        );
        workspace.write_prism_signal(&signal).unwrap();

        assert!(workspace.remove_prism_signal("session-a").unwrap());
        assert!(!workspace.remove_prism_signal("session-a").unwrap());
    }
}
