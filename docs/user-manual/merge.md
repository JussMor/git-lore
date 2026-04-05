# git-lore merge

## Description

The underlying reconciliator invoked automatically by Git during a branch merge.

Rather than conflicting standard JSON texts, it deduplicates `atom.id`s and merges overlapping rules by checking which lifecycle takes priority (e.g., `Accepted` overwrites `Proposed`).

## Usage

`git-lore merge <BASE> <CURRENT> <OTHER>`

_Note: You rarely invoke this manually. It is hooked by Git itself after running `git-lore install`._
