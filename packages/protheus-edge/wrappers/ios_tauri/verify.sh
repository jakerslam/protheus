#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/../../../.." && pwd)"
node "$ROOT_DIR/client/runtime/lib/ts_entrypoint.ts" "$ROOT_DIR/packages/protheus-edge/starter.ts" --mode=status --target=ios_tauri --benchmark=0
