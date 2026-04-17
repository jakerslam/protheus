// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_REL_PATH: &str = "sensory/eyes/catalog.json";
const DEFAULT_PROVIDER_FAMILIES: [&str; 4] = ["openai", "openrouter", "xai", "tts"];
const DEFAULT_PROVIDER_AUTH_ENV_KEYS: [&str; 4] = [
    "OPENAI_API_KEY",
    "OPENROUTER_API_KEY",
    "XAI_API_KEY",
    "ELEVENLABS_API_KEY",
];

fn usage() {
    println!("catalog-store-kernel commands:");
    println!("  protheus-ops catalog-store-kernel paths [--payload-base64=<json>]");
    println!("  protheus-ops catalog-store-kernel default-state");
    println!("  protheus-ops catalog-store-kernel normalize-state [--payload-base64=<json>]");
    println!("  protheus-ops catalog-store-kernel read-state [--payload-base64=<json>]");
    println!("  protheus-ops catalog-store-kernel ensure-state [--payload-base64=<json>]");
    println!("  protheus-ops catalog-store-kernel set-state [--payload-base64=<json>]");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out =
        json!({"ok": ok, "type": kind, "ts": ts, "date": ts[..10].to_string(), "payload": payload});
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}
fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({"ok": false, "type": kind, "ts": ts, "date": ts[..10].to_string(), "error": error, "fail_closed": true});
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
            .map_err(|err| format!("catalog_store_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("catalog_store_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("catalog_store_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("catalog_store_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let input = match value {
        Some(Value::String(v)) => v.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    };
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn runtime_root(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("PROTHEUS_RUNTIME_ROOT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let workspace = if let Ok(raw) = std::env::var("PROTHEUS_WORKSPACE_ROOT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            PathBuf::from(trimmed)
        } else {
            root.to_path_buf()
        }
    } else {
        root.to_path_buf()
    };
    let candidate = workspace.join("client/runtime");
    if candidate.exists() {
        candidate
    } else {
        workspace
    }
}

fn default_abs_path(root: &Path) -> PathBuf {
    runtime_root(root).join("adaptive").join(DEFAULT_REL_PATH)
}
fn as_catalog_path(root: &Path, raw: Option<&Value>) -> Result<PathBuf, String> {
    let canonical = default_abs_path(root);
    let requested = clean_text(raw, 520);
    if requested.is_empty() {
        return Ok(canonical);
    }
    let candidate = PathBuf::from(&requested);
    let resolved = if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    };
    if resolved != canonical {
        return Err(format!(
            "catalog_store: catalog path override denied (requested={})",
            resolved.display()
        ));
    }
    Ok(canonical)
}

fn default_catalog() -> Value {
    json!({
        "version": "1.0",
        "eyes": [{
            "id": "conversation_eye",
            "name": "Conversation Eye",
            "status": "active",
            "cadence_hours": 1,
            "allowed_domains": ["local.workspace"],
            "budgets": {"max_items": 6, "max_seconds": 8, "max_bytes": 65536, "max_requests": 1, "max_rows": 96},
            "parser_type": "conversation_eye",
            "topics": ["conversation", "decision", "insight", "directive", "t1"],
            "error_rate": 0,
            "score_ema": 50
        }],
        "global_limits": {"max_concurrent_runs": 3, "global_max_requests_per_day": 50, "global_max_bytes_per_day": 5242880},
        "scoring": {"ema_alpha": 0.3, "score_threshold_high": 70, "score_threshold_low": 30, "score_threshold_dormant": 20, "cadence_min_hours": 1, "cadence_max_hours": 168},
        "tooling_contracts": {
            "source": "openclaw_provider_family_contract_inventory_v1",
            "provider_families": DEFAULT_PROVIDER_FAMILIES,
            "auth_env_keys": DEFAULT_PROVIDER_AUTH_ENV_KEYS,
            "plugin_boundary_required": true
        }
    })
}

fn stable_uid(seed: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::new();
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    format!("e{}", &hex[..23])
}

fn normalize_catalog(catalog: Option<&Value>) -> Value {
    let src = catalog
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| default_catalog().as_object().cloned().unwrap());
    let now = now_iso();
    let mut out = src.clone();
    let mut taken = BTreeSet::<String>::new();
    let mut eyes = Vec::<Value>::new();
    for row in src
        .get("eyes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let mut eye = row.as_object().cloned().unwrap_or_default();
        let requested_uid = clean_text(eye.get("uid"), 32);
        let mut uid = if !requested_uid.is_empty()
            && requested_uid.chars().all(|ch| ch.is_ascii_alphanumeric())
            && !taken.contains(&requested_uid)
        {
            requested_uid
        } else {
            let seed = clean_text(eye.get("id"), 64);
            let seeded = if seed.is_empty() {
                stable_uid(&format!("catalog-eye|{}", now))
            } else {
                stable_uid(&format!("adaptive_eye|{}|v1", seed))
            };
            if taken.contains(&seeded) {
                stable_uid(&format!("catalog-eye|{}|{}", seeded, now))
            } else {
                seeded
            }
        };
        while taken.contains(&uid) {
            uid = stable_uid(&format!("catalog-eye|{}|{}", uid, now));
        }
        taken.insert(uid.clone());
        eye.insert("uid".to_string(), Value::String(uid));
        if clean_text(eye.get("created_ts"), 40).is_empty() {
            eye.insert("created_ts".to_string(), Value::String(now.clone()));
        }
        eye.insert("updated_ts".to_string(), Value::String(now.clone()));
        eyes.push(Value::Object(eye));
    }
    out.insert("eyes".to_string(), Value::Array(eyes));
    out.insert(
        "tooling_contracts".to_string(),
        normalize_tooling_contracts(src.get("tooling_contracts")),
    );
    Value::Object(out)
}

