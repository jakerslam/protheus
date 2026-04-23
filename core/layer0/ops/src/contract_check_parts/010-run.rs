// SPDX-License-Identifier: Apache-2.0
use crate::now_iso;
use crate::foundation_hook_enforcer_bridge::{
    evaluate_source_hook_coverage, HookCoverageReceipt, CHECK_ID_FOUNDATION_HOOKS,
    CHECK_ID_GUARD_REGISTRY_CONSUMPTION,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

const CHECK_IDS_FLAG_PREFIX: &str = "--rust-contract-check-ids=";
const GUARD_REGISTRY_REL: &str = "client/runtime/config/guard_check_registry.json";
const CONTRACT_CHECK_SOURCE_REL: &str = "core/layer0/ops/src/contract_check.rs";
const RUNTIME_MODE_STATE_REL: &str = "local/state/ops/runtime_mode.json";
const RUST_SOURCE_OF_TRUTH_POLICY_REL: &str =
    "client/runtime/config/rust_source_of_truth_policy.json";
const PROBE_EYES_INTAKE_HELP_TOKENS: &[&str] =
    &["eyes_intake.js", "create", "validate", "list-directives"];
const PROBE_CONFLICT_MARKER_HELP_TOKENS: &[&str] = &["conflict_marker_guard.js", "run", "status"];
const CHECK_ID_RUST_SOURCE_OF_TRUTH: &str = "rust_source_of_truth_contract";
const CHECK_ID_PROVIDER_DISCOVERY_RUNTIME: &str = "provider_discovery_runtime_contract";
const CHECK_ID_WEB_PROVIDER_FAST_PATH_ARTIFACTS: &str =
    "web_provider_public_artifacts_fast_path_contract";
const FETCH_RUNTIME_CONTRACT_SOURCE_REL: &str =
    "core/layer0/ops/src/web_conduit_provider_runtime_parts/019-fetch-runtime-resolution.rs";
const SEARCH_RUNTIME_CONTRACT_SOURCE_REL: &str =
    "core/layer0/ops/src/web_conduit_provider_runtime_parts/021-search-runtime-resolution.rs";
pub const GUARD_REGISTRY_REQUIRED_TOKENS: &[&str] =
    &["guard_check_registry", "required_merge_guard_ids"];
const PROVIDER_DISCOVERY_RUNTIME_REQUIRED_TOKENS: &[&str] = &[
    "provider_discovery_runtime_contract",
    "provider_discovery_contract_suite_contract",
    "provider_runtime_core_contract",
];
const WEB_PROVIDER_FAST_PATH_REQUIRED_TOKENS: &[&str] = &[
    "bundled_fast_path_contract_suite_contract",
    "provider_family_contract_suite_contract",
];
pub const FOUNDATION_HOOK_REQUIRED_TOKENS: &[&str] = &[
    "foundation_contract_gate.js",
    "scale_envelope_baseline.js",
    "simplicity_budget_gate.js",
    "phone_seed_profile.js",
    "surface_budget_controller.js",
    "compression_transfer_plane.js",
    "opportunistic_offload_plane.js",
    "gated_account_creation_organ.js",
    "siem_bridge.js",
    "soc2_type2_track.js",
    "predictive_capacity_forecast.js",
    "execution_sandbox_envelope.js",
    "organ_state_encryption_plane.js",
    "remote_tamper_heartbeat.js",
    "secure_heartbeat_endpoint.js",
    "gated_self_improvement_loop.js",
    "helix_admission_gate.js",
    "venom_containment_layer.js",
    "adaptive_defense_expansion.js",
    "confirmed_malice_quarantine.js",
    "helix_controller.js",
    "ant_colony_controller.js",
    "neural_dormant_seed.js",
    "pre_neuralink_interface.js",
    "client_relationship_manager.js",
    "capital_allocation_organ.js",
    "economic_entity_manager.js",
    "drift_aware_revenue_optimizer.js",
];

pub fn run(root: &Path, args: &[String]) -> i32 {
    let args = with_contract_check_ids(args);
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return 0;
    }

    match execute_contract_checks(root, &args) {
        Ok(mut receipt) => {
            println!("contract_check: OK");
            receipt["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&receipt));
            println!(
                "{}",
                serde_json::to_string_pretty(&receipt).unwrap_or_else(|_| "{}".to_string())
            );
            0
        }
        Err(error) => {
            eprintln!("contract_check: FAILED");
            eprintln!(" reason: {error}");
            let mut receipt = json!({
                "ok": false,
                "type": "contract_check",
                "error": error,
                "ts": now_iso(),
                "required_check_ids": contract_check_ids_from_args(&args),
            });
            receipt["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&receipt));
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&receipt).unwrap_or_else(|_| "{}".to_string())
            );
            1
        }
    }
}

