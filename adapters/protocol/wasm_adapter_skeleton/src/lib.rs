// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};

const MAX_INPUT_BYTES: usize = 1024 * 1024;
const MAX_TOP_LEVEL_KEYS: usize = 128;
const MAX_DEPTH: usize = 16;
const MAX_ARRAY_ITEMS: usize = 2048;

#[unsafe(no_mangle)]
pub extern "C" fn __infring_wasm_adapter_placeholder() {}

fn encode_output(payload: Value) -> Result<String, String> {
    serde_json::to_string(&payload).map_err(|e| format!("encode_failed:{e}"))
}

fn parse_input(input_json: &str) -> Result<Value, String> {
    if input_json.len() > MAX_INPUT_BYTES {
        return Err("input_too_large".to_string());
    }
    let parsed: Value =
        serde_json::from_str(input_json).map_err(|e| format!("invalid_input:{e}"))?;
    let Some(map) = parsed.as_object() else {
        return Err("invalid_input_object_required".to_string());
    };
    if map.len() > MAX_TOP_LEVEL_KEYS {
        return Err("input_too_many_keys".to_string());
    }
    validate_value(&parsed, 0)?;
    Ok(parsed)
}

fn validate_value(value: &Value, depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("input_too_deep".to_string());
    }
    match value {
        Value::Array(items) => {
            if items.len() > MAX_ARRAY_ITEMS {
                return Err("input_array_too_large".to_string());
            }
            for item in items {
                validate_value(item, depth + 1)?;
            }
        }
        Value::Object(map) => {
            if map.len() > MAX_TOP_LEVEL_KEYS {
                return Err("input_object_too_wide".to_string());
            }
            for nested in map.values() {
                validate_value(nested, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn fnv1a64(input: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in input {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn invoke(input_json: &str) -> Result<String, String> {
    let parsed = parse_input(input_json)?;
    let canonical = serde_json::to_vec(&parsed).map_err(|e| format!("canonicalize_failed:{e}"))?;
    encode_output(json!({
        "ok": true,
        "type": "wasm_adapter_invoke",
        "schema_version": "2026-04-20",
        "input_size_bytes": input_json.len(),
        "receipt_hash": format!("{:016x}", fnv1a64(&canonical)),
        "echo": parsed
    }))
}

pub fn health() -> String {
    "ok".to_string()
}

pub fn capabilities() -> String {
    encode_output(json!(["invoke", "health", "capabilities"]))
        .unwrap_or_else(|_| "[\"invoke\",\"health\",\"capabilities\"]".to_string())
}