fn normalize_tooling_contracts(value: Option<&Value>) -> Value {
    let source = value.and_then(Value::as_object).cloned().unwrap_or_default();
    let mut families = BTreeSet::<String>::new();
    for row in source
        .get("provider_families")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        if let Some(raw) = row.as_str() {
            let clean = raw.trim().to_ascii_lowercase();
            if DEFAULT_PROVIDER_FAMILIES
                .iter()
                .any(|expected| expected == &clean)
            {
                families.insert(clean);
            }
        }
    }
    if families.is_empty() {
        for default in DEFAULT_PROVIDER_FAMILIES {
            families.insert(default.to_string());
        }
    }

    let mut auth_env_keys = BTreeSet::<String>::new();
    for row in source
        .get("auth_env_keys")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        if let Some(raw) = row.as_str() {
            let clean = raw.trim().to_ascii_uppercase();
            if !clean.is_empty() && clean.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                auth_env_keys.insert(clean);
            }
        }
    }
    if auth_env_keys.is_empty() {
        for key in DEFAULT_PROVIDER_AUTH_ENV_KEYS {
            auth_env_keys.insert(key.to_string());
        }
    }

    let source_name = clean_text(source.get("source"), 80);
    let source_value = if source_name.is_empty() {
        "openclaw_provider_family_contract_inventory_v1".to_string()
    } else {
        source_name
    };
    json!({
        "source": source_value,
        "provider_families": families.into_iter().collect::<Vec<_>>(),
        "auth_env_keys": auth_env_keys.into_iter().collect::<Vec<_>>(),
        "plugin_boundary_required": source.get("plugin_boundary_required").and_then(Value::as_bool).unwrap_or(true)
    })
}

fn read_state(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = as_catalog_path(root, payload.get("file_path"))?;
    Ok(normalize_catalog(
        lane_utils::read_json(&path)
            .as_ref()
            .or_else(|| payload.get("fallback")),
    ))
}

fn ensure_state(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = as_catalog_path(root, payload.get("file_path"))?;
    let current = if path.exists() {
        lane_utils::read_json(&path)
    } else {
        None
    };
    let normalized = normalize_catalog(current.as_ref().or(Some(&default_catalog())));
    lane_utils::write_json(&path, &normalized)?;
    Ok(normalized)
}

fn set_state(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let path = as_catalog_path(root, payload.get("file_path"))?;
    let normalized = normalize_catalog(payload.get("state"));
    lane_utils::write_json(&path, &normalized)?;
    Ok(normalized)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("catalog_store_kernel_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let result = match command.as_str() {
        "paths" => match as_catalog_path(root, input.get("file_path")) {
            Ok(path) => cli_receipt(
                "catalog_store_kernel_paths",
                json!({ "ok": true, "default_rel_path": DEFAULT_REL_PATH, "default_abs_path": default_abs_path(root), "catalog_path": path }),
            ),
            Err(err) => cli_error("catalog_store_kernel_error", &err),
        },
        "default-state" => cli_receipt(
            "catalog_store_kernel_default_state",
            json!({ "ok": true, "state": default_catalog() }),
        ),
        "normalize-state" => cli_receipt(
            "catalog_store_kernel_normalize_state",
            json!({ "ok": true, "state": normalize_catalog(input.get("state").or_else(|| input.get("catalog"))) }),
        ),
        "read-state" => match read_state(root, input) {
            Ok(state) => cli_receipt(
                "catalog_store_kernel_read_state",
                json!({ "ok": true, "state": state }),
            ),
            Err(err) => cli_error("catalog_store_kernel_error", &err),
        },
        "ensure-state" => match ensure_state(root, input) {
            Ok(state) => cli_receipt(
                "catalog_store_kernel_ensure_state",
                json!({ "ok": true, "state": state }),
            ),
            Err(err) => cli_error("catalog_store_kernel_error", &err),
        },
        "set-state" => match set_state(root, input) {
            Ok(state) => cli_receipt(
                "catalog_store_kernel_set_state",
                json!({ "ok": true, "state": state }),
            ),
            Err(err) => cli_error("catalog_store_kernel_error", &err),
        },
        _ => cli_error(
            "catalog_store_kernel_error",
            &format!("unknown_command:{command}"),
        ),
    };
    let exit = if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    };
    print_json_line(&result);
    exit
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalize_catalog_assigns_uid() {
        let state = normalize_catalog(Some(&json!({"eyes":[{"id":"conversation_eye"}]})));
        assert!(state["eyes"][0]["uid"].as_str().unwrap().starts_with('e'));
    }
}
