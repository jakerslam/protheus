#!/usr/bin/env bash
set -euo pipefail

ROLE="operator"
DRY_RUN=0
INSTALL_IF_MISSING=1
RUN_SETUP=1
START_GATEWAY=0
INSTALL_MODE="minimal"
WORKSPACE_ROOT=""
INSTALL_PERFORMED=0
COMMAND_AVAILABLE=0
SETUP_ATTEMPTED=0
SETUP_COMPLETED=0
SETUP_STATUS_CHECK="not_requested"
SETUP_STATUS_RAW=""
GATEWAY_STARTED=0
RECEIPT_PATH_OVERRIDE=""
SUMMARY_PATH_OVERRIDE=""

usage() {
  cat <<'USAGE'
Usage: infring_onboarding_bootstrap.sh [options]
  --role=<operator|backend|infra|security>   onboarding role label (default: operator)
  --dry-run=1                                 record receipt only, do not install/start
  --install=0|1                               install if infring is missing (default: 1)
  --setup=0|1                                 run 'infring setup --yes --defaults' after install (default: 1)
  --gateway=0|1                               start gateway after bootstrap (default: 0)
  --install-mode=<minimal|full|pure|tiny-max>
                                              install mode when auto-installing (default: minimal)
  --workspace-root=<path>                     explicit workspace root containing install.sh
  --receipt-path=<path>                       override JSON receipt output path
  --summary-path=<path>                       override human-readable summary output path
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
    --setup=1) RUN_SETUP=1 ;;
    --setup=0) RUN_SETUP=0 ;;
    --gateway=1) START_GATEWAY=1 ;;
    --gateway=0) START_GATEWAY=0 ;;
    --install-mode=*) INSTALL_MODE="${arg#*=}" ;;
    --workspace-root=*) WORKSPACE_ROOT="${arg#*=}" ;;
    --receipt-path=*) RECEIPT_PATH_OVERRIDE="${arg#*=}" ;;
    --summary-path=*) SUMMARY_PATH_OVERRIDE="${arg#*=}" ;;
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

if [ "$RUN_SETUP" = "1" ] && [ "$DRY_RUN" = "0" ]; then
  if [ "$COMMAND_AVAILABLE" = "1" ]; then
    SETUP_ATTEMPTED=1
    if infring setup --yes --defaults; then
      SETUP_STATUS_RAW="$(infring setup status --json 2>/dev/null || true)"
      if [ -n "$SETUP_STATUS_RAW" ] && printf '%s' "$SETUP_STATUS_RAW" | tr -d '\n' | grep -Eiq '"status"[[:space:]]*:[[:space:]]*"completed"|\"completed\"[[:space:]]*:[[:space:]]*true'; then
        SETUP_COMPLETED=1
        SETUP_STATUS_CHECK="completed"
      else
        SETUP_STATUS_CHECK="incomplete_or_unverifiable"
        OK=0
      fi
    else
      SETUP_STATUS_CHECK="setup_command_failed"
      OK=0
    fi
  else
    SETUP_STATUS_CHECK="command_unavailable"
    OK=0
  fi
fi

if [ "$START_GATEWAY" = "1" ] && [ "$DRY_RUN" = "0" ]; then
  if [ "$COMMAND_AVAILABLE" = "1" ]; then
    infring gateway
    GATEWAY_STARTED=1
  else
    OK=0
  fi
fi

if [ "$COMMAND_AVAILABLE" = "1" ]; then
  BINARY_OUTCOME="ready"
else
  BINARY_OUTCOME="missing"
fi

if [ "$RUN_SETUP" = "1" ]; then
  if [ "$SETUP_COMPLETED" = "1" ]; then
    SETUP_OUTCOME="completed"
  elif [ "$DRY_RUN" = "1" ]; then
    SETUP_OUTCOME="skipped_dry_run"
  else
    SETUP_OUTCOME="failed"
  fi
else
  SETUP_OUTCOME="not_requested"
fi

if [ "$START_GATEWAY" = "1" ]; then
  if [ "$GATEWAY_STARTED" = "1" ]; then
    GATEWAY_OUTCOME="started"
  elif [ "$DRY_RUN" = "1" ]; then
    GATEWAY_OUTCOME="skipped_dry_run"
  else
    GATEWAY_OUTCOME="failed"
  fi
else
  GATEWAY_OUTCOME="not_requested"
fi

if [ "$DRY_RUN" = "1" ]; then
  STATUS="dry_run"
elif [ "$OK" = "1" ]; then
  STATUS="success"
else
  STATUS="failed"
fi

