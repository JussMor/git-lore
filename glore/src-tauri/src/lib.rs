use git_lore::git;
use git_lore::lore::{AtomState, LoreAtom, LoreKind, StateTransitionAuditEvent, Workspace};
use git_lore::lore::prism::PRISM_STALE_TTL_SECONDS;
use git_lore::mcp::{McpService, ProposalRequest};
use flate2::read::GzDecoder;
use notify::{recommended_watcher, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Debug, Serialize)]
struct WorkspaceSnapshot {
    root: String,
    atoms: Vec<LoreAtom>,
}

#[derive(Clone, Debug, Serialize)]
struct CheckpointSummary {
    id: String,
    message: Option<String>,
    created_unix_seconds: u64,
    atom_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct AuditTransitionSummary {
    atom_id: String,
    previous_state: String,
    target_state: String,
    reason: String,
    actor: Option<String>,
    transitioned_unix_seconds: u64,
}

#[derive(Clone, Debug, Serialize)]
struct WorkspaceTimelineReport {
    root: String,
    checkpoints: Vec<CheckpointSummary>,
    audit_events: Vec<AuditTransitionSummary>,
}

#[derive(Clone, Debug, Deserialize)]
struct StoredCheckpoint {
    id: String,
    message: Option<String>,
    created_unix_seconds: u64,
    atoms: Vec<Value>,
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

#[derive(Clone, Debug, Deserialize)]
struct CommitDiffInput {
    commit_hash: String,
    file_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct OpenFileInEditorInput {
    file_path: String,
    line: Option<u32>,
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

#[derive(Clone, Debug, Serialize)]
struct CommitDiffReport {
    commit_hash: String,
    subject: String,
    diff: String,
    truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
struct WorkspaceChangedEvent {
    root: String,
    changed_paths: Vec<String>,
    kind: String,
}

struct WorkspaceWatcherRegistration {
    root: PathBuf,
    _watcher: RecommendedWatcher,
}

#[derive(Default)]
struct WorkspaceWatcherState {
    watcher: Mutex<Option<WorkspaceWatcherRegistration>>,
}

#[derive(Clone, Debug, Deserialize)]
struct ToolContextInput {
    file_path: String,
    cursor_line: Option<usize>,
}

#[derive(Clone, Debug, Deserialize)]
struct ToolMemorySearchInput {
    query: String,
    file_path: Option<String>,
    cursor_line: Option<usize>,
    limit: Option<usize>,
}

#[derive(Clone, Debug, Deserialize)]
struct ToolMemoryPreflightInput {
    operation: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ToolTransitionPreviewInput {
    atom_id: String,
    target_state: AtomStateArg,
}

#[derive(Clone, Debug, Deserialize)]
struct ToolProposeInput {
    file_path: String,
    cursor_line: Option<usize>,
    kind: LoreKindArg,
    title: Option<String>,
    body: Option<String>,
    scope: Option<String>,
    validation_script: Option<String>,
    autofill: Option<bool>,
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
    let mut atoms = state.atoms;
    let now = now_unix_seconds();

    for signal in workspace
        .load_prism_signals()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|signal| now.saturating_sub(signal.created_unix_seconds) <= PRISM_STALE_TTL_SECONDS)
    {
        let signal_id = format!("prism-signal::{}", signal.session_id);
        if atoms.iter().any(|atom| atom.id == signal_id) {
            continue;
        }

        let title = signal
            .decision
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("PRISM: {value}"))
            .unwrap_or_else(|| format!("PRISM signal {}", signal.session_id));

        let assumptions = if signal.assumptions.is_empty() {
            "none".to_string()
        } else {
            signal.assumptions.join(" | ")
        };

        atoms.push(LoreAtom {
            id: signal_id,
            kind: LoreKind::Signal,
            state: AtomState::Deprecated,
            title,
            body: Some(format!(
                "Ephemeral PRISM signal\nagent: {}\npaths: {}\nassumptions: {}",
                signal.agent.as_deref().unwrap_or("unknown-agent"),
                signal.paths.join(", "),
                assumptions,
            )),
            scope: signal.scope,
            path: None,
            validation_script: None,
            created_unix_seconds: signal.created_unix_seconds,
        });
    }

    Ok(WorkspaceSnapshot {
        root: workspace.root().to_string_lossy().to_string(),
        atoms,
    })
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn read_json_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        return Ok(bytes);
    }

    let mut decoder = GzDecoder::new(bytes.as_slice());
    let mut decoded = Vec::new();
    decoder
        .read_to_end(&mut decoded)
        .map_err(|error| error.to_string())?;
    Ok(decoded)
}

fn load_checkpoint_summaries(workspace: &Workspace, limit: usize) -> Result<Vec<CheckpointSummary>, String> {
    let checkpoints_dir = workspace.root().join(".lore").join("checkpoints");
    if !checkpoints_dir.exists() {
        return Ok(Vec::new());
    }

    let mut checkpoints = Vec::new();
    for entry in fs::read_dir(&checkpoints_dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }

        let bytes = read_json_bytes(&path)?;
        let checkpoint: StoredCheckpoint =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        checkpoints.push(CheckpointSummary {
            id: checkpoint.id,
            message: checkpoint.message,
            created_unix_seconds: checkpoint.created_unix_seconds,
            atom_count: checkpoint.atoms.len(),
        });
    }

