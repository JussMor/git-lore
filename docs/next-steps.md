# Next Steps

This file tracks the immediate implementation order for Git-Lore.

## Working Rule

After each completed implementation slice, update this file and append a short note to [docs/progress-log.md](progress-log.md).

## Current Focus

1. Expand merge reconciliation heuristics for more complex dependency graphs.

## Recently Completed

- Updated `git-lore sync` to restore full accepted atom metadata from `refs/notes/lore` when available (including multiple atoms on the same commit), fallback to commit trailers when notes are missing, and required non-signal atoms to include `path` or `scope` anchors to reduce `<no-path>::<no-scope>` contradiction noise.

- Fixed the remote installer to call `cargo install --git ... --package git-lore` so it no longer collides with the Glore binary package.

- Made `git-lore sync` preserve existing active atom state instead of re-promoting matching refs, and made Glore Refresh reload the full workspace snapshot.

- Clarified `validation_script` as a literal shell command, blocked narrative text before execution, and added regression tests for the preflight path.

- Added a live workspace change bridge from the Tauri backend to the Glore graph so `.lore/active_intent.json` updates can reload the view and pulse changed atoms.
- Polished the Glore atom details inspector with tighter spacing, smaller typography, and a cooler muted panel palette to better match the reference UI.
- Tightened the Atom Details typography further to an XS density, making titles, metadata, and action labels more concise.
- Switched the Atom Details panel to the neutral gray app-shell background so it matches the rest of the UI instead of the earlier blue-tinted surface.
- Updated the Atom Details "Open In VS Code" action to open the file alongside its parent folder so VS Code lands in the project context.
- Compactified the Atom Details Git Context rows and moved full lore-ref details into hover tooltips to reduce cut-off content.
- Stripped bracketed internal IDs from the visible Git Context row text and widened the rail padding so the status dot no longer clips.
- Reworked the Git Context rail into per-row connector segments so the tree can grow with proper start and finish handling.
- Made the Atom Details lifecycle transition selector state-aware so it only shows legal targets for the selected atom.
- Changed the lifecycle transition selector to start blank per atom and require an explicit user choice instead of auto-preselecting the only option.
- Auto-configured `docs/capabilities-ui.html` to bootstrap from `.test/.lore` via `docs/.test-lore-manifest.json`, with graceful fallback to active-intent-only loading.
- Added `docs/capabilities-ui.html`, a dedicated UI that uses `.test/.lore` data, separates capabilities into "listar" and "explicar" views, and documents CLI/MCP command usage by phase.
- Added local `git_lore_memory_search` hybrid ranking, `git_lore_state_transition_preview`, proposal autofill support, PRISM stale-signal hygiene, and structured MCP error payload codes for retry/degradation behavior.
- Added explicit `git-lore set-state` transitions with audited reason/actor logging in `.lore/audit/state_transitions.jsonl`.
- Aligned merge-driver installation with CLI capabilities by adding `git-lore merge <base> <current> <other>` for Git merge-driver integration.
- Made `git-lore sync` idempotent by deduplicating active lore atoms by `atom.id` and compacting duplicate state entries during sync reconciliation.
- Mirrored strict `state-first` enforcement into CLI write paths so `mark`, `signal`, `propose`, `checkpoint`, `commit`, and `sync` now run preflight guards before mutating lore state.
- Enforced strict MCP `state-first` guards on write calls: `git_lore_propose` now requires fresh `state_checksum` and `snapshot_generated_unix_seconds` and rejects stale or mismatched state.
- Expanded MCP coverage with `git_lore_state_snapshot` and `git_lore_memory_preflight`, including structured block/warn/info preflight issues and workspace state checksums.
- Added an interactive `docs/workflows.html` dashboard that visualizes Git-Lore workflows and loads real `.lore` workspace data from local folders.
- Fixed `git-lore install` hook scripts to use positional path arguments (`git-lore validate .` and `git-lore sync .`) so hook-driven workflows run successfully.
- Renamed MCP tool IDs to `git_lore_context` and `git_lore_propose` so clients that reject dots in tool names can invoke them cleanly.
- Migrated the stdio MCP server transport to `rmcp` 0.16 with a `ServerHandler`-based runtime while preserving `git_lore_context` and `git_lore_propose` behavior.
- Wired `src/git` to discover repository roots through the git CLI, build commit-ready messages with lore trailers, and create actual commits from the CLI.
- Added PRISM soft-lock signaling with `.lore/prism/<session-id>.signal` files and conservative overlap warnings.
- Added Tree-sitter scope detection for Rust source files and a context/proposal service for active lore guidance.
- Added accepted decision storage under `.lore/refs/lore/accepted` with commit provenance.
- Added 3-way merge reconciliation for lore atoms across base, left, and right states.
- Added entropy scoring and contradiction reporting for workspace state and merge outcomes.
- Added JavaScript and TypeScript Tree-sitter scope detection plus MCP integration tests for those languages.
- Added a compliance matrix so spec coverage is explicitly tracked instead of implied.
- Added Git-native `refs/lore` mirroring, history-backed `git-lore explain`, `git-lore validate`, `git-lore sync`, and `git-lore install`.
- Added PRISM hard-lock blocking, sanitization scanning, and Git hook / merge-driver installation.
- Added per-atom validation scripts and gzip-compressed workspace record storage.

## Medium-Term Work

1. Add entropy trend tracking across checkpoints.
2. Expand merge CLI conflict handling to support richer manual-resolution workflows.
3. Add an MCP write tool for audited state transitions (currently preview is MCP and apply is CLI).

## Validation Targets

1. End-to-end checkpoint-to-commit flow.
2. PRISM conflict detection across two active sessions.
3. Lore extraction from a sample repository.
4. MCP context lookup for a scoped function or file.
5. `git-lore install` followed by hook-driven validation and sync in a temp Git repo.
