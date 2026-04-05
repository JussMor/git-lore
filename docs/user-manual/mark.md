# git-lore mark

## Description

Creates a new Lore Atom, which is a structured record of a rule, decision, assumption, or architectural choice.

## Usage

`git-lore mark --title <TITLE> [OPTIONS]`

## Options

- `--title <String>`: The brief identifier or name of the rule (e.g., "Use Rust format").
- `--body <String>`: Explanatory text that provides context (the "Why").
- `--scope <String>`: The scope boundary, like a function name or class.
- `--path <PathBuf>`: The target directory or file this rule binds to.
- `--validation-script <String>`: A literal shell command to run during validation, for example `cargo test -p auth`.
- `--kind <LoreKindArg>`: The typology of the lore. Allowed values: `decision` (default), `assumption`, `open-question`, `signal`.

For `decision`, `assumption`, and `open-question`, you must provide at least one location anchor: `--path` or `--scope`.
