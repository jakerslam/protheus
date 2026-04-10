// SPDX-License-Identifier: Apache-2.0

use protheus_ops_core::autonomy_controller;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn latest_path(root: &Path) -> PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("autonomy_controller")
        .join("latest.json")
}

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read json");
    serde_json::from_str(&raw).expect("decode json")
}

fn has_claim(receipt: &Value, claim_id: &str) -> bool {
    receipt
        .get("claim_evidence")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .any(|row| row.get("id").and_then(Value::as_str) == Some(claim_id))
}

fn run_cmd(root: &Path, args: &[&str]) -> i32 {
    let argv = args.iter().map(|row| row.to_string()).collect::<Vec<_>>();
    autonomy_controller::run(root, &argv)
}

fn assert_latest_type(root: &Path, expected: &str) -> Value {
    let latest = read_json(&latest_path(root));
    assert_eq!(latest.get("type").and_then(Value::as_str), Some(expected));
    latest
}

#[test]
fn v6_autonomy_and_v8_agent_era_lanes_execute_with_behavior_proof() {
    let root = tempfile::tempdir().expect("tempdir");

    assert_eq!(
        run_cmd(
            root.path(),
            &[
                "hand-new",
                "--strict=1",
                "--hand-id=alpha",
                "--template=researcher",
                "--schedule=*/15 * * * *",
                "--provider=bitnet",
                "--fallback=local-moe",
            ],
        ),
        0
    );
    let mut latest = assert_latest_type(root.path(), "autonomy_hand_new");
    assert!(has_claim(&latest, "V6-AUTONOMY-001.1"));

    assert_eq!(
        run_cmd(
            root.path(),
            &[
                "hand-cycle",
                "--strict=1",
                "--hand-id=alpha",
                "--goal=triage backlog",
                "--provider=bitnet",
            ],
        ),
        0
    );
    latest = assert_latest_type(root.path(), "autonomy_hand_cycle");
    assert!(has_claim(&latest, "V6-AUTONOMY-001.2"));
    assert!(has_claim(&latest, "V6-AUTONOMY-001.3"));
    assert!(latest
        .pointer("/chain/merkle_root")
        .and_then(Value::as_str)
        .map(|v| !v.is_empty())
        .unwrap_or(false));

    assert_eq!(
        run_cmd(
            root.path(),
            &[
                "hand-memory-page",
                "--strict=1",
                "--hand-id=alpha",
                "--op=page-in",
                "--tier=archival",
                "--key=q1-plan",
            ],
        ),
        0
    );
    latest = assert_latest_type(root.path(), "autonomy_hand_memory_page");
    assert!(has_claim(&latest, "V6-AUTONOMY-001.4"));
    assert!(latest
        .pointer("/memory/archival")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().any(|v| v.as_str() == Some("q1-plan")))
        .unwrap_or(false));

    assert_eq!(
        run_cmd(
            root.path(),
            &[
                "hand-wasm-task",
                "--strict=1",
                "--hand-id=alpha",
                "--task=render",
                "--fuel=120000",
                "--epoch-ms=1200",
            ],
        ),
        0
    );
    latest = assert_latest_type(root.path(), "autonomy_hand_wasm_task");
    assert!(has_claim(&latest, "V6-AUTONOMY-001.5"));
    assert!(latest
        .pointer("/result/work_units")
        .and_then(Value::as_u64)
        .map(|v| v > 0)
        .unwrap_or(false));

    assert_eq!(
        run_cmd(
            root.path(),
            &[
                "ephemeral-run",
                "--strict=1",
                "--goal=summarize ticket risks",
                "--domain=research",
                "--ui-leaf=1",
            ],
        ),
        0
    );
    latest = assert_latest_type(root.path(), "autonomy_ephemeral_run");
    assert!(has_claim(&latest, "V8-AGENT-ERA-001.1"));
    assert!(has_claim(&latest, "V8-AGENT-ERA-001.2"));
    assert!(has_claim(&latest, "V8-AGENT-ERA-001.3"));
    assert!(has_claim(&latest, "V8-AGENT-ERA-001.4"));
    assert!(has_claim(&latest, "V8-AGENT-ERA-001.5"));
    assert_eq!(
        latest.pointer("/run/state/discarded_runtime"),
        Some(&Value::Bool(true))
    );

    assert_eq!(run_cmd(root.path(), &["trunk-status", "--strict=1"]), 0);
    latest = assert_latest_type(root.path(), "autonomy_trunk_status");
    assert!(has_claim(&latest, "V8-AGENT-ERA-001.2"));
    assert!(has_claim(&latest, "V8-AGENT-ERA-001.5"));
    assert!(latest
        .pointer("/events/count")
        .and_then(Value::as_u64)
        .map(|v| v > 0)
        .unwrap_or(false));
}

#[test]
fn v6_autonomy_and_v8_agent_era_fail_closed_paths_are_enforced() {
    let root = tempfile::tempdir().expect("tempdir");

    assert_eq!(run_cmd(root.path(), &["hand-new", "--strict=1", "--bypass=1"]), 1);
    let mut latest = assert_latest_type(root.path(), "autonomy_controller_conduit_gate");
    assert_eq!(
        latest.get("error").and_then(Value::as_str),
        Some("conduit_bypass_rejected")
    );

    assert_eq!(
        run_cmd(root.path(), &["ephemeral-run", "--strict=1", "--domain=forbidden"]),
        1
    );
    latest = assert_latest_type(root.path(), "autonomy_ephemeral_run");
    assert_eq!(
        latest.get("error").and_then(Value::as_str),
        Some("domain_constraint_denied")
    );
}
