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
use crate::lore::{AtomEditRequest, AtomState, LoreAtom, LoreKind, Workspace, WorkspaceState};
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
    /// Start an operational lore session (signal + pre-write checkpoint)
    SessionStart(SessionStartArgs),
    /// Finish an operational lore session (validate + commit + sync + post-sync checkpoint + release)
    SessionFinish(SessionFinishArgs),
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
    /// Edit an existing Lore Atom in-place (metadata/trace)
    EditAtom(EditAtomArgs),
    /// Generates LLM integration instructions/skills (e.g. for GitHub Copilot)
    Generate(GenerateArgs),
    /// Interactively resolve active content contradictions at a specific location
    Resolve(ResolveArgs),
}

#[derive(Args, Debug)]
struct ResolveArgs {
    /// The conflict location key (e.g. "path/to/file.rs::my_scope")
    #[arg(value_name = "LOCATION")]
    location: String,
    /// Workspace path
    #[arg(long, default_value = ".")]
    path: PathBuf,
    /// Pre-select the winning atom ID (bypasses interactive prompt)
    #[arg(long)]
    winner_id: Option<String>,
    /// Optional reason for this resolution
    #[arg(long)]
    reason: Option<String>,
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
    /// A literal shell command that validates the atom when preflight runs
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
    /// Release an existing PRISM signal for a session and exit
    #[arg(long, alias = "clear", default_value_t = false)]
    release: bool,
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
struct SessionStartArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    workspace: PathBuf,
    /// Identifier for the active session (auto-generated if missing)
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
    /// Optional reason to include in the pre-write checkpoint message
    #[arg(long)]
    reason: Option<String>,
    /// Optional explicit checkpoint message
    #[arg(long = "checkpoint-message")]
    checkpoint_message: Option<String>,
}

#[derive(Args, Debug)]
struct SessionFinishArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    workspace: PathBuf,
    /// Session identifier emitted by session-start
    #[arg(long)]
    session_id: String,
    /// Your commit message subject
    #[arg(long)]
    message: String,
    /// Allow committing even if no files changed (lore changes only)
    #[arg(long, default_value_t = true)]
    allow_empty: bool,
    /// Optional owner name for the post-sync checkpoint message
    #[arg(long)]
    agent: Option<String>,
    /// Optional reason to include in the post-sync checkpoint message
    #[arg(long)]
    reason: Option<String>,
    /// Optional explicit checkpoint message
    #[arg(long = "checkpoint-message")]
    checkpoint_message: Option<String>,
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
    /// A literal shell command that validates the atom when preflight runs
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

#[derive(Args, Debug)]
struct EditAtomArgs {
    /// Workspace path
    #[arg(default_value = ".")]
    path: PathBuf,
    /// The ID of the atom to edit
    #[arg(long = "atom-id")]
    atom_id: String,
    /// Optional new lore kind
    #[arg(long, value_enum)]
    kind: Option<LoreKindArg>,
    /// Optional new title
    #[arg(long)]
    title: Option<String>,
    /// Optional new body
    #[arg(long)]
    body: Option<String>,
    /// Clear existing body
    #[arg(long, default_value_t = false)]
    clear_body: bool,
    /// Optional new scope
    #[arg(long)]
    scope: Option<String>,
    /// Clear existing scope
    #[arg(long, default_value_t = false)]
    clear_scope: bool,
    /// Optional new atom path anchor
    #[arg(long = "atom-path")]
    atom_path: Option<PathBuf>,
    /// Clear existing atom path anchor
    #[arg(long, default_value_t = false)]
    clear_atom_path: bool,
    /// Optional new validation script command
    #[arg(long = "validation-script")]
    validation_script: Option<String>,
    /// Clear existing validation script
    #[arg(long, default_value_t = false)]
    clear_validation_script: bool,
    /// Set accepted trace commit SHA
    #[arg(long = "trace-commit-sha")]
    trace_commit_sha: Option<String>,
    /// Clear accepted trace commit SHA
    #[arg(long, default_value_t = false)]
    clear_trace_commit: bool,
    /// Required reason for audit logging
    #[arg(long)]
    reason: String,
    /// The actor making the edit
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
        Commands::SessionStart(args) => session_start(args),
        Commands::SessionFinish(args) => session_finish(args),
        Commands::Context(args) => context(args),
        Commands::Propose(args) => propose(args),
        Commands::Mcp(args) => mcp(args),
        Commands::Explain(args) => explain(args),
        Commands::Validate(args) => validate(args),
        Commands::Sync(args) => sync(args),
        Commands::Install(args) => install(args),
        Commands::Merge(args) => merge(args),
        Commands::SetState(args) => set_state(args),
        Commands::EditAtom(args) => edit_atom(args),
        Commands::Generate(args) => generate(args),
        Commands::Resolve(args) => resolve(args),
    }
}

