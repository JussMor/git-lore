# PRISM

PRISM is the intent signaling layer.

## Intended behavior

- Create lightweight per-session intent signals.
- Detect overlapping file or module activity before code changes begin.
- Distinguish soft-lock warnings from hard-lock blocks.

## Current implementation

- Signals are written as JSON into `.lore/prism/<session-id>.signal`.
- Each signal records a session id, optional agent name, optional scope, path globs, assumptions, and an optional decision.
- The `git-lore signal` command writes a signal, scans existing signals, and prints soft-lock warnings for overlapping path globs.

## Notes

- Overlap detection is conservative and designed to warn early.
- Hard-lock contradiction handling is still deferred.
