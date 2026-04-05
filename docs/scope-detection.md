# Scope Detection

Git-Lore uses Tree-sitter to identify the active scope for a file and cursor position.

## Current behavior

- Rust source files are parsed with `tree-sitter-rust`.
- JavaScript source files are parsed with `tree-sitter-javascript`.
- TypeScript and TSX source files are parsed with `tree-sitter-typescript`.
- The detector chooses the smallest scope node that contains the requested line.
- Rust scopes cover functions, modules, structs, enums, traits, and impl blocks.
- JavaScript and TypeScript scopes cover functions, methods, classes, interfaces, type aliases, and arrow functions where available.
- Unsupported files fall back to a file-level scope based on the file stem.

## Intended use

- Feed scope information into context retrieval.
- Attach scope metadata to proposed lore atoms.
- Keep the MCP-style context layer lightweight and deterministic.
