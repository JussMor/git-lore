# Glore UI/UX Design Spec (GitKraken-Like)

## Purpose

Design a desktop app that gives full visual and operational coverage of Git-Lore.

The app must let a user:

1. Select any project folder.
2. Discover and inspect all lore state and history.
3. Execute every CLI capability from the UI.
4. Understand outcomes through strong visual feedback (graphs, badges, diff views, timelines).

This document is written as a handoff spec for another implementation tool.

## Product Goals

1. Full feature parity with git-lore CLI commands.
2. Fast read experience (status, context, explain) with low friction.
3. Safe write experience (mark, propose, set-state, resolve, commit, sync) with clear preflight and confirmation.
4. Visual-first reasoning, similar to GitKraken, but focused on lore atoms and rationale state.

## Primary Users

1. Human developers managing architecture rationale.
2. AI-assisted developers who need MCP and preflight visibility.
3. Reviewers and leads validating contradictions, entropy, and state transitions.

## Core Layout (GitKraken-Inspired)

### Shell

1. Left rail: Navigation modules.
2. Main center: Graph or table workspace.
3. Right inspector: Details for selected atom/signal/conflict/command result.
4. Bottom panel: Activity log, command output, errors.
5. Top bar: Project path selector, global command palette, quick actions.

### Navigation Modules

1. Overview
2. Atoms
3. Context/Explain
4. PRISM Signals
5. Checkpoints/Commits
6. Validation/Entropy
7. Merge/Resolve
8. Integrations (MCP, install, generate)
9. Command Console (advanced)

## Feature Parity Matrix (CLI -> UI)

The app must expose every command below.

| CLI Command         | UI Surface               | Primary Interaction                           | Visualization                                   |
| ------------------- | ------------------------ | --------------------------------------------- | ----------------------------------------------- |
| git-lore init       | Project Setup Wizard     | Initialize .lore in selected path             | Setup checklist + success banner                |
| git-lore mark       | Atoms > New Atom Form    | Create proposed atom                          | Form + atom card preview                        |
| git-lore status     | Overview                 | Refresh workspace status                      | KPI cards + entropy gauge + contradictions list |
| git-lore checkpoint | Checkpoints/Commits      | Create checkpoint with optional message       | Timeline node with trailer preview              |
| git-lore commit     | Checkpoints/Commits      | Commit lore state to Git                      | Commit graph node + trailer panel               |
| git-lore signal     | PRISM Signals            | Broadcast signal with path globs              | Signal timeline + overlap warnings              |
| git-lore context    | Context/Explain          | Select file + optional cursor line            | Scoped constraints panel                        |
| git-lore propose    | Atoms > Propose Form     | Propose atom for file/scope                   | Proposed badge + scope highlight                |
| git-lore mcp        | Integrations > MCP       | Start/stop MCP server                         | Server status + tool activity feed              |
| git-lore explain    | Context/Explain          | Show rationale with history                   | Historical decision timeline                    |
| git-lore validate   | Validation/Entropy       | Run validation                                | Pass/fail board + issue table                   |
| git-lore sync       | Checkpoints/Commits      | Sync from git history                         | Sync report + added/updated atom chips          |
| git-lore install    | Integrations > Git Hooks | Install hooks + merge driver                  | Integration status card                         |
| git-lore merge      | Merge/Resolve            | Run or inspect merge reconciliation           | 3-way conflict viewer                           |
| git-lore set-state  | Atoms > Lifecycle        | Transition atom state with reason             | State transition timeline                       |
| git-lore generate   | Integrations > AI Skill  | Generate integration skill file               | Preview + destination file summary              |
| git-lore resolve    | Merge/Resolve            | Resolve contradictions by picking winner atom | Conflict matrix + winner action bar             |

## Key Screens

### 1. Project Picker + Workspace Boot

1. Choose directory button opens native folder picker.
2. If .lore exists: load workspace state.
3. If .lore missing: offer Initialize Workspace action.
4. Persist recent projects list.

### 2. Overview Dashboard

1. Total atoms.
2. State distribution (Draft, Proposed, Accepted, Deprecated).
3. Entropy score out of 100.
4. Contradiction count and top items.
5. Last checkpoint and last commit hash.

### 3. Atoms Workbench

1. Table/kanban toggle.
2. Filters: state, kind, path, scope, author, date.
3. New atom form (Mark) and propose form (Propose).
4. State transition controls with reason and actor.
5. Right panel shows full atom metadata and body.

### 4. Context/Explain Explorer

1. File picker inside selected workspace.
2. Optional cursor line input.
3. Context tab: constraints and relevant atoms.
4. Explain tab: historical decisions and trailers.
5. Scope badge with start/end lines.

### 5. PRISM Signals Board

1. Active signals list.
2. TTL countdown for stale pruning.
3. Overlap warnings grouped by path/scope.
4. Signal creation form with assumptions and decision.

### 6. Checkpoints and Commits

