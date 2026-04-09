// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use regex::Regex;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_POLICY_REL: &str = "config/security_integrity_policy.json";
const DEFAULT_LOG_REL: &str = "local/state/security/integrity_violations.jsonl";

#[derive(Clone, Debug)]
struct IntegrityPolicy {
    version: String,
    target_roots: Vec<String>,
    target_extensions: Vec<String>,
    protected_files: Vec<String>,
    exclude_paths: Vec<String>,
    hashes: BTreeMap<String, String>,
    sealed_at: Option<String>,
    sealed_by: Option<String>,
    last_approval_note: Option<String>,
}

trait StringFallback {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl StringFallback for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.trim().is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

fn usage() {
    println!("security-integrity-kernel commands:");
    println!(
        "  protheus-ops security-integrity-kernel <load-policy|collect-present-files|verify|seal|append-event> [--payload-base64=<base64_json>]"
    );
}

fn receipt_envelope(kind: &str, ok: bool) -> Value {
    let ts = now_iso();
    json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
    })
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = receipt_envelope(kind, ok);
    out["payload"] = payload;
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let mut out = receipt_envelope(kind, false);
    out["error"] = Value::String(error.to_string());
    out["fail_closed"] = Value::Bool(true);
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("security_integrity_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("security_integrity_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("security_integrity_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("security_integrity_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = as_string(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn as_string_vec(value: Option<&Value>) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(Value::Array(items)) = value {
        for item in items {
            let raw = as_string(Some(item)).replace('\\', "/");
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !out.iter().any(|existing| existing == trimmed) {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

fn workspace_root(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let explicit = clean_text(payload.get("workspace_root"), 520);
    if !explicit.is_empty() {
        return PathBuf::from(explicit);
    }
    let explicit_root = clean_text(payload.get("root"), 520);
    if !explicit_root.is_empty() {
        return PathBuf::from(explicit_root);
    }
    if let Ok(raw) = std::env::var("PROTHEUS_WORKSPACE_ROOT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.to_path_buf()
}

fn runtime_root(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Ok(raw) = std::env::var("PROTHEUS_RUNTIME_ROOT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let workspace = workspace_root(root, payload);
    let candidate = workspace.join("client").join("runtime");
    if candidate.exists() {
        candidate
    } else {
        workspace
    }
}

fn rel_from_runtime(runtime_root: &Path, candidate: &str) -> String {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let path = PathBuf::from(trimmed);
    let rel = if path.is_absolute() {
        match path.strip_prefix(runtime_root) {
            Ok(v) => v.to_string_lossy().replace('\\', "/"),
            Err(_) => trimmed.replace('\\', "/"),
        }
    } else {
        trimmed.replace('\\', "/")
    };
    rel.trim_start_matches("./").to_string()
}

fn resolve_path(runtime_root: &Path, explicit: &str, fallback_rel: &str) -> PathBuf {
    let trimmed = explicit.trim();
    if trimmed.is_empty() {
        return runtime_root.join(fallback_rel);
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        runtime_root.join(trimmed)
    }
}

fn read_json_or_default(file_path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(file_path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_atomic(file_path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("security_integrity_kernel_create_dir_failed:{err}"))?;
    }
    let tmp_path = file_path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    fs::write(
        &tmp_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value)
                .map_err(|err| format!("security_integrity_kernel_encode_failed:{err}"))?
        ),
    )
    .map_err(|err| format!("security_integrity_kernel_write_failed:{err}"))?;
    fs::rename(&tmp_path, file_path)
        .map_err(|err| format!("security_integrity_kernel_rename_failed:{err}"))?;
    Ok(())
}

fn append_jsonl(file_path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("security_integrity_kernel_create_dir_failed:{err}"))?;
    }
    let mut handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(|err| format!("security_integrity_kernel_open_failed:{err}"))?;
    handle
        .write_all(
            format!(
                "{}\n",
                serde_json::to_string(row).unwrap_or_else(|_| "null".to_string())
            )
            .as_bytes(),
        )
        .map_err(|err| format!("security_integrity_kernel_append_failed:{err}"))?;
    Ok(())
}

fn sha256_file(file_path: &Path) -> Result<String, String> {
    let bytes = fs::read(file_path)
        .map_err(|err| format!("security_integrity_kernel_read_file_failed:{err}"))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn path_match(rel: &str, rule: &str) -> bool {
    let normalized = rule.trim().replace('\\', "/");
    if normalized.is_empty() {
        return false;
    }
    if let Some(prefix) = normalized.strip_suffix("/**") {
        rel.starts_with(prefix)
    } else {
        rel == normalized
    }
}

fn is_excluded(rel: &str, policy: &IntegrityPolicy) -> bool {
    policy
        .exclude_paths
        .iter()
        .any(|rule| path_match(rel, rule))
}

fn has_allowed_extension(rel: &str, policy: &IntegrityPolicy) -> bool {
    if policy.target_extensions.is_empty() {
        return true;
    }
    let ext = Path::new(rel)
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| format!(".{}", v.to_ascii_lowercase()))
        .unwrap_or_default();
    policy
        .target_extensions
        .iter()
        .any(|allowed| allowed == &ext)
}

fn sorted_hashes(hashes: &BTreeMap<String, String>) -> Value {
    let mut map = Map::new();
    for (key, value) in hashes {
        map.insert(key.clone(), Value::String(value.clone()));
    }
    Value::Object(map)
}

fn normalize_policy(runtime_root: &Path, raw: &Value) -> IntegrityPolicy {
    let obj = raw.as_object().cloned().unwrap_or_default();
    let target_roots = {
        let raw_roots = as_string_vec(obj.get("target_roots"));
        if raw_roots.is_empty() {
            vec![
                "systems/security".to_string(),
                "config/directives".to_string(),
            ]
        } else {
            raw_roots
                .into_iter()
                .map(|v| rel_from_runtime(runtime_root, &v))
                .collect()
        }
    };
    let target_extensions = {
        let raw_exts = as_string_vec(obj.get("target_extensions"));
        if raw_exts.is_empty() {
            vec![".js".to_string(), ".yaml".to_string(), ".yml".to_string()]
        } else {
            raw_exts
                .into_iter()
                .map(|v| v.to_ascii_lowercase())
                .collect()
        }
    };
    let protected_files = {
        let raw_files = as_string_vec(obj.get("protected_files"));
        if raw_files.is_empty() {
            vec!["lib/directive_resolver.ts".to_string()]
        } else {
            raw_files
                .into_iter()
                .map(|v| rel_from_runtime(runtime_root, &v))
                .collect()
        }
    };
    let exclude_paths = as_string_vec(obj.get("exclude_paths"))
        .into_iter()
        .map(|v| rel_from_runtime(runtime_root, &v))
        .collect::<Vec<_>>();

    let mut hashes = BTreeMap::new();
    if let Some(Value::Object(raw_hashes)) = obj.get("hashes") {
        for (key, value) in raw_hashes {
            let rel = rel_from_runtime(runtime_root, key);
            if rel.is_empty() || rel.starts_with("../") {
                continue;
            }
            let digest = as_string(Some(value)).to_ascii_lowercase();
            if digest.is_empty() {
                continue;
            }
            hashes.insert(rel, digest);
        }
    }

    IntegrityPolicy {
        version: clean_text(obj.get("version"), 64).if_empty_then("1.0"),
        target_roots,
        target_extensions,
        protected_files,
        exclude_paths,
        hashes,
        sealed_at: Some(clean_text(obj.get("sealed_at"), 120)).filter(|v| !v.is_empty()),
        sealed_by: Some(clean_text(obj.get("sealed_by"), 120)).filter(|v| !v.is_empty()),
        last_approval_note: Some(clean_text(obj.get("last_approval_note"), 240))
            .filter(|v| !v.is_empty()),
    }
}

fn load_policy(runtime_root: &Path, policy_path: &Path) -> IntegrityPolicy {
    let raw = read_json_or_default(policy_path, json!({}));
    normalize_policy(runtime_root, &raw)
}

fn resolve_policy(
    runtime_root: &Path,
    policy_path: &Path,
    payload: Option<&Map<String, Value>>,
) -> IntegrityPolicy {
    if let Some(raw_policy) = payload.and_then(|map| map.get("policy")) {
        return normalize_policy(runtime_root, raw_policy);
    }
    load_policy(runtime_root, policy_path)
}

fn policy_to_value(policy: &IntegrityPolicy) -> Value {
    json!({
        "version": policy.version,
        "target_roots": policy.target_roots,
        "target_extensions": policy.target_extensions,
        "protected_files": policy.protected_files,
        "exclude_paths": policy.exclude_paths,
        "hashes": sorted_hashes(&policy.hashes),
        "sealed_at": policy.sealed_at,
        "sealed_by": policy.sealed_by,
        "last_approval_note": policy.last_approval_note
    })
}

fn collect_present_files(runtime_root: &Path, policy: &IntegrityPolicy) -> Vec<String> {
    let mut out = Vec::new();
    for root_rel in &policy.target_roots {
        let abs_root = runtime_root.join(root_rel);
        if !abs_root.exists() {
            continue;
        }
        for entry in WalkDir::new(&abs_root)
            .follow_links(false)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(runtime_root)
                .map(|v| v.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if rel.is_empty() || rel.starts_with("../") {
                continue;
            }
            if is_excluded(&rel, policy) || !has_allowed_extension(&rel, policy) {
                continue;
            }
            if !out.iter().any(|existing| existing == &rel) {
                out.push(rel);
            }
        }
    }
    for rel in &policy.protected_files {
        if is_excluded(rel, policy) {
            continue;
        }
        let abs = runtime_root.join(rel);
        if abs.is_file() && !out.iter().any(|existing| existing == rel) {
            out.push(rel.clone());
        }
    }
    out.sort();
    out
}

fn summarize_violations(violations: &[Value]) -> Value {
    let mut counts = BTreeMap::new();
    for violation in violations {
        let key = violation
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *counts.entry(key).or_insert(0u64) += 1;
    }
    let mut map = Map::new();
    for (key, value) in counts {
        map.insert(key, Value::from(value));
    }
    Value::Object(map)
}

fn verify(
    runtime_root: &Path,
    policy_path: &Path,
    payload: Option<&Map<String, Value>>,
) -> Result<Value, String> {
    let policy = resolve_policy(runtime_root, policy_path, payload);
    let expected_paths = policy.hashes.keys().cloned().collect::<Vec<_>>();
    let present_paths = collect_present_files(runtime_root, &policy);
    let hash_pattern = Regex::new(r"^[a-f0-9]{64}$")
        .map_err(|err| format!("security_integrity_kernel_regex_failed:{err}"))?;
    let mut violations = Vec::new();

    if expected_paths.is_empty() {
        violations.push(json!({
            "type": "policy_unsealed",
            "file": Value::Null,
            "detail": "hashes_empty"
        }));
    }

    for rel in &expected_paths {
        let abs = runtime_root.join(rel);
        if !abs.exists() {
            violations.push(json!({
                "type": "missing_sealed_file",
                "file": rel
            }));
            continue;
        }
        let expected = policy.hashes.get(rel).cloned().unwrap_or_default();
        if !hash_pattern.is_match(&expected) {
            violations.push(json!({
                "type": "invalid_hash_entry",
                "file": rel,
                "expected": expected
            }));
            continue;
        }
        let actual = sha256_file(&abs)?;
        if actual != expected {
            violations.push(json!({
                "type": "hash_mismatch",
                "file": rel,
                "expected": expected,
                "actual": actual
            }));
        }
    }

    for rel in &present_paths {
        if !policy.hashes.contains_key(rel) {
            violations.push(json!({
                "type": "unsealed_file",
                "file": rel
            }));
        }
    }

    for rel in &expected_paths {
        let missing = !present_paths.iter().any(|existing| existing == rel);
        let already_missing = violations.iter().any(|violation| {
            violation.get("type").and_then(Value::as_str) == Some("missing_sealed_file")
                && violation.get("file").and_then(Value::as_str) == Some(rel.as_str())
        });
        if missing && !already_missing {
            violations.push(json!({
                "type": "sealed_file_outside_scope",
                "file": rel
            }));
        }
    }

    Ok(json!({
        "ok": violations.is_empty(),
        "ts": now_iso(),
        "policy_path": policy_path.to_string_lossy(),
        "policy_version": policy.version,
        "checked_present_files": present_paths.len(),
        "expected_files": expected_paths.len(),
        "violations": violations,
        "violation_counts": summarize_violations(&violations)
    }))
}

fn seal(
    runtime_root: &Path,
    policy_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let mut policy = load_policy(runtime_root, policy_path);
    let present = collect_present_files(runtime_root, &policy);
    let mut hashes = BTreeMap::new();
    for rel in &present {
        hashes.insert(rel.clone(), sha256_file(&runtime_root.join(rel))?);
    }
    policy.hashes = hashes;
    policy.sealed_at = Some(now_iso());
    policy.sealed_by = Some(
        clean_text(payload.get("sealed_by"), 120)
            .if_empty_then(&std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())),
    );
    policy.last_approval_note =
        Some(clean_text(payload.get("approval_note"), 240)).filter(|v| !v.is_empty());
    write_json_atomic(policy_path, &policy_to_value(&policy))?;
    Ok(json!({
        "ok": true,
        "policy_path": policy_path.to_string_lossy(),
        "policy_version": policy.version,
        "sealed_files": present.len(),
        "sealed_at": policy.sealed_at,
        "sealed_by": policy.sealed_by
    }))
}

fn append_event(log_path: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let entry = payload
        .get("entry")
        .cloned()
        .unwrap_or_else(|| Value::Object(payload.clone()));
    let row = if let Value::Object(mut map) = entry {
        if !map.contains_key("ts") {
            map.insert("ts".to_string(), Value::String(now_iso()));
        }
        Value::Object(map)
    } else {
        json!({
            "ts": now_iso(),
            "entry": entry
        })
    };
    append_jsonl(log_path, &row)?;
    Ok(json!({
        "ok": true,
        "log_path": log_path.to_string_lossy(),
        "entry": row
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "load-policy".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(&argv[1..]) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("security_integrity_kernel_error", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let runtime_root = runtime_root(root, payload);
    let policy_path = resolve_path(
        &runtime_root,
        &clean_text(payload.get("policy_path"), 520),
        DEFAULT_POLICY_REL,
    );
    let log_path = resolve_path(
        &runtime_root,
        &clean_text(payload.get("log_path"), 520),
        DEFAULT_LOG_REL,
    );

    let result = match command.as_str() {
        "load-policy" => {
            let policy = resolve_policy(&runtime_root, &policy_path, Some(payload));
            Ok(json!({
                "ok": true,
                "policy_path": policy_path.to_string_lossy(),
                "log_path": log_path.to_string_lossy(),
                "policy": policy_to_value(&policy)
            }))
        }
        "collect-present-files" => {
            let policy = resolve_policy(&runtime_root, &policy_path, Some(payload));
            Ok(json!({
                "ok": true,
                "policy_path": policy_path.to_string_lossy(),
                "files": collect_present_files(&runtime_root, &policy)
            }))
        }
        "verify" => verify(&runtime_root, &policy_path, Some(payload)),
        "seal" => seal(&runtime_root, &policy_path, payload),
        "append-event" => append_event(&log_path, payload),
        _ => Err(format!(
            "security_integrity_kernel_unknown_command:{command}"
        )),
    };

    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt(
                &format!("security_integrity_kernel_{}", command.replace('-', "_")),
                payload,
            ));
            0
        }
        Err(err) => {
            print_json_line(&cli_error(
                &format!("security_integrity_kernel_{}", command.replace('-', "_")),
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_runtime_file(root: &Path, rel: &str, contents: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create dir");
        }
        fs::write(path, contents).expect("write file");
    }

    #[test]
    fn security_integrity_kernel_seal_and_verify_round_trip() {
        let temp = tempdir().expect("tempdir");
        let runtime_root = temp.path().join("client").join("runtime");
        fs::create_dir_all(&runtime_root).expect("runtime root");

        write_runtime_file(
            &runtime_root,
            "systems/security/guard.js",
            "module.exports = 1;\n",
        );
        write_runtime_file(
            &runtime_root,
            "config/directives/policy.yaml",
            "mode: strict\n",
        );

        let policy_path = runtime_root.join(DEFAULT_POLICY_REL);
        let payload = Map::new();

        let sealed = seal(&runtime_root, &policy_path, &payload).expect("seal");
        assert_eq!(sealed.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(sealed.get("sealed_files").and_then(Value::as_u64), Some(2));

        let verified = verify(&runtime_root, &policy_path, None).expect("verify");
        assert_eq!(verified.get("ok").and_then(Value::as_bool), Some(true));

        write_runtime_file(
            &runtime_root,
            "systems/security/guard.js",
            "module.exports = 2;\n",
        );
        let broken = verify(&runtime_root, &policy_path, None).expect("verify mismatch");
        assert_eq!(broken.get("ok").and_then(Value::as_bool), Some(false));
        let violations = broken
            .get("violations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(violations
            .iter()
            .any(|row| { row.get("type").and_then(Value::as_str) == Some("hash_mismatch") }));
    }

    #[test]
    fn security_integrity_kernel_appends_log_rows() {
        let temp = tempdir().expect("tempdir");
        let log_path = temp.path().join("integrity.jsonl");
        let mut payload = Map::new();
        payload.insert(
            "entry".to_string(),
            json!({
                "type": "hash_mismatch",
                "file": "systems/security/guard.js"
            }),
        );

        let result = append_event(&log_path, &payload).expect("append");
        assert_eq!(result.get("ok").and_then(Value::as_bool), Some(true));

        let lines = fs::read_to_string(&log_path).expect("read log");
        assert!(lines.contains("hash_mismatch"));
        assert!(lines.contains("guard.js"));
    }
}
