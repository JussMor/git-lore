# Progress Log

This file records completed implementation slices and the next handoff point.

## 2026-04-04

- Added the initial Rust CLI scaffold, local `.lore` workspace state, checkpoint persistence, and commit trailer rendering.
- Added workspace guidance in `.github/copilot-instructions.md` and a reusable architecture agent in `.github/agents/architecture.agent.md`.
- Added this log so each completed slice can leave behind a short durable note.
- Wired `src/git` to discover repository roots through the git CLI and to build/parse commit-ready messages with lore trailers.
- Wired the CLI to create actual Git commits from lore messages and to accept active atoms after a successful commit.
- Added PRISM soft-lock signaling with JSON signal files and overlap warnings from `git-lore signal`.
- Added Tree-sitter scope detection for Rust files and an MCP-style context/proposal service built on top of parser and lore state.
- Added accepted decision storage under `.lore/refs/lore/accepted` with source-commit provenance.
- Added 3-way merge reconciliation for lore atoms across base, left, and right states.
- Added entropy scoring and contradiction reporting for workspace state and merge outcomes.
- Added JavaScript and TypeScript Tree-sitter scope detection plus MCP integration tests for those languages.
- Added stdio MCP transport wiring with `git_lore_context` and `git_lore_propose` tools, plus MCP tool-call tests.
- Added a compliance matrix that separates implemented protocol work from remaining Git-native spec gaps.
- Added Git-native `refs/lore` mirroring, history-backed `git-lore explain`, `git-lore validate`, `git-lore sync`, and `git-lore install`.
- Added PRISM hard-lock blocking, sanitization checks, and hook / merge-driver installation.
- Added per-atom validation scripts and gzip-compressed workspace record storage.
- Migrated the stdio MCP transport to `rmcp` 0.16 using a `ServerHandler` implementation and tokio-backed stdio runtime.
- Renamed MCP tool identifiers to `git_lore_context` and `git_lore_propose` for clients that only allow `[a-z0-9_-]` tool names.
- Fixed generated Git hooks from `git-lore install` to use positional path arguments, restoring working pre-commit validation and post-checkout sync.
- Added `docs/workflows.html`, an interactive local dashboard for Git-Lore workflows with live `.lore` folder loading, gzip-aware parsing, and state/checkpoint/signal visual panels.
- Expanded MCP tool coverage with `git_lore_state_snapshot` and `git_lore_memory_preflight`, including structured preflight severities and state checksum metadata for state-first workflows.
- Enforced strict `state-first` write safety in MCP transport by requiring fresh snapshot checksum and timestamp guard fields on `git_lore_propose`.
- Mirrored strict `state-first` preflight orchestration into CLI write commands so lore-mutating paths fail fast on blocking memory checks.
- Made `git-lore sync` idempotent by reconciling active lore with `refs/lore` using `atom.id` deduplication, and added regression tests for repeated sync runs and duplicate compaction.
- Added `git-lore merge` with base/current/other file reconciliation so configured Git merge-driver installation now targets an implemented CLI command.
- Added local `git_lore_memory_search` hybrid ranking, `git_lore_state_transition_preview`, and proposal autofill behavior, plus standardized MCP tool error payloads (`code`, `retryable`, `recommended_action`) for safer retry/degradation handling.
- Added explicit `git-lore set-state` transitions with audited reason/actor logging, and introduced PRISM stale-signal hygiene (ignore/prune stale signals in conflict and hard-lock checks).
- Added `docs/capabilities-ui.html`, a focused capabilities dashboard that can load `.test/.lore`, separates command views into "listar" and "explicar", and exposes all major CLI/MCP capabilities with practical command snippets.
- Added `docs/.test-lore-manifest.json` and auto-bootstrap logic so `docs/capabilities-ui.html` now self-configures from `.test/.lore` on page load with a fallback loader path.
- Polished the Glore atom details inspector with tighter spacing, smaller typography, and a cooler muted panel palette to better match the reference UI.
- Tightened the Atom Details typography further to an XS density, making titles, metadata, and action labels more concise.
- Switched the Atom Details panel to the neutral gray app-shell background so it matches the rest of the UI instead of the earlier blue-tinted surface.
- Updated the Atom Details "Open In VS Code" action to open the file alongside its parent folder so VS Code lands in the project context.
- Compactified the Atom Details Git Context rows and moved full lore-ref details into hover tooltips to reduce cut-off content.
- Stripped bracketed internal IDs from the visible Git Context row text and widened the rail padding so the status dot no longer clips.
- Reworked the Git Context rail into per-row connector segments so the tree can grow with proper start and finish handling.