1. Checkpoint creation with message.
2. Commit form with message and allow-empty toggle.
3. Commit trailer preview before execute.
4. Timeline linking checkpoint ID to commit hash.

### 7. Validation and Entropy Center

1. Run validate action.
2. Issues table with severity and recommendation.
3. Entropy notes and contradiction feed.
4. Trend mini-chart for future entropy evolution.

### 8. Merge and Resolve Studio

1. 3-way merge inspector (base/current/other).
2. Conflict list by location key path::scope.
3. Winner picker and resolution reason.
4. One-click resolve action to deprecate losers and accept winner.

### 9. Integrations Panel

1. Git integration status (hooks and merge driver).
2. MCP server controls and tool telemetry.
3. Generate skill output destination and preview.
4. Install/reinstall actions with output log.

### 10. Command Console (Power User)

1. Render exact command preview before execution.
2. Advanced flags editor for each command.
3. Output parser for success/warn/error blocks.
4. History of executed commands.

## Visualization System

### Lore Graph

1. Node = atom.
2. Color by state:
   - Draft: gray
   - Proposed: blue
   - Accepted: green
   - Deprecated: amber/red
3. Shape by kind:
   - Decision: rectangle
   - Assumption: pill
   - OpenQuestion: diamond
   - Signal: dashed capsule
4. Edge ideas:
   - Same path adjacency.
   - Same scope grouping.
   - Temporal sequence by created timestamp.

### Status Visuals

1. Entropy gauge (0-100).
2. Contradiction heatmap by file path.
3. Atom lifecycle stacked bar chart.

### Commit/Checkpoint Timeline

1. Horizontal time axis.
2. Checkpoint markers with messages.
3. Commit markers with lore trailers.
4. Click marker to open full details in right inspector.

### PRISM Conflict Visual

1. Swimlanes by agent/session.
2. Path overlap ribbons.
3. Soft-lock warnings as amber annotations.

### Merge Conflict Visual

1. Tri-pane view (base/current/other).
2. Conflict rows grouped by location.
3. Winner badge with action log.

## Interaction and Safety Rules

1. All mutating actions must show preflight outcome before final execute.
2. Any blocked preflight must disable submit and show fix guidance.
3. State transitions require reason field.
4. Destructive actions require confirmation modal.
5. Every command execution writes to Activity Log with timestamp.

## Information Architecture and Data Contracts

### Core Frontend Types

1. WorkspaceSnapshot: root, atoms.
2. LoreAtom: id, kind, state, title, body, scope, path, created_unix_seconds, validation_script.
3. ValidationIssue: severity, code, message, atom_ids.
4. PrismSignal: session_id, agent, scope, paths, assumptions, decision, created timestamp.
5. MergeConflict: kind, key, message.

### Command Adapter Strategy

Use a unified command adapter in backend:

1. Structured commands for common reads and writes.
2. Raw command runner fallback for advanced flags.
3. Standard response envelope:
   - ok: boolean
   - stdout: string
   - stderr: string
   - parsed: optional structured payload

## Proposed Backend API Surface (for Tauri)

1. choose_project (frontend native dialog)
2. load_workspace(path)
3. run_init(path)
4. run_status(path)
5. run_mark(payload)
6. run_checkpoint(payload)
7. run_commit(payload)
8. run_signal(payload)
9. run_context(payload)
10. run_propose(payload)
11. run_explain(payload)
12. run_validate(path)
13. run_sync(path)
14. run_install(path)
15. run_merge(payload)
16. run_set_state(payload)
17. run_generate(payload)
18. run_resolve(payload)
19. mcp_start(path)
20. mcp_stop()

## UX Writing and Feedback Patterns

1. Success: clear one-line summary plus details drawer.
2. Warning: actionable next step (not only error text).
3. Error: include command preview, stderr, and retry action.
4. Empty states: explain what to do next (choose project, init, or run sync).

## MVP Scope (Phase 1)

1. Project picker.
2. Overview dashboard.
3. Atoms list + detail inspector.
4. Context and Explain panel.
5. Mark, Propose, Set-State actions.
6. Validate and Status readouts.

## Phase 2

1. Checkpoint and Commit timeline.
2. PRISM signals board.
3. Sync and Install integration controls.
4. Generate skill action.

## Phase 3

1. Merge/Resolve studio.
2. MCP runtime telemetry dashboard.
3. Advanced graph analytics (entropy trend, contradiction clusters).

## Acceptance Criteria

1. Every CLI command listed in this spec is reachable from UI.
2. User can execute all commands without leaving the app.
3. User can visualize current lore state, history, conflicts, and validation results.
4. User can switch project path at runtime and load another workspace.
5. All mutating actions expose preflight status and final command output.

## Design Direction Notes

1. Keep a dark, high-contrast desktop-first style similar to GitKraken.
2. Use dense information layout with collapsible inspectors.
3. Prefer clear command-result traceability over decorative UI.
4. Prioritize legibility and workflow speed for engineering users.
