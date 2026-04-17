// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;

const EYES_STATE_DEFAULT_REL: &str = "local/state/sensory/eyes";
const META_SCHEMA_ID: &str = "collector_meta_v1";

fn usage() {
    println!("collector-state-kernel commands:");
    println!("  protheus-ops collector-state-kernel meta-load --payload-base64=<json>");
    println!("  protheus-ops collector-state-kernel meta-save --payload-base64=<json>");
    println!("  protheus-ops collector-state-kernel cadence-check --payload-base64=<json>");
    println!("  protheus-ops collector-state-kernel cache-load --payload-base64=<json>");
    println!("  protheus-ops collector-state-kernel cache-save --payload-base64=<json>");
}

#[cfg(test)]
fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn now_ms_i64() -> i64 {
    Utc::now().timestamp_millis()
}

fn clean_collector_id(payload: &Map<String, Value>) -> String {
    lane_utils::clean_token(
        payload.get("collector_id").and_then(Value::as_str),
        "collector",
    )
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(raw, max_len)
}

fn json_bool(payload: &Map<String, Value>, key: &str, fallback: bool) -> bool {
    payload
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

fn json_f64(payload: &Map<String, Value>, key: &str, fallback: f64, lo: f64, hi: f64) -> f64 {
    payload
        .get(key)
        .and_then(Value::as_f64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn resolve_eyes_state_dir(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    if let Some(raw) = payload.get("eyes_state_dir").and_then(Value::as_str) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }

    if let Ok(raw) = std::env::var("EYES_STATE_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            return if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            };
        }
    }

    root.join(EYES_STATE_DEFAULT_REL)
}

fn meta_path_for(root: &Path, payload: &Map<String, Value>, collector_id: &str) -> PathBuf {
    if let Some(raw) = payload.get("meta_path").and_then(Value::as_str) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }
    resolve_eyes_state_dir(root, payload)
        .join("collector_meta")
        .join(format!("{collector_id}.json"))
}

fn cache_path_for(root: &Path, payload: &Map<String, Value>, collector_id: &str) -> PathBuf {
    if let Some(raw) = payload.get("cache_path").and_then(Value::as_str) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }
    resolve_eyes_state_dir(root, payload)
        .join("collector_meta")
        .join(format!("{collector_id}.cache.json"))
}

fn read_json(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("collector_state_kernel_create_dir_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let body = format!(
        "{}\n",
        serde_json::to_string_pretty(value)
            .map_err(|err| format!("collector_state_kernel_encode_failed:{err}"))?
    );
    fs::write(&tmp, body).map_err(|err| format!("collector_state_kernel_write_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("collector_state_kernel_rename_failed:{err}"))
}

fn clean_seen_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if out.len() >= 120 {
            break;
        }
        let keep = ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.');
        if keep {
            out.push(ch);
        }
    }
    out
}

fn normalize_meta_value(collector_id: &str, raw: Option<&Value>) -> Value {
    let obj = raw.and_then(Value::as_object);
    let last_run = clean_text(
        obj.and_then(|o| o.get("last_run")).and_then(Value::as_str),
        80,
    );
    let last_success = clean_text(
        obj.and_then(|o| o.get("last_success"))
            .and_then(Value::as_str),
        80,
    );

    let mut seen_ids = Vec::new();
    if let Some(items) = obj
        .and_then(|o| o.get("seen_ids"))
        .and_then(Value::as_array)
    {
        for entry in items {
            if let Some(raw_id) = entry.as_str() {
                let cleaned = clean_seen_id(raw_id);
                if !cleaned.is_empty() {
                    seen_ids.push(Value::String(cleaned));
                }
            }
        }
    }
    if seen_ids.len() > 2000 {
        let split = seen_ids.len() - 2000;
        seen_ids = seen_ids.into_iter().skip(split).collect::<Vec<_>>();
    }

    json!({
        "schema_id": META_SCHEMA_ID,
        "collector_id": collector_id,
        "last_run": if last_run.is_empty() { Value::Null } else { Value::String(last_run) },
        "last_success": if last_success.is_empty() { Value::Null } else { Value::String(last_success) },
        "seen_ids": seen_ids,
    })
}

fn parse_iso_ms(raw: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn command_meta_load(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let meta_path = meta_path_for(root, payload, &collector_id);
    let meta = normalize_meta_value(
        &collector_id,
        Some(&read_json(
            &meta_path,
            normalize_meta_value(&collector_id, None),
        )),
    );

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "meta": meta,
        "meta_path": meta_path.display().to_string()
    }))
}

fn command_meta_save(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let meta_path = meta_path_for(root, payload, &collector_id);
    let meta = normalize_meta_value(&collector_id, payload.get("meta"));
    write_json_atomic(&meta_path, &meta)?;

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "meta": meta,
        "meta_path": meta_path.display().to_string()
    }))
}

