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
    println!("  protheus-ops symbiosis-coherence-kernel recursion-request [--payload-base64=<json>]");
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
        Value::Number(n) => n.as_i64().or_else(|| n.as_u64().and_then(|u| i64::try_from(u).ok())),
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
    let as_path = PathBuf::from(&chosen);
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
    let identity = clamp_number(raw.get("identity"), 0.0, 1.0, base["identity"].as_f64().unwrap_or(0.34));
    let pre_neuralink = clamp_number(raw.get("pre_neuralink"), 0.0, 1.0, base["pre_neuralink"].as_f64().unwrap_or(0.22));
    let behavioral = clamp_number(raw.get("behavioral"), 0.0, 1.0, base["behavioral"].as_f64().unwrap_or(0.22));
    let mirror = clamp_number(raw.get("mirror"), 0.0, 1.0, base["mirror"].as_f64().unwrap_or(0.22));
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
        if v.is_empty() { "1.0".to_string() } else { v }
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
    let policy_path = if let Some(policy_path_value) = payload.get("policy_path").or_else(|| payload.get("policyPath")) {
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
    let raw = payload.get("policy").cloned().unwrap_or_else(|| read_json_value(&policy_path, json!({})));
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
                if token.is_empty() { "low".to_string() } else { token }
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
    let max_recent = clamp_int(policy["history"].get("max_recent_scores"), 10, 10_000, 200) as usize;
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

fn compute_identity_component(policy: &Value, root: &Path) -> Component {
    let path = PathBuf::from(policy["paths"]["identity_latest_path"].as_str().unwrap_or_default());
    let latest = read_json_value(&path, json!({}));
    let summary = latest.get("summary").unwrap_or(&latest);
    let drift_score = clamp_number(
        summary.get("identity_drift_score").or_else(|| latest.get("identity_drift_score")),
        0.0,
        1.0,
        0.5,
    );
    let max_drift = clamp_number(
        summary.get("max_identity_drift_score").or_else(|| latest.get("max_identity_drift_score")),
        0.01,
        1.0,
        0.58,
    );
    let blocked = clamp_int(summary.get("blocked").or_else(|| latest.get("blocked")), 0, 1_000_000, 0);
    let checked = clamp_int(summary.get("checked").or_else(|| latest.get("checked")), 0, 1_000_000, 0);
    let drift_ratio = (drift_score / max_drift.max(0.0001)).clamp(0.0, 1.5);
    let blocked_ratio = if checked > 0 {
        (blocked as f64 / checked as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let score = (1.0 - ((drift_ratio * 0.75) + (blocked_ratio * 0.25))).clamp(0.0, 1.0);
    Component {
        score: round_to(score, 6),
        detail: json!({
            "drift_score": round_to(drift_score, 6),
            "max_drift_score": round_to(max_drift, 6),
            "blocked": blocked,
            "checked": checked,
            "blocked_ratio": round_to(blocked_ratio, 6),
        }),
        source_path: rel_path(root, &path),
    }
}

fn compute_pre_neuralink_component(policy: &Value, root: &Path) -> Component {
    let path = PathBuf::from(policy["paths"]["pre_neuralink_state_path"].as_str().unwrap_or_default());
    let state = read_json_value(&path, json!({}));
    let consent_state = {
        let token = normalize_token(state.get("consent_state"), 40);
        if token.is_empty() { "paused".to_string() } else { token }
    };
    let consent_score = match consent_state.as_str() {
        "granted" => 1.0,
        "paused" => 0.45,
        _ => 0.1,
    };
    let signals_total = clamp_int(state.get("signals_total"), 0, 1_000_000_000, 0);
    let routed_total = clamp_int(state.get("routed_total"), 0, 1_000_000_000, 0);
    let blocked_total = clamp_int(state.get("blocked_total"), 0, 1_000_000_000, 0);
    let routed_ratio = if signals_total > 0 {
        (routed_total as f64 / signals_total as f64).clamp(0.0, 1.0)
    } else if consent_state == "granted" {
        0.7
    } else {
        0.4
    };
    let blocked_ratio = if signals_total > 0 {
        (blocked_total as f64 / signals_total as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let score = ((consent_score * 0.6) + (routed_ratio * 0.3) + ((1.0 - blocked_ratio) * 0.1))
        .clamp(0.0, 1.0);
    Component {
        score: round_to(score, 6),
        detail: json!({
            "consent_state": consent_state,
            "signals_total": signals_total,
            "routed_total": routed_total,
            "blocked_total": blocked_total,
            "routed_ratio": round_to(routed_ratio, 6),
            "blocked_ratio": round_to(blocked_ratio, 6),
        }),
        source_path: rel_path(root, &path),
    }
}

fn compute_behavioral_component(policy: &Value, root: &Path) -> Component {
    let path = PathBuf::from(policy["paths"]["deep_symbiosis_state_path"].as_str().unwrap_or_default());
    let state = read_json_value(&path, json!({}));
    let style = state.get("style").unwrap_or(&Value::Null);
    let samples = clamp_int(state.get("samples"), 0, 1_000_000_000, 0);
    let directness = clamp_number(style.get("directness"), 0.0, 1.0, 0.75);
    let brevity = clamp_number(style.get("brevity"), 0.0, 1.0, 0.7);
    let proactive = clamp_number(style.get("proactive_delta"), 0.0, 1.0, 0.65);
    let sample_score = (samples as f64 / 50.0).clamp(0.0, 1.0);
    let style_score = ((directness + brevity + proactive) / 3.0).clamp(0.0, 1.0);
    let score = ((sample_score * 0.45) + (style_score * 0.55)).clamp(0.0, 1.0);
    Component {
        score: round_to(score, 6),
        detail: json!({
            "samples": samples,
            "sample_score": round_to(sample_score, 6),
            "style": {
                "directness": round_to(directness, 6),
                "brevity": round_to(brevity, 6),
                "proactive_delta": round_to(proactive, 6),
            },
            "style_score": round_to(style_score, 6),
        }),
        source_path: rel_path(root, &path),
    }
}

fn compute_mirror_component(policy: &Value, root: &Path) -> Component {
    let path = PathBuf::from(policy["paths"]["observer_mirror_latest_path"].as_str().unwrap_or_default());
    let latest = read_json_value(&path, json!({}));
    let mood = {
        let token = normalize_token(
            nested(&latest, &["observer", "mood"]).or_else(|| latest.get("mood")),
            40,
        );
        if token.is_empty() { "unknown".to_string() } else { token }
    };
    let mood_score = match mood.as_str() {
        "stable" => 1.0,
        "guarded" => 0.7,
        "strained" => 0.35,
        _ => 0.6,
    };
    let rates = nested(&latest, &["summary", "rates"]).unwrap_or(&Value::Null);
    let ship_rate = clamp_number(rates.get("ship_rate"), 0.0, 1.0, 0.5);
    let hold_rate = clamp_number(rates.get("hold_rate"), 0.0, 1.0, 0.3);
    let score = ((mood_score * 0.5) + (ship_rate * 0.35) + ((1.0 - hold_rate) * 0.15)).clamp(0.0, 1.0);
    Component {
        score: round_to(score, 6),
        detail: json!({
            "mood": mood,
            "mood_score": round_to(mood_score, 6),
            "ship_rate": round_to(ship_rate, 6),
            "hold_rate": round_to(hold_rate, 6),
        }),
        source_path: rel_path(root, &path),
    }
}

fn score_tier(policy: &Value, score: f64) -> &'static str {
    let low_max = clamp_number(policy["thresholds"].get("low_max"), 0.05, 0.95, 0.45);
    let medium_max = clamp_number(policy["thresholds"].get("medium_max"), 0.1, 0.99, 0.75);
    if score < low_max {
        "low"
    } else if score < medium_max {
        "medium"
    } else {
        "high"
    }
}

fn count_consecutive_high(rows: &[Value], high_min: f64) -> i64 {
    let mut streak = 0_i64;
    for row in rows.iter().rev() {
        let score = clamp_number(row.get("score"), 0.0, 1.0, 0.0);
        if score >= high_min {
            streak += 1;
        } else {
            break;
        }
    }
    streak
}

fn compute_allowed_depth(policy: &Value, score: f64, tier: &str, sustained_high_samples: i64) -> i64 {
    if tier == "low" {
        return clamp_int(policy["recursion"].get("low_depth"), 1, 1_000_000, 1);
    }
    if tier == "medium" {
        let low = clamp_number(policy["thresholds"].get("low_max"), 0.05, 0.95, 0.45);
        let medium = clamp_number(policy["thresholds"].get("medium_max"), 0.1, 0.99, 0.75);
        let denom = (medium - low).max(0.0001);
        let progress = ((score - low) / denom).clamp(0.0, 1.0);
        let extra = if progress >= 0.5 { 1 } else { 0 };
        return clamp_int(policy["recursion"].get("medium_depth"), 1, 1_000_000, 2) + extra;
    }
    let base = clamp_int(policy["recursion"].get("high_base_depth"), 1, 1_000_000, 4);
    let gain_interval = clamp_int(
        policy["recursion"].get("high_streak_gain_interval"),
        1,
        1_000_000,
        2,
    );
    let streak_gain = ((sustained_high_samples - 1).max(0) / gain_interval.max(1)).max(0);
    base + streak_gain
}

fn parse_ts(ts: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn is_fresh(ts: Option<&str>, stale_after_minutes: i64) -> bool {
    let Some(raw) = ts else {
        return false;
    };
    let Some(parsed) = parse_ts(raw) else {
        return false;
    };
    parsed >= Utc::now() - Duration::minutes(stale_after_minutes.max(1))
}

fn evaluate_signal(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let policy = load_policy(root, payload);
    if policy["enabled"].as_bool() != Some(true) {
        return Ok(json!({
            "available": false,
            "type": "symbiosis_coherence_signal",
            "ts": now_iso(),
            "policy_path": rel_path(root, Path::new(policy["policy_path"].as_str().unwrap_or_default())),
            "reason": "policy_disabled",
            "shadow_only": true,
        }));
    }

    let identity = compute_identity_component(&policy, root);
    let pre_neuralink = compute_pre_neuralink_component(&policy, root);
    let behavioral = compute_behavioral_component(&policy, root);
    let mirror = compute_mirror_component(&policy, root);

    let weights = &policy["weights"];
    let score = (
        identity.score * clamp_number(weights.get("identity"), 0.0, 1.0, 0.34)
            + pre_neuralink.score * clamp_number(weights.get("pre_neuralink"), 0.0, 1.0, 0.22)
            + behavioral.score * clamp_number(weights.get("behavioral"), 0.0, 1.0, 0.22)
            + mirror.score * clamp_number(weights.get("mirror"), 0.0, 1.0, 0.22)
    )
        .clamp(0.0, 1.0);
    let rounded_score = round_to(score, 6);
    let tier = score_tier(&policy, rounded_score);

    let mut state = load_state(&policy);
    let now = now_iso();
    let mut recent_scores = state
        .get("recent_scores")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    recent_scores.push(json!({
        "ts": now,
        "score": rounded_score,
        "tier": tier,
    }));
    let max_recent = clamp_int(policy["history"].get("max_recent_scores"), 10, 10_000, 200) as usize;
    let start = recent_scores.len().saturating_sub(max_recent);
    let next_recent = recent_scores[start..].to_vec();
    let sustained_high_samples = count_consecutive_high(
        &next_recent,
        clamp_number(policy["thresholds"].get("high_min"), 0.1, 0.99, 0.75),
    );
    let unbounded_allowed_base = rounded_score
        >= clamp_number(policy["thresholds"].get("unbounded_min"), 0.2, 1.0, 0.9)
        && sustained_high_samples
            >= clamp_int(policy["thresholds"].get("sustained_high_samples"), 1, 1000, 6);
    let consent_granted = pre_neuralink
        .detail
        .get("consent_state")
        .and_then(Value::as_str)
        .unwrap_or_default()
        == "granted";
    let identity_clear = identity
        .detail
        .get("blocked")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        <= 0;
    let unbounded_allowed = unbounded_allowed_base
        && (!bool_value(
            policy["recursion"].get("require_granted_consent_for_unbounded"),
            true,
        ) || consent_granted)
        && (!bool_value(
            policy["recursion"].get("require_identity_clear_for_unbounded"),
            true,
        ) || identity_clear);
    let allowed_depth = if unbounded_allowed {
        Value::Null
    } else {
        Value::from(compute_allowed_depth(&policy, rounded_score, tier, sustained_high_samples))
    };

    let payload_out = json!({
        "ok": true,
        "available": true,
        "type": "symbiosis_coherence_signal",
        "ts": now,
        "policy_version": policy["version"],
        "policy_path": rel_path(root, Path::new(policy["policy_path"].as_str().unwrap_or_default())),
        "shadow_only": policy["shadow_only"].as_bool().unwrap_or(true),
        "coherence_score": rounded_score,
        "coherence_tier": tier,
        "component_scores": {
            "identity": identity.score,
            "pre_neuralink": pre_neuralink.score,
            "behavioral": behavioral.score,
            "mirror": mirror.score,
        },
        "components": {
            "identity": identity.detail,
            "pre_neuralink": pre_neuralink.detail,
            "behavioral": behavioral.detail,
            "mirror_feedback": mirror.detail,
        },
        "recursion_gate": {
            "allowed_depth": allowed_depth,
            "unbounded_allowed": unbounded_allowed,
            "sustained_high_samples": sustained_high_samples,
            "required_sustained_high_samples": clamp_int(policy["thresholds"].get("sustained_high_samples"), 1, 1000, 6),
            "high_min_score": clamp_number(policy["thresholds"].get("high_min"), 0.1, 0.99, 0.75),
            "unbounded_min_score": clamp_number(policy["thresholds"].get("unbounded_min"), 0.2, 1.0, 0.9),
        },
        "source_paths": {
            "identity_latest_path": identity.source_path,
            "pre_neuralink_state_path": pre_neuralink.source_path,
            "deep_symbiosis_state_path": behavioral.source_path,
            "observer_mirror_latest_path": mirror.source_path,
            "latest_path": rel_path(root, Path::new(policy["paths"]["latest_path"].as_str().unwrap_or_default())),
        }
    });

    if bool_value(payload.get("persist"), true) {
        state["runs"] = Value::from(clamp_int(state.get("runs"), 0, 1_000_000_000, 0) + 1);
        state["recent_scores"] = Value::Array(next_recent);
        save_state(&policy, &state)?;
        write_json(
            Path::new(policy["paths"]["latest_path"].as_str().unwrap_or_default()),
            &payload_out,
        )?;
        append_jsonl(
            Path::new(policy["paths"]["receipts_path"].as_str().unwrap_or_default()),
            &payload_out,
        )?;
    }

    Ok(payload_out)
}

fn load_signal(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let policy = load_policy(root, payload);
    let refresh = bool_value(payload.get("refresh"), false);
    let latest_path = PathBuf::from(policy["paths"]["latest_path"].as_str().unwrap_or_default());
    if !refresh {
        let latest = read_json_value(&latest_path, Value::Null);
        if latest.is_object()
            && latest.get("available").and_then(Value::as_bool) == Some(true)
            && is_fresh(
                latest.get("ts").and_then(Value::as_str),
                clamp_int(policy.get("stale_after_minutes"), 1, 24 * 60, 30),
            )
        {
            let mut out = latest;
            out["latest_path"] = Value::String(latest_path.display().to_string());
            out["latest_path_rel"] = Value::String(rel_path(root, &latest_path));
            return Ok(out);
        }
    }
    let evaluated = evaluate_signal(root, payload)?;
    let mut out = evaluated;
    out["latest_path"] = Value::String(latest_path.display().to_string());
    out["latest_path_rel"] = Value::String(rel_path(root, &latest_path));
    Ok(out)
}

fn parse_depth_request(raw: Option<&Value>) -> (Option<i64>, bool) {
    match raw {
        None => (Some(1), false),
        Some(value) => {
            let token = normalize_token(Some(value), 40);
            if matches!(token.as_str(), "unbounded" | "infinite" | "max" | "none") {
                return (None, true);
            }
            let depth = clamp_int(Some(value), 1, 1_000_000_000, 1);
            (Some(depth), false)
        }
    }
}

fn recursion_request(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let signal = if let Some(signal) = payload.get("signal") {
        signal.clone()
    } else {
        load_signal(root, payload)?
    };
    let (requested_depth, parsed_unbounded) = parse_depth_request(
        payload.get("requested_depth").or_else(|| payload.get("requestedDepth")),
    );
    let require_unbounded = bool_value(payload.get("require_unbounded"), false) || parsed_unbounded;
    let allowed_depth = signal
        .get("recursion_gate")
        .and_then(|v| v.get("allowed_depth"))
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                v.as_i64().or_else(|| v.as_u64().and_then(|u| i64::try_from(u).ok()))
            }
        });
    let unbounded_allowed = signal
        .get("recursion_gate")
        .and_then(|v| v.get("unbounded_allowed"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut reasons = Vec::new();
    let mut blocked = false;
    if signal.get("available").and_then(Value::as_bool) != Some(true) {
        reasons.push(Value::String("symbiosis_signal_unavailable".to_string()));
    } else {
        if require_unbounded && !unbounded_allowed {
            blocked = true;
            reasons.push(Value::String("symbiosis_unbounded_not_allowed".to_string()));
        }
        if let (Some(requested), Some(allowed)) = (requested_depth, allowed_depth) {
            if requested > allowed {
                blocked = true;
                reasons.push(Value::String("symbiosis_depth_exceeds_allowed".to_string()));
            }
        }
    }

    let shadow_only = if payload.contains_key("shadow_only_override") {
        bool_value(payload.get("shadow_only_override"), true)
    } else {
        signal.get("shadow_only").and_then(Value::as_bool).unwrap_or(true)
    };
    let blocked_hard = blocked && !shadow_only;

    Ok(json!({
        "ok": !blocked_hard,
        "available": signal.get("available").and_then(Value::as_bool).unwrap_or(false),
        "blocked": blocked,
        "blocked_hard": blocked_hard,
        "shadow_violation": blocked && shadow_only,
        "shadow_only": shadow_only,
        "reason_codes": reasons,
        "requested_depth": requested_depth,
        "requested_unbounded": require_unbounded,
        "allowed_depth": allowed_depth,
        "unbounded_allowed": unbounded_allowed,
        "coherence_score": signal.get("coherence_score").and_then(Value::as_f64),
        "coherence_tier": signal.get("coherence_tier").cloned().unwrap_or(Value::Null),
        "sustained_high_samples": signal
            .get("recursion_gate")
            .and_then(|v| v.get("sustained_high_samples"))
            .and_then(Value::as_i64),
        "latest_path_rel": signal.get("latest_path_rel").cloned().unwrap_or_else(|| {
            signal
                .get("source_paths")
                .and_then(|v| v.get("latest_path"))
                .cloned()
                .unwrap_or(Value::Null)
        })
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|v| v.as_str()) else {
        usage();
        return 1;
    };

    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("symbiosis_coherence_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();

    let result = match command {
        "load-policy" => Ok(json!({
            "ok": true,
            "policy": load_policy(root, &payload)
        })),
        "evaluate" => evaluate_signal(root, &payload),
        "load" => load_signal(root, &payload),
        "recursion-request" => recursion_request(root, &payload),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => Err("symbiosis_coherence_kernel_unknown_command".to_string()),
    };

    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt("symbiosis_coherence_kernel", payload));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("symbiosis_coherence_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(root: &Path, rel: &str, value: &Value) {
        let path = root.join(rel);
        lane_utils::write_json(&path, value).unwrap();
    }

    #[test]
    fn evaluate_signal_persists_latest_and_recursion_gate() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let policy_path = root.join("client/runtime/config/symbiosis_coherence_policy.json");
        write(
            root,
            "client/runtime/config/symbiosis_coherence_policy.json",
            &json!({
                "version": "1.0",
                "shadow_only": true,
                "paths": {
                    "state_path": "local/state/symbiosis/coherence/state.json",
                    "latest_path": "local/state/symbiosis/coherence/latest.json",
                    "receipts_path": "local/state/symbiosis/coherence/receipts.jsonl",
                    "identity_latest_path": "local/state/autonomy/identity_anchor/latest.json",
                    "pre_neuralink_state_path": "local/state/symbiosis/pre_neuralink_interface/state.json",
                    "deep_symbiosis_state_path": "local/state/symbiosis/deep_understanding/state.json",
                    "observer_mirror_latest_path": "local/state/autonomy/observer_mirror/latest.json"
                }
            }),
        );
        write(root, "local/state/autonomy/identity_anchor/latest.json", &json!({"summary":{"identity_drift_score":0.12,"max_identity_drift_score":0.58,"blocked":0,"checked":10}}));
        write(root, "local/state/symbiosis/pre_neuralink_interface/state.json", &json!({"consent_state":"granted","signals_total":20,"routed_total":18,"blocked_total":1}));
        write(root, "local/state/symbiosis/deep_understanding/state.json", &json!({"samples":60,"style":{"directness":0.9,"brevity":0.8,"proactive_delta":0.85}}));
        write(root, "local/state/autonomy/observer_mirror/latest.json", &json!({"observer":{"mood":"stable"},"summary":{"rates":{"ship_rate":0.8,"hold_rate":0.1}}}));

        let payload = json!({
            "policy_path": policy_path,
            "persist": true
        });
        let out = evaluate_signal(root, payload.as_object().unwrap()).unwrap();
        assert_eq!(out["available"], Value::Bool(true));
        assert!(out["coherence_score"].as_f64().unwrap() > 0.7);
        assert!(out["recursion_gate"]["allowed_depth"].as_i64().unwrap() >= 3);
        assert!(root.join("local/state/symbiosis/coherence/latest.json").exists());
    }

    #[test]
    fn recursion_request_flags_depth_violation() {
        let signal = json!({
            "available": true,
            "shadow_only": true,
            "coherence_score": 0.82,
            "coherence_tier": "high",
            "latest_path_rel": "local/state/symbiosis/coherence/latest.json",
            "recursion_gate": {
                "allowed_depth": 4,
                "unbounded_allowed": false,
                "sustained_high_samples": 3
            }
        });
        let dir = tempdir().unwrap();
        let payload = json!({
            "signal": signal,
            "requested_depth": 7
        });
        let out = recursion_request(dir.path(), payload.as_object().unwrap()).unwrap();
        assert_eq!(out["blocked"], Value::Bool(true));
        assert_eq!(out["blocked_hard"], Value::Bool(false));
        assert!(out["reason_codes"].as_array().unwrap().iter().any(|v| v == "symbiosis_depth_exceeds_allowed"));
    }
}
