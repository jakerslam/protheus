// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_POLICY_REL: &str = "client/runtime/config/mech_suit_mode_policy.json";
const DEFAULT_STATUS_REL: &str = "client/runtime/local/state/ops/mech_suit_mode/latest.json";
const DEFAULT_HISTORY_REL: &str = "client/runtime/local/state/ops/mech_suit_mode/history.jsonl";
const DEFAULT_ATTENTION_QUEUE_REL: &str = "client/runtime/local/state/attention/queue.jsonl";
const DEFAULT_ATTENTION_RECEIPTS_REL: &str = "client/runtime/local/state/attention/receipts.jsonl";
const DEFAULT_ATTENTION_LATEST_REL: &str = "client/runtime/local/state/attention/latest.json";
const MAX_PAYLOAD_BYTES: usize = 256 * 1024;

fn usage() {
    println!("mech-suit-mode-kernel commands:");
    println!("  protheus-ops mech-suit-mode-kernel load-policy [--payload-base64=<json>]");
    println!("  protheus-ops mech-suit-mode-kernel approx-token-count [--payload-base64=<json>]");
    println!("  protheus-ops mech-suit-mode-kernel classify-severity [--payload-base64=<json>]");
    println!("  protheus-ops mech-suit-mode-kernel should-emit-console [--payload-base64=<json>]");
    println!("  protheus-ops mech-suit-mode-kernel update-status --payload-base64=<json>");
    println!("  protheus-ops mech-suit-mode-kernel append-attention-event --payload-base64=<json>");
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
        if raw.len() > MAX_PAYLOAD_BYTES {
            return Err(format!("mech_suit_mode_kernel_payload_too_large:{}", raw.len()));
        }
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("mech_suit_mode_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("mech_suit_mode_kernel_payload_base64_decode_failed:{err}"))?;
        if bytes.len() > MAX_PAYLOAD_BYTES {
            return Err(format!(
                "mech_suit_mode_kernel_payload_too_large:{}",
                bytes.len()
            ));
        }
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("mech_suit_mode_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("mech_suit_mode_kernel_payload_decode_failed:{err}"));
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

fn as_array<'a>(value: Option<&'a Value>) -> &'a Vec<Value> {
    value.and_then(Value::as_array).unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Vec::new)
    })
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

fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(n)) => n.as_i64().map(|v| v != 0).unwrap_or(fallback),
        Some(Value::String(v)) => lane_utils::parse_bool(Some(v.as_str()), fallback),
        _ => fallback,
    }
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(v)) => v.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn clamp_i64(value: Option<&Value>, lo: i64, hi: i64, fallback: i64) -> i64 {
    let raw = as_f64(value).unwrap_or(fallback as f64);
    if !raw.is_finite() {
        return fallback;
    }
    raw.floor().clamp(lo as f64, hi as f64) as i64
}

fn round_to(value: f64, digits: u32) -> f64 {
    let factor = 10_f64.powi(i32::try_from(digits).unwrap_or(3));
    (value * factor).round() / factor
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("mech_suit_mode_kernel_create_dir_failed:{err}"))?;
    }
    Ok(())
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = fs::File::create(&tmp)
        .map_err(|err| format!("mech_suit_mode_kernel_tmp_create_failed:{err}"))?;
    file.write_all(
        format!(
            "{}\n",
            serde_json::to_string_pretty(value)
                .map_err(|err| format!("mech_suit_mode_kernel_json_encode_failed:{err}"))?
        )
        .as_bytes(),
    )
    .map_err(|err| format!("mech_suit_mode_kernel_tmp_write_failed:{err}"))?;
    fs::rename(&tmp, path)
        .map_err(|err| format!("mech_suit_mode_kernel_atomic_rename_failed:{err}"))
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("mech_suit_mode_kernel_jsonl_open_failed:{err}"))?;
    file.write_all(
        format!(
            "{}\n",
            serde_json::to_string(row)
                .map_err(|err| format!("mech_suit_mode_kernel_json_encode_failed:{err}"))?
        )
        .as_bytes(),
    )
    .map_err(|err| format!("mech_suit_mode_kernel_jsonl_write_failed:{err}"))
}

fn workspace_root(root: &Path) -> PathBuf {
    if let Some(raw) = std::env::var_os("INFRING_WORKSPACE") {
        let p = PathBuf::from(raw);
        if p.is_absolute() {
            return p;
        }
    }
    root.to_path_buf()
}

