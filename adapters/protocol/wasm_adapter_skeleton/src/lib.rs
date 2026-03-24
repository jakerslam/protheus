// SPDX-License-Identifier: Apache-2.0
use serde_json::json;

#[unsafe(no_mangle)]
pub extern "C" fn __infring_wasm_adapter_placeholder() {}

pub fn invoke(input_json: &str) -> Result<String, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(input_json).map_err(|e| format!("invalid_input:{e}"))?;
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
