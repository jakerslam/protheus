// SPDX-License-Identifier: Apache-2.0

use protheus_ops_core::{company_plane, observability_plane, substrate_plane};
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

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read");
    serde_json::from_str(&raw).expect("parse")
}

fn latest_path(root: &Path, lane: &str) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join(lane)
        .join("latest.json")
}

fn company_ticket_history(root: &Path, team: &str) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("company_plane")
        .join("tickets")
        .join("history")
        .join(format!("{team}.jsonl"))
}

#[test]
fn v6_batch32_company_ticket_chain_detects_tamper_and_fails_closed() {
    let fixture = stage_fixture_root();
    let root = fixture.path();
    let team = "batch32";
    let ticket_id = "TKT-B32";

    let create_exit = company_plane::run(
        root,
        &[
            "ticket".to_string(),
            "--strict=1".to_string(),
            "--op=create".to_string(),
            format!("--team={team}"),
            format!("--ticket-id={ticket_id}"),
            "--title=batch32".to_string(),
            "--tool-call-id=tc-1".to_string(),
        ],
    );
    assert_eq!(create_exit, 0);

    let assign_exit = company_plane::run(
        root,
        &[
            "ticket".to_string(),
            "--strict=1".to_string(),
            "--op=assign".to_string(),
            format!("--team={team}"),
            format!("--ticket-id={ticket_id}"),
            "--assignee=alpha".to_string(),
            "--tool-call-id=tc-2".to_string(),
        ],
    );
    assert_eq!(assign_exit, 0);

    // Tamper with event details while leaving the stored event_hash untouched.
    let history_path = company_ticket_history(root, team);
    let raw = fs::read_to_string(&history_path).expect("read ticket history");
    let mut lines = raw.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
    let mut first = serde_json::from_str::<Value>(&lines[0]).expect("parse first history line");
    first["details"]["title"] = Value::String("tampered-title".to_string());
    lines[0] = serde_json::to_string(&first).expect("encode tampered line");
    fs::write(history_path, format!("{}\n", lines.join("\n"))).expect("write tampered history");

    let transition_exit = company_plane::run(
        root,
        &[
            "ticket".to_string(),
            "--strict=1".to_string(),
            "--op=transition".to_string(),
            format!("--team={team}"),
            format!("--ticket-id={ticket_id}"),
            "--to=in_review".to_string(),
            "--tool-call-id=tc-3".to_string(),
        ],
    );
    assert_eq!(transition_exit, 1);
    let latest = read_json(&latest_path(root, "company_plane"));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("company_plane_ticket")
    );
    let issues = latest
        .get("chain_issues")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        issues.iter().any(|row| {
            row.as_str()
                .map(|v| v.contains("event_hash_mismatch_row_0"))
                .unwrap_or(false)
        }),
        "expected event hash mismatch issue after tamper"
    );

    // Conduit fail-closed checks for company operations.
    let budget_bypass_exit = company_plane::run(
        root,
        &[
            "budget-enforce".to_string(),
            "--strict=1".to_string(),
            "--agent=alpha".to_string(),
            "--tokens=10".to_string(),
            "--bypass=1".to_string(),
        ],
    );
    assert_eq!(budget_bypass_exit, 1);

    let orchestrate_bypass_exit = company_plane::run(
        root,
        &[
            "orchestrate-agency".to_string(),
            "--strict=1".to_string(),
            "--team=ops".to_string(),
            "--bypass=1".to_string(),
        ],
    );
    assert_eq!(orchestrate_bypass_exit, 1);
}

