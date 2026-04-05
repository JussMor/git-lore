# git-lore sync

## Description

Ensures that the hot-workspace `.lore/` folder reconciles with the cold-storage of Git branch references (`refs/lore`) without overriding already-resolved active state.

Makes the tool idempotent on branches by reconciling the active intention layer alongside the committed history.

## Usage

`git-lore sync [PATH]`
