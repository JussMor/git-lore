# Merge Reconciliation

Git-Lore reconciles lore atoms across a 3-way base/left/right merge before code changes are finalized.

## Current behavior

- Merges are grouped by location using file path and scope.
- Identical changes on both sides are carried forward.
- Non-conflicting additions are merged additively.
- Divergent changes in the same location are reported as conflicts.
- The CLI now exposes `git-lore merge <base> <current> <other>` for Git merge-driver integration.

## Conflict kinds

- `TypeConflict`: both branches changed the same logical location in incompatible ways.
- `DependencyConflict`: one branch deprecated an atom while the other kept it active.

## Intended use

- Surface contradictory rationale before code merge time.
- Keep the reconciliation rule deterministic and testable.
- Allow Git merge-driver execution through `merge.lore.driver = git-lore merge %O %A %B`.