#[test]
fn v6_batch32_substrate_csi_contracts_emit_required_events_and_policy_denials() {
    let fixture = stage_fixture_root();
    let root = fixture.path();

    let policy_exit = substrate_plane::run(
        root,
        &[
            "csi-policy".to_string(),
            "--strict=1".to_string(),
            "--consent=1".to_string(),
            "--locality=local-only".to_string(),
            "--retention-minutes=60".to_string(),
            "--biometric-risk=medium".to_string(),
        ],
    );
    assert_eq!(policy_exit, 0);

    let capture_exit = substrate_plane::run(
        root,
        &[
            "csi-capture".to_string(),
            "--strict=1".to_string(),
            "--adapter=wifi-csi-esp32".to_string(),
        ],
    );
    assert_eq!(capture_exit, 0);
    let capture_latest = read_json(&latest_path(root, "substrate_plane"));
    let events = capture_latest
        .get("capture")
        .and_then(|v| v.get("layer_two_decode"))
        .and_then(|v| v.get("normalized_events"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|row| row.get("event").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert!(events.iter().any(|v| v == "presence"));
    assert!(events.iter().any(|v| v == "respiration"));
    assert!(events.iter().any(|v| v == "heartbeat_proxy"));
    assert!(events.iter().any(|v| v == "pose_proxy"));

    let module_register_exit = substrate_plane::run(
        root,
        &[
            "csi-module".to_string(),
            "--strict=1".to_string(),
            "--op=register".to_string(),
            "--module=sleep-staging".to_string(),
            "--input-contract=csi.normalized_events.v1".to_string(),
            "--budget-units=100".to_string(),
            "--privacy-class=restricted".to_string(),
            "--degrade-behavior=drop-to-presence-only".to_string(),
        ],
    );
    assert_eq!(module_register_exit, 0);
    let module_latest = read_json(&latest_path(root, "substrate_plane"));
    assert_eq!(
        module_latest
            .get("module")
            .and_then(|v| v.get("degrade_behavior"))
            .and_then(Value::as_str),
        Some("drop-to-presence-only")
    );

    let export_attempt_exit = substrate_plane::run(
        root,
        &[
            "csi-policy".to_string(),
            "--strict=1".to_string(),
            "--consent=1".to_string(),
            "--locality=local-only".to_string(),
            "--allow-export=1".to_string(),
        ],
    );
    assert_eq!(export_attempt_exit, 1);
    let export_latest = read_json(&latest_path(root, "substrate_plane"));
    let violations = export_latest
        .get("policy")
        .and_then(|v| v.get("violations"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(violations.iter().any(|row| {
        row.as_str()
            .map(|v| v == "export_denied_by_default")
            .unwrap_or(false)
    }));

    let invalid_eye_op_exit = substrate_plane::run(
        root,
        &[
            "eye-bind".to_string(),
            "--strict=1".to_string(),
            "--op=disable".to_string(),
            "--source=wifi".to_string(),
        ],
    );
    assert_eq!(invalid_eye_op_exit, 1);
}

#[test]
fn v6_batch32_observability_workflow_validates_schedule_and_compiles_step_traces() {
    let fixture = stage_fixture_root();
    let root = fixture.path();

    let invalid_cron_exit = observability_plane::run(
        root,
        &[
            "workflow".to_string(),
            "--strict=1".to_string(),
            "--op=upsert".to_string(),
            "--workflow-id=bad-cron".to_string(),
            "--trigger=cron".to_string(),
            "--schedule=every-5-minutes".to_string(),
        ],
    );
    assert_eq!(invalid_cron_exit, 1);

    let upsert_exit = observability_plane::run(
        root,
        &[
            "workflow".to_string(),
            "--strict=1".to_string(),
            "--op=upsert".to_string(),
            "--workflow-id=good-cron".to_string(),
            "--trigger=cron".to_string(),
            "--schedule=*/5 * * * *".to_string(),
            "--steps-json=[\"collect\",\"annotate\",\"notify\"]".to_string(),
        ],
    );
    assert_eq!(upsert_exit, 0);
    let upsert_latest = read_json(&latest_path(root, "observability_plane"));
    let node_count = upsert_latest
        .get("workflow")
        .and_then(|v| v.get("compiled_graph"))
        .and_then(|v| v.get("nodes"))
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert_eq!(node_count, 3);

    let run_exit = observability_plane::run(
        root,
        &[
            "workflow".to_string(),
            "--strict=1".to_string(),
            "--op=run".to_string(),
            "--workflow-id=good-cron".to_string(),
        ],
    );
    assert_eq!(run_exit, 0);
    let run_latest = read_json(&latest_path(root, "observability_plane"));
    let step_trace_len = run_latest
        .get("run")
        .and_then(|v| v.get("step_trace"))
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert_eq!(step_trace_len, 3);

    let bypass_exit = observability_plane::run(
        root,
        &[
            "workflow".to_string(),
            "--strict=1".to_string(),
            "--op=list".to_string(),
            "--bypass=1".to_string(),
        ],
    );
    assert_eq!(bypass_exit, 1);
}