pub fn with_contract_check_ids(args: &[String]) -> Vec<String> {
    if args
        .iter()
        .any(|arg| arg.starts_with(CHECK_IDS_FLAG_PREFIX))
    {
        return args.to_vec();
    }

    let mut defaults = crate::foundation_contract_gate::FOUNDATION_CONTRACT_CHECK_IDS
        .iter()
        .map(|id| (*id).to_string())
        .collect::<Vec<_>>();
    if !defaults
        .iter()
        .any(|id| id == CHECK_ID_RUST_SOURCE_OF_TRUTH)
    {
        defaults.push(CHECK_ID_RUST_SOURCE_OF_TRUTH.to_string());
    }

    let mut out = args.to_vec();
    out.push(format!("{CHECK_IDS_FLAG_PREFIX}{}", defaults.join(",")));
    out
}

pub fn guard_registry_contract_receipt(source: &str) -> HookCoverageReceipt {
    evaluate_source_hook_coverage(
        CHECK_ID_GUARD_REGISTRY_CONSUMPTION,
        GUARD_REGISTRY_REQUIRED_TOKENS,
        source,
    )
}

pub fn foundation_hook_coverage_receipt(source: &str) -> HookCoverageReceipt {
    evaluate_source_hook_coverage(
        CHECK_ID_FOUNDATION_HOOKS,
        FOUNDATION_HOOK_REQUIRED_TOKENS,
        source,
    )
}

fn print_usage() {
    println!("Usage:");
    println!("  infring-ops contract-check [status] [--help] [--rust-contract-check-ids=<ids>]");
    println!("Environment:");
    println!("  INFRING_RUNTIME_MODE=dist|source");
    println!("  INFRING_RUNTIME_DIST_REQUIRED=1 (required when mode=dist)");
    println!("  CONTRACT_CHECK_DIST_WRAPPER_STRICT=1 (enable dist wrapper existence checks)");
    println!("  CONTRACT_CHECK_DEEP_PROBES=1 (run runtime help probes)");
}