fn normalize_relative_token(input: &str) -> String {
    input
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn rewrite_runtime_relative(rel: &str) -> String {
    let rel = normalize_relative_token(rel);
    if rel.is_empty() {
        return rel;
    }
    if rel == "state" || rel.starts_with("state/") || rel.starts_with("local/state/") {
        let suffix = if rel == "state" {
            String::new()
        } else if let Some(rest) = rel.strip_prefix("state/") {
            rest.to_string()
        } else {
            rel.strip_prefix("local/state/").unwrap_or("").to_string()
        };
        return normalize_relative_token(&format!("client/runtime/local/state/{suffix}"));
    }
    if rel == "local" || rel.starts_with("local/") {
        let suffix = if rel == "local" {
            String::new()
        } else {
            rel.strip_prefix("local/").unwrap_or("").to_string()
        };
        return normalize_relative_token(&format!("client/runtime/local/{suffix}"));
    }
    rel
}

fn relative_path_safe(rel: &str) -> bool {
    let normalized = normalize_relative_token(rel);
    if normalized.is_empty() || normalized.contains('\0') {
        return false;
    }
    !normalized
        .split('/')
        .any(|segment| segment.is_empty() || segment == "..")
}

fn resolve_path(root: &Path, raw: &str, fallback_rel: &str) -> PathBuf {
    let expanded = raw
        .replace("${INFRING_WORKSPACE}", &root.to_string_lossy())
        .replace("$INFRING_WORKSPACE", &root.to_string_lossy());
    let candidate = if expanded.trim().is_empty() {
        rewrite_runtime_relative(fallback_rel)
    } else if Path::new(expanded.trim()).is_absolute() {
        let absolute = PathBuf::from(expanded.trim());
        if absolute.starts_with(root) {
            return absolute;
        }
        return root.join(rewrite_runtime_relative(fallback_rel));
    } else {
        rewrite_runtime_relative(expanded.trim())
    };
    if !relative_path_safe(&candidate) {
        return root.join(rewrite_runtime_relative(fallback_rel));
    }
    root.join(candidate)
}

fn text_token(value: Option<&Value>, max_len: usize) -> String {
    clean_text(value, max_len)
}

fn default_policy_value(root: &Path) -> Value {
    json!({
        "version": "1.0",
        "enabled": true,
        "state": {
            "status_path": path_to_rel(root, &root.join(DEFAULT_STATUS_REL)),
            "history_path": path_to_rel(root, &root.join(DEFAULT_HISTORY_REL))
        },
        "spine": {
            "heartbeat_hours": 4,
            "manual_triggers_allowed": false,
            "quiet_non_critical": true,
            "silent_subprocess_output": true,
            "critical_patterns": ["critical", "fail", "failed", "emergency", "blocked", "halt", "violation", "integrity", "outage"]
        },
        "eyes": {
            "push_attention_queue": true,
            "quiet_non_critical": true,
            "attention_queue_path": path_to_rel(root, &root.join(DEFAULT_ATTENTION_QUEUE_REL)),
            "receipts_path": path_to_rel(root, &root.join(DEFAULT_ATTENTION_RECEIPTS_REL)),
            "latest_path": path_to_rel(root, &root.join(DEFAULT_ATTENTION_LATEST_REL)),
            "attention_contract": {
                "max_queue_depth": 2048,
                "ttl_hours": 48,
                "dedupe_window_hours": 24,
                "backpressure_drop_below": "critical",
                "escalate_levels": ["critical"],
                "priority_map": {
                    "critical": 100,
                    "warn": 60,
                    "info": 20
                }
            },
            "push_event_types": ["external_item", "eye_run_failed", "infra_outage_state", "eye_health_quarantine_set", "eye_auto_dormant", "collector_proposal_added"],
            "focus_warn_score": 0.7,
            "critical_error_codes": ["env_blocked", "auth_denied", "integrity_blocked", "transport_blocked"]
        },
        "personas": {
            "ambient_stance": true,
            "auto_apply": true,
            "full_reload": false,
            "cache_path": "client/runtime/local/state/personas/ambient_stance/cache.json",
            "latest_path": "client/runtime/local/state/personas/ambient_stance/latest.json",
            "receipts_path": "client/runtime/local/state/personas/ambient_stance/receipts.jsonl",
            "max_personas": 256,
            "max_patch_bytes": 65536
        },
        "dopamine": {
            "threshold_breach_only": true,
            "surface_levels": ["warn", "critical"]
        },
        "receipts": {
            "silent_unless_critical": true
        }
    })
}

fn path_to_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"))
}

