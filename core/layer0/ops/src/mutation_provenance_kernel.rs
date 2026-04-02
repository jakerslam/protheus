// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("mutation-provenance-kernel commands:");
    println!("  protheus-ops mutation-provenance-kernel load-policy [--payload-base64=<json>]");
    println!("  protheus-ops mutation-provenance-kernel normalize-meta --payload-base64=<json>");
    println!("  protheus-ops mutation-provenance-kernel enforce --payload-base64=<json>");
    println!("  protheus-ops mutation-provenance-kernel record-audit --payload-base64=<json>");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true,
    });
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
            .map_err(|err| format!("mutation_provenance_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("mutation_provenance_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("mutation_provenance_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("mutation_provenance_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_object<'a>(value: Option<&'a Value>) -> Option<&'a Map<String, Value>> {
    value.and_then(Value::as_object)
}

fn as_str(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = as_str(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn workspace_root(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("PROTHEUS_WORKSPACE_ROOT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.to_path_buf()
}

fn client_root(root: &Path) -> PathBuf {
    workspace_root(root).join("client")
}

fn runtime_root(root: &Path) -> PathBuf {
    workspace_root(root).join("client").join("runtime")
}

fn normalize_path_string(raw: &str) -> String {
    raw.replace('\\', "/")
}

fn strip_private_prefix(raw: &str) -> &str {
    raw.strip_prefix("/private").unwrap_or(raw)
}

fn strip_prefix_loose<'a>(candidate: &'a str, prefix: &str) -> Option<&'a str> {
    let candidate_norm = strip_private_prefix(candidate);
    let prefix_norm = strip_private_prefix(prefix).trim_end_matches('/');
    if prefix_norm.is_empty() {
        return None;
    }
    if candidate_norm == prefix_norm {
        return Some("");
    }
    let with_sep = format!("{prefix_norm}/");
    candidate_norm.strip_prefix(&with_sep)
}

fn default_policy_path(root: &Path) -> PathBuf {
    let runtime_candidate = runtime_root(root)
        .join("config")
        .join("mutation_provenance_policy.json");
    if runtime_candidate.exists() {
        runtime_candidate
    } else {
        client_root(root)
            .join("config")
            .join("mutation_provenance_policy.json")
    }
}

fn resolve_policy_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let explicit = clean_text(payload.get("policy_path"), 520);
    if !explicit.is_empty() {
        return PathBuf::from(explicit);
    }
    if let Ok(raw) = std::env::var("MUTATION_PROVENANCE_POLICY_PATH") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    default_policy_path(root)
}

fn read_json_safe(file_path: &Path, fallback: Value) -> Value {
    fs::read_to_string(file_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(fallback)
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("mutation_provenance_kernel_create_dir_failed:{err}"))?;
    }
    Ok(())
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("mutation_provenance_kernel_append_open_failed:{err}"))?;
    file.write_all(
        format!(
            "{}\n",
            serde_json::to_string(row).unwrap_or_else(|_| "null".to_string())
        )
        .as_bytes(),
    )
    .map_err(|err| format!("mutation_provenance_kernel_append_failed:{err}"))
}

fn normalize_source(root: &Path, raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        let candidate_norm = normalize_path_string(&candidate.to_string_lossy());
        let explicit_workspace = root.to_path_buf();
        let explicit_client = explicit_workspace.join("client");
        let explicit_runtime = explicit_client.join("runtime");
        for base in [
            explicit_runtime,
            explicit_client,
            explicit_workspace,
            runtime_root(root),
            client_root(root),
            workspace_root(root),
        ] {
            if let Ok(rel) = candidate.strip_prefix(&base) {
                return normalize_path_string(&rel.to_string_lossy());
            }
            let base_norm = normalize_path_string(&base.to_string_lossy());
            if let Some(rel) = strip_prefix_loose(&candidate_norm, &base_norm) {
                return normalize_path_string(rel);
            }
        }
    }
    normalize_path_string(trimmed)
}

pub(crate) fn normalize_meta_value(
    root: &Path,
    meta: Option<&Map<String, Value>>,
    fallback_source: &str,
    default_reason: &str,
) -> Value {
    let src = meta.cloned().unwrap_or_default();
    let source_input = {
        let value = as_str(src.get("source"));
        if !value.is_empty() {
            value
        } else {
            fallback_source.trim().to_string()
        }
    };
    let actor = {
        let value = clean_text(src.get("actor"), 80);
        if value.is_empty() {
            std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
        } else {
            value
        }
    };
    let reason = {
        let value = clean_text(src.get("reason"), 160);
        if value.is_empty() {
            default_reason.trim().to_string()
        } else {
            value
        }
    };
    let source = if source_input.is_empty() {
        String::new()
    } else {
        normalize_source(root, &source_input)
    };

    let mut out = Map::new();
    for (key, value) in src {
        if matches!(key.as_str(), "source" | "actor" | "reason") {
            continue;
        }
        out.insert(key, value);
    }
    out.insert("source".to_string(), Value::String(source));
    out.insert("actor".to_string(), Value::String(actor));
    out.insert("reason".to_string(), Value::String(reason));
    Value::Object(out)
}

