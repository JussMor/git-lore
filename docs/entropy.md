# Entropy Scoring

Git-Lore uses entropy scoring to estimate how much unresolved rationale still remains in the workspace.

## Current behavior

- Accepted decisions reduce entropy.
- Drafts, proposals, open questions, and signals increase entropy.
- Contradictions add additional weight.
- The `git-lore status` command prints the current entropy score and a contradiction summary.

## Contradiction reporting

- `TypeConflict` is reported when multiple rationale variants exist for the same location.
- `DependencyConflict` is reported when an atom is deprecated while another remains active at the same location.

## Intended use

- Give a quick read on how settled the workspace rationale is.
- Surface contradictions before they spread into merge or commit operations.
