# git-lore set-state

## Description

Alters the lifecycle state of an existing Lore Atom.

Git-Lore uses an explicit state machine for its knowledge base. A single rule can evolve over its lifetime.

## Usage

`git-lore set-state --atom-id <ATOM_ID> --state <STATE> --reason <REASON> [OPTIONS]`

## Options

- `--state <STATE>`: The target lifecycle phase (`draft`, `proposed`, `accepted`, `deprecated`).
- `--reason <String>`: The explanation tracing _why_ the rule was elevated or retired (e.g., "Approved in PR #432").
- `--actor <String>`: Identifies who enacted the change.
