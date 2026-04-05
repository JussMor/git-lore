use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{AtomState, LoreAtom, WorkspaceState};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeConflictKind {
    TypeConflict,
    DependencyConflict,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MergeConflict {
    pub key: String,
    pub kind: MergeConflictKind,
    pub message: String,
    pub base: Option<LoreAtom>,
    pub left: Option<LoreAtom>,
    pub right: Option<LoreAtom>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MergeOutcome {
    pub merged: Vec<LoreAtom>,
    pub conflicts: Vec<MergeConflict>,
    pub notes: Vec<String>,
}

pub fn reconcile_lore(base: &WorkspaceState, left: &WorkspaceState, right: &WorkspaceState) -> MergeOutcome {
    let base_map = group_by_location(&base.atoms);
    let left_map = group_by_location(&left.atoms);
    let right_map = group_by_location(&right.atoms);

    let keys = base_map
        .keys()
        .chain(left_map.keys())
        .chain(right_map.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut merged = Vec::new();
    let mut conflicts = Vec::new();
    let mut notes = Vec::new();

    for key in keys {
        let base_atom = base_map.get(&key).cloned();
        let left_atom = left_map.get(&key).cloned();
        let right_atom = right_map.get(&key).cloned();

        match (&base_atom, &left_atom, &right_atom) {
            (Some(base_atom), Some(left_atom), Some(right_atom)) if equivalent_atom(left_atom, right_atom) => {
                merged.push(left_atom.clone());
                if !equivalent_atom(base_atom, left_atom) {
                    notes.push(format!("Merged identical change at {key}"));
                }
            }
            (Some(base_atom), Some(left_atom), Some(right_atom)) if equivalent_atom(base_atom, left_atom) => {
                merged.push(right_atom.clone());
                notes.push(format!("Right branch updated {key}"));
            }
            (Some(base_atom), Some(left_atom), Some(right_atom)) if equivalent_atom(base_atom, right_atom) => {
                merged.push(left_atom.clone());
                notes.push(format!("Left branch updated {key}"));
            }
            (Some(base_atom), Some(left_atom), Some(right_atom)) => {
                let (winner, conflict_kind, message) = resolve_conflict(&key, base_atom, left_atom, right_atom);
                merged.push(winner);
                conflicts.push(MergeConflict {
                    key: key.clone(),
                    kind: conflict_kind,
                    message,
                    base: Some(base_atom.clone()),
                    left: Some(left_atom.clone()),
                    right: Some(right_atom.clone()),
                });
            }
            (None, Some(left_atom), Some(right_atom)) if equivalent_atom(left_atom, right_atom) => {
                merged.push(left_atom.clone());
                notes.push(format!("Added identical atom at {key} from both branches"));
            }
            (None, Some(left_atom), Some(right_atom)) => {
                let (winner, conflict_kind, message) = resolve_conflict(&key, left_atom, left_atom, right_atom);
                merged.push(winner);
                conflicts.push(MergeConflict {
                    key: key.clone(),
                    kind: conflict_kind,
                    message,
                    base: None,
                    left: Some(left_atom.clone()),
                    right: Some(right_atom.clone()),
                });
            }
            (Some(base_atom), Some(left_atom), None) => {
                merged.push(left_atom.clone());
                if equivalent_atom(base_atom, left_atom) {
                    notes.push(format!("Carried forward unchanged atom at {key}"));
                } else {
                    notes.push(format!("Left branch changed {key}"));
                }
            }
            (Some(base_atom), None, Some(right_atom)) => {
                merged.push(right_atom.clone());
                if equivalent_atom(base_atom, right_atom) {
                    notes.push(format!("Carried forward unchanged atom at {key}"));
                } else {
                    notes.push(format!("Right branch changed {key}"));
                }
            }
            (None, Some(left_atom), None) => {
                merged.push(left_atom.clone());
                notes.push(format!("Added left-only atom at {key}"));
            }
            (None, None, Some(right_atom)) => {
                merged.push(right_atom.clone());
                notes.push(format!("Added right-only atom at {key}"));
            }
            (Some(base_atom), None, None) => {
                merged.push(base_atom.clone());
            }
            (None, None, None) => {}
        }
    }

    merged.sort_by(|left, right| location_key(left).cmp(&location_key(right)).then(left.id.cmp(&right.id)));

    MergeOutcome {
        merged,
        conflicts,
        notes,
    }
}

fn group_by_location(atoms: &[LoreAtom]) -> BTreeMap<String, LoreAtom> {
    let mut grouped = BTreeMap::new();
    for atom in atoms {
        grouped
            .entry(location_key(atom))
            .and_modify(|existing: &mut LoreAtom| {
                if is_newer(atom, existing) {
                    *existing = atom.clone();
                }
            })
            .or_insert_with(|| atom.clone());
    }

    grouped
}

fn location_key(atom: &LoreAtom) -> String {
    let path = atom
        .path
        .as_ref()
        .map(|value| value.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| "<no-path>".to_string());
    let scope = atom.scope.as_deref().unwrap_or("<no-scope>");
    format!("{path}::{scope}")
}

fn equivalent_atom(left: &LoreAtom, right: &LoreAtom) -> bool {
    left.kind == right.kind
        && left.state == right.state
        && left.title == right.title
        && left.body == right.body
        && left.scope == right.scope
        && left.path == right.path
}

fn is_newer(candidate: &LoreAtom, current: &LoreAtom) -> bool {
    candidate.created_unix_seconds > current.created_unix_seconds
        || (candidate.created_unix_seconds == current.created_unix_seconds && candidate.id > current.id)
}

fn resolve_conflict(
    key: &str,
    base: &LoreAtom,
    left: &LoreAtom,
    right: &LoreAtom,
) -> (LoreAtom, MergeConflictKind, String) {
    if left.state == AtomState::Deprecated || right.state == AtomState::Deprecated {
        let winner = if left.state != AtomState::Deprecated {
            left.clone()
        } else {
            right.clone()
        };
        return (
            winner,
            MergeConflictKind::DependencyConflict,
            format!("Dependency conflict at {key}: one branch deprecated an atom while the other kept it active"),
        );
    }

    if left.kind != right.kind || left.title != right.title {
        return (
            left.clone(),
            MergeConflictKind::TypeConflict,
            format!("Type conflict at {key}: branches disagree on the lore atom shape or intent"),
        );
    }

    if !equivalent_atom(base, left) && equivalent_atom(base, right) {
        return (
            left.clone(),
            MergeConflictKind::TypeConflict,
            format!("Left branch diverged at {key}"),
        );
    }

    if !equivalent_atom(base, right) && equivalent_atom(base, left) {
        return (
            right.clone(),
            MergeConflictKind::TypeConflict,
            format!("Right branch diverged at {key}"),
        );
    }

    (
        left.clone(),
        MergeConflictKind::TypeConflict,
        format!("Type conflict at {key}: branches made incompatible changes"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lore::{AtomState, LoreKind};
    use std::path::PathBuf;

    fn atom(id: &str, kind: LoreKind, state: AtomState, title: &str, body: Option<&str>) -> LoreAtom {
        LoreAtom {
            id: id.to_string(),
            kind,
            state,
            title: title.to_string(),
            body: body.map(str::to_string),
            scope: Some("compute".to_string()),
            path: Some(PathBuf::from("src/lib.rs")),
            validation_script: None,
            created_unix_seconds: 1,
        }
    }

    #[test]
    fn additive_merges_keep_non_conflicting_atoms() {
        let base = WorkspaceState { version: 1, atoms: vec![] };
        let left = WorkspaceState {
            version: 1,
            atoms: vec![LoreAtom {
                path: Some(PathBuf::from("src/db.rs")),
                ..atom("left-1", LoreKind::Decision, AtomState::Accepted, "Use SQLite", None)
            }],
        };
        let right = WorkspaceState {
            version: 1,
            atoms: vec![LoreAtom {
                path: Some(PathBuf::from("src/cache.rs")),
                ..atom("right-1", LoreKind::Assumption, AtomState::Accepted, "Cache stays local", None)
            }],
        };

        let outcome = reconcile_lore(&base, &left, &right);
        assert_eq!(outcome.conflicts.len(), 0);
        assert_eq!(outcome.merged.len(), 2);
        assert!(!outcome.notes.is_empty());
    }

    #[test]
    fn type_conflicts_are_reported() {
        let base = WorkspaceState { version: 1, atoms: vec![] };
        let left = WorkspaceState {
            version: 1,
            atoms: vec![atom("left-1", LoreKind::Decision, AtomState::Accepted, "Use OAuth", None)],
        };
        let right = WorkspaceState {
            version: 1,
            atoms: vec![atom("right-1", LoreKind::Decision, AtomState::Accepted, "Use SAML", None)],
        };

        let outcome = reconcile_lore(&base, &left, &right);
        assert_eq!(outcome.conflicts.len(), 1);
        assert_eq!(outcome.conflicts[0].kind, MergeConflictKind::TypeConflict);
    }

    #[test]
    fn dependency_conflicts_are_reported() {
        let base = WorkspaceState { version: 1, atoms: vec![] };
        let left = WorkspaceState {
            version: 1,
            atoms: vec![atom("left-1", LoreKind::Decision, AtomState::Deprecated, "Remove helper", None)],
        };
        let right = WorkspaceState {
            version: 1,
            atoms: vec![atom("right-1", LoreKind::Decision, AtomState::Accepted, "Keep helper", None)],
        };

        let outcome = reconcile_lore(&base, &left, &right);
        assert_eq!(outcome.conflicts.len(), 1);
        assert_eq!(outcome.conflicts[0].kind, MergeConflictKind::DependencyConflict);
    }
}