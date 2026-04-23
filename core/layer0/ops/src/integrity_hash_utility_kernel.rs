// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

fn usage() {
    println!("integrity-hash-utility-kernel commands:");
    println!(
        "  infring-ops integrity-hash-utility-kernel stable-stringify [--payload-base64=<json>]"
    );
    println!("  infring-ops integrity-hash-utility-kernel sha256-hex [--payload-base64=<json>]");
    println!(
        "  infring-ops integrity-hash-utility-kernel hash-file-sha256 [--payload-base64=<json>]"
    );
    println!(
        "  infring-ops integrity-hash-utility-kernel hash-contract-entry [--payload-base64=<json>]"
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
            .map_err(|err| format!("integrity_hash_utility_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("integrity_hash_utility_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("integrity_hash_utility_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("integrity_hash_utility_kernel_payload_decode_failed:{err}"));
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
    match value {
        Some(Value::String(v)) => v.trim().chars().take(max_len).collect(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other
            .to_string()
            .trim_matches('"')
            .trim()
            .chars()
            .take(max_len)
            .collect(),
    }
}

fn resolve_file_path(root: &Path, value: Option<&Value>) -> Result<PathBuf, String> {
    let raw = clean_text(value, 4096);
    if raw.is_empty() {
        return Err("integrity_hash_utility_kernel_missing_file_path".to_string());
    }
    let candidate = PathBuf::from(&raw);
    if candidate.is_absolute() {
        Ok(candidate)
    } else {
        Ok(root.join(candidate))
    }
}

fn stable_stringify(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(v) => {
            if *v {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(v) => v.to_string(),
        Value::String(v) => serde_json::to_string(v).unwrap_or_else(|_| "\"\"".to_string()),
        Value::Array(rows) => {
            let inner = rows
                .iter()
                .map(stable_stringify)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{inner}]")
        }
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort_by(|a, b| a.cmp(b));
            let inner = keys
                .into_iter()
                .map(|key| {
                    let encoded =
                        serde_json::to_string(&key).unwrap_or_else(|_| "\"\"".to_string());
                    format!("{encoded}:{}", stable_stringify(&map[&key]))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{inner}}}")
        }
    }
}

fn sha256_hex_string(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn hash_contract_entry(payload: &Map<String, Value>) -> Result<Value, String> {
    let plugin_id = clean_text(payload.get("plugin_id"), 120).to_ascii_lowercase();
    let provider_id = clean_text(payload.get("provider_id"), 120).to_ascii_lowercase();
    let contract = clean_text(payload.get("contract"), 120);
    if plugin_id.is_empty() {
        return Err("integrity_hash_utility_kernel_plugin_id_required".to_string());
    }
    if provider_id.is_empty() {
        return Err("integrity_hash_utility_kernel_provider_id_required".to_string());
    }
    if contract.is_empty() {
        return Err("integrity_hash_utility_kernel_contract_required".to_string());
    }
    let normalized = json!({
        "plugin_id": plugin_id,
        "provider_id": provider_id,
        "contract": contract,
        "metadata": payload.get("metadata").cloned().unwrap_or(Value::Null)
    });
    let serialized = stable_stringify(&normalized);
    Ok(json!({
        "ok": true,
        "normalized_entry": normalized,
        "serialized": serialized,
        "value": sha256_hex_string(&serialized)
    }))
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
            print_json_line(&cli_error("integrity_hash_utility_kernel_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let result = match command.as_str() {
        "stable-stringify" => cli_receipt(
            "integrity_hash_utility_kernel_stable_stringify",
            json!({
                "ok": true,
                "value": stable_stringify(input.get("value").unwrap_or(&Value::Null))
            }),
        ),
        "sha256-hex" => {
            let serialized = stable_stringify(input.get("value").unwrap_or(&Value::Null));
            cli_receipt(
                "integrity_hash_utility_kernel_sha256_hex",
                json!({
                    "ok": true,
                    "value": sha256_hex_string(&serialized),
                    "serialized": serialized,
                }),
            )
        }
        "hash-file-sha256" => match resolve_file_path(
            root,
            input.get("filePath").or_else(|| input.get("file_path")),
        ) {
            Ok(file_path) => match fs::read(&file_path) {
                Ok(bytes) => {
                    let mut hasher = Sha256::new();
                    hasher.update(bytes);
                    cli_receipt(
                        "integrity_hash_utility_kernel_hash_file_sha256",
                        json!({
                            "ok": true,
                            "file_path": file_path,
                            "value": format!("{:x}", hasher.finalize()),
                        }),
                    )
                }
                Err(err) => cli_error(
                    "integrity_hash_utility_kernel_error",
                    &format!("integrity_hash_utility_kernel_read_failed:{err}"),
                ),
            },
            Err(err) => cli_error("integrity_hash_utility_kernel_error", &err),
            },
        "hash-contract-entry" => match hash_contract_entry(input) {
            Ok(value) => cli_receipt(
                "integrity_hash_utility_kernel_hash_contract_entry",
                value,
            ),
            Err(err) => cli_error("integrity_hash_utility_kernel_error", &err),
        },
        _ => cli_error(
            "integrity_hash_utility_kernel_error",
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
    fn stable_stringify_sorts_object_keys() {
        let value = json!({"b": 2, "a": 1});
        assert_eq!(stable_stringify(&value), "{\"a\":1,\"b\":2}");
    }

    #[test]
    fn sha256_hex_stays_deterministic() {
        let value = json!({"b": 2, "a": 1});
        let serialized = stable_stringify(&value);
        assert_eq!(
            sha256_hex_string(&serialized),
            sha256_hex_string(&serialized)
        );
    }

    #[test]
    fn hash_contract_entry_is_deterministic() {
        let payload = serde_json::from_value::<Map<String, Value>>(json!({
            "plugin_id": "openrouter",
            "provider_id": "openrouter",
            "contract": "webSearchProviders",
            "metadata": {"mode":"explicit_fast_path"}
        }))
        .expect("payload");
        let one = hash_contract_entry(&payload).expect("hash");
        let two = hash_contract_entry(&payload).expect("hash");
        assert_eq!(one.get("value"), two.get("value"));
    }
}
