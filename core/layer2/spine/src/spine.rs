// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/spine (authoritative spine contracts).

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn stable_hash(payload: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(payload).unwrap_or_default());
    hex::encode(hasher.finalize())
}

pub fn normalize_spine_status(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "ok" | "success" | "succeeded" | "ready" => "success",
        "timeout" | "timed_out" | "timed-out" => "timeout",
        "throttled" | "rate_limited" | "rate-limited" | "429" => "throttled",
        _ => "error",
    }
}

pub fn spine_execution_receipt(mode: &str, status: &str, error_kind: Option<&str>) -> Value {
    let normalized_status = normalize_spine_status(status);
    let seed = json!({
        "mode": mode,
        "status": normalized_status,
        "error_kind": error_kind
    });
    let digest = stable_hash(&seed);
    json!({
        "call_id": format!("spine-{}", &digest[..16]),
        "status": normalized_status,
        "error_kind": error_kind,
        "telemetry": {
            "duration_ms": 0,
            "tokens_used": 0
        }
    })
}

pub fn spine_contract_receipt(mode: &str, date: &str, max_eyes: Option<u64>) -> Value {
    let mut out = json!({
        "ok": true,
        "type": "spine_contract_receipt",
        "authority": "core/layer2/spine",
        "mode": mode,
        "date": date,
        "max_eyes": max_eyes,
        "execution_receipt": spine_execution_receipt(mode, "success", None)
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
        assert_eq!(
            payload
                .pointer("/execution_receipt/status")
                .and_then(Value::as_str),
            Some("success")
        );
    }

    #[test]
    fn spine_status_normalization_is_stable() {
        assert_eq!(normalize_spine_status("ok"), "success");
        assert_eq!(normalize_spine_status("rate_limited"), "throttled");
        assert_eq!(normalize_spine_status("timed-out"), "timeout");
        assert_eq!(normalize_spine_status("weird"), "error");

        let left = spine_execution_receipt("daily", "ok", None);
        let right = spine_execution_receipt("daily", "success", None);
        assert_eq!(left.get("call_id"), right.get("call_id"));
    }
}
