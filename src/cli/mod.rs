use std::path::PathBuf;
use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use uuid::Uuid;

use crate::git;
use crate::lore::prism::{PrismSignal, PRISM_STALE_TTL_SECONDS};
use crate::lore::{AtomState, LoreAtom, LoreKind, Workspace, WorkspaceState};
use crate::mcp::{McpService, PreflightSeverity, ProposalRequest};

#[derive(Parser, Debug)]
#[command(
    name = "git-lore",
    version,
    about = "Capture and synchronize project rationale"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a new Git-Lore workspace
    Init(InitArgs),
    /// Create a new structured Lore Atom
    Mark(MarkArgs),
    /// Display the status of the local workspace
    Status(StatusArgs),
    /// Freeze a cryptographic snapshot of the current lore state
    Checkpoint(CheckpointArgs),
    /// Integrate lore directly into Git commit trailers
    Commit(CommitArgs),
    /// Emit ephemeral PRISM signals (soft-locks)
    Signal(SignalArgs),
    /// Fetch active lore constraints and history for a file
    Context(ContextArgs),
    /// Propose a new Lore Atom
    Propose(ProposeArgs),
    /// Spawn the Model Context Protocol (MCP) server
    Mcp(McpArgs),
    /// Explain the rationale of code based on bordered lore
    Explain(ExplainArgs),
    /// Run CI logic checks over the workspace vs canon Lore
    Validate(ValidateArgs),
    /// Synchronize the hot-workspace with cold-storage
    Sync(SyncArgs),
    /// Strap Git-Lore into local Git hooks
    Install(InstallArgs),
    /// The underlying reconciliator invoked automatically by Git
    Merge(MergeArgs),
    /// Alter the lifecycle state of an existing Lore Atom
    SetState(SetStateArgs),
    /// Generates LLM integration instructions/skills (e.g. for GitHub Copilot)
    Generate(GenerateArgs),
}

#[derive(Args, Debug)]
struct GenerateArgs {
    /// Target file to write integration skill/instruction to
    #[arg(default_value = ".github/git-lore-skills.md")]
    output: PathBuf,
}

#[derive(Args, Debug)]
struct InitArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(Args, Debug)]
struct MarkArgs {
    /// The brief identifier or name of the rule
    #[arg(long)]
    title: String,
    /// Explanatory text that provides context (the "Why")
    #[arg(long)]
    body: Option<String>,
    /// The scope boundary, like a function name or class
    #[arg(long)]
    scope: Option<String>,
    /// The target directory or file this rule binds to
    #[arg(long)]
    path: Option<PathBuf>,
    /// An optional automation script/regex for validators
    #[arg(long = "validation-script")]
    validation_script: Option<String>,
    /// The typology of the lore
    #[arg(long, value_enum, default_value = "decision")]
    kind: LoreKindArg,
}

#[derive(Args, Debug)]
struct StatusArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(Args, Debug)]
struct CheckpointArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Optional message outlining the checkpoint reason
    #[arg(long)]
    message: Option<String>,
}

#[derive(Args, Debug)]
struct CommitArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Your commit message subject
    #[arg(long)]
    message: String,
    /// Allow committing even if no files changed (lore changes only)
    #[arg(long, default_value_t = true)]
    allow_empty: bool,
}

#[derive(Args, Debug)]
struct SignalArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    workspace: PathBuf,
    /// Identifier for the active AI session (auto-generated if missing)
    #[arg(long)]
    session_id: Option<String>,
    /// The name/identity of the emitting agent
    #[arg(long)]
    agent: Option<String>,
    /// Target code scope
    #[arg(long)]
    scope: Option<String>,
    /// The affected file(s) or directories globs
    #[arg(long = "path", value_name = "GLOB")]
    paths: Vec<String>,
    /// The temporary assumptions running in memory
    #[arg(long = "assumption")]
    assumptions: Vec<String>,
    /// A tentative brief goal or decision
    #[arg(long)]
    decision: Option<String>,
}

#[derive(Args, Debug)]
struct ContextArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
    /// The target script/file
    #[arg(long)]
    file: PathBuf,
    /// Specific line number for tree-sitter drilling
    #[arg(long)]
    cursor_line: Option<usize>,
}

