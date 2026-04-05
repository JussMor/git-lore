# Commit Trailers

This document describes how Git-Lore will attach lore atoms to commits.

## Intended behavior

- Convert proposed and accepted lore atoms into Git commit trailers.
- Keep the trailer format stable so future tools can parse it reliably.
- Use the checkpoint step to preview the trailer block before commit time.

## Planned trailer keys

- `Lore-Decision`
- `Lore-Assumption`
- `Lore-Open-Question`
- `Lore-Signal`
