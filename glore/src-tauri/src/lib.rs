use git_lore::git;
use git_lore::lore::{AtomState, LoreAtom, LoreKind, Workspace};
use git_lore::mcp::McpService;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize)]
struct WorkspaceSnapshot {
    root: String,
    atoms: Vec<LoreAtom>,
}

#[derive(Clone, Debug, Serialize)]
struct ContradictionSummary {
    key: String,
    kind: String,
    message: String,
    atom_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct StatusReport {
    root: String,
    total_atoms: usize,
    entropy_score: u8,
    draft_atoms: usize,
    proposed_atoms: usize,
    accepted_atoms: usize,
    deprecated_atoms: usize,
    contradictions: Vec<ContradictionSummary>,
    notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ValidationReport {
    root: String,
    ok: bool,
    issues: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LoreKindArg {
    Decision,
    Assumption,
    OpenQuestion,
    Signal,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AtomStateArg {
    Draft,
    Proposed,
    Accepted,
    Deprecated,
}

#[derive(Clone, Debug, Deserialize)]
struct MarkAtomInput {
    title: String,
    body: Option<String>,
    scope: Option<String>,
    file_path: Option<String>,
    validation_script: Option<String>,
    kind: LoreKindArg,
}

#[derive(Clone, Debug, Deserialize)]
struct SetStateInput {
    atom_id: String,
    state: AtomStateArg,
    reason: String,
    actor: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct AtomContextInput {
    atom_id: String,
}

#[derive(Clone, Debug, Serialize)]
struct GitDecisionSummary {
    commit_hash: String,
    subject: String,
    trailer_value: String,
}

#[derive(Clone, Debug, Serialize)]
struct AtomContextReport {
    atom_id: String,
    file_path: Option<String>,
    scope: Option<String>,
    constraints: Vec<String>,
    historical_decisions: Vec<GitDecisionSummary>,
}

impl From<LoreKindArg> for LoreKind {
    fn from(value: LoreKindArg) -> Self {
        match value {
            LoreKindArg::Decision => LoreKind::Decision,
            LoreKindArg::Assumption => LoreKind::Assumption,
            LoreKindArg::OpenQuestion => LoreKind::OpenQuestion,
            LoreKindArg::Signal => LoreKind::Signal,
        }
    }
}

impl From<AtomStateArg> for AtomState {
    fn from(value: AtomStateArg) -> Self {
        match value {
            AtomStateArg::Draft => AtomState::Draft,
            AtomStateArg::Proposed => AtomState::Proposed,
            AtomStateArg::Accepted => AtomState::Accepted,
            AtomStateArg::Deprecated => AtomState::Deprecated,
        }
    }
}

fn workspace_snapshot(workspace: &Workspace) -> Result<WorkspaceSnapshot, String> {
    let state = workspace.load_state().map_err(|error| error.to_string())?;

    Ok(WorkspaceSnapshot {
        root: workspace.root().to_string_lossy().to_string(),
        atoms: state.atoms,
    })
}

fn discover_workspace(path: Option<String>) -> Result<Workspace, String> {
    let target = path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    Workspace::discover(&target).map_err(|error| error.to_string())
}

#[tauri::command]
fn load_workspace(path: Option<String>) -> Result<WorkspaceSnapshot, String> {
    let workspace = discover_workspace(path)?;
    workspace_snapshot(&workspace)
}

#[tauri::command]
fn init_workspace(path: String) -> Result<WorkspaceSnapshot, String> {
    let workspace = Workspace::init(path).map_err(|error| error.to_string())?;
    workspace_snapshot(&workspace)
}

#[tauri::command]
fn workspace_status(path: Option<String>) -> Result<StatusReport, String> {
    let workspace = discover_workspace(path)?;
    let state = workspace.load_state().map_err(|error| error.to_string())?;
    let entropy = workspace.entropy_report().map_err(|error| error.to_string())?;

    Ok(StatusReport {
        root: workspace.root().to_string_lossy().to_string(),
        total_atoms: state.atoms.len(),
        entropy_score: entropy.score,
        draft_atoms: entropy.draft_atoms,
        proposed_atoms: entropy.proposed_atoms,
        accepted_atoms: entropy.accepted_atoms,
        deprecated_atoms: entropy.deprecated_atoms,
        contradictions: entropy
            .contradictions
            .into_iter()
            .map(|item| ContradictionSummary {
                key: item.key,
                kind: format!("{:?}", item.kind),
                message: item.message,
                atom_ids: item.atoms.into_iter().map(|atom| atom.id).collect(),
            })
            .collect(),
        notes: entropy.notes,
    })
}

#[tauri::command]
fn validate_workspace(path: Option<String>) -> Result<ValidationReport, String> {
    let workspace = discover_workspace(path)?;
    let repository_root =
        git::discover_repository(workspace.root()).map_err(|error| error.to_string())?;
    let issues = git::validate_workspace_against_git(&repository_root, &workspace)
        .map_err(|error| error.to_string())?;

    Ok(ValidationReport {
        root: workspace.root().to_string_lossy().to_string(),
        ok: issues.is_empty(),
        issues,
    })
}

#[tauri::command]
fn mark_atom(path: Option<String>, input: MarkAtomInput) -> Result<WorkspaceSnapshot, String> {
    let workspace = discover_workspace(path)?;

    let atom = LoreAtom::new(
        input.kind.into(),
        AtomState::Proposed,
        input.title,
        input.body,
        input.scope,
        input.file_path.map(PathBuf::from),
    )
    .with_validation_script(input.validation_script);

    workspace.record_atom(atom).map_err(|error| error.to_string())?;
    workspace_snapshot(&workspace)
}

#[tauri::command]
fn set_atom_state(path: Option<String>, input: SetStateInput) -> Result<WorkspaceSnapshot, String> {
    let workspace = discover_workspace(path)?;

    workspace
        .transition_atom_state(&input.atom_id, input.state.into(), input.reason, input.actor)
        .map_err(|error| error.to_string())?;

    workspace_snapshot(&workspace)
}

#[tauri::command]
fn atom_context(path: Option<String>, input: AtomContextInput) -> Result<AtomContextReport, String> {
    let workspace = discover_workspace(path)?;
    let state = workspace.load_state().map_err(|error| error.to_string())?;
    let atom = state
        .atoms
        .iter()
        .find(|atom| atom.id == input.atom_id)
        .cloned()
        .ok_or_else(|| format!("atom {} not found", input.atom_id))?;

    let Some(atom_path) = atom.path.clone() else {
        return Ok(AtomContextReport {
            atom_id: atom.id,
            file_path: None,
            scope: atom.scope,
            constraints: Vec::new(),
            historical_decisions: Vec::new(),
        });
    };

    let resolved_path = if atom_path.is_absolute() {
        atom_path
    } else {
        workspace.root().join(atom_path)
    };

    let service = McpService::new(workspace.root());
    let snapshot = service
        .context(&resolved_path, None)
        .map_err(|error| error.to_string())?;

    let file_path = resolved_path
        .strip_prefix(workspace.root())
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|_| resolved_path.to_string_lossy().to_string());

    Ok(AtomContextReport {
        atom_id: atom.id,
        file_path: Some(file_path),
        scope: atom.scope,
        constraints: snapshot.constraints.into_iter().take(8).collect(),
        historical_decisions: snapshot
            .historical_decisions
            .into_iter()
            .take(6)
            .map(|decision| GitDecisionSummary {
                commit_hash: decision.commit_hash,
                subject: decision.subject,
                trailer_value: decision.trailer_value,
            })
            .collect(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_workspace,
            init_workspace,
            workspace_status,
            validate_workspace,
            mark_atom,
            set_atom_state,
            atom_context
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
