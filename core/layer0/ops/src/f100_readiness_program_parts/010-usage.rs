// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_iso, parse_args};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const LANE_ID: &str = "f100_readiness_program";
const DEFAULT_POLICY_REL: &str = "client/runtime/config/f100_readiness_program_policy.json";

const EXECUTABLE_LANES: &[&str] = &[
    "V6-F100-004",
    "V6-F100-005",
    "V6-F100-006",
    "V6-F100-007",
    "V6-F100-008",
    "V6-F100-009",
    "V6-F100-010",
    "V6-F100-011",
    "V6-F100-012",
    "V6-F100-035",
    "V6-F100-036",
    "V7-F100-005",
    "V7-F100-006",
    "V7-F100-007",
    "V7-F100-008",
];

#[derive(Debug, Clone)]
struct Policy {
    strict_default: bool,
    state_root: PathBuf,
    latest_path: PathBuf,
    history_path: PathBuf,
    policy_path: PathBuf,
    raw: Value,
}

fn usage() {
    println!("Usage:");
    println!(
        "  infring-ops f100-readiness-program run --lane=<V6-F100-XXX|V7-F100-XXX> [--strict=1|0] [--apply=1|0] [--policy=<path>]"
    );
    println!("  infring-ops f100-readiness-program run-all [--strict=1|0] [--apply=1|0] [--policy=<path>]");
    println!("  infring-ops f100-readiness-program status --lane=<V6-F100-XXX> [--policy=<path>]");
}

fn resolve_path(root: &Path, raw: Option<&str>, fallback: &str) -> PathBuf {
    let token = raw.unwrap_or(fallback).trim();
    if token.is_empty() {
        return root.join(fallback);
    }
    let candidate = PathBuf::from(token);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn bool_flag(raw: Option<&String>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

fn write_text_atomic(path: &Path, text: &str) -> Result<(), String> {
    ensure_parent(path);
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp, text).map_err(|e| format!("write_tmp_failed:{}:{e}", path.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename_tmp_failed:{}:{e}", path.display()))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path);
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open_jsonl_failed:{}:{e}", path.display()))?;
    let line = serde_json::to_string(value).map_err(|e| format!("encode_jsonl_failed:{e}"))?;
    f.write_all(line.as_bytes())
        .and_then(|_| f.write_all(b"\n"))
        .map_err(|e| format!("append_jsonl_failed:{}:{e}", path.display()))
}

fn seed_local_state_artifact(path: &Path, lane: &str, artifact_kind: &str) -> bool {
    let payload = json!({
        "ok": true,
        "seeded": true,
        "lane": lane,
        "artifact_kind": artifact_kind,
        "ts": now_iso(),
        "source": "f100_readiness_program"
    });
    let encoded = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string());
    write_text_atomic(path, &(encoded + "\n")).is_ok()
}
fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim().trim_start_matches('v');
    let mut parts = trimmed.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch_raw = parts.next()?;
    let patch = patch_raw
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|v| v.parse::<u64>().ok())?;
    Some((major, minor, patch))
}

fn lane_is_executable(lane: &str) -> bool {
    EXECUTABLE_LANES
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(lane.trim()))
}

fn sanitize_lane_state_key(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_') && !out.ends_with('_') {
            out.push('_');
        }
        if out.len() >= 96 {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

fn get_lane_policy<'a>(policy: &'a Policy, lane: &str) -> Option<&'a Value> {
    policy
        .raw
        .get("lanes")
        .and_then(Value::as_object)
        .and_then(|o| o.get(lane))
}

fn lane_state_paths(policy: &Policy, lane: &str) -> (PathBuf, PathBuf) {
    let clean = sanitize_lane_state_key(lane);
    let clean = if clean.is_empty() {
        "unknown_lane".to_string()
    } else {
        clean
    };
    (
        policy.state_root.join(&clean).join("latest.json"),
        policy.state_root.join(&clean).join("history.jsonl"),
    )
}

fn persist_lane(policy: &Policy, lane: &str, payload: &Value) -> Result<(), String> {
    if !lane_is_executable(lane) {
        return Err(format!("f100_lane_not_executable:{lane}"));
    }
    let (latest, history) = lane_state_paths(policy, lane);
    write_text_atomic(
        &latest,
        &format!(
            "{}\n",
            serde_json::to_string_pretty(payload)
                .map_err(|e| format!("encode_latest_failed:{e}"))?
        ),
    )?;
    append_jsonl(&history, payload)
}