fn load_policy(root: &Path, payload: &Map<String, Value>) -> Value {
    let fallback = json!({
        "version": "1.0-fallback",
        "channels": {
            "adaptive": {
                "allowed_source_prefixes": [
                    "systems/adaptive/",
                    "systems/sensory/",
                    "systems/strategy/",
                    "systems/autonomy/",
                    "systems/spine/",
                    "lib/"
                ],
                "require_reason": true
            },
            "memory": {
                "allowed_source_prefixes": [
                    "systems/memory/",
                    "systems/spine/",
                    "systems/adaptive/core/",
                    "lib/"
                ],
                "require_reason": true
            }
        }
    });
    let path = resolve_policy_path(root, payload);
    let mut policy = read_json_safe(&path, fallback.clone());
    if policy.get("channels").and_then(Value::as_object).is_none() {
        policy["channels"] = fallback
            .get("channels")
            .cloned()
            .unwrap_or_else(|| json!({}));
    }
    if policy
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        policy["version"] = Value::String("1.0-fallback".to_string());
    }
    policy
}

fn channel_config(policy: &Value, channel: &str) -> (Vec<String>, bool) {
    let channel_obj = policy
        .get("channels")
        .and_then(Value::as_object)
        .and_then(|row| row.get(channel))
        .and_then(Value::as_object);
    let prefixes = channel_obj
        .and_then(|row| row.get("allowed_source_prefixes"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| normalize_path_string(&as_str(Some(row))))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let require_reason = channel_obj
        .and_then(|row| row.get("require_reason"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    (prefixes, require_reason)
}

fn parse_bool(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(v)) => lane_utils::parse_bool(Some(v.as_str()), fallback),
        Some(Value::Number(n)) => n.as_i64().map(|row| row != 0).unwrap_or(fallback),
        _ => fallback,
    }
}

fn is_strict(channel: &str, opts: Option<&Map<String, Value>>) -> bool {
    if parse_bool(opts.and_then(|row| row.get("strict")), false) {
        return true;
    }
    if std::env::var("MUTATION_PROVENANCE_STRICT")
        .ok()
        .map(|row| lane_utils::parse_bool(Some(row.as_str()), false))
        .unwrap_or(false)
    {
        return true;
    }
    match channel {
        "adaptive" => std::env::var("ADAPTIVE_MUTATION_STRICT")
            .ok()
            .map(|row| lane_utils::parse_bool(Some(row.as_str()), false))
            .unwrap_or(false),
        "memory" => std::env::var("MEMORY_MUTATION_STRICT")
            .ok()
            .map(|row| lane_utils::parse_bool(Some(row.as_str()), false))
            .unwrap_or(false),
        _ => false,
    }
}

fn violation_path(root: &Path, channel: &str) -> PathBuf {
    client_root(root)
        .join("local")
        .join("state")
        .join("security")
        .join(format!("{channel}_mutation_violations.jsonl"))
}

fn audit_path(root: &Path, channel: &str) -> PathBuf {
    client_root(root)
        .join("local")
        .join("state")
        .join("security")
        .join(format!("{channel}_mutations.jsonl"))
}

fn enforce_value(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let channel = clean_text(payload.get("channel"), 64).to_ascii_lowercase();
    let fallback_source = clean_text(payload.get("fallback_source"), 240);
    let default_reason = clean_text(payload.get("default_reason"), 160);
    let opts = as_object(payload.get("opts"));
    let policy = load_policy(root, payload);
    let policy_version = as_str(policy.get("version"));
    let (prefixes, require_reason) = channel_config(&policy, &channel);
    let normalized = normalize_meta_value(
        root,
        as_object(payload.get("meta")),
        &fallback_source,
        &default_reason,
    );
    let normalized_obj = payload_obj(&normalized);
    let source = as_str(normalized_obj.get("source"));
    let mut violations = Vec::<String>::new();

    if source.is_empty() {
        violations.push("missing_source".to_string());
    } else {
        let allowed = prefixes.iter().any(|prefix| {
            let exact = prefix.trim_end_matches('/');
            source == exact || source.starts_with(prefix)
        });
        if !allowed {
            violations.push("source_not_allowlisted".to_string());
        }
    }
    if require_reason && clean_text(normalized_obj.get("reason"), 160).is_empty() {
        violations.push("missing_reason".to_string());
    }

    let out = json!({
        "ok": violations.is_empty(),
        "channel": channel,
        "policy_version": policy_version,
        "meta": normalized,
        "source_rel": source,
        "violations": violations,
    });

    if !out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        append_jsonl(
            &violation_path(
                root,
                out.get("channel")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
            ),
            &json!({
                "ts": now_iso(),
                "type": "mutation_provenance_violation",
                "channel": out.get("channel").cloned().unwrap_or(Value::Null),
                "policy_version": out.get("policy_version").cloned().unwrap_or(Value::Null),
                "source": if source.is_empty() { Value::Null } else { Value::String(source.clone()) },
                "reason": normalized_obj.get("reason").cloned().unwrap_or(Value::Null),
                "actor": normalized_obj.get("actor").cloned().unwrap_or(Value::Null),
                "context": clean_text(opts.and_then(|row| row.get("context")), 200),
                "violations": out.get("violations").cloned().unwrap_or_else(|| json!([])),
            }),
        )?;
        if is_strict(
            out.get("channel").and_then(Value::as_str).unwrap_or(""),
            opts,
        ) {
            let reasons = out
                .get("violations")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            return Err(format!(
                "mutation_provenance_blocked:{}:{}",
                out.get("channel")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                reasons
            ));
        }
    }

    Ok(out)
}

