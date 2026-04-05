# MCP

This document describes the intended Model Context Protocol surface for Git-Lore.

## Intended behavior

- Expose repository context to IDEs and agents.
- Retrieve the latest scope-relevant decisions.
- Accept proposed lore atoms before edits are made.

## Current implementation

- The `McpService` context flow reads workspace lore, detects scope with Tree-sitter, and returns a compact constraint list.
- The parser supports Rust, JavaScript, and TypeScript scope detection, so context lookup follows the active file language automatically.
- The proposal flow records a proposed lore atom using detected scope and file path metadata.
- The state snapshot flow returns workspace-level metadata (`state_checksum`, atom counts, accepted-record counts) to support `state-first` write guards.
- The memory preflight flow aggregates blocking and warning conditions (sanitization, PRISM hard-locks, validation scripts, duplicate atom IDs, entropy signals) before write operations.
- The memory search flow returns ranked lore hits using hybrid local scoring (lexical match + recency + state weighting + path/scope proximity).
- The state transition preview flow reports whether a requested atom transition is policy-allowed before a write command executes.
- Write tools now enforce strict `state-first`: they require a fresh `state_checksum` + `snapshot_generated_unix_seconds` pair from `git_lore_state_snapshot` and reject stale/mismatched state.
- Proposal payload autofill is implemented for MCP writes: missing `title`, `body`, and `scope` can be auto-generated when `autofill=true`.
- MCP tool errors now return structured payloads with `code`, `message`, `retryable`, and optional `recommended_action`.

## Transport

- Run the stdio MCP server with `git-lore mcp <repo>`.
- In this workspace, use `.test` as the throwaway repo for command testing (`git-lore mcp .test`).
- The server exposes six tools: `git_lore_context`, `git_lore_propose`, `git_lore_state_snapshot`, `git_lore_memory_preflight`, `git_lore_memory_search`, and `git_lore_state_transition_preview`.
- `git_lore_context` accepts `file_path` and optional `cursor_line`.
- `git_lore_propose` accepts `file_path`, optional `cursor_line`, `kind`, optional `title`, optional `body`, optional `scope`, optional `autofill`, plus required guard fields: `state_checksum` and `snapshot_generated_unix_seconds`.
- `git_lore_state_snapshot` accepts no required parameters and returns current state metadata for workflow guards.
- `git_lore_memory_preflight` accepts `operation` (`edit`, `commit`, or `sync`) and returns structured `block|warn|info` issues plus `can_proceed`.
- `git_lore_memory_search` accepts `query`, optional `file_path`, optional `cursor_line`, and `limit`.
- `git_lore_state_transition_preview` accepts `atom_id` and `target_state` (`draft`, `proposed`, `accepted`, `deprecated`).

## How to use

1. For local testing in this repo, use `.test` as the repository root Git-Lore should inspect.
2. Point your MCP client, editor, or agent runner at the `git-lore mcp` command.
3. Call `git_lore_state_snapshot` before write operations to get a fresh state reference.
4. Call `git_lore_memory_preflight` with `operation=edit` before proposing or editing.
5. Call `git_lore_context` and `git_lore_memory_search` to gather scope-aware and semantic memory context.
6. Call `git_lore_state_transition_preview` before changing atom states via CLI workflows.
7. Call `git_lore_propose` after preflight passes and include the snapshot guard fields; use `autofill=true` to complete missing proposal fields.
