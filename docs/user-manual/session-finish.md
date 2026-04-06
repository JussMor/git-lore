# git-lore session-finish

## Description

Finishes an operational session in one command by composing the required closeout sequence:

- validates lore and guard conditions
- creates a lore-linked Git commit
- syncs lore state from Git history
- writes a post-sync checkpoint with commit trace
- releases the session PRISM signal

If a step fails before release, the PRISM signal remains active so the session can be retried safely.

## Usage

`git-lore session-finish --session-id <SESSION_ID> --message <MESSAGE> [OPTIONS]`

## Options

- `--session-id <String>`: Required session ID emitted by `session-start`.
- `--message <String>`: Commit subject.
- `--allow-empty`: Allow commits with no file changes (defaults to true).
- `--agent <String>`: Optional owner for post-sync checkpoint metadata.
- `--reason <String>`: Optional reason for post-sync checkpoint metadata.
- `--checkpoint-message <String>`: Optional explicit post-sync checkpoint message override.
- `[WORKSPACE]`: Optional workspace root path. Defaults to `.`.
