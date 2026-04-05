# MCP

This document describes the intended Model Context Protocol surface for Git-Lore.

## Intended behavior

- Expose repository context to IDEs and agents.
- Retrieve the latest scope-relevant decisions.
- Accept proposed lore atoms before edits are made.

## Transport

- Run the stdio MCP server with `git-lore mcp <repo>`.
- To enable the experimental **Semantic Search** using local vectors and full-text AI, build and run Git-Lore using the `semantic-search` Cargo feature. When compiled this way, the `git_lore_memory_search` will query a local sub-5ms Memvid-core index transparently rebuilt on every `git-lore sync`.

Before running the semantic search for the first time, you must download the local ONNX embedding models:

```bash
mkdir -p ~/.cache/memvid/text-models
curl -L 'https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx' -o ~/.cache/memvid/text-models/bge-small-en-v1.5.onnx
curl -L 'https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/tokenizer.json' -o ~/.cache/memvid/text-models/bge-small-en-v1.5_tokenizer.json
```

```bash
cargo build --release --features semantic-search
./target/release/git-lore mcp .
```
