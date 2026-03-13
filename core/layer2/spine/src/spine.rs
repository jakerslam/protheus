// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/spine (authoritative spine contracts).

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn stable_hash(payload: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(payload).unwrap_or_default());
    hex::encode(hasher.finalize())
}

pub fn spine_contract_receipt(mode: &str, date: &str, max_eyes: Option<u64>) -> Value {
    let mut out = json!({
        "ok": true,
        "type": "spine_contract_receipt",
        "authority": "core/layer2/spine",
        "mode": mode,
        "date": date,
        "max_eyes": max_eyes
    });
    out["receipt_hash"] = Value::String(stable_hash(&out));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_is_hashed() {
        let payload = spine_contract_receipt("daily", "2026-03-08", Some(8));
        let hash = payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .expect("hash");
        assert!(!hash.is_empty());
    }
}