fn generate(args: GenerateArgs) -> Result<()> {
    if let Some(parent) = args.output.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).with_context(|| format!("failed to create dir: {}", parent.display()))?;
        }
    }
    
    let content = r#"---
description: >-
  Git-Lore Integration Skill. Helps capture architectural rationale, rules, and assumptions tightly bound to the codebase via the Git-Lore CLI and MCP server tools. Use when a user asks to establish a new codebase rule, architectural decision, or convention.
---

# Git-Lore Skills

Keep architectural decisions and knowledge strongly bound to codebase states.

**When to Use:**
- When adding notes/assumptions explicitly requested by the user.
- When a user asks "document this pattern for later", "mark this assumption", or "save this rule".
- Upon discovering a consistent convention not currently documented in `.lore`.

## Instructions

<instructions>
You are an AI assistant empowered to use `git-lore`, a tool that anchors rationale as structured "lore atoms" directly bounded to codebase paths and scopes.

### 1. Discovering Lore (Context)
When you navigate to a new file or need to understand how it should be implemented, read the context using:
- **MCP Tool:** `git_lore_context` (pass the file path) or `git_lore_memory_search` (pass a query).
- **CLI Alternative:** Tell the user to run `git-lore context --file <file>` or `git-lore explain --file <file>`.

### 2. Recording Lore (Propose / Mark)
When the user and you make an important architectural decision, or establish a convention that other AI agents should know:
- **MCP Tool:** Call `git_lore_propose`. **Crucial:** You must first call `git_lore_state_snapshot` to get the `state_checksum` and `snapshot_generated_unix_seconds` required for proposing.
- **CLI Alternative:** Suggest the user run:
  `git-lore mark --title "Your concise rule constraint" --body "The reason why this exists" --path "<relative_file_path>"`

### 3. Git Workflows
When the task is done, gently remind the user they can commit this knowledge firmly to Git by running `git-lore commit --message "feat: your task"`.

# Flujo de Trabajo: Git-Lore + Git

**¿En qué etapa de desarrollo te encuentras?**

## 1. Modificando código críptico o legado
Necesitas arreglar un bug o extender un módulo, pero el código es confuso y no sabes qué romperás si lo cambias.

**Flujo:**
*   **Git:** Crear rama: `git checkout -b fix/module`
*   **Git-Lore:** Obtener reglas: `git-lore context --file module.rs`

> **¿Cómo ayuda?** Resuelve la paradoja de "Chesterton's Fence". Antes de borrar o cambiar código, el sistema te expone *por qué* se hizo (las decisiones históricas que enmarcan ese archivo), evitantando que re-introduzcas bugs antiguos.

## 2. Tomando decisiones arquitectónicas clave
Estás liderando un nuevo feature y has decidido usar un patrón de diseño o herramienta específica para este módulo.

**Flujo:**
*   **Git:** Programar la lógica principal y hacer `git add .`
*   **Git-Lore:** Marcar: `git-lore mark --kind decision --title "Usar Patrón Builder..."`
*   **Integración:** Confirmar: `git-lore commit -m "feat: xyz"`

> **¿Cómo ayuda?** Al usar `git-lore commit`, el contexto no solo se queda local, sino que se inyecta como un *Git Trailer* en el historial puro de Git. Cualquiera (incluso sin tener git-lore) puede ver en `git log` la traza de la decisión junto al código que la implementó.

