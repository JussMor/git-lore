# Git-Lore Process Diagnosis And Fix Plan

## Scope

This document explains why you can see:

- many Lore trailers in Git commits
- zero checkpoints for a selected atom in Glore
- missing commit linkage for some new atoms

and how to stabilize the process without losing traceability.

## What Is Happening (Verified)

### 1) Checkpoints are atom-scoped by membership, not global time

In your real workspace at:

- /Users/jussmor/Developer/maxwellclinic/EverBetter-Pro/.lore

the current selected atom in your screenshot is:

- 79f1318f-e8cf-4295-bde5-156cfb957bcb

The existing checkpoint files do not contain that atom id, so Glore correctly shows:

- Checkpoints: 0 for that atom

This is expected if checkpoints were created before that atom existed (or before it was added to active state).

### 2) Audit trail is also atom-specific

Your audit file currently has one transition event for atom:

- 13915075-e50f-48f2-98ad-303433670832

So for atom 79f..., audit can be 0/empty (or unrelated).

### 3) Commit trailer growth is by design in current implementation

Current flow uses all active atoms when creating commit message trailers.
Implementation path:

- src/cli/mod.rs calls build_commit_message with state.atoms
- src/git/mod.rs render_commit_trailers builds one Lore trailer line per atom

So every git-lore commit can append many Lore-Decision/Lore-Assumption lines, even if most atoms did not change.

### 4) Lore refs are kept as moving pointers for accepted atoms

On commit, refs/lore/accepted/<atom-id> are updated for non-deprecated atoms.
This preserves latest pointer but contributes to a perception of “everything always active”.

## Why This Feels Complex

The system currently combines multiple persistence channels at once:

- active state in .lore/active_intent.json
- snapshots in .lore/checkpoints/\*.json
- per-atom refs in refs/lore/accepted/\*
- commit trailers in Git commit messages
- optional JSON in refs/notes/lore
- lifecycle transitions in .lore/audit/state_transitions.jsonl

This gives strong recovery and traceability, but if commits are full-snapshot style, signal-to-noise drops quickly.

## Immediate Operational Fix (No Code Change)

### A) If you want a checkpoint to appear for an atom, checkpoint after creating/updating that atom

Recommended order:

1. git-lore mark/propose/set-state for the atom
2. git-lore checkpoint --message "checkpoint: <atom-id> <reason>"
3. git-lore validate .
4. git-lore commit --message "..."

### B) Reduce trailer size by reducing active non-deprecated atoms

If old atoms are superseded:

1. create one consolidation decision atom
2. set superseded atoms to deprecated with explicit reason
3. checkpoint + commit

Result: future commits carry fewer active trailers.

### C) Use PRISM signals only for short-lived locks

- create signal with session id
- release with:
  git-lore signal --release --session-id <id>

Do not leave stale lock signals during long runs.

## Product Fix (Recommended)

### Goal

Keep full recoverability in lore refs/notes, while making commit messages concise.

### Option 1 (Best): Delta trailers

Add commit mode that includes trailers only for changed atoms since previous lore commit.

Rules:

- new atom: include trailer
- changed atom content/state/path/scope/title/body: include trailer
- unchanged atom: no trailer

Keep refs/notes as canonical full context.

### Option 2: Full refs, compact commit body

Commit message contains:

- subject
- one summary trailer (count, checksum, checkpoint id)

Then Glore resolves detail from refs/notes and timeline.

### Option 3: Configurable strategy

Add CLI flag and config:

- --trailers full|delta|none
- default: delta

## Implementation Plan (Code)

### Step 1

In commit path, compute atom delta before build_commit_message.

### Step 2

Change trailer generation to receive either:

- all active atoms (legacy full)
- only changed atoms (delta)

### Step 3

Update lore ref writing policy:

- write refs only for changed accepted atoms (optional but recommended)

### Step 4

Expose strategy in CLI and docs.

### Step 5

Update Glore panel labels:

- “Trailers in commit” vs “Lore refs linked”
  so users see why commit can be short but context still rich.

## Why This Preserves Memory And Recovery

Even if commit trailers are compact:

- refs/lore/accepted remains query index
- refs/notes/lore (or checkpoint payloads) keeps full atom details
- .lore/audit and checkpoints preserve chronology

So you reduce commit noise without losing traceability.

## Quick Health Checklist

- validate passes: yes
- .lore exists in target repo: yes
- checkpoints exist: yes
- selected atom appears in checkpoint: check by id
- commit trailers include only intended atoms: currently no (full snapshot)
- refs/lore configured: yes
- merge.lore config installed: yes

## Next Action

Implement delta trailers in git-lore commit as default, keep full mode behind a flag for backward compatibility.
