// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::contract_lane_utils as lane_utils;
use crate::{clean, deterministic_receipt_hash, now_iso, parse_args};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const REGISTRY_PATH: &str = "planes/contracts/metakernel_primitives_v1.json";
const CELLBUNDLE_SCHEMA_PATH: &str = "planes/contracts/cellbundle.schema.json";
const CELLBUNDLE_EXAMPLE_PATH: &str = "planes/contracts/examples/cellbundle.minimal.json";
const WIT_WORLD_REGISTRY_PATH: &str = "planes/contracts/wit/world_registry_v1.json";
const CAPABILITY_TAXONOMY_PATH: &str = "planes/contracts/capability_effect_taxonomy_v1.json";
const BUDGET_ADMISSION_POLICY_PATH: &str = "planes/contracts/budget_admission_policy_v1.json";
const EPISTEMIC_OBJECT_SCHEMA_PATH: &str = "planes/contracts/epistemic_object_v1.schema.json";
const EFFECT_JOURNAL_POLICY_PATH: &str = "planes/contracts/effect_journal_policy_v1.json";
const SUBSTRATE_REGISTRY_PATH: &str = "planes/contracts/substrate_descriptor_registry_v1.json";
const RADIX_POLICY_GUARD_PATH: &str = "planes/contracts/radix_policy_guard_v1.json";
const QUANTUM_BROKER_DOMAIN_PATH: &str = "planes/contracts/quantum_broker_domain_v1.json";
const NEURAL_CONSENT_KERNEL_PATH: &str = "planes/contracts/neural_consent_kernel_v1.json";
const ATTESTATION_GRAPH_PATH: &str = "planes/contracts/attestation_graph_v1.json";
const DEGRADATION_CONTRACT_PATH: &str = "planes/contracts/degradation_contracts_v1.json";
const EXECUTION_PROFILE_MATRIX_PATH: &str = "planes/contracts/execution_profile_matrix_v1.json";
const VARIANT_PROFILE_DIR: &str = "planes/contracts/variant_profiles";
const MPU_COMPARTMENT_PROFILE_PATH: &str = "planes/contracts/mpu_compartment_profile_v1.json";
const TOP1_SURFACE_REGISTRY_PATH: &str = "proofs/layer0/core_formal_coverage_map.json";
const CONDUIT_SCHEMA_PATH: &str = "planes/contracts/conduit_envelope.schema.json";
const TLA_BOUNDARY_PATH: &str = "planes/spec/tla/three_plane_boundary.tla";
const DEP_BOUNDARY_MANIFEST: &str = "client/runtime/config/dependency_boundary_manifest.json";
const RUST_SOURCE_OF_TRUTH_POLICY: &str = "client/runtime/config/rust_source_of_truth_policy.json";

const EXPECTED_PRIMITIVES: &[&str] = &[
    "node",
    "cell",
    "task",
    "capability",
    "object",
    "stream",
    "journal",
    "budget",
    "policy",
    "model",
    "supervisor",
    "attestation",
];
const WEB_PROVIDER_CONTRACT_TARGETS: &[&str] = &[
    "brave",
    "duckduckgo",
    "exa",
    "firecrawl",
    "google",
    "minimax",
    "moonshot",
    "perplexity",
    "tavily",
    "xai",
];

fn state_root(root: &Path) -> PathBuf {
    if let Ok(v) = std::env::var("METAKERNEL_STATE_ROOT") {
        let s = v.trim();
        if !s.is_empty() {
            return PathBuf::from(s);
        }
    }
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("metakernel")
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn history_path(root: &Path) -> PathBuf {
    state_root(root).join("history.jsonl")
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn write_json(path: &Path, value: &Value) {
    let _ = lane_utils::write_json(path, value);
}

fn append_jsonl(path: &Path, value: &Value) {
    let _ = lane_utils::append_jsonl(path, value);
}

fn parse_bool(raw: Option<&String>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw.map(String::as_str), fallback)
}

fn is_semver_triplet(raw: &str) -> bool {
    let parts = raw.trim().split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        return false;
    }
    parts
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn is_token_id(raw: &str) -> bool {
    let t = raw.trim();
    !t.is_empty()
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
}

fn normalize_web_provider_target(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "kimi" | "moonshot" => "moonshot".to_string(),
        "grok" | "xai" => "xai".to_string(),
        "duck_duck_go" | "duckduckgo" => "duckduckgo".to_string(),
        "brave_search" | "brave" => "brave".to_string(),
        other => other.to_string(),
    }
}