REMEDIATION_CODE="none"
REMEDIATION_HINT="none"
REMEDIATION_STEPS_JSON='[]'
if [ "$OK" != "1" ] && [ "$DRY_RUN" = "0" ]; then
  if [ "$COMMAND_AVAILABLE" != "1" ]; then
    REMEDIATION_CODE="infring_command_unavailable"
    REMEDIATION_HINT="run installer, reload PATH, retry bootstrap"
    REMEDIATION_STEPS_JSON='[
      "Run: curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full --install-node",
      "Reload shell path: . \"$HOME/.infring/env.sh\" && hash -r 2>/dev/null || true",
      "Retry onboarding bootstrap for the same role"
    ]'
  elif [ "$RUN_SETUP" = "1" ] && [ "$SETUP_COMPLETED" != "1" ]; then
    REMEDIATION_CODE="setup_not_completed"
    REMEDIATION_HINT="run setup defaults, confirm setup status, retry bootstrap"
    REMEDIATION_STEPS_JSON='[
      "Run: infring setup --yes --defaults",
      "Run: infring setup status --json",
      "Retry onboarding bootstrap for the same role"
    ]'
  elif [ "$START_GATEWAY" = "1" ] && [ "$GATEWAY_STARTED" != "1" ]; then
    REMEDIATION_CODE="gateway_start_failed"
    REMEDIATION_HINT="verify gateway status, then restart gateway"
    REMEDIATION_STEPS_JSON='[
      "Run: infring gateway status",
      "Run: infring gateway restart",
      "If still failing, run: infringctl doctor --json"
    ]'
  else
    REMEDIATION_CODE="unknown_bootstrap_failure"
    REMEDIATION_HINT="run doctor and inspect bootstrap receipt"
    REMEDIATION_STEPS_JSON='[
      "Run: infringctl doctor --json",
      "Inspect onboarding receipt for role-specific failure fields",
      "Retry bootstrap with --dry-run=1 and compare output"
    ]'
  fi
fi

output_dir="local/state/ops/onboarding_portal"
mkdir -p "$output_dir"
receipt_path="${RECEIPT_PATH_OVERRIDE:-$output_dir/bootstrap_${ROLE}.json}"
summary_path="${SUMMARY_PATH_OVERRIDE:-$output_dir/bootstrap_${ROLE}.txt}"
setup_status_path="$output_dir/bootstrap_${ROLE}_setup_status.json"
failure_snapshot_path="$output_dir/bootstrap_${ROLE}_failure_snapshot.json"
setup_wizard_state_path="local/state/ops/infring_setup_wizard/latest.json"
first_run_wizard_state_path="local/state/ops/first_run_onboarding_wizard/latest.json"
dashboard_ui_log_path="${HOME}/.infring/logs/dashboard_ui.log"
dashboard_watchdog_log_path="${HOME}/.infring/logs/dashboard_watchdog.log"
role_bootstrap_contract="binary_ready_setup_completed_gateway_optional"
mkdir -p "$(dirname "$receipt_path")" "$(dirname "$summary_path")"
if [ -n "$SETUP_STATUS_RAW" ]; then
  printf '%s\n' "$SETUP_STATUS_RAW" > "$setup_status_path"
elif [ "$RUN_SETUP" = "1" ]; then
  cat > "$setup_status_path" <<SETUPJSON
{"ok": false, "type": "setup_status_unavailable", "status_check": "$(json_escape "$SETUP_STATUS_CHECK")"}
SETUPJSON
fi
cat > "$receipt_path" <<JSON
{
  "schema_id": "onboarding_bootstrap_receipt",
  "schema_version": "2.1",
  "ts": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "role": "$(json_escape "$ROLE")",
  "status": "$(json_escape "$STATUS")",
  "dry_run": $(as_json_bool "$DRY_RUN"),
  "install_if_missing": $(as_json_bool "$INSTALL_IF_MISSING"),
  "setup_requested": $(as_json_bool "$RUN_SETUP"),
  "setup_attempted": $(as_json_bool "$SETUP_ATTEMPTED"),
  "setup_completed": $(as_json_bool "$SETUP_COMPLETED"),
  "setup_outcome": "$(json_escape "$SETUP_OUTCOME")",
  "setup_status_check": "$(json_escape "$SETUP_STATUS_CHECK")",
  "setup_status_path": "$(json_escape "$setup_status_path")",
  "setup_status_present": $( [ -f "$setup_status_path" ] && printf true || printf false ),
  "setup_status_source": "infring setup status --json",
  "install_mode": "$(json_escape "$INSTALL_MODE")",
  "binary_outcome": "$(json_escape "$BINARY_OUTCOME")",
  "install_performed": $(as_json_bool "$INSTALL_PERFORMED"),
  "command_available": $(as_json_bool "$COMMAND_AVAILABLE"),
  "command_path": "$(json_escape "$COMMAND_PATH")",
  "gateway_requested": $(as_json_bool "$START_GATEWAY"),
  "gateway_started": $(as_json_bool "$GATEWAY_STARTED"),
  "gateway_outcome": "$(json_escape "$GATEWAY_OUTCOME")",
  "canonical_sequence": "install_setup_gateway",
  "role_bootstrap_contract": "$(json_escape "$role_bootstrap_contract")",
  "expected_role_outcomes": {
    "binary_outcome": "ready",
    "setup_outcome": "completed",
    "gateway_outcome_when_requested": "started"
  },
  "mode": "$(json_escape "$INSTALL_MODE")",
  "workspace_status": "$(json_escape "$BINARY_OUTCOME")",
  "next_action": "$(json_escape "${REMEDIATION_HINT}")",
  "remediation_code": "$(json_escape "$REMEDIATION_CODE")",
  "remediation_steps": $REMEDIATION_STEPS_JSON,
  "verification_commands": [
    "infring setup status --json",
    "infring gateway status",
    "infringctl doctor --json"
  ],
  "recovery_command_chain": [
    "infring gateway status",
    "infring gateway restart",
    "infringctl doctor --json",
    "curl -fsS http://127.0.0.1:4173/healthz"
  ],
  "artifact_paths": {
    "receipt": "$(json_escape "$receipt_path")",
    "summary": "$(json_escape "$summary_path")",
    "setup_status": "$(json_escape "$setup_status_path")",
    "setup_wizard_state": "$(json_escape "$setup_wizard_state_path")",
    "first_run_wizard_state": "$(json_escape "$first_run_wizard_state_path")",
    "dashboard_ui_log": "$(json_escape "$dashboard_ui_log_path")",
    "dashboard_watchdog_log": "$(json_escape "$dashboard_watchdog_log_path")",
    "failure_snapshot": "$(json_escape "$failure_snapshot_path")"
  },
  "failure_snapshot_path": "$(json_escape "$failure_snapshot_path")",
  "workspace_root": "$(json_escape "$WORKSPACE_ROOT")",
  "ok": $(as_json_bool "$OK")
}
JSON

