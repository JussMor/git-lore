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
- `--validation-script <String>`: An optional automation script/regex for validators.
- `--kind <LoreKindArg>`: The typology of the lore. Allowed values: `decision` (default), `assumption`, `open-question`, `signal`.
