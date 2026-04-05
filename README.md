# Git-Lore: Synchronized Context for AI & Human Engineers

**The Problem:** In fast-paced, asynchronous, and AI-assisted development environments, the _"why"_ behind code constantly gets lost. Teammates submit PRs on different schedules, and LLM code-assistants jump into files completely blind to the larger architectural constraints or domain assumptions. Without this context, agents hallucinate, humans repeat past mistakes, and architectural drift destroys codebases.

**The Solution:** Git-Lore creates a powerful synchronous collaboration layer between human developers and Large Language Models (LLMs). By anchoring rationale as deterministic, structured "Lore Atoms" directly bounded to your codebase paths and scopes, it ensures that every human or AI agent (via the Model Context Protocol - MCP) has instant, real-time access to the exact rules, assumptions, and decisions of the code they are modifying.

Git-Lore is a local-first Rust CLI and MCP Server for capturing knowledge and persisting that intent directly into Git-oriented workflows.

See [docs/next-steps.md](docs/next-steps.md) for the current implementation order and [.github/copilot-instructions.md](.github/copilot-instructions.md) for the workspace guidance used by Copilot.
See [docs/compliance-matrix.md](docs/compliance-matrix.md) for what is implemented versus still pending from the unified specification.

## Installation

### A. Install from Crates.io (Recommended)

If you have Rust and Cargo installed, simply run:

```bash
cargo install git-lore
```

**Want Semantic Search?** To enable the local AI Memvid vector search module:
```bash
cargo install git-lore --features semantic-search
```

### B. One-Liner Script (macOS / Linux)

Run this curl command in your terminal to automatically download and install `git-lore` directly from the main repository (requires Cargo installed):

```bash
curl -fsSL https://raw.githubusercontent.com/JussMor/git-lore/main/install.sh | bash
```

To install with Semantic Search enabled via the script:
```bash
curl -fsSL https://raw.githubusercontent.com/JussMor/git-lore/main/install.sh | bash -s -- --features semantic-search
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

### Skill Generator

You can automatically generate an AI integration skill (for GitHub Copilot, Cursor, etc.) to help other developers easily adopt `git-lore` in their daily routines without needing to read all the docs. Simply run:

```bash
git-lore generate
```

This will create a `.github/git-lore-skills.md` file designed to be read by LLMs to seamlessly bind architectural decisions and knowledge directly into your codebase state.

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