#[derive(Args, Debug)]
struct ProposeArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
    /// The target script/file
    #[arg(long)]
    file: PathBuf,
    /// The headline of the new rule
    #[arg(long)]
    title: String,
    /// The context and reasoning body
    #[arg(long)]
    body: Option<String>,
    /// An optional validation script or regex
    #[arg(long = "validation-script")]
    validation_script: Option<String>,
    /// Targeted line number
    #[arg(long)]
    cursor_line: Option<usize>,
    /// Type of the proposed element
    #[arg(long, value_enum, default_value = "decision")]
    kind: LoreKindArg,
}

#[derive(Args, Debug)]
struct McpArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(Args, Debug)]
struct ExplainArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
    /// The target file
    #[arg(long)]
    file: PathBuf,
    /// Specific line number for tree-sitter drilling
    #[arg(long)]
    cursor_line: Option<usize>,
}

#[derive(Args, Debug)]
struct ValidateArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(Args, Debug)]
struct SyncArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(Args, Debug)]
struct InstallArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(Args, Debug)]
struct MergeArgs {
    /// Base commit/lore state
    base: PathBuf,
    /// Current commit/lore state
    current: PathBuf,
    /// Other branch commit/lore state
    other: PathBuf,
}

#[derive(Args, Debug)]
struct SetStateArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
    /// The ID of the atom to update
    #[arg(long = "atom-id")]
    atom_id: String,
    /// The new state for the atom
    #[arg(long, value_enum)]
    state: AtomStateArg,
    /// The reason for changing the state
    #[arg(long)]
    reason: String,
    /// The actor making the change
    #[arg(long)]
    actor: Option<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LoreKindArg {
    Decision,
    Assumption,
    OpenQuestion,
    Signal,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum AtomStateArg {
    Draft,
    Proposed,
    Accepted,
    Deprecated,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => init(args),
        Commands::Mark(args) => mark(args),
        Commands::Status(args) => status(args),
        Commands::Checkpoint(args) => checkpoint(args),
        Commands::Commit(args) => commit(args),
        Commands::Signal(args) => signal(args),
        Commands::Context(args) => context(args),
        Commands::Propose(args) => propose(args),
        Commands::Mcp(args) => mcp(args),
        Commands::Explain(args) => explain(args),
        Commands::Validate(args) => validate(args),
        Commands::Sync(args) => sync(args),
        Commands::Install(args) => install(args),
        Commands::Merge(args) => merge(args),
        Commands::SetState(args) => set_state(args),
        Commands::Generate(args) => generate(args),
    }
}

fn generate(args: GenerateArgs) -> Result<()> {
    if let Some(parent) = args.output.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).with_context(|| format!("failed to create dir: {}", parent.display()))?;
        }
    }
    
    let content = r#"# Git-Lore Skills

**Description:**
Help integrate `git-lore` natively into workflows to keep architectural decisions and knowledge strongly bound to codebase states.

**When to Use:**
- When adding notes/assumptions explicitly requested by the user.
- When a user asks "document this pattern for later".
- Upon discovering a consistent convention not currently documented in `.lore` or another team's doc.

**How it Works:**

Use the `.lore` directory or `git-lore` CLI to preserve knowledge:

1. **Mark / Propose:** Propose an atom using the `git-lore propose` tool via the MCP server or the CLI `git-lore mark`.
2. **Context:** To read context, use the `git_lore_memory_search` or `git_lore_context` MCP tools, or via `git-lore context --file <file>`.
3. **Workflow Integration:** Keep git commits enriched. Ask developers if they want changes bound using `git-lore commit`.

Please refer to the [workflows](https://github.com/JussMor/git-lore/blob/main/docs/workflows.html) for detailed decision trees.
"#;
    let mut file = std::fs::File::create(&args.output).context("failed to create output file")?;
    file.write_all(content.as_bytes()).context("failed to write content")?;
    
    println!("Successfully generated Git-Lore skill at: {}", args.output.display());
    Ok(())
}

fn init(args: InitArgs) -> Result<()> {
    let workspace = Workspace::init(&args.path)
        .with_context(|| format!("failed to initialize workspace at {}", args.path.display()))?;

    println!(
        "Initialized Git-Lore workspace at {}",
        workspace.root().display()
    );
    Ok(())
}

