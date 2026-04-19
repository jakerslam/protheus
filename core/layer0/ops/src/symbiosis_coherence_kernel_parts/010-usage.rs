// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("symbiosis-coherence-kernel commands:");
    println!("  protheus-ops symbiosis-coherence-kernel load-policy [--payload-base64=<json>]");
    println!("  protheus-ops symbiosis-coherence-kernel evaluate [--payload-base64=<json>]");
    println!("  protheus-ops symbiosis-coherence-kernel load [--payload-base64=<json>]");
    println!(
        "  protheus-ops symbiosis-coherence-kernel recursion-request [--payload-base64=<json>]"
    );
    println!(
        "  protheus-ops symbiosis-coherence-kernel profile-summary [--payload-base64=<json>]"
    );
    println!(
        "  protheus-ops symbiosis-coherence-kernel profile-update [--payload-base64=<json>]"
    );
    println!(
        "  protheus-ops symbiosis-coherence-kernel profile-reset [--payload-base64=<json>]"
    );
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("symbiosis_coherence_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("symbiosis_coherence_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("symbiosis_coherence_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("symbiosis_coherence_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn clean_text(raw: Option<&Value>, max_len: usize) -> String {
    let input = match raw {
        Some(Value::String(v)) => v.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    };
    let joined = input.split_whitespace().collect::<Vec<_>>().join(" ");
    joined.chars().take(max_len).collect::<String>()
}

fn normalize_token(raw: Option<&Value>, max_len: usize) -> String {
    let lowered = clean_text(raw, max_len).to_ascii_lowercase();
    let mut out = String::new();
    let mut prev_us = false;
    for ch in lowered.chars() {
        let mapped = if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '/' | '-') {
            ch
        } else {
            '_'
        };
        if mapped == '_' {
            if prev_us || out.is_empty() {
                continue;
            }
            prev_us = true;
            out.push('_');
        } else {
            prev_us = false;
            out.push(mapped);
        }
        if out.len() >= max_len {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

fn bool_value(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(v)) => lane_utils::parse_bool(Some(v.as_str()), fallback),
        Some(Value::Number(v)) => v.as_i64().map(|n| n != 0).unwrap_or(fallback),
        _ => fallback,
    }
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

fn clamp_number(value: Option<&Value>, lo: f64, hi: f64, fallback: f64) -> f64 {
    as_f64(value).unwrap_or(fallback).clamp(lo, hi)
}

fn clamp_int(value: Option<&Value>, lo: i64, hi: i64, fallback: i64) -> i64 {
    let parsed = value.and_then(|v| match v {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok())),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    });
    parsed.unwrap_or(fallback).clamp(lo, hi)
}

fn round_to(value: f64, digits: u32) -> f64 {
    let factor = 10_f64.powi(i32::try_from(digits).unwrap_or(6));
    (value * factor).round() / factor
}

fn nested<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cur = value;
    for key in path {
        cur = cur.get(*key)?;
    }
    Some(cur)
}

fn rel_path(root: &Path, path: &Path) -> String {
    lane_utils::rel_path(root, path)
}

fn read_json_value(path: &Path, fallback: Value) -> Value {
    lane_utils::read_json(path).unwrap_or(fallback)
}

fn append_jsonl(path: &Path, payload: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, payload)
}

fn write_json(path: &Path, payload: &Value) -> Result<(), String> {
    lane_utils::write_json(path, payload)
}

fn resolve_path(root: &Path, raw: Option<&Value>, fallback_rel: &str) -> PathBuf {
    let candidate = clean_text(raw, 520);
    let chosen = if candidate.is_empty() {
        fallback_rel.to_string()
    } else {
        candidate
    };
    let normalized = chosen.replace('\\', "/");
    let as_path = PathBuf::from(&normalized);
    if as_path
        .components()
        .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return root.join(fallback_rel);
    }
    if as_path.is_absolute() {
        as_path
    } else {
        root.join(as_path)
    }
}

fn default_policy_path(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("SYMBIOSIS_COHERENCE_POLICY_PATH") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.join("client/runtime/config/symbiosis_coherence_policy.json")
}

