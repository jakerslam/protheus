#!/usr/bin/env bash
set -euo pipefail

ROLE="operator"
DRY_RUN=0
INSTALL_IF_MISSING=1
START_GATEWAY=0
INSTALL_MODE="minimal"
WORKSPACE_ROOT=""
INSTALL_PERFORMED=0
COMMAND_AVAILABLE=0
GATEWAY_STARTED=0

usage() {
  cat <<'USAGE'
Usage: protheus_onboarding_bootstrap.sh [options]
  --role=<operator|backend|infra|security>   onboarding role label (default: operator)
  --dry-run=1                                 record receipt only, do not install/start
  --install=0|1                               install if infring is missing (default: 1)
  --gateway=0|1                               start gateway after bootstrap (default: 0)
  --install-mode=<minimal|full|pure|tiny-max>
                                              install mode when auto-installing (default: minimal)
  --workspace-root=<path>                     explicit workspace root containing install.sh
  --help                                      show this help
USAGE
}

for arg in "$@"; do
  case "$arg" in
    --role=*) ROLE="${arg#*=}" ;;
    --dry-run=1) DRY_RUN=1 ;;
    --dry-run=0) DRY_RUN=0 ;;
    --install=1) INSTALL_IF_MISSING=1 ;;
    --install=0) INSTALL_IF_MISSING=0 ;;
    --gateway=1) START_GATEWAY=1 ;;
    --gateway=0) START_GATEWAY=0 ;;
    --install-mode=*) INSTALL_MODE="${arg#*=}" ;;
    --workspace-root=*) WORKSPACE_ROOT="${arg#*=}" ;;
    --help|-h) usage; exit 0 ;;
    *)
      echo "unknown argument: $arg" >&2
      usage >&2
      exit 1
      ;;
  esac
done

case "$INSTALL_MODE" in
  minimal|full|pure|tiny-max) ;;
  *)
    echo "invalid --install-mode value: $INSTALL_MODE" >&2
    exit 1
    ;;
esac

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

as_json_bool() {
  if [ "${1:-0}" = "1" ]; then
    printf 'true'
  else
    printf 'false'
  fi
}

script_dir="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [ -z "$WORKSPACE_ROOT" ]; then
  WORKSPACE_ROOT="$(cd "$script_dir/../../../.." && pwd)"
fi

activate_runtime_path() {
  if [ -f "$HOME/.infring/env.sh" ]; then
    # shellcheck disable=SC1090
    . "$HOME/.infring/env.sh"
  fi
  if [ -d "$HOME/.local/bin" ]; then
    export PATH="$HOME/.local/bin:$PATH"
  fi
  hash -r 2>/dev/null || true
}

install_missing_infring() {
  [ "$INSTALL_IF_MISSING" = "1" ] || return 1
  [ "$DRY_RUN" = "0" ] || return 1

  local install_script="$WORKSPACE_ROOT/install.sh"
  [ -f "$install_script" ] || return 1

  local install_args=()
  case "$INSTALL_MODE" in
    full) install_args+=(--full) ;;
    pure) install_args+=(--pure) ;;
    tiny-max) install_args+=(--tiny-max) ;;
    minimal) ;;
  esac

  sh "$install_script" "${install_args[@]}"
  INSTALL_PERFORMED=1
  return 0
}

activate_runtime_path
if command -v infring >/dev/null 2>&1; then
  COMMAND_AVAILABLE=1
elif install_missing_infring; then
  activate_runtime_path
  if command -v infring >/dev/null 2>&1; then
    COMMAND_AVAILABLE=1
  fi
fi

COMMAND_PATH="$(command -v infring 2>/dev/null || true)"
if [ -n "$COMMAND_PATH" ]; then
  COMMAND_AVAILABLE=1
fi

OK=1
if [ "$COMMAND_AVAILABLE" != "1" ] && [ "$DRY_RUN" = "0" ]; then
  OK=0
fi

if [ "$START_GATEWAY" = "1" ] && [ "$DRY_RUN" = "0" ]; then
  if [ "$COMMAND_AVAILABLE" = "1" ]; then
    infring gateway
    GATEWAY_STARTED=1
  else
    OK=0
  fi
fi

mkdir -p local/state/ops/onboarding_portal
receipt_path="local/state/ops/onboarding_portal/bootstrap_${ROLE}.json"
cat > "$receipt_path" <<JSON
{
  "schema_id": "onboarding_bootstrap_receipt",
  "schema_version": "2.0",
  "ts": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "role": "$(json_escape "$ROLE")",
  "dry_run": $(as_json_bool "$DRY_RUN"),
  "install_if_missing": $(as_json_bool "$INSTALL_IF_MISSING"),
  "install_mode": "$(json_escape "$INSTALL_MODE")",
  "install_performed": $(as_json_bool "$INSTALL_PERFORMED"),
  "command_available": $(as_json_bool "$COMMAND_AVAILABLE"),
  "command_path": "$(json_escape "$COMMAND_PATH")",
  "gateway_requested": $(as_json_bool "$START_GATEWAY"),
  "gateway_started": $(as_json_bool "$GATEWAY_STARTED"),
  "workspace_root": "$(json_escape "$WORKSPACE_ROOT")",
  "ok": $(as_json_bool "$OK")
}
JSON

echo "onboarding bootstrap complete for role=$ROLE ok=$OK receipt=$receipt_path"

if [ "$OK" != "1" ] && [ "$DRY_RUN" = "0" ]; then
  echo "bootstrap failed: infring command unavailable (run install.sh manually and retry)" >&2
  exit 1
fi