fn file_contains_all(path: &Path, tokens: &[String]) -> (bool, Vec<String>) {
    let body = fs::read_to_string(path).unwrap_or_default();
    let missing = tokens
        .iter()
        .filter(|t| !body.contains(t.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    (missing.is_empty(), missing)
}

fn lane_004_compliance_bundle(root: &Path, policy: &Policy) -> Value {
    let lane_policy = get_lane_policy(policy, "V6-F100-004")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let control_map_path = resolve_path(
        root,
        lane_policy.get("control_map_path").and_then(Value::as_str),
        "client/runtime/config/compliance_control_map.json",
    );
    let bundle_path = resolve_path(
        root,
        lane_policy.get("bundle_path").and_then(Value::as_str),
        "local/state/ops/compliance_evidence_bundle/latest.json",
    );

    let map = read_json(&control_map_path).unwrap_or_else(|| json!({"controls":[]}));
    let controls = map
        .get("controls")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let now = std::time::SystemTime::now();
    let mut rows = Vec::new();
    for c in controls {
        let id = c.get("id").and_then(Value::as_str).unwrap_or("unknown");
        let max_age_days = c
            .get("max_age_days")
            .and_then(Value::as_u64)
            .unwrap_or(3650);
        let evidence_paths = c
            .get("evidence_paths")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|v| resolve_path(root, Some(v), v))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut missing = Vec::new();
        let mut stale = Vec::new();
        for p in evidence_paths {
            if !p.exists() {
                missing.push(p.to_string_lossy().to_string());
                continue;
            }
            if let Ok(meta) = fs::metadata(&p) {
                if let Ok(modified) = meta.modified() {
                    if let Ok(elapsed) = now.duration_since(modified) {
                        let age_days = elapsed.as_secs() / 86_400;
                        if age_days > max_age_days {
                            stale.push(json!({"path": p, "age_days": age_days, "max_age_days": max_age_days}));
                        }
                    }
                }
            }
        }

        rows.push(json!({
            "id": id,
            "ok": missing.is_empty() && stale.is_empty(),
            "missing": missing,
            "stale": stale
        }));
    }

    let ok = rows
        .iter()
        .all(|r| r.get("ok").and_then(Value::as_bool).unwrap_or(false));

    let bundle = json!({
        "schema_id": "compliance_evidence_bundle",
        "schema_version": "1.0",
        "ts": now_iso(),
        "controls": rows
    });
    let _ = write_text_atomic(
        &bundle_path,
        &(serde_json::to_string_pretty(&bundle).unwrap_or_else(|_| "{}".to_string()) + "\n"),
    );

    json!({
        "ok": ok,
        "lane": "V6-F100-004",
        "type": "f100_compliance_evidence_automation",
        "control_map_path": control_map_path,
        "bundle_path": bundle_path,
        "control_count": bundle.get("controls").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
        "claim_evidence": [
            {
                "id": "compliance_bundle_generated",
                "claim": "control_evidence_bundle_is_generated_and_fail_closed_when_control_evidence_is_missing_or_stale",
                "evidence": {
                    "bundle_path": bundle_path,
                    "ok": ok
                }
            }
        ]
    })
}

fn lane_005_million_user(root: &Path, policy: &Policy) -> Value {
    lane_005_million_user_with_id(root, policy, "V6-F100-005")
}

fn lane_005_million_user_with_id(root: &Path, policy: &Policy, lane: &str) -> Value {
    let lane_policy = get_lane_policy(policy, lane)
        .or_else(|| get_lane_policy(policy, "V6-F100-005"))
        .or_else(|| get_lane_policy(policy, "V7-F100-005"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let profile_path = resolve_path(
        root,
        lane_policy.get("profile_path").and_then(Value::as_str),
        "client/runtime/config/one_million_performance_profile.json",
    );
    let profile = read_json(&profile_path).unwrap_or_else(|| json!({}));
    let budgets = profile.get("budgets").cloned().unwrap_or_else(|| json!({}));
    let observed = profile
        .get("observed")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let p95 = observed
        .get("p95_ms")
        .and_then(Value::as_f64)
        .unwrap_or(9e9);
    let p99 = observed
        .get("p99_ms")
        .and_then(Value::as_f64)
        .unwrap_or(9e9);
    let error_rate = observed
        .get("error_rate")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    let saturation = observed
        .get("saturation_pct")
        .and_then(Value::as_f64)
        .unwrap_or(100.0);
    let cost = observed
        .get("cost_per_request_usd")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);

    let checks = vec![
        json!({"id":"p95_budget","ok": p95 <= budgets.get("p95_ms").and_then(Value::as_f64).unwrap_or(250.0), "value": p95}),
        json!({"id":"p99_budget","ok": p99 <= budgets.get("p99_ms").and_then(Value::as_f64).unwrap_or(500.0), "value": p99}),
        json!({"id":"error_rate_budget","ok": error_rate <= budgets.get("error_rate").and_then(Value::as_f64).unwrap_or(0.01), "value": error_rate}),
        json!({"id":"saturation_budget","ok": saturation <= budgets.get("saturation_pct").and_then(Value::as_f64).unwrap_or(80.0), "value": saturation}),
        json!({"id":"cost_budget","ok": cost <= budgets.get("cost_per_request_usd").and_then(Value::as_f64).unwrap_or(0.05), "value": cost}),
    ];
    let ok = checks
        .iter()
        .all(|r| r.get("ok").and_then(Value::as_bool).unwrap_or(false));

    json!({
        "ok": ok,
        "lane": lane,
        "type": "f100_one_million_harness",
        "profile_path": profile_path,
        "checks": checks,
        "claim_evidence": [
            {
                "id": "one_million_profile_gate",
                "claim": "one_million_user_profile_meets_latency_error_saturation_and_cost_budgets",
                "evidence": {
                    "ok": ok,
                    "profile_path": profile_path
                }
            }
        ]
    })
}

fn lane_006_multi_tenant(root: &Path, policy: &Policy) -> Value {
    lane_006_multi_tenant_with_id(root, policy, "V6-F100-006")
}

fn lane_006_multi_tenant_with_id(root: &Path, policy: &Policy, lane: &str) -> Value {
    let lane_policy = get_lane_policy(policy, lane)
        .or_else(|| get_lane_policy(policy, "V6-F100-006"))
        .or_else(|| get_lane_policy(policy, "V7-F100-006"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let contract_path = resolve_path(
        root,
        lane_policy.get("contract_path").and_then(Value::as_str),
        "client/runtime/config/multi_tenant_isolation_contract.json",
    );
    let adversarial_path = resolve_path(
        root,
        lane_policy.get("adversarial_path").and_then(Value::as_str),
        "local/state/security/multi_tenant_isolation_adversarial/latest.json",
    );

    let adv = read_json(&adversarial_path).unwrap_or_else(|| json!({}));
    let checks = vec![
        json!({"id":"contract_exists","ok": contract_path.exists()}),
        json!({"id":"cross_tenant_leaks_zero","ok": adv.get("cross_tenant_leaks").and_then(Value::as_u64).unwrap_or(1) == 0}),
        json!({"id":"delete_export_tests_pass","ok": adv.get("delete_export_pass").and_then(Value::as_bool).unwrap_or(false)}),
        json!({"id":"classification_enforced","ok": adv.get("classification_enforced").and_then(Value::as_bool).unwrap_or(false)}),
    ];
    let ok = checks
        .iter()
        .all(|r| r.get("ok").and_then(Value::as_bool).unwrap_or(false));

    json!({
        "ok": ok,
        "lane": lane,
        "type": "f100_multi_tenant_isolation",
        "contract_path": contract_path,
        "adversarial_path": adversarial_path,
        "checks": checks,
        "claim_evidence": [
            {
                "id": "isolation_fail_closed",
                "claim": "cross_tenant_isolation_and_data_governance_fail_closed_on_adversarial_violations",
                "evidence": {
                    "ok": ok,
                    "cross_tenant_leaks": adv.get("cross_tenant_leaks").cloned().unwrap_or(Value::Null)
                }
            }
        ]
    })
}

fn lane_007_interface_lifecycle(root: &Path, policy: &Policy) -> Value {
    lane_007_interface_lifecycle_with_id(root, policy, "V6-F100-007")
}

fn lane_007_interface_lifecycle_with_id(root: &Path, policy: &Policy, lane: &str) -> Value {
    let lane_policy = get_lane_policy(policy, lane)
        .or_else(|| get_lane_policy(policy, "V6-F100-007"))
        .or_else(|| get_lane_policy(policy, "V7-F100-007"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let registry_path = resolve_path(
        root,
        lane_policy.get("registry_path").and_then(Value::as_str),
        "client/runtime/config/api_cli_contract_registry.json",
    );
    let changelog_path = resolve_path(
        root,
        Some("docs/workspace/CHANGELOG.md"),
        "docs/workspace/CHANGELOG.md",
    );
    let required_dep_window = lane_policy
        .get("required_deprecation_days")
        .and_then(Value::as_u64)
        .unwrap_or(90);

    let reg = read_json(&registry_path).unwrap_or_else(|| json!({}));
    let changelog = fs::read_to_string(&changelog_path).unwrap_or_default();

    let mut bad_semver = Vec::new();
    let mut bad_deprecation = Vec::new();
    let mut missing_changelog = Vec::new();

    for list_key in ["api_contracts", "cli_contracts"] {
        if let Some(rows) = reg.get(list_key).and_then(Value::as_array) {
            for row in rows {
                let name = row.get("name").and_then(Value::as_str).unwrap_or("unknown");
                let version = row.get("version").and_then(Value::as_str).unwrap_or("");
                if parse_semver(version).is_none() {
                    bad_semver.push(name.to_string());
                }
                let status = row
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let dep_days = row
                    .get("deprecation_window_days")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                if status == "deprecated" && dep_days < required_dep_window {
                    bad_deprecation.push(name.to_string());
                }
                if status == "breaking" && !changelog.contains(name) {
                    missing_changelog.push(name.to_string());
                }
            }
        }
    }

    let checks = vec![
        json!({"id":"registry_exists","ok": registry_path.exists()}),
        json!({"id":"semver_valid","ok": bad_semver.is_empty(), "bad": bad_semver}),
        json!({"id":"deprecation_window_valid","ok": bad_deprecation.is_empty(), "bad": bad_deprecation}),
        json!({"id":"breaking_changes_logged","ok": missing_changelog.is_empty(), "missing": missing_changelog}),
    ];
    let ok = checks
        .iter()
        .all(|r| r.get("ok").and_then(Value::as_bool).unwrap_or(false));

    json!({
        "ok": ok,
        "lane": lane,
        "type": "f100_interface_lifecycle",
        "registry_path": registry_path,
        "checks": checks
    })
}