# Git-Lore

Git-Lore is a local-first Rust tool for capturing rationale as structured lore atoms and persisting that intent into Git-oriented workflows.

See [docs/next-steps.md](docs/next-steps.md) for the current implementation order and [.github/copilot-instructions.md](.github/copilot-instructions.md) for the workspace guidance used by Copilot.
See [docs/compliance-matrix.md](docs/compliance-matrix.md) for what is implemented versus still pending from the unified specification.

## Installation

### A. Install from Crates.io (Recommended)

If you have Rust and Cargo installed, simply run:

```bash
cargo install git-lore
```

### B. One-Liner Script (macOS / Linux)

Run this curl command in your terminal to automatically download and install `git-lore` directly from the main repository (requires Cargo installed):

```bash
curl -fsSL https://raw.githubusercontent.com/JussMor/git-lore/main/install.sh | bash
```

### C. MCP Server Configuration (AI Agents & IDEs)

`git-lore` includes a built-in Model Context Protocol (MCP) server to share context directly with AI tools (like Claude Desktop, Cursor, or VS Code). To enable it, add the following to your MCP configuration file (e.g., `claude_desktop_config.json` or your IDE's MCP settings):

```json
{
  "mcpServers": {
    "git-lore": {
      "command": "git-lore",
      "args": ["mcp", "."]
    }
  }
}
```

## Current slice

- Rust CLI scaffold
- Local `.lore` workspace initialization
- Lore atom recording to JSON state
- Commit trailer rendering for checkpoint output
- Git repository discovery, commit-message parsing, and actual commit creation
- PRISM soft-lock signaling and overlap warnings
- Tree-sitter scope detection for Rust, JavaScript, and TypeScript source files
- MCP-style context and proposal flow built on top of parser and lore state, exposed over stdio MCP transport
- Accepted decision storage under `.lore/refs/lore/accepted`
- Git-native `refs/lore` mirroring, `git-lore explain`, `git-lore validate`, `git-lore sync`, and `git-lore install`
- Per-atom validation scripts and gzip-compressed workspace record storage
- 3-way lore merge reconciliation across base, left, and right states
- Entropy scoring and contradiction reporting for workspace state and merge outcomes
- Protocol docs for PRISM, refs/lore, MCP, scope detection, merge reconciliation, and entropy

## Next steps

1. Expand merge reconciliation heuristics for more complex dependency graphs.

## Testing commands locally

- Use `.test` as the throwaway Git repository for testing `git-lore` commands.
- Run command examples against `.test` so experiments do not affect the main repository state.
- For path-based commands, pass `.test` explicitly (for example: `git-lore mcp .test`).
