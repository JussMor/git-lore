# git-lore checkpoint

## Description

Generates a cryptographically strong snapshot of the `.lore` workspace state at this exact moment.

Checkpoints are essential for strict _State-First_ operations. Multi-agent flows and cross-branch PRs use checkpoints to prevent race conditions and ensure they are operating on the freshest context.

## Usage

`git-lore checkpoint --message <MSG> [PATH]`

## Options

- `--message <String>`: A brief comment outlining why the checkpoint was frozen (e.g., "Pre-refactor state").
- `[PATH]`: Optional workspace root path.
