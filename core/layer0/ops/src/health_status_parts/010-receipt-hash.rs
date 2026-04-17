// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_iso};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const LANE_ID: &str = "health_status";
const REPLACEMENT: &str = "protheus-ops health-status";
const CRON_JOBS_REL: &str = "client/runtime/config/cron_jobs.json";
const RUST_SOURCE_OF_TRUTH_POLICY_REL: &str =
    "client/runtime/config/rust_source_of_truth_policy.json";
const JSONL_TAIL_MAX_BYTES: usize = 2 * 1024 * 1024;
const SPINE_RUN_FILES_MAX: usize = 7;
const SPINE_METRICS_FRESH_WINDOW_SECONDS: i64 = 24 * 60 * 60;
const DOPAMINE_METRICS_FRESH_WINDOW_SECONDS: i64 = 24 * 60 * 60;
const EXTERNAL_EYES_METRICS_FRESH_WINDOW_SECONDS: i64 = 6 * 60 * 60;
const EXTERNAL_EYES_CROSS_SIGNAL_MIN_EVENTS: u64 = 6;
const ALLOWED_DELIVERY_CHANNELS: &[&str] = &[
    "last",
    "main",
    "inbox",
    "discord",
    "slack",
    "email",
    "pagerduty",
    "stdout",
    "stderr",
    "sms",
];

fn receipt_hash(v: &Value) -> String {
    deterministic_receipt_hash(v)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops health-status status [--dashboard]");
    println!("  protheus-ops health-status run [--dashboard]");
    println!("  protheus-ops health-status dashboard");
}

fn latest_snapshot_path(root: &Path) -> PathBuf {
    root.join("client")
        .join("local")
        .join("state")
        .join("ops")
        .join(LANE_ID)
        .join("latest.json")
}

fn persist_latest(root: &Path, payload: &Value) {
    let path = latest_snapshot_path(root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(payload) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("read_json_failed:{}:{err}", path.display()))?;
    serde_json::from_str::<Value>(&raw)
        .map_err(|err| format!("parse_json_failed:{}:{err}", path.display()))
}

fn read_text_tail(path: &Path, max_bytes: usize) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let total_len = file.metadata().ok()?.len() as usize;
    if total_len == 0 {
        return Some(String::new());
    }
    let read_len = total_len.min(max_bytes.max(1));
    if total_len > read_len {
        file.seek(SeekFrom::End(-(read_len as i64))).ok()?;
    }
    let mut buf = vec![0u8; read_len];
    file.read_exact(&mut buf).ok()?;
    let mut text = String::from_utf8_lossy(&buf).to_string();
    if total_len > read_len {
        if let Some(idx) = text.find('\n') {
            text = text[idx + 1..].to_string();
        }
    }
    Some(text)
}

fn is_ts_bootstrap_wrapper(source: &str) -> bool {
    let mut normalized = source.replace("\r\n", "\n");
    if normalized.starts_with("#!") {
        if let Some((_, rest)) = normalized.split_once('\n') {
            normalized = rest.to_string();
        }
    }
    let trimmed = normalized.trim();
    let without_use_strict = trimmed
        .strip_prefix("\"use strict\";")
        .or_else(|| trimmed.strip_prefix("'use strict';"))
        .unwrap_or(trimmed)
        .trim();
    without_use_strict.contains("ts_bootstrap")
        && without_use_strict.contains(".bootstrap(__filename, module)")
}

fn missing_tokens(text: &str, tokens: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for token in tokens {
        if !text.contains(token) {
            out.push(token.clone());
        }
    }
    out
}