## 3. Delegando código complejo a una IA (Copilot, Agentes)
Le estás pidiendo a un Agente IA que genere un refactor masivo o construya un nuevo servicio desde tu editor (VS Code).

**Flujo:**
*   **MCP Server:** La IA pide contexto silenciosamente: `git_lore_context(scope)`
*   **Desarrollo:** La IA genera código respetando las restricciones inyectadas.
*   **Evolución:** La IA sugiere reglas: `git_lore_propose(...)`

> **¿Cómo ayuda?** Alimenta automáticamente a la IA (Zero-Shot compliance). Previene que el Agente alucine patrones equivocados o traiga dependencias prohibidas. La IA "nace" conociendo cómo funciona este equipo o proyecto.

## 4. Revisión de un Pull Request
Un colega sube su código para que lo apruebes y se funda con la rama principal.

**Flujo:**
*   **Git / CI:** Se levanta la Pull Request en GitHub/GitLab.
*   **Git-Lore:** CI verifica o el humano ejecuta `git-lore validate`.

> **¿Cómo ayuda?** Transforma las opiniones subjetivas en revisiones objetivas. El validador (o el revisor) puede chequear si el código en revisión rompe alguna regla que fue previamente "Acordada y Aceptada" en el lore del directorio afectado.

## 5. Explorando la Memoria del Proyecto (Discovery)
No recuerdas por qué se tomó una decisión hace meses, o le pides a una IA que investigue el proyecto antes de proponer código nuevo.

**Flujo:**
*   **MCP Server:** La IA busca intenciones difusas: `git_lore_memory_search("auth architecture")`
*   **Git-Lore:** Obtener justificación detallada: `git-lore explain --file src/auth.rs`

> **¿Cómo ayuda?** Democratiza el conocimiento histórico. A través del buscador léxico y semántico del MCP, puedes encontrar conocimiento por "intención" y "recencia", en lugar de buscar a ciegas en Slack o Jira.

## 6. Evolución del Conocimiento (Estado y Ciclo de Vida)
El código cambia, y las reglas también deben hacerlo. Una convención propuesta por IA necesita ser aceptada, o una regla antigua queda obsoleta.

**Flujo:**
*   **MCP Server:** La IA sugiere cambios: `git_lore_propose(target_state="Proposed")`
*   **Git-Lore:** El humano formaliza: `git-lore set-state --state accepted`
*   **Git-Lore:** Las reglas viejas se retiran: `git-lore set-state --state deprecated`

> **¿Cómo ayuda?** El canon (lore) nunca es inmutable y no se convierte en una Wiki zombie. Pasa por estados `Draft -> Proposed -> Accepted -> Deprecated`, dándole al equipo y agentes control explícito sobre la validez del conocimiento sobre el tiempo.

## 7. Flujos Activos Autoriales (Signals & Preflight)
Agentes IA autónomos necesitan verificar la seguridad de la memoria y alertar al equipo de sus intenciones transitorias antes de destruir estados del repositorio accidentalmente.

**Flujo:**
*   **Git-Lore:** Crear instantánea segura: `git-lore checkpoint / git-lore status`
*   **MCP Server:** Validaciones de estado: `git_lore_memory_preflight("commit")`
*   **Git-Lore:** Agentes emiten eventos cortos: `git-lore signal --agent "Codegen"`

> **¿Cómo ayuda?** Permite la colaboración segura (Safe Writes) con Inteligencia Artificial. Con verificaciones previas como `transition_preview` y `preflight`, se evita la sobrescritura y entropía donde la IA accidentalmente contradiga decisiones base de otras ramas.

## 8. Congelando el Conocimiento (Checkpoints)
Estás a punto de hacer un refactor masivo de reglas de negocio o estás orquestando múltiples agentes de IA simultáneos. Necesitas asegurar un punto de restauración seguro de las intenciones de tu equipo.

**Flujo:**
*   **Git-Lore:** Congelar el estado base: `git-lore checkpoint --message "Pre-refactor de auth"`
*   **MCP Server:** Agentes IA validan checksums: `git_lore_state_snapshot()`
*   **Integración:** Fallo rápido (Fail-fast) preventivo en caso de discrepancias temporales.

