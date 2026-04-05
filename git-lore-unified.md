# Git-Lore: Unified Specification

> **Status:** Research Specification / Spec-Driven Development (SDD)
> **Focus:** Local-first, High-Performance Intent Tracking in Rust

---

## Product Vision

Git-Lore is a **"Cognitive Sidecar"** for Git. It elevates _Rationale_ (the **why**) to a first-class versioned entity, on par with _Source Code_ (the **what**). It solves the **"Decision Shadow"** problem by capturing agentic and human intent at the moment of creation, preventing context rot in long-running projects.

**The Local-First Mandate:** Git-Lore operates entirely on the user's local machine, using Git's own Directed Acyclic Graph (DAG) to store metadata. If you have the code, you have the Lore.

---

## 1. The Protocol Stack

Git-Lore operates through four layered protocols that capture, signal, and persist intent.

### A. Smriti Protocol — The Checkpointing Layer

Converts ephemeral LLM thought-chains into structured **"Lore Atoms"** stored in `.lore/active_intent.yml`. Rather than storing raw chat history (token-heavy, noisy), Smriti extracts three categories:

- **Decisions** — Finalized architectural choices (e.g., _"Use AES-256 for encryption"_)
- **Assumptions** — Temporary beliefs (e.g., _"Assuming the API returns JSON"_)
- **Open Questions** — Unresolved blockers (e.g., _"Will this scale to 10k RPS?"_)

### B. PRISM — Proactive Intent Signaling Mechanism

Prevents **Semantic Merge Conflicts** before code is even written.

When an agent or human starts a task, they run `git lore signal`. This writes a lightweight **"Intent Lock"** to a shared namespace, broadcasting intent to the entire swarm. If a second agent attempts to modify a dependency of a locked module, PRISM triggers a warning immediately.

### C. Lore Protocol — The Persistence Layer

Binds the **"Why"** to the **"What"** inside the Git DAG by mapping Lore Atoms into **Git Commit Trailers**.

```
feat: implement session caching

Lore-Decision: [ID-42] Use Memcached for sub-ms latency.
Lore-Assumption: [ID-43] Sessions are non-persistent.
Lore-Ref: refs/lore/commits/a1b2c3d4
```

### D. MCP — Model Context Protocol

Allows IDEs (Cursor, VS Code) and Agents (Devin, OpenDevin) to **"read" the lore**. Git-Lore hosts a local MCP server exposing two core methods:

- `get_context(file_path)` — Retrieve the last 5 architectural decisions affecting a file
- `propose_decision(rationale)` — Record a new assumption before editing code

---

## 2. Git Integration

Git-Lore does not wrap Git — it **hooks into it**.

### Custom Merge Driver

Registered in `.gitconfig`, the driver compares `.lore` files during `git merge`. If Branch A assumes _"Database is Local"_ and Branch B assumes _"Database is Cloud"_, Git-Lore blocks the merge with a **"Cognitive Conflict"** even if the code diffs are clean.

```ini
[merge "lore"]
  name = Git-Lore Reasoning Merger
  driver = git-lore merge %O %A %B
```

### Shadow Refs (`refs/lore`)

Lore metadata lives in a custom Git namespace to avoid bloating the main source tree.

```bash
git update-ref refs/lore/intent <hash>
```

These refs are pushed and pulled like branches — the Lore travels with the code — but don't appear in standard `git log` unless requested.

### Git Hooks

| Hook            | Function                                                                                                              |
| --------------- | --------------------------------------------------------------------------------------------------------------------- |
| `pre-commit`    | Validates that new code aligns with active Lore Decisions                                                             |
| `post-checkout` | Reconstructs local `.lore/` state from the current commit's metadata, so agent context "time travels" with the branch |

---

## 3. Lore Atom Lifecycle (Smriti State Machine)

Every piece of rationale follows a strict lifecycle to prevent **"Assumption Leakage"** — where a half-baked agent thought accidentally becomes a hard constraint for another.

```
DRAFT  ──►  PROPOSED  ──►  ACCEPTED  ──►  DEPRECATED
(local)    (signaled)    (committed)    (superseded)
```