fn check_required_tokens_at_path(
    root: &Path,
    rel_path: &str,
    required_tokens: &[String],
) -> Result<Vec<String>, String> {
    let path = root.join(rel_path);
    let source = fs::read_to_string(&path)
        .map_err(|err| format!("read_source_failed:{}:{err}", path.display()))?;
    Ok(missing_tokens(&source, required_tokens))
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

fn path_has_allowed_prefix(path: &str, prefixes: &[String]) -> bool {
    prefixes.iter().any(|prefix| path.starts_with(prefix))
}

fn audit_rust_source_of_truth(root: &Path) -> Value {
    let policy_path = root.join(RUST_SOURCE_OF_TRUTH_POLICY_REL);
    let policy = match read_json(&policy_path) {
        Ok(v) => v,
        Err(err) => {
            return json!({
                "ok": false,
                "policy_path": RUST_SOURCE_OF_TRUTH_POLICY_REL,
                "error": err,
                "violations": ["policy_unreadable"]
            })
        }
    };

    let mut violations = Vec::<Value>::new();
    let mut checked_paths = Vec::<String>::new();

    let entrypoint_gate = match require_object(&policy, "rust_entrypoint_gate") {
        Ok(v) => v,
        Err(err) => {
            return json!({
                "ok": false,
                "policy_path": RUST_SOURCE_OF_TRUTH_POLICY_REL,
                "error": err,
                "violations": ["policy_invalid"]
            })
        }
    };
    let conduit_gate = match require_object(&policy, "conduit_strict_gate") {
        Ok(v) => v,
        Err(err) => {
            return json!({
                "ok": false,
                "policy_path": RUST_SOURCE_OF_TRUTH_POLICY_REL,
                "error": err,
                "violations": ["policy_invalid"]
            })
        }
    };
    let conduit_budget_gate = match require_object(&policy, "conduit_budget_gate") {
        Ok(v) => v,
        Err(err) => {
            return json!({
                "ok": false,
                "policy_path": RUST_SOURCE_OF_TRUTH_POLICY_REL,
                "error": err,
                "violations": ["policy_invalid"]
            })
        }
    };
    let status_dashboard_gate = match require_object(&policy, "status_dashboard_gate") {
        Ok(v) => v,
        Err(err) => {
            return json!({
                "ok": false,
                "policy_path": RUST_SOURCE_OF_TRUTH_POLICY_REL,
                "error": err,
                "violations": ["policy_invalid"]
            })
        }
    };

    let checks = vec![
        ("rust_entrypoint_gate", entrypoint_gate, ".rs"),
        ("conduit_strict_gate", conduit_gate, ".ts"),
        ("conduit_budget_gate", conduit_budget_gate, ".rs"),
        ("status_dashboard_gate", status_dashboard_gate, ".ts"),
    ];

    for (ctx, section, expected_ext) in checks {
        let rel_path = match require_rel_path(section, "path") {
            Ok(v) => v,
            Err(err) => {
                violations.push(json!({"context": ctx, "reason": err}));
                continue;
            }
        };
        let required_tokens = match require_string_array(section, "required_tokens") {
            Ok(v) => v,
            Err(err) => {
                violations.push(json!({"context": ctx, "reason": err, "path": rel_path}));
                continue;
            }
        };
        if !rel_path.ends_with(expected_ext) {
            violations.push(json!({
                "context": ctx,
                "path": rel_path,
                "reason": "path_extension_mismatch",
                "expected_extension": expected_ext
            }));
            continue;
        }

        match check_required_tokens_at_path(root, &rel_path, &required_tokens) {
            Ok(missing) => {
                if !missing.is_empty() {
                    violations.push(json!({
                        "context": ctx,
                        "path": rel_path,
                        "reason": "missing_source_tokens",
                        "missing_tokens": missing
                    }));
                }
            }
            Err(err) => {
                violations.push(json!({
                    "context": ctx,
                    "path": rel_path,
                    "reason": err
                }));
            }
        }

        checked_paths.push(rel_path);
    }

    let wrapper_contract = match require_object(&policy, "js_wrapper_contract") {
        Ok(v) => v,
        Err(err) => {
            violations.push(json!({"context": "js_wrapper_contract", "reason": err}));
            &serde_json::Map::new()
        }
    };

    if let Ok(wrapper_paths) = require_string_array(wrapper_contract, "required_wrapper_paths") {
        for rel in wrapper_paths {
            if !rel.ends_with(".js") && !rel.ends_with(".ts") {
                violations.push(json!({
                    "context": "js_wrapper_contract",
                    "path": rel,
                    "reason": "wrapper_must_be_ts_or_js"
                }));
                continue;
            }
            let path = root.join(&rel);
            match fs::read_to_string(&path) {
                Ok(source) => {
                    if rel.ends_with(".js") && !is_ts_bootstrap_wrapper(&source) {
                        violations.push(json!({
                            "context": "js_wrapper_contract",
                            "path": rel,
                            "reason": "required_wrapper_not_bootstrap"
                        }));
                    }
                }
                Err(err) => violations.push(json!({
                    "context": "js_wrapper_contract",
                    "path": rel,
                    "reason": format!("read_wrapper_failed:{err}")
                })),
            }
        }
    }

    let shim_contract = match require_object(&policy, "rust_shim_contract") {
        Ok(v) => v,
        Err(err) => {
            violations.push(json!({"context": "rust_shim_contract", "reason": err}));
            &serde_json::Map::new()
        }
    };
    let shim_entries = shim_contract
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if shim_entries.is_empty() {
        violations.push(json!({
            "context": "rust_shim_contract",
            "reason": "rust_source_of_truth_policy_empty_array:entries"
        }));
    }
    for entry in shim_entries {
        let Some(section) = entry.as_object() else {
            violations.push(json!({
                "context": "rust_shim_contract",
                "reason": "rust_source_of_truth_policy_invalid_entry:entries"
            }));
            continue;
        };
        match require_rel_path(section, "path") {
            Ok(rel) => {
                if !rel.ends_with(".js") && !rel.ends_with(".ts") {
                    violations.push(json!({
                        "context": "rust_shim_contract",
                        "path": rel,
                        "reason": "rust_shim_must_be_ts_or_js"
                    }));
                }
                checked_paths.push(rel);
            }
            Err(err) => {
                violations.push(json!({
                    "context": "rust_shim_contract",
                    "reason": err
                }));
            }
        }
    }

    let allowlist_prefixes = policy
        .get("ts_surface_allowlist_prefixes")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if allowlist_prefixes.is_empty() {
        violations.push(json!({
            "context": "ts_surface_allowlist_prefixes",
            "reason": "rust_source_of_truth_policy_empty_array:ts_surface_allowlist_prefixes"
        }));
    }

    for rel in checked_paths.iter().filter(|p| p.ends_with(".ts")) {
        if !path_has_allowed_prefix(rel, &allowlist_prefixes) {
            violations.push(json!({
                "context": "ts_surface_allowlist_prefixes",
                "path": rel,
                "reason": "ts_path_outside_allowlist"
            }));
        }
    }

    json!({
        "ok": violations.is_empty(),
        "policy_path": RUST_SOURCE_OF_TRUTH_POLICY_REL,
        "checked_paths": checked_paths,
        "allowlist_prefixes": allowlist_prefixes,
        "violations": violations
    })
}

fn allowed_delivery_channel(channel: &str) -> bool {
    ALLOWED_DELIVERY_CHANNELS.contains(&channel)
}

