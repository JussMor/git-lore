# Unified Spec Compliance Matrix

This matrix records what is implemented versus what remains pending in the unified specification.

## Implemented

| Requirement                                | Status | Notes                                                                                 |
| ------------------------------------------ | ------ | ------------------------------------------------------------------------------------- |
| Local-first Rust core                      | Done   | The tool runs locally and the core implementation is Rust.                            |
| Lore atom lifecycle                        | Done   | Draft, Proposed, Accepted, and Deprecated states exist.                               |
| PRISM soft-lock signaling                  | Done   | `.lore/prism/<session-id>.signal` files are written and scanned for overlap warnings. |
| Commit trailer persistence                 | Done   | Lore atoms are rendered into commit trailer blocks and committed through Git.         |
| MCP context/proposal flow                  | Done   | `git-lore mcp` exposes `git_lore_context` and `git_lore_propose`.                     |
| Rust/JavaScript/TypeScript scope detection | Done   | Tree-sitter scope detection is implemented for those languages.                       |
| Accepted lore storage                      | Done   | Accepted atoms are stored under `.lore/refs/lore/accepted`.                           |
| Git-native shadow refs                     | Done   | `refs/lore/accepted/*` refs are written alongside note-backed metadata.               |
| Lore-blame synthesis                       | Done   | `git-lore explain` and MCP context now surface recent Git-history decisions.          |
| 3-way logical merge                        | Done   | Base/left/right reconciliation and conflict classification are implemented.           |
| Entropy scoring                            | Done   | Workspace state and merge outcomes produce entropy reports.                           |
| Hard-lock blocking                         | Done   | Conflicting PRISM decisions now block commit flows.                                   |
| Reasoning diffs                            | Done   | `git-lore explain` presents history-backed reasoning output for a file.               |
| Merge-driver installation                  | Done   | `git-lore install` writes the `merge.lore` Git config and hook scripts.               |
| Pre-commit validation                      | Done   | The installed pre-commit hook runs `git-lore validate`.                               |
| Post-checkout reconstruction               | Done   | The installed post-checkout hook runs `git-lore sync`.                                |
| Sanitization hook                          | Done   | Lore text is scrubbed for obvious secrets before recording and before commit.         |
| Validation scripts                         | Done   | Per-atom validation commands are recorded and executed by `git-lore validate`.        |
| Compressed binary storage                  | Done   | Workspace records are stored as gzip-compressed binary JSON blobs.                    |

## Compliance Summary

- The core intent-tracking protocols and the Git-native integration slice are implemented.
- The remaining work is now ordinary feature expansion, not unified-spec compliance.
- The current code should be described as a working implementation of the unified spec.
