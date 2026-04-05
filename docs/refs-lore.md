# refs/lore

Git-Lore stores accepted decisions in the workspace-side `refs/lore` area for now.

## Intended behavior

- Preserve accepted decisions in Git-addressable history.
- Make lore retrievable without loading the entire working state.
- Serve as the cold-storage layer behind the active `.lore` workspace.

## Current implementation

- Accepted decisions are written to `.lore/refs/lore/accepted/<atom-id>.json`.
- Each record stores the accepted atom, the acceptance timestamp, and the source commit hash when available.
- The active workspace state remains in `.lore/active_intent.json`, so cold storage and hot state can evolve independently.
