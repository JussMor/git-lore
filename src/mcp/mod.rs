use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::git;
use crate::lore::prism::PRISM_STALE_TTL_SECONDS;
use crate::lore::{AtomState, LoreAtom, LoreKind, StateTransitionPreview, Workspace};
use crate::parser::{detect_scope, ScopeContext};

pub mod transport;

#[cfg(feature = "semantic-search")]
pub mod semantic;

pub use transport::McpServer;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub workspace_root: PathBuf,
    pub file_path: PathBuf,
    pub cursor_line: Option<usize>,
    pub scope: Option<ScopeContext>,
    pub historical_decisions: Vec<HistoricalDecision>,
    pub relevant_atoms: Vec<LoreAtom>,
    pub constraints: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoricalDecision {
    pub commit_hash: String,
    pub subject: String,
    pub trailer_value: String,
    pub file_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ProposalRequest {
    pub file_path: PathBuf,
    pub cursor_line: Option<usize>,
    pub kind: LoreKind,
    pub title: String,
    pub body: Option<String>,
    pub scope: Option<String>,
    /// A literal shell command that validates the atom when preflight runs.
    pub validation_script: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProposalResult {
    pub atom: LoreAtom,
    pub scope: Option<ScopeContext>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProposalAutofill {
    pub title: String,
    pub body: Option<String>,
    pub scope: Option<String>,
    pub filled_fields: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemorySearchHit {
    pub atom: LoreAtom,
    pub source: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemorySearchReport {
    pub workspace_root: PathBuf,
    pub query: String,
    pub file_path: Option<PathBuf>,
    pub cursor_line: Option<usize>,
    pub results: Vec<MemorySearchHit>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub workspace_root: PathBuf,
    pub generated_unix_seconds: u64,
    pub state_checksum: String,
    pub total_atoms: usize,
    pub draft_atoms: usize,
    pub proposed_atoms: usize,
    pub accepted_atoms: usize,
    pub deprecated_atoms: usize,
    pub accepted_records: usize,
    pub lore_refs: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreflightSeverity {
    Block,
    Warn,
    Info,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreflightIssue {
    pub severity: PreflightSeverity,
    pub code: String,
    pub message: String,
    pub atom_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryPreflightReport {
    pub workspace_root: PathBuf,
    pub operation: String,
    pub generated_unix_seconds: u64,
    pub state_checksum: String,
    pub can_proceed: bool,
    pub issues: Vec<PreflightIssue>,
}

#[derive(Clone, Debug)]
pub struct McpService {
    workspace_hint: PathBuf,
}

impl McpService {
    pub fn new(workspace_hint: impl AsRef<Path>) -> Self {
        Self {
            workspace_hint: workspace_hint.as_ref().to_path_buf(),
        }
    }

    pub fn context(&self, file_path: impl AsRef<Path>, cursor_line: Option<usize>) -> Result<ContextSnapshot> {
        let workspace = Workspace::discover(&self.workspace_hint)?;
        let file_path = file_path.as_ref().to_path_buf();
        let scope = detect_scope(&file_path, cursor_line).ok();
        let repository_root = git::discover_repository(&workspace.root()).ok();
        let state = workspace.load_state()?;
        let relevant_atoms = relevant_atoms(&state.atoms, &file_path, scope.as_ref());
        let historical_decisions = repository_root
            .as_ref()
            .map(|root| git::collect_recent_decisions_for_path(root, &file_path, 5))
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .map(|decision| HistoricalDecision {
                commit_hash: decision.commit_hash,
                subject: decision.subject,
                trailer_value: decision.trailer.value,
                file_path: decision.file_path,
            })
            .collect::<Vec<_>>();

        let mut constraints = historical_decisions
            .iter()
            .map(|decision| format!("Decision [{}]: {}", decision.commit_hash, decision.trailer_value))
            .collect::<Vec<_>>();
        constraints.extend(relevant_atoms.iter().map(render_constraint));

        Ok(ContextSnapshot {
            workspace_root: workspace.root().to_path_buf(),
            file_path,
            cursor_line,
            scope,
            historical_decisions,
            relevant_atoms,
            constraints,
        })
    }

    pub fn propose(&self, request: ProposalRequest) -> Result<ProposalResult> {
        let workspace = Workspace::discover(&self.workspace_hint)?;
        let scope = detect_scope(&request.file_path, request.cursor_line).ok();
        let scope_name = request
            .scope
            .or_else(|| scope.as_ref().map(|value| value.name.clone()));

        let atom = LoreAtom::new(
            request.kind,
            AtomState::Proposed,
            request.title,
            request.body,
            scope_name,
            Some(request.file_path.clone()),
        )
        .with_validation_script(request.validation_script);

        workspace.record_atom(atom.clone())?;

        Ok(ProposalResult { atom, scope })
    }

    pub fn autofill_proposal(
        &self,
        file_path: impl AsRef<Path>,
        cursor_line: Option<usize>,
        kind: LoreKind,
        title: Option<String>,
        body: Option<String>,
        scope: Option<String>,
    ) -> Result<ProposalAutofill> {
        let file_path = file_path.as_ref().to_path_buf();
        let detected_scope = detect_scope(&file_path, cursor_line).ok();
        let has_explicit_scope = scope
            .as_deref()
            .map(str::trim)
            .map(|value| !value.is_empty())
            .unwrap_or(false);
        let scope_name = scope
            .filter(|value| !value.trim().is_empty())
            .or_else(|| detected_scope.as_ref().map(|value| value.name.clone()));

        let mut filled_fields = Vec::new();

        let resolved_title = match title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(existing) => existing.to_string(),
            None => {
                filled_fields.push("title".to_string());
                let anchor = scope_name
                    .clone()
                    .unwrap_or_else(|| file_path.display().to_string());
                format!("{} for {}", kind_label(&kind), anchor)
            }
        };

        let resolved_body = match body
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(existing) => Some(existing.to_string()),
            None => {
                filled_fields.push("body".to_string());
                let location = scope_name
                    .clone()
                    .unwrap_or_else(|| file_path.display().to_string());
                Some(format!(
                    "Autofilled rationale for {} at {}. Update with implementation-specific details before commit.",
                    kind_label(&kind).to_lowercase(),
                    location
                ))
            }
        };

        if scope_name.is_some() && !has_explicit_scope {
            filled_fields.push("scope".to_string());
        }

        Ok(ProposalAutofill {
            title: resolved_title,
            body: resolved_body,
            scope: scope_name,
            filled_fields,
        })
    }

    pub fn memory_search(
        &self,
        query: impl AsRef<str>,
        file_path: Option<PathBuf>,
        cursor_line: Option<usize>,
        limit: usize,
    ) -> Result<MemorySearchReport> {
        let query = query.as_ref().trim().to_string();
        if query.is_empty() {
            return Err(anyhow::anyhow!("query must not be empty"));
        }

        let workspace = Workspace::discover(&self.workspace_hint)?;
        let state = workspace.load_state()?;
        let accepted = workspace.load_accepted_atoms()?;
        let scope = file_path
            .as_ref()
            .and_then(|path| detect_scope(path, cursor_line).ok());

        let query_tokens = tokenize(&query);
        let query_lower = query.to_ascii_lowercase();
        let newest_timestamp = state
            .atoms
            .iter()
            .map(|atom| atom.created_unix_seconds)
            .chain(accepted.iter().map(|record| record.atom.created_unix_seconds))
            .max()
            .unwrap_or(0);

        let mut candidates = BTreeMap::<String, (LoreAtom, String)>::new();
        for atom in state.atoms {
            candidates.insert(atom.id.clone(), (atom, "active".to_string()));
        }
        for record in accepted {
            candidates
                .entry(record.atom.id.clone())
                .or_insert((record.atom, "accepted_archive".to_string()));
        }

        let mut hits = Vec::new();

        #[cfg(feature = "semantic-search")]
        {
            if semantic::index_exists(workspace.root()) {
                if let Ok(semantic_results) = semantic::search(workspace.root(), &query, limit.max(5) * 2) {
                    for (id, base_score, source) in semantic_results {
                        if let Some((atom, _)) = candidates.get(&id) {
                            let mut hit = MemorySearchHit {
                                atom: atom.clone(),
                                source,
                                score: base_score * 12.0, // Scale base semantic score
                                reasons: vec![format!("semantic:{:.2}", base_score)],
                            };

                            let state_bonus = match atom.state {
                                AtomState::Accepted => 12.0,
                                AtomState::Proposed => 8.0,
                                AtomState::Draft => 4.0,
                                AtomState::Deprecated => -3.0,
                            };
                            hit.score += state_bonus;
                            hit.reasons.push(format!("state:{:?}", atom.state));

                            if newest_timestamp > 0 && atom.created_unix_seconds > 0 {
                                let normalized = (atom.created_unix_seconds as f64 / newest_timestamp as f64).min(1.0);
                                let recency_bonus = normalized * 6.0;
                                hit.score += recency_bonus;
                                hit.reasons.push(format!("recency:{:.2}", recency_bonus));
                            }

                            if let Some(target_path) = file_path.as_ref() {
                                if atom.path.as_ref() == Some(target_path) {
                                    hit.score += 10.0;
                                    hit.reasons.push("path:exact".to_string());
                                } else if let (Some(atom_path), Some(parent)) = (atom.path.as_ref(), target_path.parent()) {
                                    if atom_path.starts_with(parent) {
                                        hit.score += 4.0;
                                        hit.reasons.push("path:near".to_string());
                                    }
                                }
                            }

                            if let Some(scope_hint) = scope.as_ref() {
                                if atom.scope.as_deref() == Some(scope_hint.name.as_str()) {
                                    hit.score += 6.0;
                                    hit.reasons.push("scope:exact".to_string());
                                }
                            }

                            hits.push(hit);
                        }
                    }
                }
            }
        }

        if hits.is_empty() {
            hits = candidates
                .into_values()
                .filter_map(|(atom, source)| {
                    score_memory_hit(
                        &atom,
                        &source,
                        &query_lower,
                        &query_tokens,
                        file_path.as_ref(),
                        scope.as_ref(),
                        newest_timestamp,
                    )
                })
                .collect::<Vec<_>>();
        }

        hits.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(left.atom.id.cmp(&right.atom.id))
        });
        hits.truncate(limit.max(1));

        Ok(MemorySearchReport {
            workspace_root: workspace.root().to_path_buf(),
            query,
            file_path,
            cursor_line,
            results: hits,
        })
    }

    pub fn state_transition_preview(
        &self,
        atom_id: impl AsRef<str>,
        target_state: AtomState,
    ) -> Result<StateTransitionPreview> {
        let workspace = Workspace::discover(&self.workspace_hint)?;
        workspace.preview_state_transition(atom_id.as_ref(), target_state)
    }

    pub fn state_snapshot(&self) -> Result<StateSnapshot> {
        let workspace = Workspace::discover(&self.workspace_hint)?;
        let state = workspace.load_state()?;
        let encoded_state = serde_json::to_vec(&state)?;
        let state_checksum = fnv1a_hex(&encoded_state);

        let mut draft_atoms = 0usize;
        let mut proposed_atoms = 0usize;
        let mut accepted_atoms = 0usize;
        let mut deprecated_atoms = 0usize;

        for atom in &state.atoms {
            match atom.state {
                AtomState::Draft => draft_atoms += 1,
                AtomState::Proposed => proposed_atoms += 1,
                AtomState::Accepted => accepted_atoms += 1,
                AtomState::Deprecated => deprecated_atoms += 1,
            }
        }

        let accepted_records = workspace.load_accepted_atoms()?.len();
        let lore_refs = git::discover_repository(workspace.root())
            .ok()
            .and_then(|root| git::list_lore_refs(&root).ok().map(|refs| refs.len()))
            .unwrap_or(0);

        Ok(StateSnapshot {
            workspace_root: workspace.root().to_path_buf(),
            generated_unix_seconds: now_unix_seconds(),
            state_checksum,
            total_atoms: state.atoms.len(),
            draft_atoms,
            proposed_atoms,
            accepted_atoms,
            deprecated_atoms,
            accepted_records,
            lore_refs,
        })
    }

    pub fn memory_preflight(&self, operation: impl AsRef<str>) -> Result<MemoryPreflightReport> {
        let operation = operation.as_ref().to_string();
        let workspace = Workspace::discover(&self.workspace_hint)?;
        let state = workspace.load_state()?;
        let snapshot = self.state_snapshot()?;
        let mut issues = Vec::new();

        let mut duplicate_counter = BTreeMap::<String, usize>::new();
        for atom in &state.atoms {
            *duplicate_counter.entry(atom.id.clone()).or_insert(0) += 1;
        }

        let duplicate_ids = duplicate_counter
            .into_iter()
            .filter_map(|(atom_id, count)| (count > 1).then_some(atom_id))
            .collect::<Vec<_>>();

        if !duplicate_ids.is_empty() {
            issues.push(PreflightIssue {
                severity: PreflightSeverity::Block,
                code: "duplicate_atom_ids".to_string(),
                message: "duplicate atom ids detected in active state; run reconciliation before continuing"
                    .to_string(),
                atom_ids: duplicate_ids,
            });
        }

        for issue in workspace.sanitize_report()? {
            issues.push(PreflightIssue {
                severity: PreflightSeverity::Block,
                code: "sanitize_sensitive_content".to_string(),
                message: format!(
                    "sensitive content in {}.{}: {}",
                    issue.atom_id, issue.field, issue.reason
                ),
                atom_ids: vec![issue.atom_id],
            });
        }

        for violation in workspace.scan_prism_hard_locks(&state.atoms)? {
            issues.push(PreflightIssue {
                severity: PreflightSeverity::Block,
                code: "prism_hard_lock".to_string(),
                message: violation.message,
                atom_ids: violation.atom_ids,
            });
        }

        let stale_signal_count = workspace.count_stale_prism_signals(PRISM_STALE_TTL_SECONDS)?;
        if stale_signal_count > 0 {
            issues.push(PreflightIssue {
                severity: PreflightSeverity::Warn,
                code: "prism_stale_signals".to_string(),
                message: format!(
                    "{} stale PRISM signal(s) detected; consider cleanup",
                    stale_signal_count
                ),
                atom_ids: Vec::new(),
            });
        }

        for issue in workspace.validation_report()? {
            issues.push(PreflightIssue {
                severity: PreflightSeverity::Block,
                code: "validation_script_failed".to_string(),
                message: format!("{} ({})", issue.reason, issue.command),
                atom_ids: vec![issue.atom_id],
            });
        }

        let entropy = workspace.entropy_report()?;
        if !entropy.contradictions.is_empty() {
            issues.push(PreflightIssue {
                severity: PreflightSeverity::Warn,
                code: "entropy_contradictions".to_string(),
                message: format!(
                    "{} contradiction(s) detected in active lore",
                    entropy.contradictions.len()
                ),
                atom_ids: Vec::new(),
            });
        }

        if entropy.score >= 70 {
            issues.push(PreflightIssue {
                severity: PreflightSeverity::Warn,
                code: "entropy_high".to_string(),
                message: format!("entropy score is high ({}/100)", entropy.score),
                atom_ids: Vec::new(),
            });
        } else if entropy.score >= 35 {
            issues.push(PreflightIssue {
                severity: PreflightSeverity::Info,
                code: "entropy_moderate".to_string(),
                message: format!("entropy score is moderate ({}/100)", entropy.score),
                atom_ids: Vec::new(),
            });
        }

        if operation == "commit" && state.atoms.is_empty() {
            issues.push(PreflightIssue {
                severity: PreflightSeverity::Warn,
                code: "empty_lore_state".to_string(),
                message: "no active lore atoms recorded for this commit".to_string(),
                atom_ids: Vec::new(),
            });
        }

        if issues.is_empty() {
            issues.push(PreflightIssue {
                severity: PreflightSeverity::Info,
                code: "preflight_clean".to_string(),
                message: "no blocking issues detected".to_string(),
                atom_ids: Vec::new(),
            });
        }

        let can_proceed = !issues
            .iter()
            .any(|issue| issue.severity == PreflightSeverity::Block);

        Ok(MemoryPreflightReport {
            workspace_root: workspace.root().to_path_buf(),
            operation,
            generated_unix_seconds: now_unix_seconds(),
            state_checksum: snapshot.state_checksum,
            can_proceed,
            issues,
        })
    }
}

fn relevant_atoms<'a>(atoms: &'a [LoreAtom], file_path: &Path, scope: Option<&ScopeContext>) -> Vec<LoreAtom> {
    let scope_name = scope.map(|value| value.name.as_str());
    atoms
        .iter()
        .rev()
        .filter(|atom| {
            let path_matches = atom.path.as_ref().map(|path| path == file_path).unwrap_or(false);
            let scope_matches = atom
                .scope
                .as_deref()
                .map(|value| scope_name.map(|scope_name| value == scope_name || value.contains(scope_name)).unwrap_or(false))
                .unwrap_or(false);

            path_matches || scope_matches
        })
        .take(5)
        .cloned()
        .collect()
}

fn render_constraint(atom: &LoreAtom) -> String {
    match atom.kind {
        LoreKind::Decision => format!("Decision [{}]: {}", atom.id, atom.title),
        LoreKind::Assumption => format!("Assumption [{}]: {}", atom.id, atom.title),
        LoreKind::OpenQuestion => format!("Open question [{}]: {}", atom.id, atom.title),
        LoreKind::Signal => format!("Signal [{}]: {}", atom.id, atom.title),
    }
}

fn kind_label(kind: &LoreKind) -> &'static str {
    match kind {
        LoreKind::Decision => "Decision",
        LoreKind::Assumption => "Assumption",
        LoreKind::OpenQuestion => "Open question",
        LoreKind::Signal => "Signal",
    }
}

fn score_memory_hit(
    atom: &LoreAtom,
    source: &str,
    query_lower: &str,
    query_tokens: &[String],
    target_file_path: Option<&PathBuf>,
    target_scope: Option<&ScopeContext>,
    newest_timestamp: u64,
) -> Option<MemorySearchHit> {
    let title = atom.title.to_ascii_lowercase();
    let body = atom
        .body
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let scope = atom
        .scope
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let path = atom
        .path
        .as_ref()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut score = 0.0f64;
    let mut reasons = Vec::new();

    let lexical_hits = query_tokens
        .iter()
        .filter(|token| {
            title.contains(token.as_str())
                || body.contains(token.as_str())
                || scope.contains(token.as_str())
                || path.contains(token.as_str())
        })
        .count();

    if lexical_hits == 0 {
        let joined = format!("{} {} {} {}", title, body, scope, path);
        if !joined.contains(query_lower) {
            return None;
        }
    }

    if lexical_hits > 0 {
        let lexical_score = lexical_hits as f64 * 8.0;
        score += lexical_score;
        reasons.push(format!("lexical:{lexical_hits}"));
    } else {
        score += 6.0;
        reasons.push("lexical:fallback-substring".to_string());
    }

    let state_bonus = match atom.state {
        AtomState::Accepted => 12.0,
        AtomState::Proposed => 8.0,
        AtomState::Draft => 4.0,
        AtomState::Deprecated => -3.0,
    };
    score += state_bonus;
    reasons.push(format!("state:{:?}", atom.state));

    if newest_timestamp > 0 && atom.created_unix_seconds > 0 {
        let normalized = (atom.created_unix_seconds as f64 / newest_timestamp as f64).min(1.0);
        let recency_bonus = normalized * 6.0;
        score += recency_bonus;
        reasons.push(format!("recency:{:.2}", recency_bonus));
    }

    if let Some(target_path) = target_file_path {
        if atom.path.as_ref() == Some(target_path) {
            score += 10.0;
            reasons.push("path:exact".to_string());
        } else if let (Some(atom_path), Some(parent)) = (atom.path.as_ref(), target_path.parent()) {
            if atom_path.starts_with(parent) {
                score += 4.0;
                reasons.push("path:near".to_string());
            }
        }
    }

    if let Some(scope_hint) = target_scope {
        if atom.scope.as_deref() == Some(scope_hint.name.as_str()) {
            score += 6.0;
            reasons.push("scope:exact".to_string());
        }
    }

    Some(MemorySearchHit {
        atom: atom.clone(),
        source: source.to_string(),
        score,
        reasons,
    })
}

fn tokenize(input: &str) -> Vec<String> {
    input
        .to_ascii_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.trim().is_empty())
        .map(|token| token.to_string())
        .collect()
}

fn fnv1a_hex(bytes: &[u8]) -> String {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x00000100000001b3;

    let mut hash = OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }

    format!("{hash:016x}")
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
    use crate::lore::WorkspaceState;
    use crate::parser::ScopeKind;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn context_snapshot_includes_relevant_atoms() {
        let root = std::env::temp_dir().join(format!("git-lore-mcp-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let workspace = Workspace::init(&root).unwrap();
        let source = root.join("src.rs");
        fs::write(
            &source,
            r#"
pub fn compute() {
    let value = 1;
}
"#,
        )
        .unwrap();

        let atom = LoreAtom::new(
            LoreKind::Decision,
            AtomState::Accepted,
            "Keep compute synchronous".to_string(),
            None,
            Some("compute".to_string()),
            Some(source.clone()),
        );
        workspace.record_atom(atom).unwrap();

        let service = McpService::new(&root);
        let snapshot = service.context(&source, Some(2)).unwrap();

        assert_eq!(snapshot.relevant_atoms.len(), 1);
        assert_eq!(snapshot.constraints[0], "Decision [".to_string() + &snapshot.relevant_atoms[0].id + "]: Keep compute synchronous");
        assert!(snapshot.scope.is_some());
    }

    #[test]
    fn propose_records_a_proposed_atom() {
        let root = std::env::temp_dir().join(format!("git-lore-mcp-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let workspace = Workspace::init(&root).unwrap();
        let source = root.join("src.rs");
        fs::write(&source, "pub fn compute() {}\n").unwrap();

        let service = McpService::new(&root);
        let result = service
            .propose(ProposalRequest {
                file_path: source.clone(),
                cursor_line: Some(1),
                kind: LoreKind::Decision,
                title: "Use tree-sitter scope context".to_string(),
                body: Some("Capture active function context before edits".to_string()),
                scope: None,
                validation_script: None,
            })
            .unwrap();

        assert_eq!(result.atom.state, AtomState::Proposed);
        assert_eq!(result.atom.path.as_ref(), Some(&source));

        let state = workspace.load_state().unwrap();
        assert_eq!(state.atoms.len(), 1);
    }

    #[test]
    fn state_snapshot_reports_atom_counts() {
        let root = std::env::temp_dir().join(format!("git-lore-mcp-snapshot-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        Workspace::init(&root).unwrap();

        let service = McpService::new(&root);
        let source = root.join("lib.rs");
        fs::write(&source, "pub fn run() {}\n").unwrap();

        service
            .propose(ProposalRequest {
                file_path: source,
                cursor_line: Some(1),
                kind: LoreKind::Decision,
                title: "Capture snapshot state".to_string(),
                body: None,
                scope: None,
                validation_script: None,
            })
            .unwrap();

        let snapshot = service.state_snapshot().unwrap();
        assert_eq!(snapshot.total_atoms, 1);
        assert_eq!(snapshot.proposed_atoms, 1);
        assert!(!snapshot.state_checksum.is_empty());
    }

    #[test]
    fn memory_preflight_blocks_duplicate_atom_ids() {
        let root = std::env::temp_dir().join(format!("git-lore-mcp-preflight-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let workspace = Workspace::init(&root).unwrap();

        let atom = LoreAtom::new(
            LoreKind::Decision,
            AtomState::Proposed,
            "One decision".to_string(),
            None,
            Some("scope".to_string()),
            Some(PathBuf::from("src/lib.rs")),
        );

        let duplicated = LoreAtom {
            title: "Duplicated id decision".to_string(),
            ..atom.clone()
        };

        workspace
            .set_state(&WorkspaceState {
                version: 1,
                atoms: vec![atom, duplicated],
            })
            .unwrap();

        let service = McpService::new(&root);
        let report = service.memory_preflight("edit").unwrap();
        assert!(!report.can_proceed);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "duplicate_atom_ids"));
    }

    #[test]
    fn memory_preflight_flags_sanitization_issues() {
        let root = std::env::temp_dir().join(format!("git-lore-mcp-preflight-sanitize-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let workspace = Workspace::init(&root).unwrap();

        let sensitive_atom = LoreAtom {
            id: Uuid::new_v4().to_string(),
            kind: LoreKind::Decision,
            state: AtomState::Proposed,
            title: "Rotate API token for service".to_string(),
            body: None,
            scope: Some("auth".to_string()),
            path: Some(PathBuf::from("src/auth.rs")),
            validation_script: None,
            created_unix_seconds: 1,
        };

        workspace
            .set_state(&WorkspaceState {
                version: 1,
                atoms: vec![sensitive_atom],
            })
            .unwrap();

        let service = McpService::new(&root);
        let report = service.memory_preflight("edit").unwrap();
        assert!(!report.can_proceed);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == "sanitize_sensitive_content"));
    }

    #[test]
    fn memory_search_returns_ranked_results() {
        let root = std::env::temp_dir().join(format!("git-lore-mcp-search-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let workspace = Workspace::init(&root).unwrap();

        let strong_match = LoreAtom {
            id: Uuid::new_v4().to_string(),
            kind: LoreKind::Decision,
            state: AtomState::Accepted,
            title: "Use sqlite cache for local mode".to_string(),
            body: Some("Cache hit latency target".to_string()),
            scope: Some("cache".to_string()),
            path: Some(PathBuf::from("src/cache.rs")),
            validation_script: None,
            created_unix_seconds: 100,
        };
        let weak_match = LoreAtom {
            id: Uuid::new_v4().to_string(),
            kind: LoreKind::Decision,
            state: AtomState::Draft,
            title: "Investigate distributed storage".to_string(),
            body: Some("Potential future work".to_string()),
            scope: Some("storage".to_string()),
            path: Some(PathBuf::from("src/storage.rs")),
            validation_script: None,
            created_unix_seconds: 10,
        };

        workspace
            .set_state(&crate::lore::WorkspaceState {
                version: 1,
                atoms: vec![weak_match, strong_match],
            })
            .unwrap();

        let service = McpService::new(&root);
        let report = service
            .memory_search("sqlite cache", Some(PathBuf::from("src/cache.rs")), None, 5)
            .unwrap();

        assert!(!report.results.is_empty());
        assert!(report.results[0].atom.title.contains("sqlite cache"));
    }

    #[test]
    fn state_transition_preview_is_exposed_from_service() {
        let root = std::env::temp_dir().join(format!("git-lore-mcp-transition-preview-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let workspace = Workspace::init(&root).unwrap();

        let atom = LoreAtom::new(
            LoreKind::Decision,
            AtomState::Proposed,
            "Keep parser deterministic".to_string(),
            None,
            Some("parser".to_string()),
            Some(PathBuf::from("src/parser/mod.rs")),
        );
        let atom_id = atom.id.clone();
        workspace.record_atom(atom).unwrap();

        let service = McpService::new(&root);
        let preview = service
            .state_transition_preview(&atom_id, AtomState::Accepted)
            .unwrap();

        assert!(preview.allowed);
        assert_eq!(preview.code, "state_transition_allowed");
    }

    #[test]
    fn autofill_proposal_fills_missing_fields() {
        let root = std::env::temp_dir().join(format!("git-lore-mcp-autofill-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        Workspace::init(&root).unwrap();
        let source = root.join("src.rs");
        fs::write(&source, "pub fn compute() {}\n").unwrap();

        let service = McpService::new(&root);
        let autofilled = service
            .autofill_proposal(
                &source,
                Some(1),
                LoreKind::Decision,
                None,
                None,
                None,
            )
            .unwrap();

        assert!(!autofilled.title.trim().is_empty());
        assert!(autofilled.body.is_some());
        assert!(autofilled.filled_fields.contains(&"title".to_string()));
        assert!(autofilled.filled_fields.contains(&"body".to_string()));
    }

        #[test]
        fn context_snapshot_uses_javascript_scope_detection() {
                let root = std::env::temp_dir().join(format!("git-lore-mcp-js-test-{}", Uuid::new_v4()));
                fs::create_dir_all(&root).unwrap();
                let workspace = Workspace::init(&root).unwrap();
                let source = root.join("index.js");
                fs::write(
                        &source,
                        r#"
function outer() {
    function inner() {
        return 1;
    }
}
"#,
                )
                .unwrap();

                let atom = LoreAtom::new(
                        LoreKind::Decision,
                        AtomState::Accepted,
                        "Keep inner synchronous".to_string(),
                        None,
                        Some("inner".to_string()),
                        Some(source.clone()),
                );
                workspace.record_atom(atom).unwrap();

                let service = McpService::new(&root);
                let snapshot = service.context(&source, Some(3)).unwrap();

                let scope = snapshot.scope.expect("expected javascript scope");
                assert_eq!(scope.language, "javascript");
                assert_eq!(scope.kind, ScopeKind::Function);
                assert_eq!(scope.name, "inner");
                assert_eq!(snapshot.relevant_atoms.len(), 1);
        }

        #[test]
        fn propose_records_a_typescript_atom_with_detected_scope() {
                let root = std::env::temp_dir().join(format!("git-lore-mcp-ts-test-{}", Uuid::new_v4()));
                fs::create_dir_all(&root).unwrap();
                let workspace = Workspace::init(&root).unwrap();
                let source = root.join("service.ts");
                fs::write(
                        &source,
                        r#"
class Service {
    run(): void {
        return;
    }
}
"#,
                )
                .unwrap();

                let service = McpService::new(&root);
                let result = service
                        .propose(ProposalRequest {
                                file_path: source.clone(),
                                cursor_line: Some(3),
                                kind: LoreKind::Decision,
                                title: "Keep the class method synchronous".to_string(),
                                body: Some("The runtime depends on this method staying blocking".to_string()),
                                scope: None,
                        validation_script: None,
                        })
                        .unwrap();

                let scope = result.scope.expect("expected typescript scope");
                assert_eq!(scope.language, "typescript");
                assert_eq!(scope.kind, ScopeKind::Method);
                assert_eq!(scope.name, "run");
                assert_eq!(result.atom.state, AtomState::Proposed);
                assert_eq!(result.atom.scope.as_deref(), Some("run"));

                let state = workspace.load_state().unwrap();
                assert_eq!(state.atoms.len(), 1);
        }
}
