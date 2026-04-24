use crate::runtime_lane::{run_runtime_lane, RuntimeLaneRequest};
use serde_json::json;

#[test]
fn runtime_lane_pack_permission_ask_fail_closes() {
    let response = run_runtime_lane(RuntimeLaneRequest {
        name: "lane-agent".to_string(),
        preamble: Some("You are concise.".to_string()),
        initial_prompt: "Try issue ops".to_string(),
        provider: Some("local-echo".to_string()),
        model: None,
        tools: vec!["web.search".to_string()],
        capability_packs: vec!["issue-ops".to_string()],
        lifespan_seconds: Some(120),
        metadata: json!({}),
        permissions_manifest: Some(json!({
            "grants": {
                "github.issue.create": 0,
                "memory.read": 1,
                "memory.write": 1
            }
        })),
        wasm_sandbox: None,
        voice_session: None,
        receipt_merkle: None,
        previous_receipt_root: None,
        schedule_interval_seconds: None,
        schedule_max_runs: None,
    })
    .expect("runtime lane");
    assert!(!response.ok);
    assert_eq!(
        response.error.as_deref(),
        Some("runtime_lane_pack_permission_denied")
    );
    let contract = response.contract.to_string();
    assert!(contract.contains("\"enforcement_mode\":\"strict_fail_closed\""));
    assert!(contract.contains("\"blocked_permission_key_lineage\""));
    assert!(contract.contains("\"permissions_parent_snapshot\""));
}

#[test]
fn runtime_lane_pack_permission_deny_fail_closes() {
    let response = run_runtime_lane(RuntimeLaneRequest {
        name: "lane-agent".to_string(),
        preamble: Some("You are concise.".to_string()),
        initial_prompt: "Try issue ops".to_string(),
        provider: Some("local-echo".to_string()),
        model: None,
        tools: vec!["web.search".to_string()],
        capability_packs: vec!["issue-ops".to_string()],
        lifespan_seconds: Some(120),
        metadata: json!({}),
        permissions_manifest: Some(json!({
            "grants": {
                "github.issue.create": -1,
                "memory.read": 1,
                "memory.write": 1
            }
        })),
        wasm_sandbox: None,
        voice_session: None,
        receipt_merkle: None,
        previous_receipt_root: None,
        schedule_interval_seconds: None,
        schedule_max_runs: None,
    })
    .expect("runtime lane");
    assert!(!response.ok);
    assert_eq!(
        response.error.as_deref(),
        Some("runtime_lane_pack_permission_denied")
    );
}

#[test]
fn runtime_lane_schedule_zero_interval_fail_closes() {
    let response = run_runtime_lane(RuntimeLaneRequest {
        name: "lane-agent".to_string(),
        preamble: Some("You are concise.".to_string()),
        initial_prompt: "Run schedule".to_string(),
        provider: Some("local-echo".to_string()),
        model: None,
        tools: vec!["web.search".to_string()],
        capability_packs: vec!["research".to_string()],
        lifespan_seconds: Some(120),
        metadata: json!({}),
        permissions_manifest: Some(json!({
            "grants": {
                "memory.read": 1,
                "web.search": 1
            }
        })),
        wasm_sandbox: None,
        voice_session: None,
        receipt_merkle: None,
        previous_receipt_root: None,
        schedule_interval_seconds: Some(0),
        schedule_max_runs: Some(4),
    })
    .expect("runtime lane");
    assert!(!response.ok);
    assert_eq!(
        response.error.as_deref(),
        Some("runtime_lane_schedule_interval_invalid")
    );
}

#[test]
fn runtime_lane_schedule_zero_max_runs_fail_closes() {
    let response = run_runtime_lane(RuntimeLaneRequest {
        name: "lane-agent".to_string(),
        preamble: Some("You are concise.".to_string()),
        initial_prompt: "Run schedule".to_string(),
        provider: Some("local-echo".to_string()),
        model: None,
        tools: vec!["web.search".to_string()],
        capability_packs: vec!["research".to_string()],
        lifespan_seconds: Some(120),
        metadata: json!({}),
        permissions_manifest: Some(json!({
            "grants": {
                "memory.read": 1,
                "web.search": 1
            }
        })),
        wasm_sandbox: None,
        voice_session: None,
        receipt_merkle: None,
        previous_receipt_root: None,
        schedule_interval_seconds: Some(120),
        schedule_max_runs: Some(0),
    })
    .expect("runtime lane");
    assert!(!response.ok);
    assert_eq!(
        response.error.as_deref(),
        Some("runtime_lane_schedule_max_runs_invalid")
    );
}
