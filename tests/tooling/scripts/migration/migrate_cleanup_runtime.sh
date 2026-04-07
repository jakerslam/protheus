#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

node client/runtime/lib/ts_entrypoint.ts client/runtime/systems/ops/run_protheus_ops.ts spine sleep-cleanup run --apply=1
