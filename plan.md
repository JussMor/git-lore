What to improve to become a better AI memory

Retrieval precision and ranking
Prioritize context by scope relevance, recency, acceptance status, and contradiction risk; reduce noisy context.
Token-budgeted MCP responses
Return compact summaries first, then expandable details; avoid long raw dumps that waste model context.
Blast-radius context in MCP
Expose affected symbols/files with confidence scores so AI can reason about impact before editing.
Staleness lifecycle with ownership
Introduce review queues, aging thresholds, and ownership metadata so stale lore is resolved quickly.
Learn from proposal outcomes
Track accepted/rejected AI proposals and feed that back into ranking and prompt shaping.
Benchmark suite for AI memory quality
Measure before/after with scenarios: large refactor, overlapping team edits, contradiction recovery, and context accuracy.
Optional ingestion helper (outside core determinism)
Generate propose-ready actions from RFC/PR text, but keep final state changes reviewed and deterministic.
