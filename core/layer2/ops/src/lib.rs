// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (authoritative daemon control contracts).

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn stable_hash(payload: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(payload).unwrap_or_default());
    hex::encode(hasher.finalize())
}

pub fn daemon_control_receipt(command: &str, mode: Option<&str>) -> Value {
    let mut out = json!({
        "ok": true,
        "type": "daemon_control_receipt",
        "authority": "core/layer2/ops",
        "command": command,
        "mode": mode
    });
    out["receipt_hash"] = Value::String(stable_hash(&out));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_receipt_has_hash() {
        let payload = daemon_control_receipt("status", Some("persistent"));
        assert!(payload.get("receipt_hash").and_then(Value::as_str).is_some());
    }
}