fn manifest_requires_web_contract(manifest: &Value) -> bool {
    manifest
        .get("capabilities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|row| row.trim().to_ascii_lowercase())
        .any(|cap| {
            cap == "web_search"
                || cap == "web_fetch"
                || cap.contains("web")
                || cap.contains("search")
        })
}

fn print_receipt(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn gather_primitives_from_registry(registry: &Value) -> Result<Vec<String>, String> {
    let version = registry
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if version != "v1" {
        return Err("registry_version_must_be_v1".to_string());
    }
    let Some(primitives) = registry.get("primitives").and_then(Value::as_array) else {
        return Err("registry_missing_primitives_array".to_string());
    };
    let mut out = Vec::new();
    for item in primitives {
        let id = item
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if id.is_empty() {
            return Err("registry_primitive_id_missing".to_string());
        }
        out.push(id);
    }
    Ok(out)
}

fn validate_registry_payload(registry: &Value) -> (bool, Value) {
    let mut errors: Vec<String> = Vec::new();
    if registry
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "metakernel_primitives_registry"
    {
        errors.push("registry_kind_must_be_metakernel_primitives_registry".to_string());
    }
    let primitives = match gather_primitives_from_registry(registry) {
        Ok(items) => items,
        Err(err) => {
            errors.push(err);
            Vec::new()
        }
    };
    let descriptions_ok = registry
        .get("primitives")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .all(|row| {
            row.get("description")
                .and_then(Value::as_str)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        });
    if !descriptions_ok {
        errors.push("registry_primitive_description_required".to_string());
    }

    let mut dedup = BTreeSet::new();
    let mut duplicates = Vec::new();
    for p in &primitives {
        if !is_token_id(p) {
            errors.push("registry_primitive_id_invalid".to_string());
        }
        if !dedup.insert(p.clone()) {
            duplicates.push(p.clone());
        }
    }
    if !duplicates.is_empty() {
        errors.push("registry_duplicate_primitives".to_string());
    }

    let have = dedup;
    let expected: BTreeSet<String> = EXPECTED_PRIMITIVES.iter().map(|v| v.to_string()).collect();
    let missing: Vec<String> = expected.difference(&have).cloned().collect();
    let unknown: Vec<String> = have.difference(&expected).cloned().collect();
    if !missing.is_empty() {
        errors.push("registry_missing_expected_primitives".to_string());
    }
    if primitives.len() != EXPECTED_PRIMITIVES.len() {
        errors.push("registry_primitive_cardinality_mismatch".to_string());
    }

    (
        errors.is_empty(),
        json!({
            "missing_expected": missing,
            "unknown_primitives": unknown,
            "duplicates": duplicates,
            "errors": errors
        }),
    )
}

fn collect_unknown_primitive_usage(root: &Path, valid: &HashSet<String>) -> Vec<Value> {
    fn is_primitive_ref_key(key: &str) -> bool {
        matches!(key, "primitive" | "primitive_id" | "primitiveId")
    }

    fn is_primitive_ref_list_key(key: &str) -> bool {
        matches!(key, "primitives" | "primitive_ids" | "primitiveIds")
    }

    fn walk_json(path: &Path, value: &Value, valid: &HashSet<String>, out: &mut Vec<Value>) {
        match value {
            Value::Object(map) => {
                for (k, v) in map {
                    if is_primitive_ref_key(k) && v.is_string() {
                        let raw = v.as_str().unwrap_or_default().trim().to_ascii_lowercase();
                        if is_token_id(&raw) && !valid.contains(&raw) {
                            out.push(json!({
                                "path": path.display().to_string(),
                                "key": k,
                                "value": raw
                            }));
                        }
                    }
                    if is_primitive_ref_list_key(k) {
                        for row in v.as_array().cloned().unwrap_or_default() {
                            let raw = row.as_str().unwrap_or_default().trim().to_ascii_lowercase();
                            if is_token_id(&raw) && !valid.contains(&raw) {
                                out.push(json!({
                                    "path": path.display().to_string(),
                                    "key": k,
                                    "value": raw
                                }));
                            }
                        }
                    }
                    walk_json(path, v, valid, out);
                }
            }
            Value::Array(arr) => {
                for v in arr {
                    walk_json(path, v, valid, out);
                }
            }
            _ => {}
        }
    }

    let mut out = Vec::new();
    let root_dir = root.join("client/runtime/config");
    for entry in WalkDir::new(&root_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(raw) = fs::read_to_string(entry.path()).ok() else {
            continue;
        };
        let Some(val) = serde_json::from_str::<Value>(&raw).ok() else {
            continue;
        };
        walk_json(entry.path(), &val, valid, &mut out);
    }
    out
}

fn validate_manifest_payload(
    manifest: &Value,
    valid_primitives: &HashSet<String>,
    strict: bool,
) -> (bool, Value) {
    let mut errors = Vec::new();

    let bundle_id = manifest
        .get("bundle_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if bundle_id.is_empty() {
        errors.push("bundle_id_required");
    }

    let version = manifest
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if version.is_empty() {
        errors.push("version_required");
    }

    let world = manifest
        .get("world")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if world.is_empty() {
        errors.push("world_required");
    }

    let caps = manifest
        .get("capabilities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if caps.is_empty() {
        errors.push("capabilities_required");
    }
    let mut unknown_caps = Vec::new();
    for cap in caps {
        let id = cap.as_str().unwrap_or_default().trim().to_ascii_lowercase();
        if id.is_empty() {
            errors.push("capability_id_empty");
            continue;
        }
        if !valid_primitives.contains(&id) {
            unknown_caps.push(id);
        }
    }
    if !unknown_caps.is_empty() {
        errors.push("capabilities_include_unknown_primitive");
    }

    let requires_web_contract = manifest_requires_web_contract(manifest);
    let web_provider = manifest
        .pointer("/web_tooling/provider")
        .and_then(Value::as_str)
        .or_else(|| manifest.get("web_provider").and_then(Value::as_str))
        .map(normalize_web_provider_target)
        .unwrap_or_default();
    let web_discovery_contract = manifest
        .pointer("/web_tooling/discovery_contract")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let web_auth_contract_count = manifest
        .pointer("/web_tooling/auth_contract")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    if strict && requires_web_contract && web_provider.is_empty() {
        errors.push("web_tooling_provider_required");
    }
    if !web_provider.is_empty()
        && !WEB_PROVIDER_CONTRACT_TARGETS
            .iter()
            .any(|target| target == &web_provider.as_str())
    {
        errors.push("web_tooling_provider_invalid");
    }
    if strict && requires_web_contract && !web_discovery_contract {
        errors.push("web_tooling_discovery_contract_required");
    }
    if strict && requires_web_contract && web_auth_contract_count == 0 {
        errors.push("web_tooling_auth_contract_required");
    }

    let budgets = manifest
        .get("budgets")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let budget_fields = [
        "cpu_ms",
        "ram_mb",
        "storage_mb",
        "network_kb",
        "tokens",
        "power_mw",
        "privacy_points",
        "cognitive_load",
    ];
    let mut budget_missing = Vec::new();
    for field in budget_fields {
        let ok = budgets
            .get(field)
            .and_then(Value::as_i64)
            .map(|v| v >= 0)
            .unwrap_or(false);
        if !ok {
            budget_missing.push(field.to_string());
        }
    }
    if !budget_missing.is_empty() {
        errors.push("budgets_missing_or_invalid_fields");
    }

    let provenance = manifest
        .get("provenance")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let provenance_source = provenance
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let provenance_digest = provenance
        .get("digest")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if provenance_source.is_empty() || provenance_digest.is_empty() {
        errors.push("provenance_source_and_digest_required");
    }

    let ok = if strict { errors.is_empty() } else { true };
    (
        ok,
        json!({
            "bundle_id": bundle_id,
            "version": version,
            "world": world,
            "unknown_capabilities": unknown_caps,
            "missing_budget_fields": budget_missing,
            "errors": errors
        }),
    )
}
