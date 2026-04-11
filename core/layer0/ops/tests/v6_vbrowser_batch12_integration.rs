// SPDX-License-Identifier: Apache-2.0

use protheus_ops_core::{health_status, vbrowser_plane};
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
        .join("vbrowser_plane")
        .join("latest.json")
}

fn health_latest_path(root: &Path) -> PathBuf {
    root.join("client")
        .join("local")
        .join("state")
        .join("ops")
        .join("health_status")
        .join("latest.json")
}

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read");
    serde_json::from_str(&raw).expect("parse")
}

fn assert_claim(payload: &Value, claim_id: &str) {
    let has = payload
        .get("claim_evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some(claim_id));
    assert!(has, "missing claim evidence id={claim_id}");
}

#[test]
fn v6_vbrowser_batch12_core_lanes_execute_with_receipts() {
    let fixture = stage_fixture_root();
    let root = fixture.path();

    let start_exit = vbrowser_plane::run(
        root,
        &[
            "session-start".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--url=https://example.com".to_string(),
            "--shadow=alpha".to_string(),
        ],
    );
    assert_eq!(start_exit, 0);
    let start_latest = read_json(&latest_path(root));
    assert_eq!(
        start_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_session_start")
    );
    assert_eq!(start_latest.get("ok").and_then(Value::as_bool), Some(true));
    assert_claim(&start_latest, "V6-VBROWSER-001.1");
    assert_claim(&start_latest, "V6-VBROWSER-001.5");
    assert_claim(&start_latest, "V6-VBROWSER-001.6");

    let goto_exit = vbrowser_plane::run(
        root,
        &[
            "goto".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--url=docs.rs".to_string(),
            "--wait-until=domcontentloaded".to_string(),
        ],
    );
    assert_eq!(goto_exit, 0);
    let goto_latest = read_json(&latest_path(root));
    assert_eq!(
        goto_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_goto")
    );
    assert_eq!(
        goto_latest
            .pointer("/navigation/url")
            .and_then(Value::as_str),
        Some("https://docs.rs")
    );
    assert_eq!(
        goto_latest
            .pointer("/navigation/wait_until")
            .and_then(Value::as_str),
        Some("domcontentloaded")
    );
    assert_eq!(
        goto_latest
            .pointer("/session/target_url")
            .and_then(Value::as_str),
        Some("https://docs.rs")
    );
    assert_claim(&goto_latest, "V11-STAGEHAND-007");

    let navback_exit = vbrowser_plane::run(
        root,
        &[
            "navback".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--wait-until=domcontentloaded".to_string(),
        ],
    );
    assert_eq!(navback_exit, 0);
    let navback_latest = read_json(&latest_path(root));
    assert_eq!(
        navback_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_navback")
    );
    assert_eq!(
        navback_latest
            .pointer("/navigation/replay_step/type")
            .and_then(Value::as_str),
        Some("navback")
    );
    assert_eq!(
        navback_latest
            .pointer("/navigation/wait_until")
            .and_then(Value::as_str),
        Some("domcontentloaded")
    );
    assert_claim(&navback_latest, "V11-STAGEHAND-008");

    let wait_exit = vbrowser_plane::run(
        root,
        &[
            "wait".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--time-ms=1".to_string(),
        ],
    );
    assert_eq!(wait_exit, 0);
    let wait_latest = read_json(&latest_path(root));
    assert_eq!(
        wait_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_wait")
    );
    assert_eq!(
        wait_latest.pointer("/wait/time_ms").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        wait_latest
            .pointer("/wait/replay_step/type")
            .and_then(Value::as_str),
        Some("wait")
    );
    assert_claim(&wait_latest, "V11-STAGEHAND-009");

    let scroll_exit = vbrowser_plane::run(
        root,
        &[
            "scroll".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--direction=down".to_string(),
            "--percentage=50".to_string(),
        ],
    );
    assert_eq!(scroll_exit, 0);
    let scroll_latest = read_json(&latest_path(root));
    assert_eq!(
        scroll_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_scroll")
    );
    assert_eq!(
        scroll_latest
            .pointer("/scroll/replay_step/type")
            .and_then(Value::as_str),
        Some("scroll")
    );
    assert_eq!(
        scroll_latest
            .pointer("/scroll/scrolled_pixels")
            .and_then(Value::as_u64),
        Some(360)
    );
    assert_claim(&scroll_latest, "V11-STAGEHAND-010");

    let click_exit = vbrowser_plane::run(
        root,
        &[
            "click".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--coordinates=120,240".to_string(),
            "--describe=primary cta".to_string(),
        ],
    );
    assert_eq!(click_exit, 0);
    let click_latest = read_json(&latest_path(root));
    assert_eq!(
        click_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_click")
    );
    assert_eq!(
        click_latest
            .pointer("/click/coordinates/0")
            .and_then(Value::as_u64),
        Some(120)
    );
    assert_eq!(
        click_latest
            .pointer("/click/coordinates/1")
            .and_then(Value::as_u64),
        Some(240)
    );
    assert_eq!(
        click_latest
            .pointer("/click/replay_step/type")
            .and_then(Value::as_str),
        Some("click")
    );
    assert_claim(&click_latest, "V11-STAGEHAND-011");

    let type_exit = vbrowser_plane::run(
        root,
        &[
            "type".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--coordinates=140,260".to_string(),
            "--describe=email input".to_string(),
            "--text=hello %name%".to_string(),
            r#"--variables-json={"name":"Jay"}"#.to_string(),
        ],
    );
    assert_eq!(type_exit, 0);
    let type_latest = read_json(&latest_path(root));
    assert_eq!(
        type_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_type")
    );
    assert_eq!(
        type_latest
            .pointer("/type_input/text")
            .and_then(Value::as_str),
        Some("hello %name%")
    );
    assert_eq!(
        type_latest
            .pointer("/type_input/resolved_text")
            .and_then(Value::as_str),
        Some("hello Jay")
    );
    assert_eq!(
        type_latest
            .pointer("/type_input/replay_step/playwright_arguments/text")
            .and_then(Value::as_str),
        Some("hello Jay")
    );
    assert_claim(&type_latest, "V11-STAGEHAND-012");

    let join_exit = vbrowser_plane::run(
        root,
        &[
            "session-control".to_string(),
            "--strict=1".to_string(),
            "--op=join".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--actor=alice".to_string(),
            "--role=shared-control".to_string(),
        ],
    );
    assert_eq!(join_exit, 0);
    let join_latest = read_json(&latest_path(root));
    assert_eq!(
        join_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_session_control")
    );
    assert_eq!(join_latest.get("op").and_then(Value::as_str), Some("join"));
    assert_claim(&join_latest, "V6-VBROWSER-001.2");

    let join_bob_exit = vbrowser_plane::run(
        root,
        &[
            "session-control".to_string(),
            "--strict=1".to_string(),
            "--op=join".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--actor=bob".to_string(),
            "--role=watch-only".to_string(),
        ],
    );
    assert_eq!(join_bob_exit, 0);

    let handoff_exit = vbrowser_plane::run(
        root,
        &[
            "session-control".to_string(),
            "--strict=1".to_string(),
            "--op=handoff".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--actor=alice".to_string(),
            "--to=bob".to_string(),
        ],
    );
    assert_eq!(handoff_exit, 0);
    let handoff_latest = read_json(&latest_path(root));
    assert_eq!(
        handoff_latest.get("op").and_then(Value::as_str),
        Some("handoff")
    );
    assert_eq!(
        handoff_latest
            .get("session")
            .and_then(|v| v.get("handoffs"))
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0)
            > 0,
        true
    );
    assert_claim(&handoff_latest, "V6-VBROWSER-001.2");

    let automate_exit = vbrowser_plane::run(
        root,
        &[
            "automate".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--actions=navigate,extract".to_string(),
        ],
    );
    assert_eq!(automate_exit, 0);
    let automate_latest = read_json(&latest_path(root));
    assert_eq!(
        automate_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_automate")
    );
    assert_eq!(
        automate_latest
            .get("run")
            .and_then(|v| v.get("telemetry"))
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(2)
    );
    assert_claim(&automate_latest, "V6-VBROWSER-001.3");

    let privacy_exit = vbrowser_plane::run(
        root,
        &[
            "privacy-guard".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--network=restricted".to_string(),
            "--recording=1".to_string(),
            "--allow-recording=1".to_string(),
            "--budget-tokens=3456".to_string(),
        ],
    );
    assert_eq!(privacy_exit, 0);
    let privacy_latest = read_json(&latest_path(root));
    assert_eq!(
        privacy_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_privacy_guard")
    );
    assert_eq!(
        privacy_latest.get("ok").and_then(Value::as_bool),
        Some(true)
    );
    assert_claim(&privacy_latest, "V6-VBROWSER-001.4");

    let snapshot_exit = vbrowser_plane::run(
        root,
        &[
            "snapshot".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--refs=1".to_string(),
        ],
    );
    assert_eq!(snapshot_exit, 0);
    let snapshot_latest = read_json(&latest_path(root));
    assert_eq!(
        snapshot_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_snapshot")
    );
    assert_eq!(
        snapshot_latest
            .get("snapshot")
            .and_then(|v| v.get("links"))
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0)
            > 0,
        true
    );
    assert_claim(&snapshot_latest, "V6-VBROWSER-002.1");

    let screenshot_exit = vbrowser_plane::run(
        root,
        &[
            "screenshot".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--annotate=1".to_string(),
            "--delay-ms=0".to_string(),
        ],
    );
    assert_eq!(screenshot_exit, 0);
    let screenshot_latest = read_json(&latest_path(root));
    assert_eq!(
        screenshot_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_screenshot")
    );
    let svg_path = screenshot_latest
        .get("artifact")
        .and_then(|v| v.get("svg_path"))
        .and_then(Value::as_str)
        .expect("svg path");
    assert!(Path::new(svg_path).exists(), "screenshot svg missing");
    assert_eq!(
        screenshot_latest
            .get("map")
            .and_then(|v| v.get("delay_ms"))
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_claim(&screenshot_latest, "V6-VBROWSER-002.2");

    let key_input_exit = vbrowser_plane::run(
        root,
        &[
            "key-input".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--method=press".to_string(),
            "--value=cmd+a".to_string(),
            "--repeat=2".to_string(),
            "--delay-ms=40".to_string(),
        ],
    );
    assert_eq!(key_input_exit, 0);
    let key_input_latest = read_json(&latest_path(root));
    assert_eq!(
        key_input_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_key_input")
    );
    assert_eq!(
        key_input_latest
            .pointer("/key_input/normalized_value")
            .and_then(Value::as_str),
        Some("Meta+A")
    );
    assert_eq!(
        key_input_latest
            .pointer("/key_input/repeat")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        key_input_latest
            .pointer("/key_input/replay_step/playwright_arguments/keys")
            .and_then(Value::as_str),
        Some("Meta+A")
    );
    assert_claim(&key_input_latest, "V11-STAGEHAND-005");

    let key_type_exit = vbrowser_plane::run(
        root,
        &[
            "key-input".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--method=type".to_string(),
            "--value=hello %name%".to_string(),
            r#"--variables-json={"name":"Jay"}"#.to_string(),
            "--repeat=1".to_string(),
            "--delay-ms=25".to_string(),
        ],
    );
    assert_eq!(key_type_exit, 0);
    let key_type_latest = read_json(&latest_path(root));
    assert_eq!(
        key_type_latest
            .pointer("/key_input/replay_step/playwright_arguments/text")
            .and_then(Value::as_str),
        Some("hello Jay")
    );
    assert_eq!(
        key_type_latest
            .pointer("/key_input/value")
            .and_then(Value::as_str),
        Some("hello %name%")
    );
    assert_eq!(
        key_type_latest
            .pointer("/key_input/resolved_value")
            .and_then(Value::as_str),
        Some("hello Jay")
    );

    let policy_fail_exit = vbrowser_plane::run(
        root,
        &[
            "action-policy".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--action=submit".to_string(),
            "--confirm=0".to_string(),
        ],
    );
    assert_eq!(policy_fail_exit, 1);
    let policy_fail_latest = read_json(&latest_path(root));
    assert_eq!(
        policy_fail_latest.get("error").and_then(Value::as_str),
        Some("confirmation_required")
    );

    let policy_pass_exit = vbrowser_plane::run(
        root,
        &[
            "action-policy".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-vb".to_string(),
            "--action=submit".to_string(),
            "--confirm=1".to_string(),
        ],
    );
    assert_eq!(policy_pass_exit, 0);
    let policy_pass_latest = read_json(&latest_path(root));
    assert_eq!(
        policy_pass_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_action_policy")
    );
    assert_claim(&policy_pass_latest, "V6-VBROWSER-002.3");

    let auth_save_exit = vbrowser_plane::run(
        root,
        &[
            "auth-save".to_string(),
            "--strict=1".to_string(),
            "--provider=github".to_string(),
            "--profile=ops".to_string(),
            "--username=alice".to_string(),
            "--secret=token-123".to_string(),
        ],
    );
    assert_eq!(auth_save_exit, 0);
    let auth_save_latest = read_json(&latest_path(root));
    assert_eq!(
        auth_save_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_auth_save")
    );
    assert_claim(&auth_save_latest, "V6-VBROWSER-002.4");

    let auth_login_exit = vbrowser_plane::run(
        root,
        &[
            "auth-login".to_string(),
            "--strict=1".to_string(),
            "--provider=github".to_string(),
            "--profile=ops".to_string(),
        ],
    );
    assert_eq!(auth_login_exit, 0);
    let auth_login_latest = read_json(&latest_path(root));
    assert_eq!(
        auth_login_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_auth_login")
    );
    assert_claim(&auth_login_latest, "V6-VBROWSER-002.4");

    let native_exit = vbrowser_plane::run(
        root,
        &[
            "native".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-native".to_string(),
            "--url=https://example.org/native".to_string(),
        ],
    );
    assert_eq!(native_exit, 0);
    let native_latest = read_json(&latest_path(root));
    assert_eq!(
        native_latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_native")
    );
    assert_claim(&native_latest, "V6-VBROWSER-002.5");

    let dashboard_exit = health_status::run(root, &["dashboard".to_string()]);
    assert_eq!(dashboard_exit, 0);
    let dashboard_latest = read_json(&health_latest_path(root));
    let metric = dashboard_latest
        .get("dashboard_metrics")
        .and_then(|v| v.get("vbrowser_session_surface"));
    assert!(metric.is_some(), "missing vbrowser dashboard metric");
}

