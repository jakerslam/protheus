use infring_ops_core::agency_plane;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn lifecycle_path(root: &Path) -> PathBuf {
    let candidates = [
        root.join("core/local/state/ops/agency_plane/conduit/lifecycle.json"),
        root.join("local/state/ops/agency_plane/conduit/lifecycle.json"),
        root.join("local/state/agency_plane/conduit/lifecycle.json"),
    ];
    for path in candidates {
        if path.exists() {
            return path;
        }
    }
    root.join("core/local/state/ops/agency_plane/conduit/lifecycle.json")
}

fn read_lifecycle(root: &Path) -> Value {
    let path = lifecycle_path(root);
    let raw = fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| Value::Null)
}

#[test]
fn agency_plane_conduit_state_machine_fails_closed_then_recovers() {
    let temp = tempfile::tempdir().expect("tempdir");

    let fail_closed_code = agency_plane::run(
        temp.path(),
        &[
            "create".to_string(),
            "--template=frontend-wizard".to_string(),
            "--strict=1".to_string(),
            "--bypass=1".to_string(),
        ],
    );
    assert_eq!(fail_closed_code, 2);
    let failed_closed = read_lifecycle(temp.path());
    assert_eq!(
        failed_closed.get("state").and_then(Value::as_str),
        Some("failed_closed")
    );
    assert_eq!(
        failed_closed
            .get("failed_closed_count")
            .and_then(Value::as_u64),
        Some(1)
    );

    let reconnect_code = agency_plane::run(
        temp.path(),
        &[
            "create".to_string(),
            "--template=frontend-wizard".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(reconnect_code, 0);
    let reconnecting = read_lifecycle(temp.path());
    assert_eq!(
        reconnecting.get("state").and_then(Value::as_str),
        Some("reconnecting")
    );

    let healthy_code = agency_plane::run(
        temp.path(),
        &[
            "create".to_string(),
            "--template=frontend-wizard".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(healthy_code, 0);
    let healthy = read_lifecycle(temp.path());
    assert_eq!(
        healthy.get("state").and_then(Value::as_str),
        Some("healthy")
    );
    assert_eq!(
        healthy.get("recovered_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        healthy.get("transition_count").and_then(Value::as_u64),
        Some(3)
    );
}

#[test]
fn agency_plane_conduit_status_does_not_mutate_state_machine() {
    let temp = tempfile::tempdir().expect("tempdir");
    let seed_code = agency_plane::run(
        temp.path(),
        &[
            "create".to_string(),
            "--template=frontend-wizard".to_string(),
            "--strict=1".to_string(),
            "--bypass=1".to_string(),
        ],
    );
    assert_eq!(seed_code, 2);
    let before = read_lifecycle(temp.path());
    let transition_before = before
        .get("transition_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let status_code = agency_plane::run(temp.path(), &["status".to_string()]);
    assert_eq!(status_code, 0);
    let after = read_lifecycle(temp.path());
    let transition_after = after
        .get("transition_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert_eq!(transition_after, transition_before);
    assert_eq!(after.get("state"), before.get("state"));
}
