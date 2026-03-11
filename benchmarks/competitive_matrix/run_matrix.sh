#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

node client/runtime/systems/ops/competitive_benchmark_matrix.ts run --scenario="${SCENARIO:-deterministic_001}" "$@"
node client/runtime/systems/ops/competitive_benchmark_matrix.ts status
