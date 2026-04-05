# git-lore commit

## Description

Integrates your Git branch with Git-Lore.

It triggers a `git commit` under the hood, but appends all the active/pending Lore atoms as `Git Trailers` (e.g., `Lore-Decision: [id] message`) directly into the Git commit message. This effectively glues the "Why" (the Lore) to the "What" (the changed files in Git's object tree).

## Usage

`git-lore commit --message <MESSAGE> [OPTIONS]`

## Options

- `--message <MESSAGE>`: Your commit subject.
- `--allow-empty`: Permits the creation of a commit with no file changes (useful for pure Lore state migrations).
