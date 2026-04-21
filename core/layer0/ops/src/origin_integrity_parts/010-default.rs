// SPDX-License-Identifier: Apache-2.0
use crate::{now_iso, parse_args};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_POLICY_REL: &str = "client/runtime/config/origin_integrity_policy.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct OriginIntegrityPolicy {
    version: String,
    strict_default: bool,
    verify_script_relpath: String,
    dependency_boundary_policy_path: String,
    safety_plane_paths: Vec<String>,
    constitution: ConstitutionContract,
    paths: OriginIntegrityPaths,
}

impl Default for OriginIntegrityPolicy {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            strict_default: true,
            verify_script_relpath: "verify.sh".to_string(),
            dependency_boundary_policy_path:
                "client/runtime/config/dependency_boundary_manifest.json".to_string(),
            safety_plane_paths: vec![
                "docs/workspace/AGENT-CONSTITUTION.md".to_string(),
                "client/runtime/config/dependency_boundary_manifest.json".to_string(),
                "client/runtime/config/rust_source_of_truth_policy.json".to_string(),
                "client/runtime/config/constitution_guardian_policy.json".to_string(),
                "client/runtime/config/rsi_bootstrap_policy.json".to_string(),
                "core/layer0/ops/src/main.rs".to_string(),
                "core/layer0/ops/src/contract_check.rs".to_string(),
                "core/layer0/ops/src/foundation_contract_gate.rs".to_string(),
                "core/layer0/ops/src/spine.rs".to_string(),
                "core/layer0/ops/src/contract_lane_utils.rs".to_string(),
                "core/layer0/ops/src/web_conduit_provider_runtime_parts/019-fetch-runtime-resolution.rs".to_string(),
                "core/layer0/ops/src/web_conduit_provider_runtime_parts/021-search-runtime-resolution.rs".to_string(),
                "core/layer0/ops/src/daemon_control_parts/020-dashboard-health-ok.rs".to_string(),
                "client/runtime/config/secret_broker_policy.json".to_string(),
            ],
            constitution: ConstitutionContract::default(),
            paths: OriginIntegrityPaths::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct ConstitutionContract {
    constitution_path: String,
    guardian_policy_path: String,
    rsi_bootstrap_policy_path: String,
}

impl Default for ConstitutionContract {
    fn default() -> Self {
        Self {
            constitution_path: "docs/workspace/AGENT-CONSTITUTION.md".to_string(),
            guardian_policy_path: "client/runtime/config/constitution_guardian_policy.json"
                .to_string(),
            rsi_bootstrap_policy_path: "client/runtime/config/rsi_bootstrap_policy.json"
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct OriginIntegrityPaths {
    latest_path: String,
    receipts_path: String,
    certificate_path: String,
}

impl Default for OriginIntegrityPaths {
    fn default() -> Self {
        Self {
            latest_path: "client/runtime/local/state/security/origin_integrity/latest.json".to_string(),
            receipts_path: "client/runtime/local/state/security/origin_integrity/receipts.jsonl".to_string(),
            certificate_path: "client/runtime/local/state/security/origin_integrity/origin_verify_certificate.json"
                .to_string(),
        }
    }
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops origin-integrity run [--strict=1|0] [--policy=<path>]");
    println!("  protheus-ops origin-integrity status [--policy=<path>]");
    println!("  protheus-ops origin-integrity certificate [--strict=1|0] [--policy=<path>]");
    println!(
        "  protheus-ops origin-integrity seed-bootstrap-verify --certificate=<path> [--policy=<path>]"
    );
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("read_json_failed:{}:{err}", path.display()))?;
    serde_json::from_str::<Value>(&raw)
        .map_err(|err| format!("parse_json_failed:{}:{err}", path.display()))
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("encode_json_failed:{}:{err}", path.display()))?;
    fs::write(&tmp, &payload).map_err(|err| format!("write_tmp_failed:{}:{err}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|err| {
        format!(
            "rename_tmp_failed:{}:{}:{err}",
            tmp.display(),
            path.display()
        )
    })
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let line = serde_json::to_string(value)
        .map_err(|err| format!("encode_jsonl_failed:{}:{err}", path.display()))?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("open_jsonl_failed:{}:{err}", path.display()))?;
    writeln!(file, "{line}").map_err(|err| format!("append_jsonl_failed:{}:{err}", path.display()))
}

fn normalize_rel(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches('/').to_string()
}

fn resolve_path(root: &Path, raw: &str) -> PathBuf {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return root.join(".");
    }
    let candidate = Path::new(cleaned);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(normalize_rel(cleaned))
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("read_file_failed:{}:{err}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

fn parse_last_json(stdout: &str) -> Option<Value> {
    let raw = stdout.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        return Some(value);
    }
    for line in raw.lines().rev() {
        if let Ok(value) = serde_json::from_str::<Value>(line.trim()) {
            return Some(value);
        }
    }
    None
}

fn parse_bool(v: Option<&str>, fallback: bool) -> bool {
    let Some(raw) = v else {
        return fallback;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn load_policy(
    root: &Path,
    policy_path: Option<&str>,
) -> Result<(OriginIntegrityPolicy, PathBuf), String> {
    let path = resolve_path(root, policy_path.unwrap_or(DEFAULT_POLICY_REL));
    if !path.exists() {
        return Ok((OriginIntegrityPolicy::default(), path));
    }
    let raw = fs::read_to_string(&path).map_err(|err| {
        format!(
            "read_origin_integrity_policy_failed:{}:{err}",
            path.display()
        )
    })?;
    let parsed = serde_json::from_str::<OriginIntegrityPolicy>(&raw).map_err(|err| {
        format!(
            "parse_origin_integrity_policy_failed:{}:{err}",
            path.display()
        )
    })?;
    Ok((parsed, path))
}

fn collect_safety_plane_state(root: &Path, policy: &OriginIntegrityPolicy) -> Value {
    let mut entries = Vec::<Value>::new();
    let mut digest_parts = Vec::<String>::new();
    let mut missing_count = 0usize;

    for rel in &policy.safety_plane_paths {
        let rel_norm = normalize_rel(rel);
        let abs = resolve_path(root, &rel_norm);
        if !abs.exists() {
            missing_count += 1;
            entries.push(json!({
                "path": rel_norm,
                "exists": false,
                "sha256": null,
                "missing": true
            }));
            digest_parts.push(format!("{}:missing", rel_norm));
            continue;
        }
        if !abs.is_file() {
            missing_count += 1;
            entries.push(json!({
                "path": rel_norm,
                "exists": true,
                "is_file": false,
                "sha256": null,
                "missing": true
            }));
            digest_parts.push(format!("{}:not_file", rel_norm));
            continue;
        }
        let sha = sha256_file(&abs).unwrap_or_default();
        entries.push(json!({
            "path": rel_norm,
            "exists": true,
            "sha256": sha,
            "missing": false
        }));
        digest_parts.push(format!("{}:{}", rel_norm, sha));
    }

    digest_parts.sort();
    let state_hash = sha256_bytes(digest_parts.join("|").as_bytes());

    json!({
        "state_hash": state_hash,
        "missing_count": missing_count,
        "paths": entries
    })
}

fn run_dependency_boundary_check(root: &Path, policy: &OriginIntegrityPolicy) -> Value {
    let policy_path = resolve_path(root, &policy.dependency_boundary_policy_path);
    let script = root.join("tests/tooling/scripts/ci/dependency_boundary_guard.ts");

    if script.exists() {
        let output = Command::new("node")
            .arg(script.as_os_str())
            .arg("check")
            .arg("--strict=1")
            .arg(format!("--policy={}", policy_path.display()))
            .current_dir(root)
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let payload = parse_last_json(&stdout).unwrap_or_else(|| json!({}));
            let ok =
                out.status.success() && payload.get("ok").and_then(Value::as_bool) == Some(true);
            return json!({
                "ok": ok,
                "status": out.status.code(),
                "policy_path": normalize_rel(&policy_path.to_string_lossy()),
                "payload": payload,
                "stderr": if stderr.is_empty() { Value::Null } else { Value::String(stderr) },
                "engine": "node"
            });
        }
    }

    match run_dependency_boundary_check_native(root, &policy_path) {
        Ok(payload) => json!({
            "ok": payload.get("ok").and_then(Value::as_bool) == Some(true),
            "status": Value::Null,
            "policy_path": normalize_rel(&policy_path.to_string_lossy()),
            "payload": payload,
            "stderr": Value::Null,
            "engine": "native"
        }),
        Err(err) => json!({
            "ok": false,
            "error": "dependency_boundary_guard_spawn_failed",
            "detail": err,
            "policy_path": normalize_rel(&policy_path.to_string_lossy()),
            "engine": "native"
        }),
    }
}

fn json_string_vec(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(normalize_rel)
                .filter(|row| !row.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn json_string_map(value: Option<&Value>) -> BTreeMap<String, Vec<String>> {
    value
        .and_then(Value::as_object)
        .map(|rows| {
            rows.iter()
                .map(|(key, entry)| (key.trim().to_string(), json_string_vec(Some(entry))))
                .collect()
        })
        .unwrap_or_default()
}

fn list_boundary_files(
    root: &Path,
    include_dirs: &[String],
    include_ext: &BTreeSet<String>,
    exclude_contains: &[String],
) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::<PathBuf>::new();
    let mut stack = include_dirs
        .iter()
        .map(|dir| resolve_path(root, dir))
        .filter(|dir| dir.exists())
        .collect::<Vec<_>>();

    while let Some(cur) = stack.pop() {
        let entries = fs::read_dir(&cur).map_err(|err| {
            format!(
                "dependency_boundary_read_dir_failed:{}:{err}",
                cur.display()
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|err| {
                format!(
                    "dependency_boundary_read_dir_entry_failed:{}:{err}",
                    cur.display()
                )
            })?;
            let path = entry.path();
            let rel = normalize_rel(&path.strip_prefix(root).unwrap_or(&path).to_string_lossy());
            if exclude_contains.iter().any(|token| rel.contains(token)) {
                continue;
            }
            let file_type = entry.file_type().map_err(|err| {
                format!(
                    "dependency_boundary_file_type_failed:{}:{err}",
                    path.display()
                )
            })?;
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .map(|v| format!(".{}", v.to_string_lossy().to_ascii_lowercase()))
                .unwrap_or_default();
            if include_ext.contains(&ext) {
                out.push(path);
            }
        }
    }

    out.sort();
    Ok(out)
}

fn detect_layer(rel_path: &str, layers: &BTreeMap<String, Vec<String>>) -> Option<String> {
    for (layer, roots) in layers {
        for root in roots {
            let normalized = normalize_rel(root).trim_end_matches('/').to_string();
            if rel_path == normalized || rel_path.starts_with(&format!("{normalized}/")) {
                return Some(layer.clone());
            }
        }
    }
    None
}
