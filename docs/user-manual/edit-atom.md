# git-lore edit-atom

## Description

Edits an existing lore atom in-place, without creating a replacement atom.

Use this command to correct metadata (title/body/scope/path/validation script/kind) and to close accepted trace linkage with a real commit SHA.

All edits are audited in `.lore/audit/atom_edits.jsonl` with reason and actor.

## Usage

`git-lore edit-atom --atom-id <ATOM_ID> --reason <REASON> [OPTIONS]`

## Options

- `--atom-id <ATOM_ID>`: Required atom identifier.
- `--reason <REASON>`: Required audit reason for the in-place edit.
- `--actor <ACTOR>`: Optional actor identity.
- `--kind <KIND>`: Optional new kind (`decision|assumption|open-question|signal`).
- `--title <TITLE>`: Optional new title.
- `--body <BODY>`: Optional new body.
- `--clear-body`: Clear body.
- `--scope <SCOPE>`: Optional new scope anchor.
- `--clear-scope`: Clear scope anchor.
- `--atom-path <PATH>`: Optional new file path anchor.
- `--clear-atom-path`: Clear path anchor.
- `--validation-script <SCRIPT>`: Optional new validation command.
- `--clear-validation-script`: Clear validation script.
- `--trace-commit-sha <SHA>`: Set accepted trace commit SHA.
- `--clear-trace-commit`: Clear accepted trace commit SHA.
- `[PATH]`: Optional workspace root path. Defaults to `.`.

## Notes

- `--trace-commit-sha` and `--clear-trace-commit` only apply to atoms already in `accepted` state.
- For non-`signal` atoms, at least one anchor must remain (`scope` or `atom-path`).
- You cannot pass a set flag and its clear flag at the same time (for example `--body` with `--clear-body`).
