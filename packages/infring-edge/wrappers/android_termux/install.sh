#!/usr/bin/env sh
set -eu

ROOT_DIR="$(cd "$(dirname "$0")/../../../.." && pwd)"
ENTRYPOINT="$ROOT_DIR/client/runtime/lib/ts_entrypoint.ts"
STARTER="$ROOT_DIR/packages/infring-edge/starter.ts"

echo "[infring-edge] installing Android/Termux wrapper"
node "$ENTRYPOINT" "$STARTER" --mode=status --target=android_termux --benchmark=0
