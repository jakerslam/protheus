#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

SCENARIO="${SCENARIO:-}"
if [ -n "$SCENARIO" ]; then
  echo "SCENARIO is deprecated for benchmark-matrix and is ignored for compatibility." >&2
fi

node client/runtime/lib/ts_entrypoint.ts client/runtime/systems/ops/run_protheus_ops.ts benchmark-matrix run --refresh-runtime=1 "$@"
node client/runtime/lib/ts_entrypoint.ts client/runtime/systems/ops/run_protheus_ops.ts benchmark-matrix status
