use crate::runtime_lane::{run_runtime_lane, RuntimeLaneRequest};
use serde_json::{json, Value};

#[test]
fn runtime_lane_executes_with_capability_pack_defaults() {
    let response = run_runtime_lane(RuntimeLaneRequest {
        name: "lane-agent".to_string(),
        preamble: Some("You are concise.".to_string()),
        initial_prompt: "Summarize system status in one line.".to_string(),
        provider: Some("local-echo".to_string()),
        model: None,
        tools: vec!["web.search".to_string()],
        capability_packs: vec!["research".to_string()],
        lifespan_seconds: Some(120),
        metadata: json!({"lane":"runtime"}),
        permissions_manifest: Some(json!({
            "grants": {
                "voice.realtime": 1,
                "memory.read": 1,
                "web.search": 1
            }
        })),
        wasm_sandbox: Some(json!({
            "enabled": true,
            "allow_network": true,
            "allowed_modules": ["planner.module"]
        })),
        voice_session: Some(json!({
            "transport": "webrtc",
            "provider": "realtime",
            "model": "gpt-realtime"
        })),
        receipt_merkle: Some(json!({
            "enabled": true,
            "seed": "wave4"
        })),
        previous_receipt_root: Some("root0".to_string()),
        schedule_interval_seconds: Some(120),
        schedule_max_runs: Some(12),
    })
    .expect("runtime lane");
    assert!(response.ok);
    assert_eq!(
        response
            .receipt
            .get("type")
            .and_then(serde_json::Value::as_str),
        Some("agent_run_receipt")
    );
    assert!(
        response
            .contract
            .get("receipt_merkle")
            .and_then(|value| value.get("root"))
            .and_then(Value::as_str)
            .is_some()
    );
}

#[test]
fn runtime_lane_fail_closes_when_memory_write_permission_is_not_allowed() {
    let response = run_runtime_lane(RuntimeLaneRequest {
        name: "lane-agent".to_string(),
        preamble: Some("You are concise.".to_string()),
        initial_prompt: "Try mutation".to_string(),
        provider: Some("local-echo".to_string()),
        model: None,
        tools: vec!["memory.write".to_string()],
        capability_packs: vec![],
        lifespan_seconds: Some(120),
        metadata: json!({"lane":"runtime"}),
        permissions_manifest: Some(json!({
            "grants": {
                "memory.write": 0
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
        Some("runtime_lane_memory_write_denied")
    );
}
