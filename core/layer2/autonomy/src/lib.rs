// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/autonomy (authoritative).

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn stable_hash(payload: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(payload).unwrap_or_default());
    hex::encode(hasher.finalize())
}

pub fn autonomy_receipt(command: &str, objective: Option<&str>) -> Value {
    let mut out = json!({
        "ok": true,
        "type": "autonomy_contract_receipt",
        "authority": "core/layer2/autonomy",
        "command": command,
        "objective": objective
    });
    out["receipt_hash"] = Value::String(stable_hash(&out));
    out
}

pub fn workflow_receipt(command: &str, scope: Option<&str>) -> Value {
    let mut out = json!({
        "ok": true,
        "type": "workflow_contract_receipt",
        "authority": "core/layer2/autonomy",
        "command": command,
        "scope": scope
    });
    out["receipt_hash"] = Value::String(stable_hash(&out));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autonomy_receipt_has_hash() {
        let payload = autonomy_receipt("status", Some("default"));
        assert!(payload.get("receipt_hash").and_then(Value::as_str).is_some());
    }
}

