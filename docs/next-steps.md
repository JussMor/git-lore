# Next Steps

This file tracks the immediate implementation order for Git-Lore.

## Working Rule

After each completed implementation slice, update this file and append a short note to [docs/progress-log.md](progress-log.md).

## Current Focus

1. Expand merge reconciliation heuristics for more complex dependency graphs.

## Recently Completed

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
