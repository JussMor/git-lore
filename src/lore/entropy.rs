use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::merge::{MergeConflictKind, MergeOutcome};
use super::{AtomState, LoreAtom, Workspace, WorkspaceState};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Contradiction {
    pub key: String,
    pub kind: MergeConflictKind,
    pub message: String,
    pub atoms: Vec<LoreAtom>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntropyReport {
    pub score: u8,
    pub total_atoms: usize,
    pub draft_atoms: usize,
    pub proposed_atoms: usize,
    pub accepted_atoms: usize,
    pub deprecated_atoms: usize,
    pub open_question_atoms: usize,
    pub signal_atoms: usize,
    pub distinct_locations: usize,
    pub contradictions: Vec<Contradiction>,
    pub notes: Vec<String>,
}

pub fn analyze_workspace(state: &WorkspaceState) -> EntropyReport {
    build_report(&state.atoms, Vec::new())
}

pub fn analyze_merge_outcome(outcome: &MergeOutcome) -> EntropyReport {
    let merge_contradictions = outcome
        .conflicts
        .iter()
        .map(contradiction_from_merge_conflict)
        .collect::<Vec<_>>();

    build_report(&outcome.merged, merge_contradictions)
}

impl Workspace {
    pub fn contradiction_report(&self) -> anyhow::Result<EntropyReport> {
        self.entropy_report()
    }
}

fn build_report(atoms: &[LoreAtom], extra_contradictions: Vec<Contradiction>) -> EntropyReport {
    let counts = atom_counts(atoms);
    let mut contradictions = locate_contradictions(atoms);
    contradictions.extend(extra_contradictions);
    contradictions = deduplicate_contradictions(contradictions);

    let distinct_locations = distinct_location_count(atoms);
    let score = entropy_score(&counts, distinct_locations, contradictions.len());
    let mut notes = entropy_notes(score, contradictions.len(), counts.accepted_atoms, counts.proposed_atoms);

    if contradictions.is_empty() {
        notes.push("No contradictions detected.".to_string());
    } else {
        notes.push(format!("{} contradiction(s) reported.", contradictions.len()));
    }

    EntropyReport {
        score,
        total_atoms: atoms.len(),
        draft_atoms: counts.draft_atoms,
        proposed_atoms: counts.proposed_atoms,
        accepted_atoms: counts.accepted_atoms,
        deprecated_atoms: counts.deprecated_atoms,
        open_question_atoms: counts.open_question_atoms,
        signal_atoms: counts.signal_atoms,
        distinct_locations,
        contradictions,
        notes,
    }
}

#[derive(Default)]
struct AtomCounts {
    draft_atoms: usize,
    proposed_atoms: usize,
    accepted_atoms: usize,
    deprecated_atoms: usize,
    open_question_atoms: usize,
    signal_atoms: usize,
}

fn atom_counts(atoms: &[LoreAtom]) -> AtomCounts {
    let mut counts = AtomCounts::default();

    for atom in atoms {
        match atom.state {
            AtomState::Draft => counts.draft_atoms += 1,
            AtomState::Proposed => counts.proposed_atoms += 1,
            AtomState::Accepted => counts.accepted_atoms += 1,
            AtomState::Deprecated => counts.deprecated_atoms += 1,
        }

        match atom.kind {
            super::LoreKind::OpenQuestion => counts.open_question_atoms += 1,
            super::LoreKind::Signal => counts.signal_atoms += 1,
            _ => {}
        }
    }

    counts
}

fn entropy_score(counts: &AtomCounts, distinct_locations: usize, contradiction_count: usize) -> u8 {
    let unresolved = counts.draft_atoms + counts.proposed_atoms + counts.open_question_atoms + counts.signal_atoms;
    let mut score = 0i32;

    score += (unresolved as i32) * 12;
    score += (distinct_locations as i32) * 2;
    score += (contradiction_count as i32) * 18;
    score -= (counts.accepted_atoms as i32) * 6;
    score -= (counts.deprecated_atoms as i32) * 4;

    score.clamp(0, 100) as u8
}

fn entropy_notes(score: u8, contradiction_count: usize, accepted_atoms: usize, proposed_atoms: usize) -> Vec<String> {
    let mut notes = Vec::new();

    if score >= 70 {
        notes.push("High entropy: unresolved rationale dominates the current state.".to_string());
    } else if score >= 35 {
        notes.push("Moderate entropy: active rationale still competes with finalized decisions.".to_string());
    } else {
        notes.push("Low entropy: accepted decisions dominate the current state.".to_string());
    }

    if contradiction_count > 0 {
        notes.push(format!("{} contradiction(s) contribute to the score.", contradiction_count));
    }

    if accepted_atoms > proposed_atoms {
        notes.push("Decision-heavy state detected.".to_string());
    }

    notes
}

fn distinct_location_count(atoms: &[LoreAtom]) -> usize {
    atoms
        .iter()
        .map(location_key)
        .collect::<BTreeSet<_>>()
        .len()
}

fn locate_contradictions(atoms: &[LoreAtom]) -> Vec<Contradiction> {
    let mut grouped: BTreeMap<String, Vec<LoreAtom>> = BTreeMap::new();

    for atom in atoms {
        if atom.state != AtomState::Deprecated {
            grouped.entry(location_key(atom)).or_default().push(atom.clone());
        }
    }

    let mut contradictions = Vec::new();

    for (key, group) in grouped {
        if group.len() < 2 {
            continue;
        }

        if let Some(contradiction) = type_contradiction(&key, &group) {
            contradictions.push(contradiction);
        }
    }

    contradictions
}

fn type_contradiction(key: &str, atoms: &[LoreAtom]) -> Option<Contradiction> {
    let first = &atoms[0];
    let mut variations = atoms.iter().filter(|atom| !same_lore_content(first, atom));

    if variations.next().is_some() {
        Some(Contradiction {
            key: key.to_string(),
            kind: MergeConflictKind::TypeConflict,
            message: format!("Type conflict at {key}: multiple rationale variants exist for the same location"),
            atoms: atoms.to_vec(),
        })
    } else {
        None
    }
}

fn contradiction_from_merge_conflict(conflict: &super::merge::MergeConflict) -> Contradiction {
    let mut atoms = Vec::new();
    if let Some(atom) = conflict.base.clone() {
        atoms.push(atom);
    }
    if let Some(atom) = conflict.left.clone() {
        atoms.push(atom);
    }
    if let Some(atom) = conflict.right.clone() {
        atoms.push(atom);
    }

    Contradiction {
        key: conflict.key.clone(),
        kind: conflict.kind.clone(),
        message: conflict.message.clone(),
        atoms,
    }
}

fn deduplicate_contradictions(contradictions: Vec<Contradiction>) -> Vec<Contradiction> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();

    for contradiction in contradictions {
        let mut atom_ids = contradiction
            .atoms
            .iter()
            .map(|atom| atom.id.as_str())
            .collect::<Vec<_>>();
        atom_ids.sort_unstable();
        let signature = format!("{}::{:?}::{}", contradiction.key, contradiction.kind, atom_ids.join(","));

        if seen.insert(signature) {
            deduped.push(contradiction);
        }
    }

    deduped.sort_by(|left, right| {
        left.key
            .cmp(&right.key)
            .then(format!("{:?}", left.kind).cmp(&format!("{:?}", right.kind)))
    });
    deduped
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

fn same_lore_content(left: &LoreAtom, right: &LoreAtom) -> bool {
    left.kind == right.kind
        && left.title == right.title
        && left.body == right.body
        && left.scope == right.scope
        && left.path == right.path
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
    fn entropy_report_flags_conflicting_state() {
        let state = WorkspaceState {
            version: 1,
            atoms: vec![
                atom("accepted-1", LoreKind::Decision, AtomState::Accepted, "Use SQLite", None),
                atom("proposed-1", LoreKind::Decision, AtomState::Proposed, "Use PostgreSQL", None),
            ],
        };

        let report = analyze_workspace(&state);
        assert!(report.score > 0);
        assert_eq!(report.contradictions.len(), 1);
        assert_eq!(report.contradictions[0].kind, MergeConflictKind::TypeConflict);
    }

    #[test]
    fn merge_outcome_contradictions_feed_entropy_reports() {
        let conflict = super::super::merge::MergeConflict {
            key: "src/lib.rs::compute".to_string(),
            kind: MergeConflictKind::DependencyConflict,
            message: "Dependency conflict at src/lib.rs::compute".to_string(),
            base: Some(atom("base", LoreKind::Decision, AtomState::Accepted, "Keep helper", None)),
            left: Some(atom("left", LoreKind::Decision, AtomState::Deprecated, "Remove helper", None)),
            right: Some(atom("right", LoreKind::Decision, AtomState::Accepted, "Keep helper", None)),
        };

        let outcome = MergeOutcome {
            merged: vec![atom("merged", LoreKind::Decision, AtomState::Accepted, "Keep helper", None)],
            conflicts: vec![conflict],
            notes: vec![],
        };

        let report = analyze_merge_outcome(&outcome);
        assert_eq!(report.contradictions.len(), 1);
        assert_eq!(report.contradictions[0].kind, MergeConflictKind::DependencyConflict);
    }
}