    checkpoints.sort_by(|left, right| {
        right
            .created_unix_seconds
            .cmp(&left.created_unix_seconds)
            .then(left.id.cmp(&right.id))
    });
    checkpoints.truncate(limit);
    Ok(checkpoints)
}

fn load_audit_events(workspace: &Workspace, limit: usize) -> Result<Vec<AuditTransitionSummary>, String> {
    let audit_path = workspace
        .root()
        .join(".lore")
        .join("audit")
        .join("state_transitions.jsonl");

    if !audit_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&audit_path).map_err(|error| error.to_string())?;
    let mut events = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let event: StateTransitionAuditEvent =
            serde_json::from_str(trimmed).map_err(|error| error.to_string())?;
        events.push(AuditTransitionSummary {
            atom_id: event.atom_id,
            previous_state: format!("{:?}", event.previous_state).to_ascii_lowercase(),
            target_state: format!("{:?}", event.target_state).to_ascii_lowercase(),
            reason: event.reason,
            actor: event.actor,
            transitioned_unix_seconds: event.transitioned_unix_seconds,
        });
    }

    events.sort_by(|left, right| {
        right
            .transitioned_unix_seconds
            .cmp(&left.transitioned_unix_seconds)
            .then(left.atom_id.cmp(&right.atom_id))
    });
    events.truncate(limit);
    Ok(events)
}

fn discover_workspace(path: Option<String>) -> Result<Workspace, String> {
    let target = path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    Workspace::discover(&target).map_err(|error| error.to_string())
}

fn resolve_tool_file_path(workspace: &Workspace, file_path: &str) -> PathBuf {
    let candidate = PathBuf::from(file_path);
    if candidate.is_absolute() {
        return candidate;
    }
    workspace.root().join(candidate)
}

fn truncate_for_preview(value: &str, max_chars: usize) -> (String, bool) {
    let mut indices = value.char_indices();
    let Some((cutoff, _)) = indices.nth(max_chars) else {
        return (value.to_string(), false);
    };

    (
        format!(
            "{}\n\n[diff truncated after {} characters]",
            &value[..cutoff],
            max_chars
        ),
        true,
    )
}