> **¿Cómo ayuda?** Resuelve la desalineación de estados o condiciones de carrera entre ramas, humanos y Agentes de IA. Un 'checkpoint' crea una fotografía instantánea del *Lore*. Si el código muta o un agente propone un cambio basándose en información desactualizada, el archivo bloquea la sobre-escritura (Strict State-First Check).

## 9. Fusión y Reconciliación de Conocimiento (Merge)
Trabajas en una rama feature donde propusiste nuevas decisiones, mientras que en la rama 'main' otra persona agregó o deprecó otras reglas. Ahora necesitas fusionar ambas ramas sin perder ni contradecir el Lore.

**Flujo:**
*   **Git:** Comienza la fusión de archivos: `git merge feature/branch`
*   **Git-Lore:** Git dispara el merge driver: `git-lore merge <base> <current> <other>`
*   **Git-Lore:** Reconciliación: deduplica IDs, verifica estados (Ej. "Accepted" vence a "Proposed").

> **¿Cómo ayuda?** Git-Lore se instala como un 'Merge Driver' personalizado (vía `git-lore install`). A diferencia de fusionar código o JSON manualmente, este previene colisiones semánticas. Si un átomo en 'main' fue marcado como `Deprecated`, pero en tu rama lo habías actualizado, el algoritmo de reconciliación lo fusionará inteligentemente.

## 10. Proposiciones y Señales Contextuales (Propose & Signal)
Durante un sprint rápido, un desarrollador o una IA lanza una "Suposición" temporal (Signal) al aire para que la IA que trabaje en el código asociado la tenga en cuenta temporalmente, o "proponga" formalmente (Propose) una nueva convención.

**Flujo:**
*   **Git-Lore:** Crear señal efímera: `git-lore signal --assumption "Asumo que la API devuelve XML" --path src/`
*   **MCP Server:** Subagentes leen la señal: `git_lore_memory_search()` expone la suposición fresca.
*   **Git-Lore:** Validación: `git-lore propose --title "API responde JSON" --kind decision` reemplaza la suposición.

> **¿Cómo funciona el salto de Señal a Decisión internamente?**
>
> 1.  **La Señal (Conocimiento Efímero):** `git-lore signal` NO crea un Registro permanente. Crea un archivo temporal (PrismSignal) con un Tiempo de Vida (TTL) programado para expirar. Actúa como un cerrojo suave ("Soft-lock") para avisar a otros agentes: *"Ojo, estoy asumiendo esto en la memoria ahora mismo"*.
> 2.  **La Decisión (Conocimiento Canónico):** `git-lore propose --kind decision` crea un "Átomo" real, un archivo JSON estructurado con un UUID que entra formalmente al ciclo de evaluación (Proposed / Accepted).
> 3.  **El Reemplazo:** La "asunción" inicial no se sobre-escribe mágicamente código sobre código. En cambio, cuando el agente termina su trabajo y formaliza la regla con `propose`, el servidor inscribe el Átomo permanente. En procesos de guardado posteriores, Git-Lore invoca una limpieza (`prune_stale_prism_signals`) evaporando las señales vencidas de la carpeta `.lore/signals/`. El conocimiento fugaz muere, y el canon estructurado prevalece inmutable.
</instructions>
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

    let hash = run_lore_commit(&workspace, &args.message, args.allow_empty)?;

    println!("Committed lore checkpoint {}", hash);
    Ok(())
}

fn run_lore_commit(workspace: &Workspace, message: impl AsRef<str>, allow_empty: bool) -> Result<String> {
    let repository_root = git::discover_repository(workspace.root())?;
    let state = workspace.load_state()?;

    let validation_issues = git::validate_workspace_against_git(&repository_root, workspace)?;
    if !validation_issues.is_empty() {
        anyhow::bail!(
            "validation failed: {}",
            validation_issues.join("; ")
        );
    }

    let commit_message = git::build_commit_message(message, &state.atoms);

    let hash = git::commit_lore_message(&repository_root, commit_message, allow_empty)?;
    workspace.accept_active_atoms(Some(&hash))?;

    for atom in state.atoms.iter().filter(|atom| atom.state != AtomState::Deprecated) {
        git::write_lore_ref(&repository_root, atom, &hash)?;
    }

    Ok(hash)
}

