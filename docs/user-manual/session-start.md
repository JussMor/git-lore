# git-lore session-start

## Description

Starts an operational session in one command by composing two required controls:

- emits a PRISM signal (soft-lock)
- writes a pre-write checkpoint for evidence

This command is designed to reduce manual setup steps while preserving state-first safety checks.

## Usage

`git-lore session-start [OPTIONS]`

## Options

- `--session-id <String>`: Optional session identifier (auto-generated if omitted).
- `--agent <String>`: Agent or operator name.
- `--scope <String>`: Optional scope label.
- `--path <GLOB>`: Affected paths (repeatable). Defaults to `.` if omitted.
- `--assumption <String>`: Session assumptions (repeatable).
- `--decision <String>`: Tentative session goal/decision.
- `--reason <String>`: Optional reason included in auto-generated checkpoint message.
- `--checkpoint-message <String>`: Optional explicit checkpoint message override.
- `[WORKSPACE]`: Optional workspace root path. Defaults to `.`.

## Typical Flow

1. `git-lore session-start --agent EB-GitLore --path "src/components/layout/default-layout.tsx"`
2. `git-lore propose --file src/components/layout/default-layout.tsx --title "..."`
3. `git-lore session-finish --session-id <SESSION_ID> --message "feat(...): ..."`