fn record_audit_value(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let channel = clean_text(payload.get("channel"), 64).to_ascii_lowercase();
    let row = as_object(payload.get("row")).cloned().unwrap_or_default();
    let mut audit_row = Map::new();
    audit_row.insert("ts".to_string(), Value::String(now_iso()));
    audit_row.insert("channel".to_string(), Value::String(channel.clone()));
    for (key, value) in row {
        audit_row.insert(key, value);
    }
    let target = audit_path(root, &channel);
    append_jsonl(&target, &Value::Object(audit_row))?;
    Ok(json!({
        "ok": true,
        "channel": channel,
        "audit_path": target.to_string_lossy(),
    }))
}

fn run_command(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "load-policy" => {
            let policy = load_policy(root, payload);
            Ok(json!({
                "ok": true,
                "policy": policy,
                "policy_path": resolve_policy_path(root, payload).to_string_lossy(),
            }))
        }
        "normalize-meta" => Ok(json!({
            "ok": true,
            "meta": normalize_meta_value(
                root,
                as_object(payload.get("meta")),
                &clean_text(payload.get("fallback_source"), 240),
                &clean_text(payload.get("default_reason"), 160),
            ),
        })),
        "enforce" => enforce_value(root, payload),
        "record-audit" => record_audit_value(root, payload),
        _ => Err("mutation_provenance_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|row| row.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("mutation_provenance_kernel", &err));
            return 1;
        }
    };
    match run_command(root, command, payload_obj(&payload)) {
        Ok(out) => {
            print_json_line(&cli_receipt("mutation_provenance_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("mutation_provenance_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn normalize_meta_prefers_runtime_relative_source() {
        let tmp = tempdir().expect("tempdir");
        std::env::set_var("PROTHEUS_WORKSPACE_ROOT", tmp.path());
        let runtime_source = tmp
            .path()
            .join("client")
            .join("runtime")
            .join("systems")
            .join("adaptive")
            .join("planner.ts");
        let meta = json!({ "source": runtime_source, "reason": "sync" });
        let normalized = normalize_meta_value(tmp.path(), as_object(Some(&meta)), "", "fallback");
        assert_eq!(
            normalized.get("source").and_then(Value::as_str),
            Some("systems/adaptive/planner.ts")
        );
    }

    #[test]
    fn strict_enforcement_blocks_violation() {
        let tmp = tempdir().expect("tempdir");
        std::env::set_var("PROTHEUS_WORKSPACE_ROOT", tmp.path());
        let policy_path = tmp
            .path()
            .join("client")
            .join("runtime")
            .join("config")
            .join("mutation_provenance_policy.json");
        ensure_parent(&policy_path).expect("policy dir");
        fs::write(
            &policy_path,
            serde_json::to_string_pretty(&json!({
                "version": "test",
                "channels": {
                    "adaptive": {
                        "allowed_source_prefixes": ["systems/adaptive/"],
                        "require_reason": true
                    }
                }
            }))
            .expect("encode"),
        )
        .expect("write policy");
        let payload = json!({
            "channel": "adaptive",
            "meta": { "source": tmp.path().join("bad.ts"), "reason": "" },
            "opts": { "strict": true }
        });
        let err = enforce_value(tmp.path(), payload_obj(&payload)).expect_err("strict block");
        assert!(err.starts_with("mutation_provenance_blocked:adaptive:"));
    }
}
