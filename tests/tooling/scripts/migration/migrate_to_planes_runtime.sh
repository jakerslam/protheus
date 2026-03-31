#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

node client/runtime/lib/ts_entrypoint.ts client/runtime/systems/ops/local_runtime_partitioner.ts init >/dev/null
node client/runtime/lib/ts_entrypoint.ts client/runtime/systems/sensory/conversation_eye_bootstrap.ts ensure --apply=1 >/dev/null
node client/runtime/lib/ts_entrypoint.ts client/runtime/systems/ops/migrate_to_planes.ts run --apply=1 --move-untracked=1
