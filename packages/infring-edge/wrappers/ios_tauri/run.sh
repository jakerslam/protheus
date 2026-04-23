#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/../../../.." && pwd)"
ENTRYPOINT="$ROOT_DIR/client/runtime/lib/ts_entrypoint.ts"
STARTER="$ROOT_DIR/packages/infring-edge/starter.ts"

echo "[infring-edge] checking iOS/Tauri edge compatibility surface"
node "$ENTRYPOINT" "$STARTER" --mode=status --target=ios_tauri --benchmark=0