fn command_cadence_check(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let force = json_bool(payload, "force", false);
    let min_hours = json_f64(payload, "min_hours", 4.0, 0.0, 24.0 * 365.0);

    let meta_path = meta_path_for(root, payload, &collector_id);
    let meta = normalize_meta_value(
        &collector_id,
        Some(&read_json(
            &meta_path,
            normalize_meta_value(&collector_id, None),
        )),
    );

    let last_run_ms = meta
        .get("last_run")
        .and_then(Value::as_str)
        .and_then(parse_iso_ms);

    let hours_since = last_run_ms.map(|ms| ((now_ms_i64() - ms) as f64 / 3_600_000.0).max(0.0));
    let skipped = !force && hours_since.map(|h| h < min_hours).unwrap_or(false);

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "force": force,
        "min_hours": min_hours,
        "hours_since_last": hours_since,
        "skipped": skipped,
        "reason": if skipped { Value::String("cadence".to_string()) } else { Value::Null },
        "meta": meta,
        "meta_path": meta_path.display().to_string()
    }))
}

fn command_cache_load(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let cache_path = cache_path_for(root, payload, &collector_id);
    let envelope = read_json(&cache_path, json!({ "items": [] }));
    let items = envelope
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "cache_path": cache_path.display().to_string(),
        "items": items,
        "cache_hit": !items.is_empty()
    }))
}

fn command_cache_save(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let collector_id = clean_collector_id(payload);
    let cache_path = cache_path_for(root, payload, &collector_id);
    let items = payload
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let envelope = json!({ "items": items });
    write_json_atomic(&cache_path, &envelope)?;

    Ok(json!({
        "ok": true,
        "collector_id": collector_id,
        "cache_path": cache_path.display().to_string(),
        "items": envelope.get("items").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
    }))
}

fn dispatch(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "meta-load" => command_meta_load(root, payload),
        "meta-save" => command_meta_save(root, payload),
        "cadence-check" => command_cadence_check(root, payload),
        "cache-load" => command_cache_load(root, payload),
        "cache-save" => command_cache_save(root, payload),
        _ => Err("collector_state_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "collector_state_kernel") {
        Ok(value) => value,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "collector_state_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = lane_utils::payload_obj(&payload);

    match dispatch(root, &command, payload_obj) {
        Ok(out) => {
            lane_utils::print_json_line(&lane_utils::cli_receipt("collector_state_kernel", out));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "collector_state_kernel_error",
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

    #[test]
    fn cadence_skip_and_cache_roundtrip() {
        let tmp = tempdir().expect("tmp");
        let root = tmp.path();
        let payload = json!({
            "collector_id": "demo",
            "eyes_state_dir": root.display().to_string(),
            "force": false,
            "min_hours": 24.0
        });
        let payload_obj = lane_utils::payload_obj(&payload);

        let cadence1 = command_cadence_check(root, payload_obj).expect("cadence1");
        assert_eq!(
            cadence1.get("skipped").and_then(Value::as_bool),
            Some(false)
        );

        let saved = command_meta_save(
            root,
            lane_utils::payload_obj(&json!({
                "collector_id": "demo",
                "eyes_state_dir": root.display().to_string(),
                "meta": {
                    "last_run": now_iso(),
                    "last_success": now_iso(),
                    "seen_ids": ["a", "b"]
                }
            })),
        )
        .expect("meta save");
        assert_eq!(saved.get("ok").and_then(Value::as_bool), Some(true));

        let cadence2 = command_cadence_check(root, payload_obj).expect("cadence2");
        assert_eq!(cadence2.get("skipped").and_then(Value::as_bool), Some(true));

        let cache_saved = command_cache_save(
            root,
            lane_utils::payload_obj(&json!({
                "collector_id": "demo",
                "eyes_state_dir": root.display().to_string(),
                "items": [{"id":"x"}]
            })),
        )
        .expect("cache save");
        assert_eq!(cache_saved.get("ok").and_then(Value::as_bool), Some(true));

        let cache_loaded = command_cache_load(root, payload_obj).expect("cache load");
        assert_eq!(
            cache_loaded
                .get("items")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn meta_load_roots_relative_env_state_dir_under_root() {
        let tmp = tempdir().expect("tmp");
        let root = tmp.path();
        let previous = std::env::var("EYES_STATE_DIR").ok();
        std::env::set_var("EYES_STATE_DIR", "relative/eyes");

        let out = command_meta_load(
            root,
            lane_utils::payload_obj(&json!({ "collector_id": "demo" })),
        )
        .expect("meta load");
        assert_eq!(
            out.get("meta_path").and_then(Value::as_str),
            Some(
                root.join("relative/eyes")
                    .join("collector_meta")
                    .join("demo.json")
                    .to_string_lossy()
                    .as_ref()
            )
        );

        if let Some(value) = previous {
            std::env::set_var("EYES_STATE_DIR", value);
        } else {
            std::env::remove_var("EYES_STATE_DIR");
        }
    }
}
