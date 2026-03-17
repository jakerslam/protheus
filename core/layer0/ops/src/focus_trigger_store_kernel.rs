// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::Utc;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_REL_PATH: &str = "sensory/eyes/focus_triggers.json";
const MUTATION_LOG_REL: &str = "client/runtime/local/state/security/adaptive_mutations.jsonl";
const POINTERS_REL: &str = "client/runtime/local/state/memory/adaptive_pointers.jsonl";
const POINTER_INDEX_REL: &str = "client/runtime/local/state/memory/adaptive_pointer_index.json";

fn usage() {
    println!("focus-trigger-store-kernel commands:");
    println!("  protheus-ops focus-trigger-store-kernel paths [--payload-base64=<json>]");
    println!("  protheus-ops focus-trigger-store-kernel default-state");
    println!("  protheus-ops focus-trigger-store-kernel normalize-state [--payload-base64=<json>]");
    println!("  protheus-ops focus-trigger-store-kernel read-state [--payload-base64=<json>]");
    println!("  protheus-ops focus-trigger-store-kernel ensure-state [--payload-base64=<json>]");
    println!("  protheus-ops focus-trigger-store-kernel set-state --payload-base64=<json>");
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
            .map_err(|err| format!("focus_trigger_store_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("focus_trigger_store_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("focus_trigger_store_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("focus_trigger_store_kernel_payload_decode_failed:{err}"));
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

fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(v)) => matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
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

fn clamp_number(value: Option<&Value>, lo: f64, hi: f64, fallback: f64) -> f64 {
    let raw = as_f64(value).unwrap_or(fallback);
    if !raw.is_finite() {
        return fallback;
    }
    raw.clamp(lo, hi)
}

fn clamp_i64(value: Option<&Value>, lo: i64, hi: i64, fallback: i64) -> i64 {
    let raw = as_f64(value).unwrap_or(fallback as f64);
    if !raw.is_finite() {
        return fallback;
    }
    raw.floor().clamp(lo as f64, hi as f64) as i64
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

fn normalize_key(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    let mut prev_us = false;
    for ch in raw.chars() {
        let lower = ch.to_ascii_lowercase();
        let keep = matches!(lower, 'a'..='z' | '0'..='9' | ':' | '_' | '-');
        if keep {
            out.push(lower);
            prev_us = false;
        } else if !prev_us {
            out.push('_');
            prev_us = true;
        }
        if out.len() >= max_len {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

fn normalize_tag(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in raw.chars() {
        let lower = ch.to_ascii_lowercase();
        let keep = matches!(lower, 'a'..='z' | '0'..='9' | '_' | '-');
        if keep {
            out.push(lower);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
        if out.len() >= 32 {
            break;
        }
    }
    out.trim_matches('-').to_string()
}

fn is_alnum(raw: &str) -> bool {
    !raw.is_empty() && raw.chars().all(|ch| ch.is_ascii_alphanumeric())
}

fn stable_uid(seed: &str, prefix: &str, length: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::new();
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    let mut out = normalize_tag(prefix).replace('-', "");
    let body_len = length.saturating_sub(out.len()).max(8);
    out.push_str(&hex[..body_len.min(hex.len())]);
    out.truncate(length.max(8).min(48));
    out
}

fn random_uid(prefix: &str, length: usize) -> String {
    stable_uid(
        &format!(
            "{}:{}:{}",
            prefix,
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ),
        prefix,
        length,
    )
}

fn workspace_root(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("PROTHEUS_WORKSPACE_ROOT") {
        let raw = raw.trim();
        if !raw.is_empty() {
            return PathBuf::from(raw);
        }
    }
    root.to_path_buf()
}

fn runtime_root(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("PROTHEUS_RUNTIME_ROOT") {
        let raw = raw.trim();
        if !raw.is_empty() {
            return PathBuf::from(raw);
        }
    }
    let workspace = workspace_root(root);
    let candidate = workspace.join("client").join("runtime");
    if candidate.exists() {
        candidate
    } else {
        workspace
    }
}

fn default_abs_path(root: &Path) -> PathBuf {
    runtime_root(root).join("adaptive").join(DEFAULT_REL_PATH)
}

fn mutation_log_path(root: &Path) -> PathBuf {
    workspace_root(root).join(MUTATION_LOG_REL)
}

fn adaptive_pointers_path(root: &Path) -> PathBuf {
    workspace_root(root).join(POINTERS_REL)
}

fn adaptive_pointer_index_path(root: &Path) -> PathBuf {
    workspace_root(root).join(POINTER_INDEX_REL)
}

fn store_abs_path(root: &Path, payload: &Map<String, Value>) -> Result<PathBuf, String> {
    let canonical = default_abs_path(root);
    let raw = clean_text(payload.get("file_path"), 520);
    if raw.is_empty() {
        return Ok(canonical);
    }
    let requested = PathBuf::from(raw);
    let resolved = if requested.is_absolute() {
        requested
    } else {
        workspace_root(root).join(requested)
    };
    if resolved != canonical {
        return Err(format!(
            "focus_trigger_store: path override denied (requested={})",
            resolved.display()
        ));
    }
    Ok(canonical)
}

fn write_json_atomic(file_path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("focus_trigger_store_kernel_create_dir_failed:{err}"))?;
    }
    let tmp_path = file_path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    fs::write(
        &tmp_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value)
                .map_err(|err| format!("focus_trigger_store_kernel_encode_failed:{err}"))?
        ),
    )
    .map_err(|err| format!("focus_trigger_store_kernel_write_failed:{err}"))?;
    fs::rename(&tmp_path, file_path)
        .map_err(|err| format!("focus_trigger_store_kernel_rename_failed:{err}"))?;
    Ok(())
}

fn read_json_value(file_path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(file_path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn append_jsonl(file_path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("focus_trigger_store_kernel_create_dir_failed:{err}"))?;
    }
    let mut handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(|err| format!("focus_trigger_store_kernel_append_open_failed:{err}"))?;
    handle
        .write_all(
            format!(
                "{}\n",
                serde_json::to_string(row).unwrap_or_else(|_| "null".to_string())
            )
            .as_bytes(),
        )
        .map_err(|err| format!("focus_trigger_store_kernel_append_failed:{err}"))?;
    Ok(())
}

fn normalize_terms_array(raw: Option<&Value>, max_count: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    if let Some(Value::Array(rows)) = raw {
        for row in rows {
            let term = normalize_key(&as_str(Some(row)), 120);
            if term.is_empty() || seen.contains(&term) {
                continue;
            }
            seen.insert(term.clone());
            out.push(term);
            if out.len() >= max_count {
                break;
            }
        }
    }
    out
}

fn normalize_term_weights(raw: Option<&Value>, include_terms: &[String], max_weight: i64) -> Value {
    let allowed: BTreeSet<String> = include_terms.iter().cloned().collect();
    let mut out = Map::new();
    if let Some(Value::Object(rows)) = raw {
        for (key, value) in rows {
            let term = normalize_key(key, 120);
            if term.is_empty() || !allowed.contains(&term) {
                continue;
            }
            out.insert(
                term,
                Value::Number(serde_json::Number::from(clamp_i64(
                    Some(value),
                    1,
                    max_weight,
                    1,
                ))),
            );
        }
    }
    for term in allowed {
        out.entry(term)
            .or_insert_with(|| Value::Number(serde_json::Number::from(1)));
    }
    Value::Object(out)
}

fn normalize_eye_lenses(raw: Option<&Value>, policy: &Map<String, Value>) -> Value {
    let max_terms = clamp_i64(policy.get("lens_max_terms"), 4, 64, 16) as usize;
    let max_exclude = clamp_i64(policy.get("lens_max_exclude_terms"), 0, 32, 6) as usize;
    let max_weight = clamp_i64(policy.get("lens_max_weight"), 1, 60, 20);
    let mut out = Map::new();
    if let Some(Value::Object(rows)) = raw {
        for (eye_raw, lens_raw) in rows {
            let eye_id = normalize_key(eye_raw, 120);
            if eye_id.is_empty() {
                continue;
            }
            let lens = as_object(Some(lens_raw));
            let include_terms =
                normalize_terms_array(lens.and_then(|v| v.get("include_terms")), max_terms);
            let exclude_terms =
                normalize_terms_array(lens.and_then(|v| v.get("exclude_terms")), max_exclude)
                    .into_iter()
                    .filter(|term| !include_terms.contains(term))
                    .take(max_exclude)
                    .collect::<Vec<_>>();
            let mut merged = Map::new();
            merged.insert("eye_id".to_string(), Value::String(eye_id.clone()));
            merged.insert(
                "include_terms".to_string(),
                Value::Array(include_terms.iter().cloned().map(Value::String).collect()),
            );
            merged.insert(
                "exclude_terms".to_string(),
                Value::Array(exclude_terms.into_iter().map(Value::String).collect()),
            );
            merged.insert(
                "term_weights".to_string(),
                normalize_term_weights(
                    lens.and_then(|v| v.get("term_weights")),
                    &include_terms,
                    max_weight,
                ),
            );
            merged.insert(
                "baseline_topics".to_string(),
                Value::Array(
                    normalize_terms_array(lens.and_then(|v| v.get("baseline_topics")), max_terms)
                        .into_iter()
                        .map(Value::String)
                        .collect(),
                ),
            );
            merged.insert(
                "focus_hits_total".to_string(),
                Value::Number(serde_json::Number::from(clamp_i64(
                    lens.and_then(|v| v.get("focus_hits_total")),
                    0,
                    100_000_000,
                    0,
                ))),
            );
            merged.insert(
                "update_count".to_string(),
                Value::Number(serde_json::Number::from(clamp_i64(
                    lens.and_then(|v| v.get("update_count")),
                    0,
                    100_000_000,
                    0,
                ))),
            );
            merged.insert(
                "created_ts".to_string(),
                Value::String(as_str(
                    lens.and_then(|v| v.get("created_ts"))
                        .or(Some(&Value::String(now_iso()))),
                )),
            );
            merged.insert(
                "updated_ts".to_string(),
                Value::String(as_str(
                    lens.and_then(|v| v.get("updated_ts"))
                        .or(Some(&Value::String(now_iso()))),
                )),
            );
            out.insert(eye_id, Value::Object(merged));
        }
    }
    Value::Object(out)
}

fn normalize_recent_map(raw: Option<&Value>, policy: &Map<String, Value>) -> Value {
    let max_age_hours = clamp_i64(policy.get("dedupe_window_hours"), 1, 14 * 24, 36);
    let cutoff = Utc::now().timestamp_millis() - (max_age_hours * 60 * 60 * 1000);
    let mut out = Map::new();
    if let Some(Value::Object(rows)) = raw {
        for (key_raw, value) in rows {
            let key = normalize_key(key_raw, 120);
            if key.is_empty() {
                continue;
            }
            let ts_raw = as_str(Some(value));
            let parsed = chrono::DateTime::parse_from_rfc3339(ts_raw.as_str())
                .ok()
                .map(|dt| dt.timestamp_millis());
            match parsed {
                Some(ms) if ms >= cutoff => {
                    out.insert(
                        key,
                        Value::String(
                            chrono::DateTime::<Utc>::from_timestamp_millis(ms)
                                .unwrap_or_else(Utc::now)
                                .to_rfc3339(),
                        ),
                    );
                }
                _ => {}
            }
        }
    }
    Value::Object(out)
}

fn normalize_policy(raw: Option<&Value>) -> Value {
    let src = as_object(raw);
    let value = json!({
        "refresh_hours": clamp_i64(src.and_then(|v| v.get("refresh_hours")), 1, 24, 4),
        "max_triggers": clamp_i64(src.and_then(|v| v.get("max_triggers")), 8, 200, 48),
        "min_focus_score": clamp_i64(src.and_then(|v| v.get("min_focus_score")), 1, 100, 58),
        "dynamic_focus_gate_enabled": as_bool(src.and_then(|v| v.get("dynamic_focus_gate_enabled")), true),
        "dynamic_focus_window_hours": clamp_i64(src.and_then(|v| v.get("dynamic_focus_window_hours")), 1, 72, 6),
        "dynamic_focus_target_per_window": clamp_i64(src.and_then(|v| v.get("dynamic_focus_target_per_window")), 0, 500, 8),
        "dynamic_focus_floor_score": clamp_i64(src.and_then(|v| v.get("dynamic_focus_floor_score")), 1, 100, 35),
        "dynamic_focus_ceiling_score": clamp_i64(src.and_then(|v| v.get("dynamic_focus_ceiling_score")), 1, 100, 85),
        "dynamic_focus_response": clamp_i64(src.and_then(|v| v.get("dynamic_focus_response")), 0, 60, 14),
        "lens_enabled": as_bool(src.and_then(|v| v.get("lens_enabled")), true),
        "lens_refresh_hours": clamp_i64(src.and_then(|v| v.get("lens_refresh_hours")), 1, 72, 6),
        "lens_window_hours": clamp_i64(src.and_then(|v| v.get("lens_window_hours")), 6, 24 * 14, 48),
        "lens_max_terms": clamp_i64(src.and_then(|v| v.get("lens_max_terms")), 4, 64, 16),
        "lens_min_weight": clamp_i64(src.and_then(|v| v.get("lens_min_weight")), 1, 40, 2),
        "lens_max_weight": clamp_i64(src.and_then(|v| v.get("lens_max_weight")), 1, 60, 20),
        "lens_decay": clamp_number(src.and_then(|v| v.get("lens_decay")), 0.5, 0.99, 0.9),
        "lens_step_up": clamp_i64(src.and_then(|v| v.get("lens_step_up")), 1, 10, 2),
        "lens_step_down": clamp_i64(src.and_then(|v| v.get("lens_step_down")), 1, 10, 1),
        "lens_exclude_threshold": clamp_i64(src.and_then(|v| v.get("lens_exclude_threshold")), 1, 50, 4),
        "lens_max_exclude_terms": clamp_i64(src.and_then(|v| v.get("lens_max_exclude_terms")), 0, 32, 6),
        "lens_min_support": clamp_i64(src.and_then(|v| v.get("lens_min_support")), 1, 20, 2),
        "lens_cross_signal_boost": clamp_i64(src.and_then(|v| v.get("lens_cross_signal_boost")), 0, 20, 3),
        "max_focus_items_per_eye": clamp_i64(src.and_then(|v| v.get("max_focus_items_per_eye")), 1, 10, 2),
        "max_focus_items_per_run": clamp_i64(src.and_then(|v| v.get("max_focus_items_per_run")), 1, 50, 6),
        "dedupe_window_hours": clamp_i64(src.and_then(|v| v.get("dedupe_window_hours")), 1, 14 * 24, 36),
        "expand_fetch_enabled": as_bool(src.and_then(|v| v.get("expand_fetch_enabled")), true),
        "focus_fetch_timeout_ms": clamp_i64(src.and_then(|v| v.get("focus_fetch_timeout_ms")), 500, 15000, 4500),
        "focus_fetch_max_bytes": clamp_i64(src.and_then(|v| v.get("focus_fetch_max_bytes")), 4096, 1048576, 131072),
        "llm_backstop_enabled": as_bool(src.and_then(|v| v.get("llm_backstop_enabled")), false),
        "llm_uncertain_min_score": clamp_i64(src.and_then(|v| v.get("llm_uncertain_min_score")), 1, 99, 48),
        "llm_uncertain_max_score": clamp_i64(src.and_then(|v| v.get("llm_uncertain_max_score")), 1, 100, 57),
    });
    value
}

fn normalize_trigger(
    raw: &Map<String, Value>,
    taken: &mut BTreeSet<String>,
    now_ts: &str,
) -> Option<Value> {
    let key = normalize_key(&as_str(raw.get("key").or_else(|| raw.get("pattern"))), 120);
    if key.is_empty() {
        return None;
    }
    let candidate = as_str(raw.get("uid"));
    let uid = if !candidate.is_empty() && is_alnum(&candidate) && !taken.contains(&candidate) {
        candidate
    } else {
        let seeded = stable_uid(&format!("focus_trigger|{key}|v1"), "ft", 24);
        if !taken.contains(&seeded) {
            seeded
        } else {
            let mut generated = random_uid("ft", 24);
            let mut attempts = 0;
            while taken.contains(&generated) && attempts < 8 {
                generated = random_uid("ft", 24);
                attempts += 1;
            }
            generated
        }
    };
    taken.insert(uid.clone());
    let source_signals = raw
        .get("source_signals")
        .and_then(Value::as_array)
        .map(|rows| {
            let mut out = Vec::new();
            let mut seen = BTreeSet::new();
            for row in rows {
                let signal = normalize_key(&as_str(Some(row)), 120);
                if signal.is_empty() || seen.contains(&signal) {
                    continue;
                }
                seen.insert(signal.clone());
                out.push(Value::String(signal));
                if out.len() >= 8 {
                    break;
                }
            }
            out
        })
        .unwrap_or_default();
    let status_raw = as_str(raw.get("status")).to_ascii_lowercase();
    let pattern_value = {
        let value =
            clean_text(raw.get("pattern").or_else(|| raw.get("key")), 240).to_ascii_lowercase();
        if value.is_empty() {
            normalize_key(&as_str(raw.get("key")), 120)
        } else {
            value
        }
    };
    let source_value = {
        let value = clean_text(raw.get("source"), 120).to_ascii_lowercase();
        if value.is_empty() {
            "auto".to_string()
        } else {
            value
        }
    };
    let last_hit_ts = {
        let value = as_str(raw.get("last_hit_ts"));
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let created_ts = {
        let value = as_str(raw.get("created_ts"));
        if value.is_empty() {
            Value::String(now_ts.to_string())
        } else {
            Value::String(value)
        }
    };
    let updated_ts = {
        let value = as_str(raw.get("updated_ts"));
        if value.is_empty() {
            Value::String(now_ts.to_string())
        } else {
            Value::String(value)
        }
    };
    Some(json!({
        "uid": uid,
        "key": key,
        "pattern": pattern_value,
        "mode": if as_str(raw.get("mode")).to_ascii_lowercase() == "exact" { "exact" } else { "contains" },
        "source": source_value,
        "source_signals": source_signals,
        "status": if status_raw == "disabled" { "disabled" } else { "active" },
        "weight": clamp_i64(raw.get("weight"), 1, 100, 1),
        "cooldown_minutes": clamp_i64(raw.get("cooldown_minutes"), 0, 24 * 60, 90),
        "hit_count": clamp_i64(raw.get("hit_count"), 0, 1_000_000, 0),
        "last_hit_ts": last_hit_ts,
        "created_ts": created_ts,
        "updated_ts": updated_ts
    }))
}

fn default_focus_state() -> Value {
    json!({
        "version": "1.0",
        "policy": normalize_policy(None),
        "triggers": [],
        "eye_lenses": {},
        "recent_focus_items": {},
        "last_refresh_ts": Value::Null,
        "last_refresh_sources": {},
        "last_lens_refresh_ts": Value::Null,
        "last_lens_refresh_sources": {},
        "stats": {
            "refresh_count": 0,
            "lens_refresh_count": 0,
            "focused_items_total": 0,
            "last_focus_ts": Value::Null
        }
    })
}

fn normalize_state(raw: Option<&Value>, fallback: Option<&Value>) -> Value {
    let base = default_focus_state();
    let src = raw
        .and_then(Value::as_object)
        .or_else(|| fallback.and_then(Value::as_object));
    let now_ts = now_iso();
    let policy_value = normalize_policy(src.and_then(|v| v.get("policy")));
    let policy = policy_value.as_object().cloned().unwrap_or_default();

    let mut taken = BTreeSet::new();
    let mut triggers = Vec::new();
    if let Some(rows) = src
        .and_then(|v| v.get("triggers"))
        .and_then(Value::as_array)
    {
        for row in rows {
            if let Some(obj) = row.as_object() {
                if let Some(normalized) = normalize_trigger(obj, &mut taken, &now_ts) {
                    triggers.push(normalized);
                }
            }
        }
    }
    triggers.sort_by(|a, b| {
        let aw = a.get("weight").and_then(Value::as_i64).unwrap_or(0);
        let bw = b.get("weight").and_then(Value::as_i64).unwrap_or(0);
        bw.cmp(&aw).then_with(|| {
            let ak = a.get("key").and_then(Value::as_str).unwrap_or("");
            let bk = b.get("key").and_then(Value::as_str).unwrap_or("");
            ak.cmp(bk)
        })
    });
    let max_triggers = clamp_i64(policy.get("max_triggers"), 8, 200, 48) as usize;
    triggers.truncate(max_triggers);

    let stats = src.and_then(|v| v.get("stats")).and_then(Value::as_object);
    let last_refresh_ts = {
        let value = as_str(src.and_then(|v| v.get("last_refresh_ts")));
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let last_lens_refresh_ts = {
        let value = as_str(src.and_then(|v| v.get("last_lens_refresh_ts")));
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let last_focus_ts = {
        let value = as_str(stats.and_then(|v| v.get("last_focus_ts")));
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    json!({
        "version": as_str(src.and_then(|v| v.get("version")).or(Some(&base["version"]))),
        "policy": policy_value,
        "triggers": triggers,
        "eye_lenses": normalize_eye_lenses(src.and_then(|v| v.get("eye_lenses")), &policy),
        "recent_focus_items": normalize_recent_map(src.and_then(|v| v.get("recent_focus_items")), &policy),
        "last_refresh_ts": last_refresh_ts,
        "last_refresh_sources": src.and_then(|v| v.get("last_refresh_sources")).cloned().unwrap_or_else(|| json!({})),
        "last_lens_refresh_ts": last_lens_refresh_ts,
        "last_lens_refresh_sources": src.and_then(|v| v.get("last_lens_refresh_sources")).cloned().unwrap_or_else(|| json!({})),
        "stats": {
            "refresh_count": clamp_i64(stats.and_then(|v| v.get("refresh_count")), 0, 1_000_000, 0),
            "lens_refresh_count": clamp_i64(stats.and_then(|v| v.get("lens_refresh_count")), 0, 1_000_000, 0),
            "focused_items_total": clamp_i64(stats.and_then(|v| v.get("focused_items_total")), 0, 100_000_000, 0),
            "last_focus_ts": last_focus_ts
        }
    })
}

fn load_pointer_index(root: &Path) -> Value {
    read_json_value(&adaptive_pointer_index_path(root))
        .filter(|value| value.get("pointers").and_then(Value::as_object).is_some())
        .unwrap_or_else(|| json!({"version":"1.0","pointers":{}}))
}

fn save_pointer_index(root: &Path, value: &Value) -> Result<(), String> {
    write_json_atomic(&adaptive_pointer_index_path(root), value)
}

fn append_pointer_rows(root: &Path, abs_path: &Path, state: &Value) -> Result<(), String> {
    let pointer_path = adaptive_pointers_path(root);
    let mut index = load_pointer_index(root);
    let pointers = index
        .get_mut("pointers")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "focus_trigger_store_kernel_invalid_pointer_index".to_string())?;
    let triggers = state
        .get("triggers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for trigger in triggers {
        let key = format!(
            "focus_trigger|{}|{}",
            as_str(trigger.get("uid")),
            abs_path.display()
        );
        let row = json!({
            "kind": "focus_trigger",
            "uid": trigger.get("uid").cloned().unwrap_or(Value::Null),
            "entity_id": trigger.get("key").cloned().unwrap_or(Value::Null),
            "path_ref": abs_path.to_string_lossy(),
            "status": trigger.get("status").cloned().unwrap_or(Value::Null),
            "summary": trigger.get("pattern").cloned().unwrap_or(Value::Null),
            "tags": ["focus", "adaptive"],
            "ts": now_iso()
        });
        let digest = deterministic_receipt_hash(&row);
        let existing = pointers.get(&key).and_then(Value::as_str).unwrap_or("");
        if existing == digest {
            continue;
        }
        append_jsonl(&pointer_path, &row)?;
        pointers.insert(key, Value::String(digest));
    }
    save_pointer_index(root, &index)
}

fn append_mutation_log(
    root: &Path,
    abs_path: &Path,
    meta: &Map<String, Value>,
    state: &Value,
    reason: &str,
) -> Result<(), String> {
    let reason_value = {
        let value = clean_text(meta.get("reason"), 160);
        if value.is_empty() {
            reason.to_string()
        } else {
            value
        }
    };
    let source_value = {
        let value = clean_text(meta.get("source"), 180);
        if value.is_empty() {
            "core/layer0/ops::focus_trigger_store_kernel".to_string()
        } else {
            value
        }
    };
    let actor_value = {
        let value = clean_text(meta.get("actor"), 80);
        if value.is_empty() {
            std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
        } else {
            value
        }
    };
    let row = json!({
        "kind": "focus_trigger_store",
        "ts": now_iso(),
        "path": abs_path.to_string_lossy(),
        "reason": reason_value,
        "source": source_value,
        "actor": actor_value,
        "trigger_count": state.get("triggers").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "receipt_hash": deterministic_receipt_hash(state)
    });
    append_jsonl(&mutation_log_path(root), &row)
}

fn run_paths(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let abs = store_abs_path(root, payload)?;
    Ok(json!({
        "ok": true,
        "default_rel_path": DEFAULT_REL_PATH,
        "default_abs_path": default_abs_path(root).to_string_lossy(),
        "store_path": abs.to_string_lossy(),
        "mutation_log_path": mutation_log_path(root).to_string_lossy(),
        "pointer_path": adaptive_pointers_path(root).to_string_lossy(),
        "pointer_index_path": adaptive_pointer_index_path(root).to_string_lossy()
    }))
}

fn run_default_state() -> Value {
    json!({ "ok": true, "state": default_focus_state() })
}

fn run_normalize_state(payload: &Map<String, Value>) -> Value {
    json!({
        "ok": true,
        "state": normalize_state(payload.get("state"), payload.get("fallback"))
    })
}

fn run_read_state(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let abs = store_abs_path(root, payload)?;
    let fallback = payload.get("fallback");
    let state = read_json_value(&abs);
    Ok(json!({
        "ok": true,
        "exists": state.is_some(),
        "path": abs.to_string_lossy(),
        "state": normalize_state(state.as_ref(), fallback)
    }))
}

fn run_ensure_state(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let abs = store_abs_path(root, payload)?;
    let meta = payload
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let existed = abs.exists();
    let state = if existed {
        normalize_state(read_json_value(&abs).as_ref(), Some(&default_focus_state()))
    } else {
        let state = default_focus_state();
        write_json_atomic(&abs, &state)?;
        state
    };
    append_mutation_log(
        root,
        &abs,
        &meta,
        &state,
        if existed {
            "ensure_focus_state_existing"
        } else {
            "ensure_focus_state"
        },
    )?;
    append_pointer_rows(root, &abs, &state)?;
    Ok(json!({
        "ok": true,
        "path": abs.to_string_lossy(),
        "created": !existed,
        "state": state
    }))
}

fn run_set_state(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let abs = store_abs_path(root, payload)?;
    let meta = payload
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let state = normalize_state(
        payload.get("state").or_else(|| payload.get("value")),
        Some(&default_focus_state()),
    );
    write_json_atomic(&abs, &state)?;
    append_mutation_log(root, &abs, &meta, &state, "set_focus_state")?;
    append_pointer_rows(root, &abs, &state)?;
    Ok(json!({
        "ok": true,
        "path": abs.to_string_lossy(),
        "state": state
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("focus_trigger_store_kernel_error", err.as_str()));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let result = match command.as_str() {
        "paths" => run_paths(root, payload),
        "default-state" => Ok(run_default_state()),
        "normalize-state" => Ok(run_normalize_state(payload)),
        "read-state" => run_read_state(root, payload),
        "ensure-state" => run_ensure_state(root, payload),
        "set-state" => run_set_state(root, payload),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => Err(format!(
            "focus_trigger_store_kernel_unknown_command:{command}"
        )),
    };

    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt("focus_trigger_store_kernel", payload));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("focus_trigger_store_kernel_error", err.as_str()));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_state_assigns_uid_and_sorts_by_weight() {
        let state = normalize_state(
            Some(&json!({
                "triggers": [
                    {"key":"beta signal", "weight": 3},
                    {"key":"alpha signal", "weight": 9}
                ]
            })),
            None,
        );
        let rows = state
            .get("triggers")
            .and_then(Value::as_array)
            .expect("triggers");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].get("key").and_then(Value::as_str),
            Some("alpha_signal")
        );
        assert!(rows[0]
            .get("uid")
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn explicit_path_override_is_denied() {
        let temp = tempfile::tempdir().expect("tempdir");
        let payload = json!({ "file_path": temp.path().join("other.json").to_string_lossy() });
        let err = store_abs_path(temp.path(), payload.as_object().unwrap()).unwrap_err();
        assert!(err.contains("path override denied"));
    }
}
