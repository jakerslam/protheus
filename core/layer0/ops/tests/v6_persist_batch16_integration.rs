// SPDX-License-Identifier: Apache-2.0

use protheus_ops_core::persist_plane;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(3)
        .expect("workspace ancestor")
        .to_path_buf()
}

fn copy_tree(src: &Path, dst: &Path) {
    for entry in WalkDir::new(src).into_iter().filter_map(Result::ok) {
        let rel = entry.path().strip_prefix(src).expect("strip prefix");
        let out = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&out).expect("mkdir");
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).expect("mkdir parent");
        }
        fs::copy(entry.path(), &out).expect("copy file");
    }
}

fn stage_fixture_root() -> TempDir {
    let workspace = workspace_root();
    let tmp = tempfile::tempdir().expect("tempdir");
    copy_tree(
        &workspace.join("planes").join("contracts"),
        &tmp.path().join("planes").join("contracts"),
    );
    tmp
}

fn latest_path(root: &Path) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("persist_plane")
        .join("latest.json")
}

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read");
    serde_json::from_str(&raw).expect("parse")
}

fn assert_claim(payload: &Value, claim_id: &str) {
    assert_no_runtime_context_leak(&payload.to_string());
    let has = payload
        .get("claim_evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some(claim_id));
    assert!(has, "missing claim evidence id={claim_id}");
}

fn assert_no_runtime_context_leak(raw: &str) {
    const FORBIDDEN: [&str; 6] = [
        "You are an expert Python programmer.",
        "[PATCH v2",
        "List Leaves (25",
        "BEGIN_OPENCLAW_INTERNAL_CONTEXT",
        "END_OPENCLAW_INTERNAL_CONTEXT",
        "UNTRUSTED_CHILD_RESULT_DELIMITER",
    ];
    for marker in FORBIDDEN {
        assert!(
            !raw.contains(marker),
            "runtime payload leaked forbidden marker `{marker}`: {raw}"
        );
    }
}

#[test]
fn v6_persist_batch16_continuity_connector_cowork_are_receipted() {
    let fixture = stage_fixture_root();
    let root = fixture.path();

    let checkpoint_exit = persist_plane::run(
        root,
        &[
            "continuity".to_string(),
            "--strict=1".to_string(),
            "--op=checkpoint".to_string(),
            "--session-id=batch16-s1".to_string(),
            "--context-json={\"context\":[\"a\"],\"user_model\":{\"style\":\"direct\"},\"active_tasks\":[\"t\"]}".to_string(),
        ],
    );
    assert_eq!(checkpoint_exit, 0);
    let reconstruct_exit = persist_plane::run(
        root,
        &[
            "continuity".to_string(),
            "--strict=1".to_string(),
            "--op=reconstruct".to_string(),
            "--session-id=batch16-s1".to_string(),
        ],
    );
    assert_eq!(reconstruct_exit, 0);
    let continuity_latest = read_json(&latest_path(root));
    assert_eq!(
        continuity_latest.get("type").and_then(Value::as_str),
        Some("persist_plane_continuity")
    );
    assert_claim(&continuity_latest, "V6-PERSIST-001.3");

    let connector_add_exit = persist_plane::run(
        root,
        &[
            "connector".to_string(),
            "--strict=1".to_string(),
            "--op=add".to_string(),
            "--provider=slack".to_string(),
            "--policy-template=slack-enterprise".to_string(),
        ],
    );
    assert_eq!(connector_add_exit, 0);
    let connector_status_exit = persist_plane::run(
        root,
        &[
            "connector".to_string(),
            "--strict=1".to_string(),
            "--op=status".to_string(),
            "--provider=slack".to_string(),
        ],
    );
    assert_eq!(connector_status_exit, 0);
    let connector_latest = read_json(&latest_path(root));
    assert_eq!(
        connector_latest.get("type").and_then(Value::as_str),
        Some("persist_plane_connector")
    );
    assert_claim(&connector_latest, "V6-PERSIST-001.4");

    let cowork_delegate_exit = persist_plane::run(
        root,
        &[
            "cowork".to_string(),
            "--strict=1".to_string(),
            "--op=delegate".to_string(),
            "--task=ship batch16".to_string(),
            "--parent=lead".to_string(),
            "--child=worker".to_string(),
            "--mode=sub-agent".to_string(),
            "--budget-ms=1500".to_string(),
        ],
    );
    assert_eq!(cowork_delegate_exit, 0);
    let cowork_tick_exit = persist_plane::run(
        root,
        &[
            "cowork".to_string(),
            "--strict=1".to_string(),
            "--op=tick".to_string(),
        ],
    );
    assert_eq!(cowork_tick_exit, 0);
    let cowork_latest = read_json(&latest_path(root));
    assert_eq!(
        cowork_latest.get("type").and_then(Value::as_str),
        Some("persist_plane_cowork")
    );
    assert_claim(&cowork_latest, "V6-PERSIST-001.5");
    assert_claim(&cowork_latest, "V6-PERSIST-001.6");
}

#[test]
fn v6_persist_batch16_rejects_connector_bypass_when_strict() {
    let fixture = stage_fixture_root();
    let root = fixture.path();
    let exit = persist_plane::run(
        root,
        &[
            "connector".to_string(),
            "--strict=1".to_string(),
            "--op=list".to_string(),
            "--bypass=1".to_string(),
        ],
    );
    assert_eq!(exit, 1);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("persist_plane_conduit_gate")
    );
}

#[test]
fn v6_persist_batch16_mobile_daemon_is_policy_bound_and_receipted() {
    let fixture = stage_fixture_root();
    let root = fixture.path();

    let enable_exit = persist_plane::run(
        root,
        &[
            "mobile-daemon".to_string(),
            "--strict=1".to_string(),
            "--op=enable".to_string(),
            "--platform=android".to_string(),
            "--edge-backend=bitnet".to_string(),
            "--sensor-lanes=camera,mic,gps".to_string(),
        ],
    );
    assert_eq!(enable_exit, 0);
    let enabled_latest = read_json(&latest_path(root));
    assert_eq!(
        enabled_latest.get("type").and_then(Value::as_str),
        Some("persist_plane_mobile_daemon")
    );
    assert_claim(&enabled_latest, "V7-MOBILE-001.1");
    assert_eq!(
        enabled_latest
            .pointer("/state/edge_backend")
            .and_then(Value::as_str),
        Some("bitnet")
    );

    let handoff_exit = persist_plane::run(
        root,
        &[
            "mobile-daemon".to_string(),
            "--strict=1".to_string(),
            "--op=handoff".to_string(),
            "--handoff=cloud".to_string(),
        ],
    );
    assert_eq!(handoff_exit, 0);
    let handoff_latest = read_json(&latest_path(root));
    assert_eq!(
        handoff_latest
            .pointer("/state/handoff_mode")
            .and_then(Value::as_str),
        Some("cloud")
    );

    let invalid_profile_exit = persist_plane::run(
        root,
        &[
            "mobile-daemon".to_string(),
            "--strict=1".to_string(),
            "--op=enable".to_string(),
            "--platform=desktop".to_string(),
            "--edge-backend=bitnet".to_string(),
        ],
    );
    assert_eq!(invalid_profile_exit, 1);
}