if [ "$OK" != "1" ] && [ "$DRY_RUN" = "0" ]; then
  cat > "$failure_snapshot_path" <<FAILUREJSON
{
  "schema_id": "onboarding_bootstrap_failure_snapshot",
  "schema_version": "1.0",
  "ts": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "role": "$(json_escape "$ROLE")",
  "status": "$(json_escape "$STATUS")",
  "remediation_code": "$(json_escape "$REMEDIATION_CODE")",
  "remediation_hint": "$(json_escape "$REMEDIATION_HINT")",
  "verification_commands": [
    "infring setup status --json",
    "infring gateway status",
    "infringctl doctor --json"
  ],
  "recovery_command_chain": [
    "infring gateway status",
    "infring gateway restart",
    "infringctl doctor --json",
    "curl -fsS http://127.0.0.1:4173/healthz"
  ],
  "artifact_paths": {
    "bootstrap_receipt": "$(json_escape "$receipt_path")",
    "bootstrap_summary": "$(json_escape "$summary_path")",
    "setup_status": "$(json_escape "$setup_status_path")",
    "setup_wizard_state": "$(json_escape "$setup_wizard_state_path")",
    "first_run_wizard_state": "$(json_escape "$first_run_wizard_state_path")",
    "dashboard_ui_log": "$(json_escape "$dashboard_ui_log_path")",
    "dashboard_watchdog_log": "$(json_escape "$dashboard_watchdog_log_path")"
  }
}
FAILUREJSON
fi

cat > "$summary_path" <<SUMMARY
Onboarding bootstrap summary
role: $ROLE
status: $STATUS
workspace_root: $WORKSPACE_ROOT
mode: $INSTALL_MODE
binary_outcome: $BINARY_OUTCOME
setup_outcome: $SETUP_OUTCOME
setup_status_check: $SETUP_STATUS_CHECK
setup_status_path: $setup_status_path
gateway_outcome: $GATEWAY_OUTCOME
receipt_path: $receipt_path
summary_path: $summary_path
failure_snapshot_path: $failure_snapshot_path
remediation_code: $REMEDIATION_CODE
remediation_hint: $REMEDIATION_HINT
artifact_setup_wizard_state: $setup_wizard_state_path
artifact_first_run_wizard_state: $first_run_wizard_state_path
artifact_dashboard_ui_log: $dashboard_ui_log_path
artifact_dashboard_watchdog_log: $dashboard_watchdog_log_path
expected_outcomes: binary_outcome=ready, setup_outcome=completed, gateway_outcome=$( [ "$START_GATEWAY" = "1" ] && printf started || printf not_requested )
role_bootstrap_contract: $role_bootstrap_contract
verification_commands: infring setup status --json ; infring gateway status
recovery_chain: infring gateway status ; infring gateway restart ; infringctl doctor --json ; curl -fsS http://127.0.0.1:4173/healthz
SUMMARY

echo "onboarding bootstrap complete role=$ROLE status=$STATUS receipt=$receipt_path summary=$summary_path"

if [ "$OK" != "1" ] && [ "$DRY_RUN" = "0" ]; then
  echo "bootstrap failed: remediation_code=$REMEDIATION_CODE hint=$REMEDIATION_HINT" >&2
  exit 1
fi
