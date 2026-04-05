# git-lore mcp

## Description

Spawns the Model Context Protocol (MCP) server.

This command initiates standard I/O communication, turning your local repository into a knowledge server so IDE extensions (like VS Code Copilot) and agentic frameworks can read context, preflight checks, and propose code completely autonomously using `git-lore`.

## Usage

`git-lore mcp [PATH]`

## Available MCP Tools

The MCP server exposes exactly 6 tools. This minimalist design enforces strict deterministic behaviors from AI agents and prevents them from unilaterally altering the project's canonical laws. The tools are divided into three categories:

### 1. Read Tools (Knowledge Exploration)

- **`git_lore_memory_search`**: Allows the AI to search for vague intentions or conventions without knowing the specific file (Open Discovery).
- **`git_lore_context`**: When the AI knows the file it's working on, this extracts strict constraints linked to that file or syntax tree node (Directed Retrieval).
- **`git_lore_state_snapshot`**: Generates a cryptographic signature (checksum) of the current `.lore/` state, a required proof-of-freshness before any mutation attempt.

### 2. Preventive Tools (Safe Writes & Preflight)

- **`git_lore_memory_preflight`**: Allows the agent to verify if a planned massive file change or commit will break any fundamental protected rule in the team's memory.
- **`git_lore_state_transition_preview`**: Lets the agent simulate lifecycle transitions ("Can this Proposed rule change to Accepted?") to prevent infinite AI loops.

### 3. Mutation Tools (Creation)

- **`git_lore_propose`**: The only tool capable of creating new lore. It requires exact checksums from a recent Snapshot and strictly outputs atoms into the `Proposed` state.

### Architectural Principle

**Why doesn't the AI have tools to directly accept or deprecate rules?**
The design intentionally restricts the AI to the role of a _Promoter_ (`Propose`) or an _Obedient Executor_ (via Context and Preflight). Human developers retain the local CLI commands (e.g., `git-lore set-state accepted`) to decide when an AI's proposal becomes a Canonical Law.