fn default_policy(root: &Path, policy_path: &Path) -> Value {
    json!({
        "version": "1.0",
        "enabled": true,
        "shadow_only": true,
        "stale_after_minutes": 30,
        "weights": {
            "identity": 0.34,
            "pre_neuralink": 0.22,
            "behavioral": 0.22,
            "mirror": 0.22
        },
        "thresholds": {
            "low_max": 0.45,
            "medium_max": 0.75,
            "high_min": 0.75,
            "unbounded_min": 0.9,
            "sustained_high_samples": 6
        },
        "recursion": {
            "low_depth": 1,
            "medium_depth": 2,
            "high_base_depth": 4,
            "high_streak_gain_interval": 2,
            "require_granted_consent_for_unbounded": true,
            "require_identity_clear_for_unbounded": true
        },
        "history": {
            "max_recent_scores": 200
        },
        "paths": {
            "state_path": resolve_path(root, None, "local/state/symbiosis/coherence/state.json"),
            "latest_path": resolve_path(root, None, "local/state/symbiosis/coherence/latest.json"),
            "receipts_path": resolve_path(root, None, "local/state/symbiosis/coherence/receipts.jsonl"),
            "identity_latest_path": resolve_path(root, None, "local/state/autonomy/identity_anchor/latest.json"),
            "pre_neuralink_state_path": resolve_path(root, None, "local/state/symbiosis/pre_neuralink_interface/state.json"),
            "deep_symbiosis_state_path": resolve_path(root, None, "local/state/symbiosis/deep_understanding/state.json"),
            "observer_mirror_latest_path": resolve_path(root, None, "local/state/autonomy/observer_mirror/latest.json")
        },
        "policy_path": policy_path,
    })
}

fn normalize_weights(raw: &Value, base: &Value) -> Value {
    let identity = clamp_number(
        raw.get("identity"),
        0.0,
        1.0,
        base["identity"].as_f64().unwrap_or(0.34),
    );
    let pre_neuralink = clamp_number(
        raw.get("pre_neuralink"),
        0.0,
        1.0,
        base["pre_neuralink"].as_f64().unwrap_or(0.22),
    );
    let behavioral = clamp_number(
        raw.get("behavioral"),
        0.0,
        1.0,
        base["behavioral"].as_f64().unwrap_or(0.22),
    );
    let mirror = clamp_number(
        raw.get("mirror"),
        0.0,
        1.0,
        base["mirror"].as_f64().unwrap_or(0.22),
    );
    let total = identity + pre_neuralink + behavioral + mirror;
    if total <= 0.0 {
        return base.clone();
    }
    json!({
        "identity": round_to(identity / total, 6),
        "pre_neuralink": round_to(pre_neuralink / total, 6),
        "behavioral": round_to(behavioral / total, 6),
        "mirror": round_to(mirror / total, 6),
    })
}

