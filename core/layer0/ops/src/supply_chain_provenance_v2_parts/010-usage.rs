// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_iso, parse_args};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const LANE_ID: &str = "supply_chain_provenance_v2";
const DEFAULT_POLICY_REL: &str = "client/runtime/config/supply_chain_provenance_v2_policy.json";
const PROVIDER_FAMILY_CONTRACT_TARGETS: &[&str] =
    &["anthropic", "fal", "google", "minimax", "moonshot"];

#[derive(Debug, Clone)]
struct ArtifactRequirement {
    id: String,
    artifact_path: PathBuf,
    sbom_path: PathBuf,
    signature_path: PathBuf,
}

#[derive(Debug, Clone)]
struct VulnerabilitySla {
    max_critical: u64,
    max_high: u64,
    max_medium: u64,
    max_report_age_hours: i64,
}

#[derive(Debug, Clone)]
struct Policy {
    strict_default: bool,
    required_artifacts: Vec<ArtifactRequirement>,
    bundle_path: PathBuf,
    vulnerability_summary_path: PathBuf,
    rollback_policy_path: PathBuf,
    vulnerability_sla: VulnerabilitySla,
    latest_path: PathBuf,
    history_path: PathBuf,
    policy_path: PathBuf,
}

fn usage() {
    println!("Usage:");
    println!(
        "  infring-ops supply-chain-provenance-v2 prepare [--strict=1|0] [--policy=<path>] [--bundle-path=<path>] [--vuln-summary-path=<path>] [--tag=<id>] [--last-known-good-tag=<id>]"
    );
    println!(
        "  infring-ops supply-chain-provenance-v2 run [--strict=1|0] [--policy=<path>] [--bundle-path=<path>] [--vuln-summary-path=<path>]"
    );
    println!("  infring-ops supply-chain-provenance-v2 status [--policy=<path>]");
    println!(
        "  provider-family contract targets: {}",
        PROVIDER_FAMILY_CONTRACT_TARGETS.join(",")
    );
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn bool_flag(raw: Option<&String>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
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

fn load_json(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn default_required_artifacts(root: &Path) -> Vec<ArtifactRequirement> {
    vec![
        ArtifactRequirement {
            id: "infring-ops".to_string(),
            artifact_path: root.join("target/release/infring-ops"),
            sbom_path: root.join("local/state/release/provenance/sbom/infring-ops.cdx.json"),
            signature_path: root
                .join("local/state/release/provenance/signatures/infring-ops.sig"),
        },
        ArtifactRequirement {
            id: "conduit-daemon".to_string(),
            artifact_path: root.join("target/release/conduit_daemon"),
            sbom_path: root.join("local/state/release/provenance/sbom/conduit_daemon.cdx.json"),
            signature_path: root
                .join("local/state/release/provenance/signatures/conduit_daemon.sig"),
        },
        ArtifactRequirement {
            id: "provider-family-contract-matrix".to_string(),
            artifact_path: root.join("client/runtime/config/rust_source_of_truth_policy.json"),
            sbom_path: root
                .join("local/state/release/provenance/sbom/provider_family_contract_matrix.cdx.json"),
            signature_path: root
                .join("local/state/release/provenance/signatures/provider_family_contract_matrix.sig"),
        },
    ]
}

fn parse_required_artifacts(root: &Path, raw: &Value) -> Vec<ArtifactRequirement> {
    let Some(arr) = raw.get("required_artifacts").and_then(Value::as_array) else {
        return default_required_artifacts(root);
    };

    let mut out = Vec::new();
    for row in arr {
        let Some(obj) = row.as_object() else {
            continue;
        };
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if id.is_empty() {
            continue;
        }

        let artifact_path = resolve_path(
            root,
            obj.get("artifact_path").and_then(Value::as_str),
            "target/release/infring-ops",
        );
        let sbom_path = resolve_path(
            root,
            obj.get("sbom_path").and_then(Value::as_str),
            "local/state/release/provenance/sbom/infring-ops.cdx.json",
        );
        let signature_path = resolve_path(
            root,
            obj.get("signature_path").and_then(Value::as_str),
            "local/state/release/provenance/signatures/infring-ops.sig",
        );

        out.push(ArtifactRequirement {
            id,
            artifact_path,
            sbom_path,
            signature_path,
        });
    }

    if out.is_empty() {
        default_required_artifacts(root)
    } else {
        out
    }
}

fn load_policy(root: &Path, policy_override: Option<&String>) -> Policy {
    let policy_path = policy_override
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(DEFAULT_POLICY_REL));
    let raw = load_json(&policy_path);

    let outputs = raw.get("outputs").and_then(Value::as_object);
    let sla = raw.get("vulnerability_sla").and_then(Value::as_object);

    Policy {
        strict_default: raw
            .get("strict_default")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        required_artifacts: parse_required_artifacts(root, &raw),
        bundle_path: resolve_path(
            root,
            raw.get("bundle_path").and_then(Value::as_str),
            "local/state/release/provenance_bundle/latest.json",
        ),
        vulnerability_summary_path: resolve_path(
            root,
            raw.get("vulnerability_summary_path")
                .and_then(Value::as_str),
            "local/state/release/provenance_bundle/dependency_vulnerability_summary.json",
        ),
        rollback_policy_path: resolve_path(
            root,
            raw.get("rollback_policy_path").and_then(Value::as_str),
            "client/runtime/config/release_rollback_policy.json",
        ),
        vulnerability_sla: VulnerabilitySla {
            max_critical: sla
                .and_then(|s| s.get("max_critical"))
                .and_then(Value::as_u64)
                .unwrap_or(0),
            max_high: sla
                .and_then(|s| s.get("max_high"))
                .and_then(Value::as_u64)
                .unwrap_or(0),
            max_medium: sla
                .and_then(|s| s.get("max_medium"))
                .and_then(Value::as_u64)
                .unwrap_or(10),
            max_report_age_hours: sla
                .and_then(|s| s.get("max_report_age_hours"))
                .and_then(Value::as_i64)
                .unwrap_or(24),
        },
        latest_path: resolve_path(
            root,
            outputs
                .and_then(|o| o.get("latest_path"))
                .and_then(Value::as_str),
            "local/state/ops/supply_chain_provenance_v2/latest.json",
        ),
        history_path: resolve_path(
            root,
            outputs
                .and_then(|o| o.get("history_path"))
                .and_then(Value::as_str),
            "local/state/ops/supply_chain_provenance_v2/history.jsonl",
        ),
        policy_path,
    }
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|e| format!("read_for_hash_failed:{}:{e}", path.display()))?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok(format!("{:x}", h.finalize()))
}

fn normalize_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn bundle_artifact_map(bundle: &Value) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    if let Some(arr) = bundle.get("artifacts").and_then(Value::as_array) {
        for row in arr {
            if let Some(id) = row.get("id").and_then(Value::as_str) {
                out.insert(id.to_string(), row.clone());
            }
        }
    }
    out
}

