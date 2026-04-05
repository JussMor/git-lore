# git-lore context

## Description

Fetches the active Lore Constraints and Historical Decisions affecting a specific file.

By using Tree-sitter and Git `log` history, this command connects the dots between a file's location, the functions inside it, and the past Git commits that tied Lore Atoms to it.

## Usage

`git-lore context --file <FILE> [OPTIONS]`

## Options

- `--file <PathBuf>`: The target script/file.
- `--cursor-line <usize>`: Line number (to drill down via AST/Tree-sitter scope).