fn normalize_policy(raw: &Value, root: &Path, policy_path: &Path) -> Value {
    let base = default_policy(root, policy_path);
    let weights = normalize_weights(raw.get("weights").unwrap_or(&Value::Null), &base["weights"]);
    let thresholds = raw.get("thresholds").unwrap_or(&Value::Null);
    let recursion = raw.get("recursion").unwrap_or(&Value::Null);
    let history = raw.get("history").unwrap_or(&Value::Null);
    let paths = raw.get("paths").unwrap_or(&Value::Null);
    let version = {
        let v = clean_text(raw.get("version"), 32);
        if v.is_empty() {
            "1.0".to_string()
        } else {
            v
        }
    };
    json!({
        "version": version,
        "enabled": raw.get("enabled").map(Value::as_bool).flatten().unwrap_or(true),
        "shadow_only": bool_value(raw.get("shadow_only"), true),
        "stale_after_minutes": clamp_int(raw.get("stale_after_minutes"), 1, 24 * 60, 30),
        "weights": weights,
        "thresholds": {
            "low_max": clamp_number(thresholds.get("low_max"), 0.05, 0.95, 0.45),
            "medium_max": clamp_number(thresholds.get("medium_max"), 0.1, 0.99, 0.75),
            "high_min": clamp_number(thresholds.get("high_min"), 0.1, 0.99, 0.75),
            "unbounded_min": clamp_number(thresholds.get("unbounded_min"), 0.2, 1.0, 0.9),
            "sustained_high_samples": clamp_int(thresholds.get("sustained_high_samples"), 1, 1000, 6)
        },
        "recursion": {
            "low_depth": clamp_int(recursion.get("low_depth"), 1, 10_000, 1),
            "medium_depth": clamp_int(recursion.get("medium_depth"), 1, 10_000, 2),
            "high_base_depth": clamp_int(recursion.get("high_base_depth"), 1, 100_000, 4),
            "high_streak_gain_interval": clamp_int(recursion.get("high_streak_gain_interval"), 1, 10_000, 2),
            "require_granted_consent_for_unbounded": bool_value(recursion.get("require_granted_consent_for_unbounded"), true),
            "require_identity_clear_for_unbounded": bool_value(recursion.get("require_identity_clear_for_unbounded"), true)
        },
        "history": {
            "max_recent_scores": clamp_int(history.get("max_recent_scores"), 10, 10_000, 200)
        },
        "paths": {
            "state_path": resolve_path(root, paths.get("state_path"), "local/state/symbiosis/coherence/state.json"),
            "latest_path": resolve_path(root, paths.get("latest_path"), "local/state/symbiosis/coherence/latest.json"),
            "receipts_path": resolve_path(root, paths.get("receipts_path"), "local/state/symbiosis/coherence/receipts.jsonl"),
            "identity_latest_path": resolve_path(root, paths.get("identity_latest_path"), "local/state/autonomy/identity_anchor/latest.json"),
            "pre_neuralink_state_path": resolve_path(root, paths.get("pre_neuralink_state_path"), "local/state/symbiosis/pre_neuralink_interface/state.json"),
            "deep_symbiosis_state_path": resolve_path(root, paths.get("deep_symbiosis_state_path"), "local/state/symbiosis/deep_understanding/state.json"),
            "observer_mirror_latest_path": resolve_path(root, paths.get("observer_mirror_latest_path"), "local/state/autonomy/observer_mirror/latest.json")
        },
        "policy_path": policy_path,
    })
}

fn load_policy(root: &Path, payload: &Map<String, Value>) -> Value {
    let default_path = default_policy_path(root);
    let policy_path = if let Some(policy_path_value) = payload
        .get("policy_path")
        .or_else(|| payload.get("policyPath"))
    {
        let requested = clean_text(Some(policy_path_value), 520);
        if requested.is_empty() {
            default_path.clone()
        } else {
            let candidate = PathBuf::from(requested);
            if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            }
        }
    } else {
        default_path.clone()
    };
    let raw = payload
        .get("policy")
        .cloned()
        .unwrap_or_else(|| read_json_value(&policy_path, json!({})));
    normalize_policy(&raw, root, &policy_path)
}

fn load_state(policy: &Value) -> Value {
    let state_path = PathBuf::from(policy["paths"]["state_path"].as_str().unwrap_or_default());
    let src = read_json_value(&state_path, json!({}));
    let rows = src
        .get("recent_scores")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let ts = clean_text(row.get("ts"), 60);
            if ts.is_empty() {
                return None;
            }
            let tier = {
                let token = normalize_token(row.get("tier"), 24);
                if token.is_empty() {
                    "low".to_string()
                } else {
                    token
                }
            };
            Some(json!({
                "ts": ts,
                "score": clamp_number(row.get("score"), 0.0, 1.0, 0.0),
                "tier": tier
            }))
        })
        .collect::<Vec<_>>();
    json!({
        "schema_id": "symbiosis_coherence_state",
        "schema_version": "1.0",
        "updated_at": src.get("updated_at").cloned().unwrap_or(Value::Null),
        "runs": clamp_int(src.get("runs"), 0, 1_000_000_000, 0),
        "recent_scores": rows,
    })
}

fn save_state(policy: &Value, state: &Value) -> Result<(), String> {
    let max_recent =
        clamp_int(policy["history"].get("max_recent_scores"), 10, 10_000, 200) as usize;
    let rows = state
        .get("recent_scores")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let start = rows.len().saturating_sub(max_recent);
    let payload = json!({
        "schema_id": "symbiosis_coherence_state",
        "schema_version": "1.0",
        "updated_at": now_iso(),
        "runs": clamp_int(state.get("runs"), 0, 1_000_000_000, 0),
        "recent_scores": rows[start..].to_vec(),
    });
    let state_path = PathBuf::from(policy["paths"]["state_path"].as_str().unwrap_or_default());
    write_json(&state_path, &payload)
}

#[derive(Debug, Clone)]
struct Component {
    score: f64,
    detail: Value,
    source_path: String,
}
