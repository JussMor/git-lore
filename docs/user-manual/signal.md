# git-lore signal

## Description

Emits ephemeral, short-lived PRISM signals.

A Signal acts as a "soft-lock" for automated AI Agents and co-workers in real time ("I am currently assuming this component does X"). Signals expire via TTL and DO NOT persist as firm JSON Atoms like decisions do.

For the recommended low-friction flow, prefer `git-lore session-start`, which emits the signal and writes the pre-write checkpoint in one step.

## Usage

`git-lore signal [OPTIONS]`

## Options

- `--agent <String>`: The identity/name of the bot or person emitting the signal.
- `--session-id <String>`: (Auto-generated if missing) Identifier for the active AI session.
- `--scope <String>`: Optional code scope (like a method).
- `--path <GLOB>`: The file(s) or directories affected.
- `--assumption <String>`: The temporary theory running in memory.
- `--decision <String>`: A brief tentative goal.
