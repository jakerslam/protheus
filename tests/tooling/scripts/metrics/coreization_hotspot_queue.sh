#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

LIMIT="${1:-40}"
OUT="${2:-docs/workspace/coreization_hotspots.md}"

tmp_all="$(mktemp)"
tmp_sys="$(mktemp)"
trap 'rm -f "$tmp_all" "$tmp_sys"' EXIT

git ls-files '*.ts' \
  | xargs -I{} wc -l {} \
  | sort -nr \
  | head -n "$LIMIT" > "$tmp_all"

git ls-files 'client/runtime/systems/**/*.ts' \
  | xargs -I{} wc -l {} \
  | sort -nr \
  | head -n "$LIMIT" > "$tmp_sys"

{
  echo "# Coreization Hotspots"
  echo
  echo "Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo
  echo "## Top TS Files (Repo-Wide)"
  echo
  echo "| LoC | File | JS Wrapper Mode |"
  echo "|---:|---|---|"
  while read -r loc file; do
    js="${file%.ts}.js"
    mode="none"
    if [[ -f "$js" ]]; then
      if rg -q "createOpsLaneBridge" "$js"; then
        mode="rust-lane-wrapper"
      elif rg -q "ts_bootstrap" "$js"; then
        mode="ts-bootstrap-wrapper"
      else
        mode="custom-js"
      fi
    fi
    echo "| ${loc} | \`${file}\` | ${mode} |"
  done < "$tmp_all"

  echo
  echo "## Top TS Files (`client/runtime/systems`)"
  echo
  echo "| LoC | File | JS Wrapper Mode |"
  echo "|---:|---|---|"
  while read -r loc file; do
    js="${file%.ts}.js"
    mode="none"
    if [[ -f "$js" ]]; then
      if rg -q "createOpsLaneBridge" "$js"; then
        mode="rust-lane-wrapper"
      elif rg -q "ts_bootstrap" "$js"; then
        mode="ts-bootstrap-wrapper"
      else
        mode="custom-js"
      fi
    fi
    echo "| ${loc} | \`${file}\` | ${mode} |"
  done < "$tmp_sys"
} > "$OUT"

echo "Wrote hotspot queue: $OUT"