fn session_start(args: SessionStartArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.workspace)?;
    enforce_cli_signal_guard(workspace.root())?;

    let pruned_stale = workspace.prune_stale_prism_signals(PRISM_STALE_TTL_SECONDS)?;
    if pruned_stale > 0 {
        println!("Pruned {pruned_stale} stale PRISM signal(s) before starting session");
    }

    let mut paths = args.paths;
    if paths.is_empty() {
        paths.push(".".to_string());
    }

    let signal = PrismSignal::new(
        args.session_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        args.agent,
        args.scope,
        paths,
        args.assumptions,
        args.decision,
    );

    workspace.write_prism_signal(&signal)?;
    let conflicts = workspace.scan_prism_conflicts(&signal)?;

    enforce_cli_write_guard(workspace.root(), "commit")?;
    let checkpoint_message = args.checkpoint_message.unwrap_or_else(|| {
        build_session_checkpoint_message(
            "pre-write",
            &signal.session_id,
            signal.agent.as_deref(),
            args.reason.as_deref(),
            None,
        )
    });
    let checkpoint = workspace.write_checkpoint(Some(checkpoint_message))?;

    println!("Session started: {}", signal.session_id);
    println!("Pre-write checkpoint: {}", checkpoint.id);

    if conflicts.is_empty() {
        println!("No soft-lock conflicts detected.");
    } else {
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
    }

    println!(
        "Next: run propose/mark updates, then session-finish with --session-id {}",
        signal.session_id
    );

    Ok(())
}

fn session_finish(args: SessionFinishArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.workspace)?;
    enforce_cli_write_guard(workspace.root(), "commit")?;

    let signal_owner = workspace
        .load_prism_signals()?
        .into_iter()
        .find(|signal| signal.session_id == args.session_id)
        .and_then(|signal| signal.agent);

    let hash = run_lore_commit(&workspace, &args.message, args.allow_empty)?;
    let repository_root = git::discover_repository(workspace.root())?;

    enforce_cli_write_guard(workspace.root(), "sync")?;
    let atoms = git::sync_workspace_from_git_history(&repository_root, &workspace)?;

    let checkpoint_owner = args.agent.or(signal_owner);
    let post_checkpoint_message = args.checkpoint_message.unwrap_or_else(|| {
        build_session_checkpoint_message(
            "post-sync",
            &args.session_id,
            checkpoint_owner.as_deref(),
            args.reason.as_deref(),
            Some(&hash),
        )
    });
    let checkpoint = workspace.write_checkpoint(Some(post_checkpoint_message))?;

    let released = workspace.remove_prism_signal(&args.session_id)?;

    println!("Committed lore checkpoint {}", hash);
    println!("Synchronized {} lore atoms from Git history", atoms.len());
    println!("Post-sync checkpoint: {}", checkpoint.id);
    if released {
        println!("Released PRISM signal {}", args.session_id);
    } else {
        println!("No PRISM signal found for session {}", args.session_id);
    }

    Ok(())
}

fn build_session_checkpoint_message(
    stage: &str,
    session_id: &str,
    owner: Option<&str>,
    reason: Option<&str>,
    commit_sha: Option<&str>,
) -> String {
    let owner = owner
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");

    let mut parts = vec![
        format!("owner={owner}"),
        format!("session={session_id}"),
        format!("stage={stage}"),
    ];

    if let Some(reason) = reason.map(str::trim).filter(|value| !value.is_empty()) {
        parts.push(format!("reason={reason}"));
    }

    if let Some(commit_sha) = commit_sha
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(format!("commit={commit_sha}"));
    }

    parts.join("; ")
}
fn signal(args: SignalArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.workspace)?;

    if args.release {
        let session_id = args
            .session_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("--session-id is required when using --release"))?;

        let removed = workspace.remove_prism_signal(session_id)?;
        if removed {
            println!("Released PRISM signal {session_id}");
        } else {
            println!("No PRISM signal found for session {session_id}");
        }
        return Ok(());
    }

    if args.paths.is_empty() {
        anyhow::bail!("at least one --path glob is required to broadcast a PRISM signal");
    }

    enforce_cli_signal_guard(workspace.root())?;

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

    #[cfg(feature = "semantic-search")]
    {
        let state = workspace.load_state()?;
        let accepted = workspace.load_accepted_atoms()?;
        crate::mcp::semantic::rebuild_index(workspace.root(), &state.atoms, &accepted)?;
        println!("Rebuilt Memvid semantic local index.");
    }

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

