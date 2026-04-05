use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod merge;
pub mod entropy;
pub mod prism;
pub mod refs;
pub mod sanitize;
pub mod validation;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoreKind {
    Decision,
    Assumption,
    OpenQuestion,
    Signal,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AtomState {
    Draft,
    Proposed,
    Accepted,
    Deprecated,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoreAtom {
    pub id: String,
    pub kind: LoreKind,
    pub state: AtomState,
    pub title: String,
    pub body: Option<String>,
    pub scope: Option<String>,
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_script: Option<String>,
    pub created_unix_seconds: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub version: u16,
    pub atoms: Vec<LoreAtom>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub message: Option<String>,
    pub created_unix_seconds: u64,
    pub atoms: Vec<LoreAtom>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateTransitionPreview {
    pub atom_id: String,
    pub current_state: Option<AtomState>,
    pub target_state: AtomState,
    pub allowed: bool,
    pub code: String,
    pub message: String,
    pub reason_required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateTransitionAuditEvent {
    pub atom_id: String,
    pub previous_state: AtomState,
    pub target_state: AtomState,
    pub reason: String,
    pub actor: Option<String>,
    pub transitioned_unix_seconds: u64,
}

#[derive(Clone, Debug)]
pub struct Workspace {
    root: PathBuf,
}

impl LoreAtom {
    pub fn new(
        kind: LoreKind,
        state: AtomState,
        title: String,
        body: Option<String>,
        scope: Option<String>,
        path: Option<PathBuf>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind,
            state,
            title,
            body,
            scope,
            path,
            validation_script: None,
            created_unix_seconds: now_unix_seconds(),
        }
    }

    pub fn with_validation_script(mut self, validation_script: Option<String>) -> Self {
        self.validation_script = validation_script;
        self
    }
}

impl WorkspaceState {
    pub fn empty() -> Self {
        Self {
            version: 1,
            atoms: Vec::new(),
        }
    }
}

impl Workspace {
    pub fn init(path: impl AsRef<Path>) -> Result<Self> {
        let root = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().to_path_buf());
        let workspace = Self { root };
        workspace.ensure_layout()?;
        Ok(workspace)
    }

    pub fn discover(path: impl AsRef<Path>) -> Result<Self> {
        let mut current = path.as_ref();

        loop {
            let candidate = current.join(".lore");
            if candidate.exists() {
                return Ok(Self {
                    root: current.to_path_buf(),
                });
            }

            match current.parent() {
                Some(parent) => current = parent,
                None => bail!(
                    "could not find a .lore workspace starting from {}",
                    path.as_ref().display()
                ),
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn load_state(&self) -> Result<WorkspaceState> {
        let state_path = self.state_path();
        if !state_path.exists() {
            return Ok(WorkspaceState::empty());
        }

        self.read_json(&state_path)
    }

    pub fn record_atom(&self, atom: LoreAtom) -> Result<()> {
        self.ensure_layout()?;

        if atom.kind != LoreKind::Signal {
            let has_path = atom
                .path
                .as_ref()
                .map(|path| !path.as_os_str().is_empty())
                .unwrap_or(false);
            let has_scope = atom
                .scope
                .as_deref()
                .map(str::trim)
                .map(|scope| !scope.is_empty())
                .unwrap_or(false);

            if !has_path && !has_scope {
                bail!(
                    "non-signal atoms require at least one anchor; provide --path or --scope"
                );
            }
        }

        if let Some(script) = atom.validation_script.as_deref() {
            validation::validate_script(script)?;
        }

        if let Some(issue) = sanitize::scan_atoms(std::slice::from_ref(&atom)).into_iter().next() {
            return Err(anyhow::anyhow!(
                "sensitive content detected in atom {} field {}: {}",
                issue.atom_id,
                issue.field,
                issue.reason
            ));
        }

        let mut state = self.load_state()?;
        let atom_path = self.active_atom_path(&atom.id);

        state.atoms.push(atom.clone());
        self.write_json(&self.state_path(), &state)?;
        self.write_json(&atom_path, &atom)?;
        Ok(())
    }

    pub fn write_checkpoint(&self, message: Option<String>) -> Result<Checkpoint> {
        self.ensure_layout()?;

        let state = self.load_state()?;
        let checkpoint = Checkpoint {
            id: Uuid::new_v4().to_string(),
            message,
            created_unix_seconds: now_unix_seconds(),
            atoms: state.atoms,
        };

        let checkpoint_path = self
            .checkpoints_dir()
            .join(format!("{}.json", checkpoint.id));
        self.write_json(&checkpoint_path, &checkpoint)?;
        Ok(checkpoint)
    }

    pub fn entropy_report(&self) -> Result<entropy::EntropyReport> {
        let state = self.load_state()?;
        Ok(entropy::analyze_workspace(&state))
    }

    pub fn sanitize_report(&self) -> Result<Vec<sanitize::SanitizationIssue>> {
        let state = self.load_state()?;
        Ok(sanitize::scan_atoms(&state.atoms))
    }

    pub fn validation_report(&self) -> Result<Vec<validation::ValidationIssue>> {
        let state = self.load_state()?;
        Ok(validation::scan_atoms(self.root(), &state.atoms))
    }

    pub fn set_state(&self, state: &WorkspaceState) -> Result<()> {
        self.ensure_layout()?;
        self.write_json(&self.state_path(), state)
    }

    pub fn preview_state_transition(
        &self,
        atom_id: &str,
        target_state: AtomState,
    ) -> Result<StateTransitionPreview> {
        self.ensure_layout()?;
        let state = self.load_state()?;
        let current_state = state
            .atoms
            .iter()
            .find(|atom| atom.id == atom_id)
            .map(|atom| atom.state.clone());

        let evaluation = match current_state.clone() {
            Some(current) => evaluate_state_transition(current, target_state.clone()),
            None => TransitionEvaluation {
                allowed: false,
                code: "atom_not_found",
                message: "atom id was not found in active lore state",
            },
        };

        Ok(StateTransitionPreview {
            atom_id: atom_id.to_string(),
            current_state,
            target_state,
            allowed: evaluation.allowed,
            code: evaluation.code.to_string(),
            message: evaluation.message.to_string(),
            reason_required: true,
        })
    }

    pub fn transition_atom_state(
        &self,
        atom_id: &str,
        target_state: AtomState,
        reason: impl Into<String>,
        actor: Option<String>,
    ) -> Result<LoreAtom> {
        self.ensure_layout()?;
        let reason = reason.into();
        if reason.trim().is_empty() {
            bail!("state transition requires a non-empty reason");
        }

        let mut state = self.load_state()?;
        let atom = state
            .atoms
            .iter_mut()
            .find(|atom| atom.id == atom_id)
            .ok_or_else(|| anyhow::anyhow!("atom {} not found in active lore state", atom_id))?;

        let previous_state = atom.state.clone();
        let evaluation = evaluate_state_transition(previous_state.clone(), target_state.clone());
        if !evaluation.allowed {
            if evaluation.code == "state_noop" {
                return Ok(atom.clone());
            }
            bail!(
                "state transition rejected [{}]: {}",
                evaluation.code,
                evaluation.message
            );
        }

        atom.state = target_state.clone();
        let updated_atom = atom.clone();

        self.write_json(&self.state_path(), &state)?;
        self.write_json(&self.active_atom_path(&updated_atom.id), &updated_atom)?;

        if updated_atom.state == AtomState::Accepted {
            self.write_accepted_atom(&updated_atom, None)?;
        }

        self.append_state_transition_audit(&StateTransitionAuditEvent {
            atom_id: updated_atom.id.clone(),
            previous_state,
            target_state,
            reason,
            actor,
            transitioned_unix_seconds: now_unix_seconds(),
        })?;

        Ok(updated_atom)
    }

    pub fn accept_active_atoms(&self, source_commit: Option<&str>) -> Result<()> {
        self.ensure_layout()?;

        let mut state = self.load_state()?;
        for atom in &mut state.atoms {
            if atom.state != AtomState::Deprecated {
                atom.state = AtomState::Accepted;
                self.write_accepted_atom(atom, source_commit)?;
            }
        }

        self.write_json(&self.state_path(), &state)?;
        Ok(())
    }

    fn ensure_layout(&self) -> Result<()> {
        fs::create_dir_all(self.lore_dir())?;
        fs::create_dir_all(self.active_dir())?;
        fs::create_dir_all(self.checkpoints_dir())?;
        fs::create_dir_all(self.prism_dir())?;
        fs::create_dir_all(self.refs_lore_accepted_dir())?;
        fs::create_dir_all(self.audit_dir())?;
        Ok(())
    }

    fn append_state_transition_audit(&self, event: &StateTransitionAuditEvent) -> Result<()> {
        let path = self.state_transition_audit_path();
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("failed to open state transition audit log {}", path.display()))?;

        let line = serde_json::to_string(event)?;
        file.write_all(line.as_bytes())
            .with_context(|| format!("failed to write state transition audit log {}", path.display()))?;
        file.write_all(b"\n")
            .with_context(|| format!("failed to finalize state transition audit log {}", path.display()))?;

        Ok(())
    }

    fn write_json<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        let content = serde_json::to_vec_pretty(value)?;
        let compressed = gzip_compress(&content)
            .with_context(|| format!("failed to compress {}", path.display()))?;
        fs::write(path, compressed).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    fn read_json<T: DeserializeOwned>(&self, path: &Path) -> Result<T> {
        let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
        let content = if bytes.starts_with(&[0x1f, 0x8b]) {
            gzip_decompress_file(path).with_context(|| format!("failed to decompress {}", path.display()))?
        } else {
            bytes
        };
        let value = serde_json::from_slice(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(value)
    }

    fn lore_dir(&self) -> PathBuf {
        self.root.join(".lore")
    }

    fn state_path(&self) -> PathBuf {
        self.lore_dir().join("active_intent.json")
    }

    fn active_dir(&self) -> PathBuf {
        self.lore_dir().join("active")
    }

    fn checkpoints_dir(&self) -> PathBuf {
        self.lore_dir().join("checkpoints")
    }

    fn prism_dir(&self) -> PathBuf {
        self.lore_dir().join("prism")
    }

    fn refs_lore_dir(&self) -> PathBuf {
        self.lore_dir().join("refs").join("lore")
    }

    fn refs_lore_accepted_dir(&self) -> PathBuf {
        self.refs_lore_dir().join("accepted")
    }

    fn audit_dir(&self) -> PathBuf {
        self.lore_dir().join("audit")
    }

    fn state_transition_audit_path(&self) -> PathBuf {
        self.audit_dir().join("state_transitions.jsonl")
    }

    fn active_atom_path(&self, atom_id: &str) -> PathBuf {
        self.active_dir().join(format!("{atom_id}.json"))
    }
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[derive(Clone, Debug)]
struct TransitionEvaluation {
    allowed: bool,
    code: &'static str,
    message: &'static str,
}

fn evaluate_state_transition(current: AtomState, target: AtomState) -> TransitionEvaluation {
    if current == target {
        return TransitionEvaluation {
            allowed: false,
            code: "state_noop",
            message: "atom is already in the target state",
        };
    }

    let allowed = matches!(
        (current.clone(), target.clone()),
        (AtomState::Draft, AtomState::Proposed)
            | (AtomState::Draft, AtomState::Deprecated)
            | (AtomState::Proposed, AtomState::Accepted)
            | (AtomState::Proposed, AtomState::Deprecated)
            | (AtomState::Accepted, AtomState::Deprecated)
    );

    if allowed {
        TransitionEvaluation {
            allowed: true,
            code: "state_transition_allowed",
            message: "state transition is allowed",
        }
    } else {
        TransitionEvaluation {
            allowed: false,
            code: "state_transition_blocked",
            message: "requested state transition is not allowed by policy",
        }
    }
}

fn gzip_compress(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut child = Command::new("gzip")
        .arg("-c")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn gzip for compression")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(bytes).context("failed to feed gzip input")?;
    }

    let output = child.wait_with_output().context("failed to finish gzip compression")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow::anyhow!("gzip compression failed: {stderr}"));
    }

    Ok(output.stdout)
}

fn gzip_decompress_file(path: &Path) -> Result<Vec<u8>> {
    let child = Command::new("gzip")
        .arg("-dc")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg(path)
        .spawn()
        .context("failed to spawn gzip for decompression")?;

    let output = child.wait_with_output().context("failed to finish gzip decompression")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("gzip decompression failed"));
    }

    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn checkpoint_contains_recorded_atoms() {
        let temp_root = std::env::temp_dir().join(format!("git-lore-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let workspace = Workspace::init(&temp_root).unwrap();

        let atom = LoreAtom::new(
            LoreKind::Decision,
            AtomState::Proposed,
            "Use Postgres".to_string(),
            Some("Spatial queries need PostGIS".to_string()),
            Some("db".to_string()),
            Some(PathBuf::from("src/db.rs")),
        );

        workspace.record_atom(atom.clone()).unwrap();
        let checkpoint = workspace
            .write_checkpoint(Some("initial checkpoint".to_string()))
            .unwrap();

        assert_eq!(checkpoint.atoms.len(), 1);
        assert_eq!(checkpoint.atoms[0].id, atom.id);
        assert_eq!(checkpoint.message.as_deref(), Some("initial checkpoint"));
    }

    #[test]
    fn accept_active_atoms_promotes_recorded_atoms() {
        let temp_root = std::env::temp_dir().join(format!("git-lore-accept-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let workspace = Workspace::init(&temp_root).unwrap();

        let atom = LoreAtom::new(
            LoreKind::Decision,
            AtomState::Proposed,
            "Use SQLite".to_string(),
            None,
            Some("db".to_string()),
            None,
        );

        workspace.record_atom(atom).unwrap();
        workspace.accept_active_atoms(None).unwrap();

        let state = workspace.load_state().unwrap();
        assert_eq!(state.atoms[0].state, AtomState::Accepted);
    }

    #[test]
    fn transition_atom_state_updates_state_and_writes_audit() {
        let temp_root = std::env::temp_dir().join(format!("git-lore-transition-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let workspace = Workspace::init(&temp_root).unwrap();

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

        let transitioned = workspace
            .transition_atom_state(
                &atom_id,
                AtomState::Accepted,
                "validated in integration test",
                Some("unit-test".to_string()),
            )
            .unwrap();

        assert_eq!(transitioned.state, AtomState::Accepted);
        let state = workspace.load_state().unwrap();
        assert_eq!(state.atoms[0].state, AtomState::Accepted);

        let audit_path = temp_root.join(".lore/audit/state_transitions.jsonl");
        let audit = fs::read_to_string(audit_path).unwrap();
        assert!(audit.contains(&atom_id));
        assert!(audit.contains("validated in integration test"));
    }

    #[test]
    fn transition_preview_reports_blocked_transition() {
        let temp_root = std::env::temp_dir().join(format!("git-lore-transition-preview-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let workspace = Workspace::init(&temp_root).unwrap();

        let atom = LoreAtom::new(
            LoreKind::Decision,
            AtomState::Accepted,
            "Keep sync idempotent".to_string(),
            None,
            Some("sync".to_string()),
            Some(PathBuf::from("src/git/mod.rs")),
        );
        let atom_id = atom.id.clone();
        workspace.record_atom(atom).unwrap();

        let preview = workspace
            .preview_state_transition(&atom_id, AtomState::Proposed)
            .unwrap();

        assert!(!preview.allowed);
        assert_eq!(preview.code, "state_transition_blocked");
    }

    #[test]
    fn record_atom_rejects_non_signal_without_path_or_scope() {
        let temp_root = std::env::temp_dir().join(format!("git-lore-anchor-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).unwrap();
        let workspace = Workspace::init(&temp_root).unwrap();

        let atom = LoreAtom::new(
            LoreKind::Decision,
            AtomState::Proposed,
            "Anchor required".to_string(),
            None,
            None,
            None,
        );

        let error = workspace.record_atom(atom).unwrap_err();
        assert!(error
            .to_string()
            .contains("provide --path or --scope"));
    }
}