| State          | Description                                                                                     |
| -------------- | ----------------------------------------------------------------------------------------------- |
| **DRAFT**      | Local-only. Created during an LLM's thought phase. Not yet signaled.                            |
| **PROPOSED**   | Written to `.lore/intent.lock` via PRISM. Broadcasts intent to the swarm.                       |
| **ACCEPTED**   | Bound to a Git commit hash. Now part of the project's "Lineage of Truth."                       |
| **DEPRECATED** | Linked to a newer Decision UUID. Preserved for archeology; ignored by active context synthesis. |

---

## 4. PRISM: Soft-Locking Algorithm

PRISM solves the **"Agent Collision"** problem without a centralized coordinator.

**Signaling Flow:**

1. Agent generates a `SessionID` (UUID-v4)
2. Agent writes a JSON signal to `.lore/prism/<SessionID>.signal`
3. Before writing code, the agent reads all files in `.lore/prism/`
4. Overlap logic is applied:

| Condition                                               | Result                                               |
| ------------------------------------------------------- | ---------------------------------------------------- |
| `Signal_A.Path` intersects `Signal_B.Path` (glob match) | **Soft-Lock Warning** issued                         |
| `Signal_A.Assumption` contradicts `Signal_B.Decision`   | **Hard-Lock Block** — `git-lore commit` is prevented |

---

## 5. 3-Way Logical Merge (Reconciliation)

Standard Git merges text lines. Git-Lore merges **Belief Trees**.

**Algorithm: `Reconcile(Base, A, B)`**

1. **Atom Extraction** — Collect all UUIDs from `refs/lore` for both branches
2. **Divergence Detection** — Identify UUIDs present in A but not B (and vice versa) since the merge-base
3. **Contradiction Matrix** — Classify conflicts by entropy level:

| Conflict Type       | Example                                                               | Resolution                     |
| ------------------- | --------------------------------------------------------------------- | ------------------------------ |
| Type Conflict       | Branch A: `AuthMethod = OAuth` vs Branch B: `AuthMethod = SAML`       | **HIGH ENTROPY** — Block merge |
| Dependency Conflict | Branch A deletes a function; Branch B has a Lore Assumption it exists | **HIGH ENTROPY** — Block merge |
| Additive Merge      | Branch A adds Logging decision; Branch B adds Styling decision        | **LOW ENTROPY** — Auto-merge   |

---

## 6. Cognitive Entropy

Git-Lore measures the **"health" of a codebase's rationale**:

- **High Entropy** — Many active Assumptions, few finalized Decisions, or conflicting Intent Signals
- **Low Entropy** — High ratio of Decisions to Assumptions; all code changes linked to a specific Lore Atom

---

## 7. MCP Synthesis Loop

When an LLM requests context, the Rust engine performs:

1. **Tree-Sitter Scan** — Identifies the active function/class at the cursor
2. **Lore-Blame** — Traverses the Git DAG to find the last 5 Lore-Decision trailers affecting that scope
3. **Prompt Compression** — Strips metadata, delivering a concise constraint list to the agent:
   > _"Constraint: This function must remain synchronous due to Decision [ID-502]."_

---

## 8. Resolved Downsides

| Downside            | Resolution in v1.0                                                                                          |
| ------------------- | ----------------------------------------------------------------------------------------------------------- |
| Repository Bloat    | **Refs-Shadowing** — Lore atoms stored in compressed binary format in `refs/lore` or Git Notes              |
| Review Overhead     | **Semantic Summarization** — CLI generates "Reasoning Diffs" highlighting only logical contradictions       |
| Hallucination/Trust | **Constraint Verification** — Decisions linked to Validation Scripts; violations trigger pre-commit failure |
| Security Risks      | **Auto-Sanitization Hook** — Regex-based Lore-Scrubber prevents API keys or PII from entering Git history   |
| Small Model Failure | **Standardized Intent Atoms** — Strictly typed JSON/TOML schema forces structure on smaller models          |

---

## 9. Use Cases