#[test]
fn v6_vbrowser_batch12_rejects_bypass_when_strict() {
    let fixture = stage_fixture_root();
    let root = fixture.path();
    let exit = vbrowser_plane::run(
        root,
        &[
            "session-start".to_string(),
            "--strict=1".to_string(),
            "--bypass=1".to_string(),
        ],
    );
    assert_eq!(exit, 1);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_conduit_gate")
    );
}

#[test]
fn v6_vbrowser_batch12_aliases_are_canonical_and_dashboard_metric_uses_receipt_fields() {
    let fixture = stage_fixture_root();
    let root = fixture.path();

    let open_exit = vbrowser_plane::run(
        root,
        &[
            "open".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-alias".to_string(),
            "--url=example.net/path".to_string(),
        ],
    );
    assert_eq!(open_exit, 0);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("vbrowser_plane_session_start")
    );
    assert_claim(&latest, "V6-VBROWSER-001.1");
    assert_eq!(
        latest
            .pointer("/session/session_id")
            .and_then(Value::as_str),
        Some("batch12-alias")
    );

    let dashboard_exit = health_status::run(root, &["dashboard".to_string()]);
    assert_eq!(dashboard_exit, 0);
    let dashboard_latest = read_json(&health_latest_path(root));
    let metric = dashboard_latest
        .get("dashboard_metrics")
        .and_then(|v| v.get("vbrowser_session_surface"))
        .expect("vbrowser metric");
    assert_eq!(metric.get("status").and_then(Value::as_str), Some("pass"));
    assert_eq!(
        metric.get("session_id").and_then(Value::as_str),
        Some("batch12-alias")
    );
    assert_eq!(
        metric.get("stream_latency_ms").and_then(Value::as_u64),
        Some(60)
    );
    assert_eq!(
        metric.get("receipt_type").and_then(Value::as_str),
        Some("vbrowser_plane_session_start")
    );
    assert_eq!(
        metric.get("receipt_hash_present").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn v6_vbrowser_batch12_strict_handoff_requires_existing_session_and_joined_target() {
    let fixture = stage_fixture_root();
    let root = fixture.path();

    let missing_session_exit = vbrowser_plane::run(
        root,
        &[
            "control".to_string(),
            "--strict=1".to_string(),
            "--op=handoff".to_string(),
            "--session-id=batch12-missing".to_string(),
            "--actor=alice".to_string(),
            "--to=bob".to_string(),
        ],
    );
    assert_eq!(missing_session_exit, 1);
    let missing_latest = read_json(&latest_path(root));
    assert!(missing_latest
        .get("errors")
        .and_then(Value::as_array)
        .map(|rows| rows
            .iter()
            .any(|row| row.as_str() == Some("vbrowser_session_not_found")))
        .unwrap_or(false));

    let start_exit = vbrowser_plane::run(
        root,
        &[
            "start".to_string(),
            "--strict=1".to_string(),
            "--session-id=batch12-handoff".to_string(),
            "--url=https://example.com".to_string(),
        ],
    );
    assert_eq!(start_exit, 0);
    let join_alice_exit = vbrowser_plane::run(
        root,
        &[
            "control".to_string(),
            "--strict=1".to_string(),
            "--op=join".to_string(),
            "--session-id=batch12-handoff".to_string(),
            "--actor=alice".to_string(),
            "--role=shared-control".to_string(),
        ],
    );
    assert_eq!(join_alice_exit, 0);
    let invalid_handoff_exit = vbrowser_plane::run(
        root,
        &[
            "control".to_string(),
            "--strict=1".to_string(),
            "--op=handoff".to_string(),
            "--session-id=batch12-handoff".to_string(),
            "--actor=alice".to_string(),
            "--to=bob".to_string(),
        ],
    );
    assert_eq!(invalid_handoff_exit, 1);
    let invalid_handoff_latest = read_json(&latest_path(root));
    assert!(invalid_handoff_latest
        .get("errors")
        .and_then(Value::as_array)
        .map(|rows| rows
            .iter()
            .any(|row| row.as_str() == Some("vbrowser_handoff_target_not_joined")))
        .unwrap_or(false));

    let join_bob_exit = vbrowser_plane::run(
        root,
        &[
            "control".to_string(),
            "--strict=1".to_string(),
            "--op=join".to_string(),
            "--session-id=batch12-handoff".to_string(),
            "--actor=bob".to_string(),
            "--role=watch-only".to_string(),
        ],
    );
    assert_eq!(join_bob_exit, 0);
    let valid_handoff_exit = vbrowser_plane::run(
        root,
        &[
            "control".to_string(),
            "--strict=1".to_string(),
            "--op=handoff".to_string(),
            "--session-id=batch12-handoff".to_string(),
            "--actor=alice".to_string(),
            "--to=bob".to_string(),
        ],
    );
    assert_eq!(valid_handoff_exit, 0);
    let handoff_latest = read_json(&latest_path(root));
    assert_eq!(
        handoff_latest.get("op").and_then(Value::as_str),
        Some("handoff")
    );
    assert_eq!(
        handoff_latest
            .get("session_exists")
            .and_then(Value::as_bool),
        Some(true)
    );
}