fn mark(args: MarkArgs) -> Result<()> {
    let workspace = Workspace::discover(".")?;
    enforce_cli_write_guard(workspace.root(), "edit")?;

    let atom = LoreAtom::new(
        args.kind.into(),
        AtomState::Proposed,
        args.title,
        args.body,
        args.scope,
        args.path,
    )
    .with_validation_script(args.validation_script);
    let atom_id = atom.id.clone();

    workspace.record_atom(atom)?;
    println!("Recorded proposed lore atom {atom_id}");
    Ok(())
}

fn status(args: StatusArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.path)?;
    let state = workspace.load_state()?;
    let report = workspace.entropy_report()?;

    println!("Workspace: {}", workspace.root().display());
    println!("Total atoms: {}", state.atoms.len());
    println!("Entropy score: {}/100", report.score);

    for atom in state.atoms.iter().rev().take(5) {
        println!(
            "- [{}] {:?} {:?}: {}",
            atom.id, atom.kind, atom.state, atom.title
        );
    }

    if report.contradictions.is_empty() {
        println!("Contradictions: none");
    } else {
        println!("Contradictions:");
        for contradiction in report.contradictions.iter().take(5) {
            println!("- {:?} {}: {}", contradiction.kind, contradiction.key, contradiction.message);
        }
    }

    if !report.notes.is_empty() {
        println!("Entropy notes:");
        for note in report.notes {
            println!("- {note}");
        }
    }

    Ok(())
}

fn checkpoint(args: CheckpointArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.path)?;
    enforce_cli_write_guard(workspace.root(), "commit")?;

    let checkpoint = workspace.write_checkpoint(args.message)?;
    let subject = checkpoint
        .message
        .as_deref()
        .unwrap_or("git-lore checkpoint");
    let commit_message = git::build_commit_message(subject, &checkpoint.atoms);

    if let Ok(repository) = git::discover_repository(&workspace.root()) {
        println!("Git repository: {}", git::repository_root(&repository).display());
    }

    println!("Checkpoint {} written", checkpoint.id);
    if !commit_message.is_empty() {
        println!();
        println!("{commit_message}");
    }

    Ok(())
}

fn commit(args: CommitArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.path)?;
    enforce_cli_write_guard(workspace.root(), "commit")?;

    let repository_root = git::discover_repository(workspace.root())?;
    let state = workspace.load_state()?;

    let validation_issues = git::validate_workspace_against_git(&repository_root, &workspace)?;
    if !validation_issues.is_empty() {
        anyhow::bail!(
            "validation failed: {}",
            validation_issues.join("; ")
        );
    }

    let commit_message = git::build_commit_message(args.message, &state.atoms);

    let hash = git::commit_lore_message(&repository_root, commit_message, args.allow_empty)?;
    workspace.accept_active_atoms(Some(&hash))?;

    for atom in state.atoms.iter().filter(|atom| atom.state != AtomState::Deprecated) {
        git::write_lore_ref(&repository_root, atom, &hash)?;
    }

    println!("Committed lore checkpoint {}", hash);
    Ok(())
}

fn signal(args: SignalArgs) -> Result<()> {
    if args.paths.is_empty() {
        anyhow::bail!("at least one --path glob is required to broadcast a PRISM signal");
    }

    let workspace = Workspace::discover(&args.workspace)?;
    enforce_cli_write_guard(workspace.root(), "edit")?;

    let pruned_stale = workspace.prune_stale_prism_signals(PRISM_STALE_TTL_SECONDS)?;
    if pruned_stale > 0 {
        println!("Pruned {pruned_stale} stale PRISM signal(s) before broadcasting");
    }

    let signal = PrismSignal::new(
        args.session_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        args.agent,
        args.scope,
        args.paths,
        args.assumptions,
        args.decision,
    );

    workspace.write_prism_signal(&signal)?;
    let conflicts = workspace.scan_prism_conflicts(&signal)?;

    println!("Broadcast PRISM signal {}", signal.session_id);

    if conflicts.is_empty() {
        println!("No soft-lock conflicts detected.");
        return Ok(());
    }

    println!("Soft-lock warnings:");
    for conflict in conflicts {
        let agent = conflict.agent.as_deref().unwrap_or("unknown-agent");
        let scope = conflict.scope.as_deref().unwrap_or("unknown-scope");
        let decision = conflict.decision.as_deref().unwrap_or("no decision recorded");
        println!(
            "- session {} ({agent}, {scope}) overlaps on {}: {decision}",
            conflict.session_id,
            conflict.overlapping_paths.join(", "),
        );
    }

    Ok(())
}

