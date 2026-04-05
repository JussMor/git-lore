#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

if [ -x "$ROOT/target/debug/git-lore" ]; then
  exec "$ROOT/target/debug/git-lore" mcp "$ROOT/.test"
fi

cd "$ROOT"
exec cargo run -- mcp .test