impl WorkspaceWatcherState {
    fn watch(&self, app: &AppHandle, workspace: &Workspace) -> Result<String, String> {
        let root = workspace.root().to_path_buf();
        let state_path = root.join(".lore").join("active_intent.json");

        {
            let guard = self
                .watcher
                .lock()
                .map_err(|_| "workspace watcher state is poisoned".to_string())?;

            if guard
                .as_ref()
                .map(|registration| registration.root == root)
                .unwrap_or(false)
            {
                return Ok(state_path.to_string_lossy().to_string());
            }
        }

        let event_root = root.to_string_lossy().to_string();
        let app_handle = app.clone();

        let mut watcher = recommended_watcher(move |result: Result<Event, notify::Error>| {
            let Ok(event) = result else {
                return;
            };

            let changed_paths: Vec<String> = event
                .paths
                .into_iter()
                .filter(|path| {
                    let is_active_intent =
                        path.file_name().and_then(|name| name.to_str()) == Some("active_intent.json");
                    let is_prism_signal =
                        path.extension().and_then(|extension| extension.to_str()) == Some("signal")
                            && path
                                .parent()
                                .and_then(|parent| parent.file_name())
                                .and_then(|name| name.to_str())
                                == Some("prism");
                    let is_checkpoint =
                        path.extension().and_then(|extension| extension.to_str()) == Some("json")
                            && path
                                .parent()
                                .and_then(|parent| parent.file_name())
                                .and_then(|name| name.to_str())
                                == Some("checkpoints");
                    let is_audit_log =
                        path.file_name().and_then(|name| name.to_str())
                            == Some("state_transitions.jsonl")
                            && path
                                .parent()
                                .and_then(|parent| parent.file_name())
                                .and_then(|name| name.to_str())
                                == Some("audit");

                    is_active_intent || is_prism_signal || is_checkpoint || is_audit_log
                })
                .map(|path| path.to_string_lossy().to_string())
                .collect();

            if changed_paths.is_empty() {
                return;
            }

            let payload = WorkspaceChangedEvent {
                root: event_root.clone(),
                changed_paths,
                kind: format!("{:?}", event.kind),
            };

            let _ = app_handle.emit("lore://workspace-changed", payload);
        })
        .map_err(|error| error.to_string())?;

        watcher
            .watch(&root.join(".lore"), RecursiveMode::Recursive)
            .map_err(|error| error.to_string())?;

        let mut guard = self
            .watcher
            .lock()
            .map_err(|_| "workspace watcher state is poisoned".to_string())?;
        *guard = Some(WorkspaceWatcherRegistration {
            root,
            _watcher: watcher,
        });

        Ok(state_path.to_string_lossy().to_string())
    }
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
fn watch_workspace(
    app: AppHandle,
    watcher_state: State<'_, WorkspaceWatcherState>,
    path: Option<String>,
) -> Result<String, String> {
    let workspace = discover_workspace(path)?;
    watcher_state.watch(&app, &workspace)
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
fn workspace_timeline(path: Option<String>, limit: Option<usize>) -> Result<WorkspaceTimelineReport, String> {
    let workspace = discover_workspace(path)?;
    let limit = limit.unwrap_or(40).clamp(1, 200);

    let checkpoints = load_checkpoint_summaries(&workspace, limit)?;
    let audit_events = load_audit_events(&workspace, limit)?;

    Ok(WorkspaceTimelineReport {
        root: workspace.root().to_string_lossy().to_string(),
        checkpoints,
        audit_events,
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

#[tauri::command]
fn commit_diff(path: Option<String>, input: CommitDiffInput) -> Result<CommitDiffReport, String> {
    let workspace = discover_workspace(path)?;
    let repository_root =
        git::discover_repository(workspace.root()).map_err(|error| error.to_string())?;

    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(&repository_root)
        .arg("show")
        .arg("--no-color")
        .arg("--pretty=format:%s")
        .arg("--patch")
        .arg(input.commit_hash.trim());

    if let Some(file_path) = input
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        command.arg("--").arg(file_path);
    }

    let output = command.output().map_err(|error| {
        format!(
            "failed to read git diff for commit {}: {}",
            input.commit_hash.trim(),
            error
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "git show failed for commit {}: {}",
            input.commit_hash.trim(),
            stderr
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("git returned invalid utf-8 diff output: {}", error))?;

    let mut lines = stdout.lines();
    let subject = lines.next().unwrap_or_default().trim().to_string();
    let diff_body = lines.collect::<Vec<_>>().join("\n");
    let (diff, truncated) = truncate_for_preview(diff_body.trim(), 40_000);

    Ok(CommitDiffReport {
        commit_hash: input.commit_hash,
        subject,
        diff,
        truncated,
    })
}

#[tauri::command]
fn open_file_in_editor(
    path: Option<String>,
    input: OpenFileInEditorInput,
) -> Result<String, String> {
    let workspace = discover_workspace(path)?;
    let target = resolve_tool_file_path(&workspace, input.file_path.trim());

    if !target.exists() {
        return Err(format!(
            "file does not exist: {}",
            target.to_string_lossy()
        ));
    }

    let parent_folder = target
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.root().to_path_buf());

    let code_target = if let Some(line) = input.line.filter(|value| *value > 0) {
        format!("{}:{}", target.to_string_lossy(), line)
    } else {
        target.to_string_lossy().to_string()
    };

    let code_status = Command::new("code")
        .arg("--reuse-window")
        .arg(parent_folder.as_os_str())
        .arg("--goto")
        .arg(&code_target)
        .status();

    if let Ok(status) = code_status {
        if status.success() {
            return Ok(target.to_string_lossy().to_string());
        }
    }

    #[cfg(target_os = "macos")]
    {
        let open_status = Command::new("open")
            .arg("-a")
            .arg("Visual Studio Code")
            .arg("--args")
            .arg("--reuse-window")
            .arg(parent_folder.as_os_str())
            .arg("--goto")
            .arg(&code_target)
            .status()
            .map_err(|error| format!("failed to open file in VS Code: {}", error))?;

        if open_status.success() {
            return Ok(target.to_string_lossy().to_string());
        }
    }

    Err("could not open file in VS Code (install the 'code' shell command in VS Code)".to_string())
}

#[tauri::command]
fn tool_context(path: Option<String>, input: ToolContextInput) -> Result<Value, String> {
    let workspace = discover_workspace(path)?;
    let service = McpService::new(workspace.root());
    let target = resolve_tool_file_path(&workspace, &input.file_path);

    let snapshot = service
        .context(&target, input.cursor_line)
        .map_err(|error| error.to_string())?;

    serde_json::to_value(snapshot).map_err(|error| error.to_string())
}

#[tauri::command]
fn tool_memory_search(path: Option<String>, input: ToolMemorySearchInput) -> Result<Value, String> {
    let workspace = discover_workspace(path)?;
    let service = McpService::new(workspace.root());

    let target_path = input
        .file_path
        .as_deref()
        .map(|value| resolve_tool_file_path(&workspace, value));

    let report = service
        .memory_search(
            input.query,
            target_path,
            input.cursor_line,
            input.limit.unwrap_or(10),
        )
        .map_err(|error| error.to_string())?;

    serde_json::to_value(report).map_err(|error| error.to_string())
}

#[tauri::command]
fn tool_state_snapshot(path: Option<String>) -> Result<Value, String> {
    let workspace = discover_workspace(path)?;
    let service = McpService::new(workspace.root());
    let snapshot = service
        .state_snapshot()
        .map_err(|error| error.to_string())?;

    serde_json::to_value(snapshot).map_err(|error| error.to_string())
}

#[tauri::command]
fn tool_memory_preflight(path: Option<String>, input: ToolMemoryPreflightInput) -> Result<Value, String> {
    let workspace = discover_workspace(path)?;
    let service = McpService::new(workspace.root());

    let report = service
        .memory_preflight(input.operation.unwrap_or_else(|| "edit".to_string()))
        .map_err(|error| error.to_string())?;

    serde_json::to_value(report).map_err(|error| error.to_string())
}

#[tauri::command]
fn tool_state_transition_preview(path: Option<String>, input: ToolTransitionPreviewInput) -> Result<Value, String> {
    let workspace = discover_workspace(path)?;
    let service = McpService::new(workspace.root());

    let preview = service
        .state_transition_preview(&input.atom_id, input.target_state.into())
        .map_err(|error| error.to_string())?;

    serde_json::to_value(preview).map_err(|error| error.to_string())
}

#[tauri::command]
fn tool_propose_with_guard(path: Option<String>, input: ToolProposeInput) -> Result<Value, String> {
    let workspace = discover_workspace(path)?;
    let service = McpService::new(workspace.root());
    let target = resolve_tool_file_path(&workspace, &input.file_path);
    let kind: LoreKind = input.kind.into();
    let autofill = input.autofill.unwrap_or(true);

    let (title, body, scope, autofill_report) = if autofill {
        let report = service
            .autofill_proposal(
                &target,
                input.cursor_line,
                kind.clone(),
                input.title,
                input.body,
                input.scope,
            )
            .map_err(|error| error.to_string())?;

        let report_json = serde_json::to_value(&report).map_err(|error| error.to_string())?;

        (
            report.title,
            report.body,
            report.scope,
            Some(report_json),
        )
    } else {
        let title = input
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "title is required when autofill is disabled".to_string())?
            .to_string();

        (
            title,
            input.body,
            input.scope,
            None,
        )
    };

    let snapshot = service
        .state_snapshot()
        .map_err(|error| error.to_string())?;

    let proposal = service
        .propose(ProposalRequest {
            file_path: target,
            cursor_line: input.cursor_line,
            kind,
            title,
            body,
            scope,
            validation_script: input.validation_script,
        })
        .map_err(|error| error.to_string())?;

    Ok(json!({
        "state_snapshot": snapshot,
        "proposal": proposal,
        "autofill": autofill_report
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(WorkspaceWatcherState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_workspace,
            init_workspace,
            watch_workspace,
            workspace_status,
            workspace_timeline,
            validate_workspace,
            mark_atom,
            set_atom_state,
            atom_context,
            commit_diff,
            open_file_in_editor,
            tool_context,
            tool_memory_search,
            tool_state_snapshot,
            tool_memory_preflight,
            tool_state_transition_preview,
            tool_propose_with_guard
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