fn context(args: ContextArgs) -> Result<()> {
    let service = McpService::new(&args.path);
    let snapshot = service.context(&args.file, args.cursor_line)?;

    println!("Workspace: {}", snapshot.workspace_root.display());
    println!("File: {}", snapshot.file_path.display());

    if let Some(scope) = snapshot.scope {
        println!("Scope: {} {} ({}-{})", scope.kind_label(), scope.name, scope.start_line, scope.end_line);
    }

    if snapshot.constraints.is_empty() {
        println!("No matching lore constraints.");
    } else {
        println!("Constraints:");
        for constraint in snapshot.constraints {
            println!("- {constraint}");
        }
    }

    Ok(())
}

fn propose(args: ProposeArgs) -> Result<()> {
    let service = McpService::new(&args.path);
    enforce_cli_write_guard(&args.path, "edit")?;

    let result = service.propose(ProposalRequest {
        file_path: args.file,
        cursor_line: args.cursor_line,
        kind: args.kind.into(),
        title: args.title,
        body: args.body,
        scope: None,
        validation_script: args.validation_script,
    })?;

    println!("Proposed lore atom {}", result.atom.id);
    if let Some(scope) = result.scope {
        println!("Scope: {} {} ({}-{})", scope.kind_label(), scope.name, scope.start_line, scope.end_line);
    }

    Ok(())
}

fn mcp(args: McpArgs) -> Result<()> {
    let server = crate::mcp::McpServer::new(&args.path);
    server.run_stdio()
}

fn explain(args: ExplainArgs) -> Result<()> {
    let service = McpService::new(&args.path);
    let snapshot = service.context(&args.file, args.cursor_line)?;

    println!("Workspace: {}", snapshot.workspace_root.display());
    println!("File: {}", snapshot.file_path.display());

    if let Some(scope) = snapshot.scope {
        println!("Scope: {} {} ({}-{})", scope.kind_label(), scope.name, scope.start_line, scope.end_line);
    }

    if snapshot.historical_decisions.is_empty() {
        println!("Historical decisions: none");
    } else {
        println!("Historical decisions:");
        for decision in snapshot.historical_decisions {
            println!("- {} {}", decision.commit_hash, decision.trailer_value);
        }
    }

    if snapshot.constraints.is_empty() {
        println!("No matching constraints.");
    } else {
        println!("Constraints:");
        for constraint in snapshot.constraints {
            println!("- {constraint}");
        }
    }

    Ok(())
}

fn validate(args: ValidateArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.path)?;
    let repository_root = git::discover_repository(workspace.root())?;
    let issues = git::validate_workspace_against_git(&repository_root, &workspace)?;

    if issues.is_empty() {
        println!("Validation passed");
        return Ok(());
    }

    println!("Validation issues:");
    for issue in issues {
        println!("- {issue}");
    }

    anyhow::bail!("validation failed");
}

fn sync(args: SyncArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.path)?;
    enforce_cli_write_guard(workspace.root(), "sync")?;

    let repository_root = git::discover_repository(workspace.root())?;
    let atoms = git::sync_workspace_from_git_history(&repository_root, &workspace)?;

    println!("Synchronized {} lore atoms from Git history", atoms.len());
    Ok(())
}

fn install(args: InstallArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.path)?;
    let repository_root = git::discover_repository(workspace.root())?;
    git::install_git_lore_integration(&repository_root)?;

    println!("Installed Git-Lore hooks and merge driver in {}", repository_root.display());
    Ok(())
}