fn read_counts(summary: &Value) -> (u64, u64, u64) {
    if let Some(counts) = summary.get("counts").and_then(Value::as_object) {
        let c = counts.get("critical").and_then(Value::as_u64).unwrap_or(0);
        let h = counts.get("high").and_then(Value::as_u64).unwrap_or(0);
        let m = counts.get("medium").and_then(Value::as_u64).unwrap_or(0);
        return (c, h, m);
    }

    let mut critical = 0u64;
    let mut high = 0u64;
    let mut medium = 0u64;
    for key in ["cargo", "npm"] {
        if let Some(obj) = summary.get(key).and_then(Value::as_object) {
            critical =
                critical.saturating_add(obj.get("critical").and_then(Value::as_u64).unwrap_or(0));
            high = high.saturating_add(obj.get("high").and_then(Value::as_u64).unwrap_or(0));
            medium = medium.saturating_add(obj.get("medium").and_then(Value::as_u64).unwrap_or(0));
        }
    }
    (critical, high, medium)
}

fn report_age_hours(summary: &Value) -> Option<i64> {
    let raw = summary
        .get("generated_at")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if raw.is_empty() {
        return None;
    }
    let parsed = DateTime::parse_from_rfc3339(raw).ok()?;
    let age = Utc::now().signed_duration_since(parsed.with_timezone(&Utc));
    Some(age.num_hours())
}