fn contract_check_ids_from_args(args: &[String]) -> Vec<String> {
    args.iter()
        .find_map(|arg| arg.strip_prefix(CHECK_IDS_FLAG_PREFIX))
        .map(|raw| {
            raw.split(',')
                .map(|id| id.trim())
                .filter(|id| !id.is_empty())
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn should_run_rust_subcheck(selected: &HashSet<String>, id: &str) -> bool {
    selected.is_empty() || selected.contains(CHECK_ID_RUST_SOURCE_OF_TRUTH) || selected.contains(id)
}

fn should_run_contract_check(selected: &HashSet<String>, id: &str) -> bool {
    selected.is_empty() || selected.contains(id)
}

fn execute_contract_checks(root: &Path, args: &[String]) -> Result<Value, String> {
    let status_only = args.iter().any(|arg| arg == "status");
    let deep_probes = env_flag("CONTRACT_CHECK_DEEP_PROBES", false);
    let selected_ids = contract_check_ids_from_args(args)
        .into_iter()
        .collect::<HashSet<_>>();
    let mut checks = vec![
        check_dist_runtime_guardrails(root)?,
        check_rust_source_of_truth_contract(root, &selected_ids)?,
        check_guard_registry_contracts(root)?,
        check_source_tokens(
            root,
            CONTRACT_CHECK_SOURCE_REL,
            GUARD_REGISTRY_REQUIRED_TOKENS,
            CHECK_ID_GUARD_REGISTRY_CONSUMPTION,
        )?,
        check_source_tokens(
            root,
            CONTRACT_CHECK_SOURCE_REL,
            FOUNDATION_HOOK_REQUIRED_TOKENS,
            CHECK_ID_FOUNDATION_HOOKS,
        )?,
    ];
    if should_run_contract_check(&selected_ids, CHECK_ID_PROVIDER_DISCOVERY_RUNTIME) {
        checks.push(check_source_tokens(
            root,
            FETCH_RUNTIME_CONTRACT_SOURCE_REL,
            PROVIDER_DISCOVERY_RUNTIME_REQUIRED_TOKENS,
            CHECK_ID_PROVIDER_DISCOVERY_RUNTIME,
        )?);
    }
    if should_run_contract_check(&selected_ids, CHECK_ID_WEB_PROVIDER_FAST_PATH_ARTIFACTS) {
        checks.push(check_source_tokens(
            root,
            SEARCH_RUNTIME_CONTRACT_SOURCE_REL,
            WEB_PROVIDER_FAST_PATH_REQUIRED_TOKENS,
            CHECK_ID_WEB_PROVIDER_FAST_PATH_ARTIFACTS,
        )?);
    }

    if !status_only && deep_probes {
        checks.push(check_script_help_tokens(
            root,
            "client/runtime/systems/sensory/eyes_intake.js",
            PROBE_EYES_INTAKE_HELP_TOKENS,
        )?);
        checks.push(check_script_help_tokens(
            root,
            "client/runtime/systems/security/conflict_marker_guard.js",
            PROBE_CONFLICT_MARKER_HELP_TOKENS,
        )?);
    }

    Ok(json!({
        "ok": true,
        "type": "contract_check",
        "mode": if status_only { "status" } else { "run" },
        "deep_probes": deep_probes,
        "ts": now_iso(),
        "required_check_ids": contract_check_ids_from_args(args),
        "checks": checks,
    }))
}

fn require_object<'a>(
    value: &'a Value,
    field: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .get(field)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("rust_source_of_truth_policy_missing_object:{field}"))
}

fn require_rel_path(section: &serde_json::Map<String, Value>, key: &str) -> Result<String, String> {
    let rel = section
        .get(key)
        .and_then(Value::as_str)
        .map(|raw| raw.trim().to_string())
        .unwrap_or_default();
    if rel.is_empty() {
        return Err(format!("rust_source_of_truth_policy_missing_path:{key}"));
    }
    Ok(rel)
}

fn require_string_array(
    section: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, String> {
    let arr = section
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("rust_source_of_truth_policy_missing_array:{key}"))?;
    let values = arr
        .iter()
        .filter_map(Value::as_str)
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(format!("rust_source_of_truth_policy_empty_array:{key}"));
    }
    Ok(values)
}

fn check_required_tokens_at_path(
    root: &Path,
    rel_path: &str,
    required_tokens: &[String],
    context: &str,
) -> Result<(), String> {
    let path = root.join(rel_path);
    let source = fs::read_to_string(&path)
        .map_err(|err| format!("read_source_failed:{}:{err}", path.display()))?;
    let missing = missing_tokens(&source, required_tokens);
    if !missing.is_empty() {
        return Err(format!(
            "missing_source_tokens:{}:{}:{}",
            context,
            rel_path,
            missing.join(",")
        ));
    }
    Ok(())
}