fn merge(args: MergeArgs) -> Result<()> {
    let base = read_workspace_state_file(&args.base)
        .with_context(|| format!("failed to read base merge file {}", args.base.display()))?;
    let current = read_workspace_state_file(&args.current)
        .with_context(|| format!("failed to read current merge file {}", args.current.display()))?;
    let other = read_workspace_state_file(&args.other)
        .with_context(|| format!("failed to read other merge file {}", args.other.display()))?;

    let merged_version = base
        .state
        .version
        .max(current.state.version)
        .max(other.state.version);
    let outcome = crate::lore::merge::reconcile_lore(&base.state, &current.state, &other.state);
    let merged_state = WorkspaceState {
        version: merged_version,
        atoms: outcome.merged,
    };

    let write_gzip = base.was_gzip || current.was_gzip || other.was_gzip;
    write_workspace_state_file(&args.current, &merged_state, write_gzip)
        .with_context(|| format!("failed to write merged lore file {}", args.current.display()))?;

    if outcome.conflicts.is_empty() {
        println!("Merged lore state with {} atom(s)", merged_state.atoms.len());
        return Ok(());
    }

    eprintln!(
        "Lore merge produced {} conflict(s); manual review required",
        outcome.conflicts.len()
    );
    for conflict in outcome.conflicts {
        eprintln!("- {:?} {}: {}", conflict.kind, conflict.key, conflict.message);
    }

    anyhow::bail!("lore merge requires manual resolution")
}

fn set_state(args: SetStateArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.path)?;
    enforce_cli_write_guard(workspace.root(), "edit")?;

    let actor = args.actor.or_else(|| std::env::var("USER").ok());
    let updated = workspace.transition_atom_state(
        &args.atom_id,
        args.state.into(),
        args.reason,
        actor,
    )?;

    println!(
        "Transitioned lore atom {} to {:?}",
        updated.id, updated.state
    );
    Ok(())
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

#[derive(Clone, Debug)]
struct EncodedWorkspaceState {
    state: WorkspaceState,
    was_gzip: bool,
}

fn read_workspace_state_file(path: &std::path::Path) -> Result<EncodedWorkspaceState> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read lore state file {}", path.display()))?;
    let is_gzip = bytes.starts_with(&[0x1f, 0x8b]);

    let content = if is_gzip {
        let mut decoder = GzDecoder::new(bytes.as_slice());
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .with_context(|| format!("failed to decompress lore state file {}", path.display()))?;
        decoded
    } else {
        bytes
    };

    let state: WorkspaceState = serde_json::from_slice(&content)
        .with_context(|| format!("failed to parse lore state file {}", path.display()))?;

    Ok(EncodedWorkspaceState {
        state,
        was_gzip: is_gzip,
    })
}

fn write_workspace_state_file(
    path: &std::path::Path,
    state: &WorkspaceState,
    write_gzip: bool,
) -> Result<()> {
    let encoded = serde_json::to_vec_pretty(state)
        .with_context(|| format!("failed to encode merged lore state {}", path.display()))?;

    let bytes = if write_gzip {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&encoded)
            .with_context(|| format!("failed to gzip lore state {}", path.display()))?;
        encoder
            .finish()
            .with_context(|| format!("failed to finalize gzip lore state {}", path.display()))?
    } else {
        encoded
    };

    std::fs::write(path, bytes)
        .with_context(|| format!("failed to write lore state file {}", path.display()))?;

    Ok(())
}

fn enforce_cli_write_guard(path: impl AsRef<std::path::Path>, operation: &str) -> Result<()> {
    let service = McpService::new(path);
    let snapshot = service.state_snapshot()?;
    let report = service.memory_preflight(operation)?;

    if snapshot.state_checksum != report.state_checksum {
        anyhow::bail!(
            "state-first guard failed: state drift detected during preflight (snapshot {}, preflight {})",
            snapshot.state_checksum,
            report.state_checksum
        );
    }

    if report
        .issues
        .iter()
        .any(|issue| issue.severity == PreflightSeverity::Block)
    {
        println!("Preflight issues:");
        for issue in report.issues {
            println!("- {:?} {}: {}", issue.severity, issue.code, issue.message);
        }
        anyhow::bail!(
            "state-first preflight blocked {} operation; resolve issues and retry",
            operation
        );
    }

    for issue in report
        .issues
        .iter()
        .filter(|issue| issue.severity != PreflightSeverity::Info)
    {
        println!("Preflight {:?} {}: {}", issue.severity, issue.code, issue.message);
    }

    Ok(())
}
