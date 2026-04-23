#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/../../../.." && pwd)"
node "$ROOT_DIR/client/runtime/lib/ts_entrypoint.ts" "$ROOT_DIR/packages/infring-edge/starter.ts" --mode=status --target=android_termux --benchmark=0