fn edit_atom(args: EditAtomArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.path)?;
    enforce_cli_write_guard(workspace.root(), "edit")?;

    if args.body.is_some() && args.clear_body {
        anyhow::bail!("cannot use --body and --clear-body together");
    }
    if args.scope.is_some() && args.clear_scope {
        anyhow::bail!("cannot use --scope and --clear-scope together");
    }
    if args.atom_path.is_some() && args.clear_atom_path {
        anyhow::bail!("cannot use --atom-path and --clear-atom-path together");
    }
    if args.validation_script.is_some() && args.clear_validation_script {
        anyhow::bail!("cannot use --validation-script and --clear-validation-script together");
    }
    if args.trace_commit_sha.is_some() && args.clear_trace_commit {
        anyhow::bail!("cannot use --trace-commit-sha and --clear-trace-commit together");
    }

    let body_update = if args.clear_body {
        Some(None)
    } else {
        args.body.map(Some)
    };
    let scope_update = if args.clear_scope {
        Some(None)
    } else {
        args.scope.map(Some)
    };
    let path_update = if args.clear_atom_path {
        Some(None)
    } else {
        args.atom_path.map(Some)
    };
    let validation_script_update = if args.clear_validation_script {
        Some(None)
    } else {
        args.validation_script.map(Some)
    };
    let trace_commit_update = if args.clear_trace_commit {
        Some(None)
    } else {
        args.trace_commit_sha.map(Some)
    };

    let actor = args.actor.or_else(|| std::env::var("USER").ok());
    let result = workspace.edit_atom(
        &args.atom_id,
        AtomEditRequest {
            kind: args.kind.map(Into::into),
            title: args.title,
            body: body_update,
            scope: scope_update,
            path: path_update,
            validation_script: validation_script_update,
            trace_commit_sha: trace_commit_update,
        },
        args.reason,
        actor,
    )?;

    if result.changed_fields.is_empty() {
        println!("No changes applied to lore atom {}", result.atom.id);
        return Ok(());
    }

    println!(
        "Edited lore atom {} in-place ({})",
        result.atom.id,
        result.changed_fields.join(", "),
    );
    if let Some(source_commit) = result.source_commit {
        println!("Trace commit: {source_commit}");
    }

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

fn enforce_cli_signal_guard(path: impl AsRef<std::path::Path>) -> Result<()> {
    let service = McpService::new(path);
    let snapshot = service.state_snapshot()?;
    let report = service.memory_preflight("edit")?;

    if snapshot.state_checksum != report.state_checksum {
        anyhow::bail!(
            "state-first guard failed: state drift detected during preflight (snapshot {}, preflight {})",
            snapshot.state_checksum,
            report.state_checksum
        );
    }

    let blocking_issues = report
        .issues
        .iter()
        .filter(|issue| issue.severity == PreflightSeverity::Block)
        .filter(|issue| issue.code != "prism_hard_lock")
        .collect::<Vec<_>>();

    if !blocking_issues.is_empty() {
        println!("Preflight issues:");
        for issue in report.issues {
            println!("- {:?} {}: {}", issue.severity, issue.code, issue.message);
        }
        anyhow::bail!(
            "state-first preflight blocked signal operation; resolve issues and retry"
        );
    }

    for issue in report
        .issues
        .iter()
        .filter(|issue| issue.code != "prism_hard_lock")
        .filter(|issue| issue.severity != PreflightSeverity::Info)
    {
        println!("Preflight {:?} {}: {}", issue.severity, issue.code, issue.message);
    }

    Ok(())
}

