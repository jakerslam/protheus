// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};

const MAX_INPUT_BYTES: usize = 1024 * 1024;
const MAX_TOP_LEVEL_KEYS: usize = 128;

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
    Ok(parsed)
}

pub fn invoke(input_json: &str) -> Result<String, String> {
    let parsed = parse_input(input_json)?;
    encode_output(json!({
        "ok": true,
        "type": "wasm_adapter_invoke",
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
