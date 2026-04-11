#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERIFY_TIMEOUT_SEC="${PROTHEUS_VERIFY_TIMEOUT_SEC:-45}"
VERIFY_DEFER_HOST_STALL="${PROTHEUS_VERIFY_DEFER_HOST_STALL:-1}"
VERIFY_NPM_TIMEOUT_SEC="${PROTHEUS_VERIFY_NPM_TIMEOUT_SEC:-60}"
VERIFY_RUST_TIMEOUT_SEC="${PROTHEUS_VERIFY_RUST_TIMEOUT_SEC:-180}"
VERIFY_PROOF_TIMEOUT_SEC="${PROTHEUS_VERIFY_PROOF_TIMEOUT_SEC:-420}"
VERIFY_ARTIFACT_MODE="${PROTHEUS_VERIFY_ARTIFACT_MODE:-ephemeral}"
VERIFY_PROFILE="${PROTHEUS_VERIFY_PROFILE:-release}"
VERIFY_PROFILES_PATH="tests/tooling/config/verify_profiles.json"

if [[ "$VERIFY_ARTIFACT_MODE" == "ephemeral" ]]; then
  VERIFY_TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/protheus-verify-XXXXXX")"
  trap 'rm -rf "${VERIFY_TMP_DIR:-}"' EXIT
  CLIENT_LAYER_AUDIT_OUT="$VERIFY_TMP_DIR/client_layer_boundary_audit_current.json"
  MODULE_COHESION_OUT_JSON="$VERIFY_TMP_DIR/module_cohesion_audit_current.json"
  MODULE_COHESION_OUT_MD="$VERIFY_TMP_DIR/MODULE_COHESION_AUDIT_CURRENT.md"
  CLIENT_IMPORT_INTEGRITY_OUT="$VERIFY_TMP_DIR/client_import_integrity_audit_current.json"
  CLIENT_SCOPE_OUT="$VERIFY_TMP_DIR/client_scope_inventory_current.json"
  CLIENT_SURFACE_OUT="$VERIFY_TMP_DIR/client_surface_disposition_current.json"
  CLIENT_TARGET_OUT="$VERIFY_TMP_DIR/client_target_contract_audit_current.json"
  VERIFY_PROFILE_OUT="$VERIFY_TMP_DIR/verify_profile_current.json"
else
  CLIENT_LAYER_AUDIT_OUT="$ROOT/core/local/artifacts/client_layer_boundary_audit_current.json"
  MODULE_COHESION_OUT_JSON="$ROOT/core/local/artifacts/module_cohesion_audit_current.json"
  MODULE_COHESION_OUT_MD="$ROOT/local/workspace/reports/MODULE_COHESION_AUDIT_CURRENT.md"
  CLIENT_IMPORT_INTEGRITY_OUT="$ROOT/core/local/artifacts/client_import_integrity_audit_current.json"
  CLIENT_SCOPE_OUT="$ROOT/core/local/artifacts/client_scope_inventory_current.json"
  CLIENT_SURFACE_OUT="$ROOT/core/local/artifacts/client_surface_disposition_current.json"
  CLIENT_TARGET_OUT="$ROOT/core/local/artifacts/client_target_contract_audit_current.json"
  VERIFY_PROFILE_OUT="$ROOT/core/local/artifacts/verify_profile_current.json"
fi

export VERIFY_TIMEOUT_SEC
export VERIFY_DEFER_HOST_STALL
export VERIFY_NPM_TIMEOUT_SEC
export VERIFY_RUST_TIMEOUT_SEC
export VERIFY_PROOF_TIMEOUT_SEC
export VERIFY_ARTIFACT_MODE
export VERIFY_PROFILE
export VERIFY_PROFILES_PATH
export CLIENT_LAYER_AUDIT_OUT
export MODULE_COHESION_OUT_JSON
export MODULE_COHESION_OUT_MD
export CLIENT_IMPORT_INTEGRITY_OUT
export CLIENT_SCOPE_OUT
export CLIENT_SURFACE_OUT
export CLIENT_TARGET_OUT
export VERIFY_PROFILE_OUT

(
  cd "$ROOT"
  node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/tooling_registry_runner.ts \
    profile \
    --profiles="$VERIFY_PROFILES_PATH" \
    --id="$VERIFY_PROFILE" \
    --strict=1 \
    --out="$VERIFY_PROFILE_OUT"
)