fn resolve(args: ResolveArgs) -> Result<()> {
    let workspace = Workspace::discover(&args.path)?;
    enforce_cli_write_guard(workspace.root(), "edit")?;

    let state = workspace.load_state()?;
    let target_location = args.location.clone();
    
    let candidate_atoms: Vec<LoreAtom> = state.atoms.into_iter()
        .filter(|atom| {
            let p = atom.path.as_ref().map(|v| v.to_string_lossy().replace('\\', "/")).unwrap_or_else(|| "<no-path>".to_string());
            let s = atom.scope.as_deref().unwrap_or("<no-scope>");
            format!("{}::{}", p, s) == target_location && atom.state != AtomState::Deprecated
        })
        .collect();

    if candidate_atoms.is_empty() {
        println!("No active contradictions found at location {target_location}");
        return Ok(());
    }

    let winner_id = if let Some(id) = args.winner_id {
        if !candidate_atoms.iter().any(|a| a.id == id) {
            anyhow::bail!("Atom ID {} not found among active atoms at location {}", id, target_location);
        }
        id
    } else {
        println!("Found {} active atoms at {}:", candidate_atoms.len(), target_location);
        for (i, atom) in candidate_atoms.iter().enumerate() {
            println!("[{}] {} ({:?}) - {}", i, atom.id, atom.state, atom.title);
        }
        
        let idx = loop {
            print!("Select the winning atom index (0-{}): ", candidate_atoms.len() - 1);
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if let Ok(idx) = input.trim().parse::<usize>() {
                if idx < candidate_atoms.len() {
                    break idx;
                }
            }
            println!("Invalid selection.");
        };
        candidate_atoms[idx].id.clone()
    };

    let actor = std::env::var("USER").ok();
    
    for atom in candidate_atoms {
        if atom.id != winner_id {
            // Note: workspace.transition_atom_state requires (id, target_state, reason, actor)
            println!("Deprecating lost atom {}...", atom.id);
            let res = workspace.transition_atom_state(
                &atom.id,
                AtomState::Deprecated,
                args.reason.clone().unwrap_or_else(|| "Resolved via CLI".to_string()),
                actor.clone(),
            );
            if let Err(e) = res {
                println!("Warning: failed to deprecate {}: {}", atom.id, e);
            }
        }
    }
    
    // Attempt to accept the winner if it isn't accepted yet
    let winner_atom = workspace.load_state()?.atoms.into_iter().find(|a| a.id == winner_id).unwrap();
    if winner_atom.state != AtomState::Accepted {
        println!("Accepting winning atom {}...", winner_id);
        let res = workspace.transition_atom_state(
            &winner_id,
            AtomState::Accepted,
            args.reason.clone().unwrap_or_else(|| "Resolved via CLI".to_string()),
            actor.clone(),
        );
        if let Err(e) = res {
            println!("Warning: failed to accept winner {}: {}", winner_id, e);
        }
    }

    println!("Conflict at {} resolved successfully. [{}] is the winner.", target_location, winner_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::build_session_checkpoint_message;

    #[test]
    fn session_checkpoint_message_includes_required_fields() {
        let message = build_session_checkpoint_message(
            "pre-write",
            "session-123",
            Some("agent-x"),
            Some("capture evidence"),
            None,
        );

        assert!(message.contains("owner=agent-x"));
        assert!(message.contains("session=session-123"));
        assert!(message.contains("stage=pre-write"));
        assert!(message.contains("reason=capture evidence"));
        assert!(!message.contains("commit="));
    }

    #[test]
    fn session_checkpoint_message_includes_commit_when_present() {
        let message = build_session_checkpoint_message(
            "post-sync",
            "session-123",
            None,
            None,
            Some("abc123"),
        );

        assert!(message.contains("owner=unknown"));
        assert!(message.contains("stage=post-sync"));
        assert!(message.contains("commit=abc123"));
    }
}