fn normalize_string_array(
    value: Option<&Value>,
    max_len: usize,
    lowercase: bool,
    fallback: &[&str],
) -> Value {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let rows = as_array(value);
    if rows.is_empty() {
        for row in fallback {
            let token = if lowercase {
                row.to_ascii_lowercase()
            } else {
                row.to_string()
            };
            if seen.insert(token.clone()) {
                out.push(Value::String(token));
            }
        }
        return Value::Array(out);
    }
    for row in rows {
        let mut token = text_token(Some(row), max_len);
        if lowercase {
            token = token.to_ascii_lowercase();
        }
        if token.is_empty() {
            continue;
        }
        if seen.insert(token.clone()) {
            out.push(Value::String(token));
        }
    }
    Value::Array(out)
}

fn normalize_policy(raw: Option<&Map<String, Value>>, root: &Path, policy_path: &Path) -> Value {
    let base = default_policy_value(root);
    let base_obj = payload_obj(&base);
    let src = raw.cloned().unwrap_or_default();

    let state_src = as_object(src.get("state"));
    let spine_src = as_object(src.get("spine"));
    let eyes_src = as_object(src.get("eyes"));
    let contract_src = eyes_src.and_then(|v| as_object(v.get("attention_contract")));
    let personas_src = as_object(src.get("personas"));
    let dopamine_src = as_object(src.get("dopamine"));
    let receipts_src = as_object(src.get("receipts"));
    let base_state = as_object(base_obj.get("state")).unwrap();
    let base_spine = as_object(base_obj.get("spine")).unwrap();
    let base_eyes = as_object(base_obj.get("eyes")).unwrap();
    let base_contract = as_object(base_eyes.get("attention_contract")).unwrap();
    let base_personas = as_object(base_obj.get("personas")).unwrap();
    let base_dopamine = as_object(base_obj.get("dopamine")).unwrap();
    let base_receipts = as_object(base_obj.get("receipts")).unwrap();
    let version = text_token(src.get("version"), 40);
    let normalized_version = if version.is_empty() {
        base_obj
            .get("version")
            .cloned()
            .unwrap_or(Value::String("mech-suit-mode/v1".to_string()))
    } else {
        Value::String(version)
    };

    json!({
        "version": normalized_version,
        "enabled": std::env::var("MECH_SUIT_MODE_FORCE")
            .ok()
            .map(|raw| lane_utils::parse_bool(Some(raw.as_str()), as_bool(src.get("enabled"), true)))
            .unwrap_or_else(|| as_bool(src.get("enabled"), true)),
        "state": {
            "status_path": text_token(state_src.and_then(|v| v.get("status_path")), 400)
                .if_empty_then(as_str(base_state.get("status_path"))),
            "history_path": text_token(state_src.and_then(|v| v.get("history_path")), 400)
                .if_empty_then(as_str(base_state.get("history_path")))
        },
        "spine": {
            "heartbeat_hours": clamp_i64(spine_src.and_then(|v| v.get("heartbeat_hours")), 1, 8760, 4),
            "manual_triggers_allowed": as_bool(spine_src.and_then(|v| v.get("manual_triggers_allowed")), false),
            "quiet_non_critical": as_bool(spine_src.and_then(|v| v.get("quiet_non_critical")), as_bool(base_spine.get("quiet_non_critical"), true)),
            "silent_subprocess_output": as_bool(spine_src.and_then(|v| v.get("silent_subprocess_output")), as_bool(base_spine.get("silent_subprocess_output"), true)),
            "critical_patterns": normalize_string_array(spine_src.and_then(|v| v.get("critical_patterns")), 80, true, &["critical", "fail", "failed", "emergency", "blocked", "halt", "violation", "integrity", "outage"])
        },
        "eyes": {
            "push_attention_queue": as_bool(eyes_src.and_then(|v| v.get("push_attention_queue")), as_bool(base_eyes.get("push_attention_queue"), true)),
            "quiet_non_critical": as_bool(eyes_src.and_then(|v| v.get("quiet_non_critical")), as_bool(base_eyes.get("quiet_non_critical"), true)),
            "attention_queue_path": text_token(eyes_src.and_then(|v| v.get("attention_queue_path")), 400).if_empty_then(as_str(base_eyes.get("attention_queue_path"))),
            "receipts_path": text_token(eyes_src.and_then(|v| v.get("receipts_path")), 400).if_empty_then(as_str(base_eyes.get("receipts_path"))),
            "latest_path": text_token(eyes_src.and_then(|v| v.get("latest_path")), 400).if_empty_then(as_str(base_eyes.get("latest_path"))),
            "attention_contract": {
                "max_queue_depth": clamp_i64(contract_src.and_then(|v| v.get("max_queue_depth")), 1, 1_000_000, base_contract.get("max_queue_depth").and_then(Value::as_i64).unwrap_or(2048)),
                "ttl_hours": clamp_i64(contract_src.and_then(|v| v.get("ttl_hours")), 1, 24*365, base_contract.get("ttl_hours").and_then(Value::as_i64).unwrap_or(48)),
                "dedupe_window_hours": clamp_i64(contract_src.and_then(|v| v.get("dedupe_window_hours")), 1, 24*365, base_contract.get("dedupe_window_hours").and_then(Value::as_i64).unwrap_or(24)),
                "backpressure_drop_below": text_token(contract_src.and_then(|v| v.get("backpressure_drop_below")), 24).to_ascii_lowercase().if_empty_then(as_str(base_contract.get("backpressure_drop_below"))),
                "escalate_levels": normalize_string_array(contract_src.and_then(|v| v.get("escalate_levels")), 24, true, &["critical"]),
                "priority_map": {
                    "critical": clamp_i64(contract_src.and_then(|v| v.get("priority_map")).and_then(|v| as_object(Some(v))).and_then(|v| v.get("critical")), 0, 1000, 100),
                    "warn": clamp_i64(contract_src.and_then(|v| v.get("priority_map")).and_then(|v| as_object(Some(v))).and_then(|v| v.get("warn")), 0, 1000, 60),
                    "info": clamp_i64(contract_src.and_then(|v| v.get("priority_map")).and_then(|v| as_object(Some(v))).and_then(|v| v.get("info")), 0, 1000, 20)
                }
            },
            "push_event_types": normalize_string_array(eyes_src.and_then(|v| v.get("push_event_types")), 80, false, &["external_item", "eye_run_failed", "infra_outage_state", "eye_health_quarantine_set", "eye_auto_dormant", "collector_proposal_added"]),
            "focus_warn_score": round_to(as_f64(eyes_src.and_then(|v| v.get("focus_warn_score"))).unwrap_or(0.7).clamp(0.0, 1.0), 3),
            "critical_error_codes": normalize_string_array(eyes_src.and_then(|v| v.get("critical_error_codes")), 80, true, &["env_blocked", "auth_denied", "integrity_blocked", "transport_blocked"])
        },
        "personas": {
            "ambient_stance": as_bool(personas_src.and_then(|v| v.get("ambient_stance")), as_bool(base_personas.get("ambient_stance"), true)),
            "auto_apply": as_bool(personas_src.and_then(|v| v.get("auto_apply")), as_bool(base_personas.get("auto_apply"), true)),
            "full_reload": as_bool(personas_src.and_then(|v| v.get("full_reload")), as_bool(base_personas.get("full_reload"), false)),
            "cache_path": text_token(personas_src.and_then(|v| v.get("cache_path")), 400).if_empty_then(as_str(base_personas.get("cache_path"))),
            "latest_path": text_token(personas_src.and_then(|v| v.get("latest_path")), 400).if_empty_then(as_str(base_personas.get("latest_path"))),
            "receipts_path": text_token(personas_src.and_then(|v| v.get("receipts_path")), 400).if_empty_then(as_str(base_personas.get("receipts_path"))),
            "max_personas": clamp_i64(personas_src.and_then(|v| v.get("max_personas")), 1, 100000, 256),
            "max_patch_bytes": clamp_i64(personas_src.and_then(|v| v.get("max_patch_bytes")), 256, 10_000_000, 65536)
        },
        "dopamine": {
            "threshold_breach_only": as_bool(dopamine_src.and_then(|v| v.get("threshold_breach_only")), as_bool(base_dopamine.get("threshold_breach_only"), true)),
            "surface_levels": normalize_string_array(dopamine_src.and_then(|v| v.get("surface_levels")), 40, true, &["warn", "critical"])
        },
        "receipts": {
            "silent_unless_critical": as_bool(receipts_src.and_then(|v| v.get("silent_unless_critical")), as_bool(base_receipts.get("silent_unless_critical"), true))
        },
        "_policy_path": path_to_rel(root, policy_path),
        "_root": root.to_string_lossy().to_string()
    })
}

trait StringExt {
    fn if_empty_then(self, fallback: String) -> String;
}
