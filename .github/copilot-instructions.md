# Git-Lore Project Guidelines

## Code Style

- Keep Rust code small, explicit, and testable.
- Prefer clear domain types in `src/lore` over ad hoc strings.
- Keep CLI parsing in `src/cli` and persistence logic in `src/lore` or `src/git`.

## Architecture

- `src/lore` owns lore atoms, lifecycle state, checkpoints, and workspace state.
- `src/git` owns Git-facing rendering and later commit/ref integration.
- `src/cli` is the user-facing command layer and should stay thin.
- `src/mcp` will expose context and proposal tools once the core model is stable.
- `src/parser` will handle scope discovery and code-location awareness.

## Build and Test

- Run `cargo check` after structural changes.
- Run `cargo test` after changes to lore state, checkpointing, or serialization.

## Conventions

- Keep `.lore` as the local working state and reserve `refs/lore` for later historical storage.
- Document new protocol surfaces in `docs/` before wiring them into code.
- Prefer incremental implementation slices with one end-to-end path at a time.
- After each completed implementation slice, update `docs/next-steps.md` and append a short note to `docs/progress-log.md`.

# Git-Lore Project Guidelines

## Code Style

- Keep Rust code small, explicit, and testable.
- Prefer clear domain types in `src/lore` over ad hoc strings.
- Keep CLI parsing in `src/cli` and persistence logic in `src/lore` or `src/git`.

## Architecture

- `src/lore` owns lore atoms, lifecycle state, checkpoints, and workspace state.
- `src/git` owns Git-facing rendering and later commit/ref integration.
- `src/cli` is the user-facing command layer and should stay thin.
- `src/mcp` will expose context and proposal tools once the core model is stable.
- `src/parser` will handle scope discovery and code-location awareness.

## Build and Test

- Run `cargo check` after structural changes.
- Run `cargo test` after changes to lore state, checkpointing, or serialization.

## Conventions

- Keep `.lore` as the local working state and reserve `refs/lore` for later historical storage.
- Document new protocol surfaces in `docs/` before wiring them into code.
- Prefer incremental implementation slices with one end-to-end path at a time.
