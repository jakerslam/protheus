// Layer ownership: core/layer0/desktop (authoritative)
// SPDX-License-Identifier: Apache-2.0

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_STATE_PATH_LEN: usize = 4_096;

fn sanitize_text(input: &str, max_len: usize) -> String {
    input
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .filter(|c| !c.is_control())
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn with_receipt(mut payload: Value) -> Value {
    let receipt_hash = deterministic_receipt_hash(&payload);
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("receipt_hash".to_string(), Value::String(receipt_hash));
    }
    payload
}

pub fn deterministic_receipt_hash(payload: &Value) -> String {
    let mut hasher = Sha256::new();
    let canonical = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn now_iso_stub() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{ts}")
}

pub fn status_payload(root: &Path) -> Value {
    let dashboard_state =
        root.join("client/runtime/local/state/ui/infring_dashboard/latest_snapshot.json");
    let dashboard_ready = dashboard_state.is_file();
    let state_path_raw = sanitize_text(
        dashboard_state.to_string_lossy().as_ref(),
        MAX_STATE_PATH_LEN,
    );
    let state_path = if state_path_raw.is_empty() {
        "unknown".to_string()
    } else {
        state_path_raw
    };
    let ts = now_iso_stub();
    with_receipt(json!({
        "ok": true,
        "type": "infring_desktop_status",
        "authority": "core/layer0/desktop",
        "dashboard_ready": dashboard_ready,
        "state_path": state_path,
        "state_path_known": state_path != "unknown",
        "ts": ts
    }))
}

pub fn launch_payload(root: &Path) -> Value {
    let status = status_payload(root);
    with_receipt(json!({
        "ok": true,
        "type": "infring_desktop_launch",
        "authority": "core/layer0/desktop",
        "status": status,
        "hint": "launch_dashboard_via_infring_gateway"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_payload_emits_receipt() {
        let root = tempfile::tempdir().expect("tempdir");
        let payload = status_payload(root.path());
        assert!(payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .is_some());
    }
}