### Case 1: Assumption Drift (Multi-Agent Swarms)

**Scenario:** Agent A renames a database column but hasn't committed. Agent B writes code assuming the old name.

- **Standard Git:** Conflict only appears at end-of-day integration.
- **Git-Lore:** Agent A's PRISM signal warns Agent B immediately — _"Warning: `users.id` is marked for Rename-Decision [ID-99]."_

### Case 2: Legacy Archeology

**Scenario:** A developer fixes a bug in a module written 3 years ago. The original author is gone.

- **Standard Git:** `git blame` shows _who_ changed a line, not _why_.
- **Git-Lore:** `git lore explain <file>` traverses `refs/lore` history and synthesizes: _"This module uses a recursive approach because Decision [ID-12] in 2021 prioritized memory over CPU due to IoT hardware constraints."_

### Case 3: Hallucination Guardrail

**Scenario:** An LLM agent suggests `Axum`, a library the team previously rejected for security reasons.

- **Standard Git:** The LLM proceeds; a human must catch it in code review.
- **Git-Lore:** The `pre-commit` hook detects `Axum` is blacklisted in Decision [ID-50] and rejects the commit: _"Error: Intent violates Decision [ID-50]. Use Actix instead."_

### Case 4: Legacy Refactor (Multi-Agent Swarm)

**Scenario:** A swarm migrates a 10-year-old PHP monolith to Rust. The lead agent assumes MySQL. A human developer made a shadow decision in 2019 to use a Postgres extension for GIS data.

- **Git-Lore:** `git lore context` surfaces Lore-Decision Atom `GIS-001` from 2019: _"Tech: Postgres — Reason: PostGIS required for spatial queries."_ The agent pivots before a single line of code is written.

### Case 5: Change-File Awareness

**Scenario:** An agent opens a file after a recent change and needs to understand what changed, why it changed, and which lore atoms are now relevant before making another edit.

- **Standard Git:** The agent sees the diff, but not the reasoning behind the change or the downstream constraints it introduced.
- **Git-Lore:** `git lore context <file>` combines the change file, recent commit trailers, and scope-specific lore so the agent can infer the active intent before editing: _"This file changed to support streaming uploads, so avoid synchronous buffering and preserve the new validation path."_

---

## 10. Rust Implementation

### Crate Stack

```toml
[dependencies]
# CLI and Error Handling
clap = { version = "4.0", features = ["derive"] }
anyhow = "1.0"
thiserror = "1.0"

# Git Integration
git2 = "0.18"          # Low-level libgit2 bindings

# Performance & Parsing
tree-sitter = "0.20"   # AST-aware context extraction
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Async & MCP Server
tokio = { version = "1.0", features = ["full"] }
fastmcp = "0.1"        # Fast Model Context Protocol implementation
```

### Module Structure

| Module        | Responsibility                                                  |
| ------------- | --------------------------------------------------------------- |
| `src/cli/`    | Command-line interface (`git lore init`, `git lore checkpoint`) |
| `src/git/`    | `git2` wrappers for commit trailers and custom refs             |
| `src/lore/`   | Core logic for Decision and Assumption objects                  |
| `src/mcp/`    | MCP server implementation for IDE integration                   |
| `src/parser/` | Tree-sitter logic to map Lore Atoms to functions/classes        |

---

## 11. Installation & Setup

### Build from Source

```bash
git clone https://github.com/your-repo/git-lore
cd git-lore
cargo build --release
cp target/release/git-lore /usr/local/bin/
```

### Install via Cargo

```bash
cargo install git-lore
```

### Initialize a Repository

```bash
git lore init
# Creates .lore/ and configures git hooks automatically
```

### Basic Workflow

```bash
# Record a decision
git lore decision "Use SQLite for local caching"

# Record an assumption
git lore mark "Switching to Memcached for sub-ms latency"

# Check active constraints
git lore status

# Commit with lore bundled automatically
git commit -m "feat: add cache"

# Generate a prompt-ready context summary for your LLM
git lore context

# Explain the rationale behind a file
git lore explain <file>
```
