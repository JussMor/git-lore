# Git-Lore MCP Launcher

This folder contains a local launcher for the Git-Lore MCP server bound to the throwaway test repo in `.test`.

## Start the server

```sh
chmod +x .mcp/launch.sh
./.mcp/launch.sh
```

The launcher prefers `target/debug/git-lore` if you already built the project.
If the binary is missing, it falls back to `cargo run`.

VS Code uses [.vscode/mcp.json](../.vscode/mcp.json), which starts the launcher through `/bin/sh` so the MCP server works consistently on macOS.

## What it serves

- `git_lore_context`
- `git_lore_propose`

Both tools inspect and write lore state in `.test`.

## Typical flow

1. Open `.test` as the active repo.
2. Start the MCP server with `./.mcp/launch.sh`.
3. Use `git_lore_context` for scope-aware context on `src/app.rs`.
4. Use `git_lore_propose` to record a new decision before editing.
