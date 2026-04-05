# git-lore propose

## Description

Creates a formalized Proposal for a new Lore Atom, acting as the standard gateway for Agents (via MCP) or CI bots.

Unlike `signal`, this persists the rule to `.lore/atoms/` within the `Proposed` state, ensuring it enters the review cycle.

## Usage

`git-lore propose --title <TITLE> --file <PathBuf> [OPTIONS]`

## Options

Similar to `git-lore mark` (needs `title`, `body`, `kind`, `cursor-line`, etc.).
