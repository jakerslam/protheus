// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};

const MAX_INPUT_BYTES: usize = 1024 * 1024;

#[unsafe(no_mangle)]
pub extern "C" fn __infring_wasm_adapter_placeholder() {}

fn parse_input(input_json: &str) -> Result<Value, String> {
    if input_json.len() > MAX_INPUT_BYTES {
        return Err("input_too_large".to_string());
    }
    let parsed: Value =
        serde_json::from_str(input_json).map_err(|e| format!("invalid_input:{e}"))?;
    if !parsed.is_object() {
        return Err("invalid_input_object_required".to_string());
    }
    Ok(parsed)
}

pub fn invoke(input_json: &str) -> Result<String, String> {
    let parsed = parse_input(input_json)?;
    serde_json::to_string(&json!({
        "ok": true,
        "type": "wasm_adapter_invoke",
        "echo": parsed
    }))
    .map_err(|e| format!("encode_failed:{e}"))
}

pub fn health() -> String {
    "ok".to_string()
}

pub fn capabilities() -> String {
    json!(["invoke", "health", "capabilities"]).to_string()
}
