// Layer ownership: core/layer0/desktop (authoritative)
// SPDX-License-Identifier: Apache-2.0

use serde_json::{json, Value};
use std::path::Path;

pub fn status_payload(root: &Path) -> Value {
    let dashboard_state =
        root.join("client/runtime/local/state/ui/infring_dashboard/latest_snapshot.json");
    let dashboard_ready = dashboard_state.is_file();
    let payload = json!({
        "ok": true,
        "type": "infring_desktop_status",
        "authority": "core/layer0/desktop",
        "dashboard_ready": dashboard_ready,
        "state_path": dashboard_state.to_string_lossy().to_string(),
        "ts": protheus_ops_core::now_iso()
    });
    let receipt_hash = protheus_ops_core::deterministic_receipt_hash(&payload);
    json!({
        "ok": true,
        "type": "infring_desktop_status",
        "authority": "core/layer0/desktop",
        "dashboard_ready": dashboard_ready,
        "state_path": dashboard_state.to_string_lossy().to_string(),
        "ts": protheus_ops_core::now_iso(),
        "receipt_hash": receipt_hash
    })
}

pub fn launch_payload(root: &Path) -> Value {
    let status = status_payload(root);
    json!({
        "ok": true,
        "type": "infring_desktop_launch",
        "authority": "core/layer0/desktop",
        "status": status,
        "hint": "launch_dashboard_via_infring_gateway",
        "receipt_hash": protheus_ops_core::deterministic_receipt_hash(&json!({
            "type": "infring_desktop_launch",
            "state_path": status.get("state_path").cloned().unwrap_or(Value::Null)
        }))
    })
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
