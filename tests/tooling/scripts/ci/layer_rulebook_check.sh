#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/layer_rulebook_check.ts
