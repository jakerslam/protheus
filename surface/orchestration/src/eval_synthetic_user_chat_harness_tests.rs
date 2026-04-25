// SRS: V12-SYNTHETIC-USER-HARNESS-001
use super::*;
use std::fs;
use std::path::Path;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{name}_{}", now_iso_like().replace(':', "_")))
}

fn write_case_file(root: &Path, payload: &Value) -> PathBuf {
    let path = root.join("cases.json");
    fs::create_dir_all(root).expect("temp root");
    write_json(path.to_str().unwrap(), payload).expect("case write");
    path
}

fn harness_args(root: &Path, cases: &Path, strict: bool) -> Vec<String> {
    vec![
        format!("--cases={}", cases.display()),
        format!("--out={}", root.join("out.json").display()),
        format!("--out-latest={}", root.join("latest.json").display()),
        format!("--out-markdown={}", root.join("report.md").display()),
        format!("--failures-out={}", root.join("failures.jsonl").display()),
        format!("--attention-dir={}", root.join("attention").display()),
        format!("--strict={}", if strict { "1" } else { "0" }),
    ]
}

#[test]
fn synthetic_user_harness_preserves_normal_user_message_contract() {
    let root = temp_path("synthetic_user_harness_pass");
    let cases = write_case_file(
        &root,
        &json!({
            "thresholds": {
                "min_cases": 1,
                "min_pass_rate": 1.0,
                "max_failures": 0,
                "simple_direct_max_latency_ms": 5000,
                "simple_direct_max_response_tokens": 24,
                "simple_direct_max_stage_count": 2
            },
            "defaults": {"agent_id": "agent-synthetic"},
            "cases": [{
                "id": "hello",
                "turns": [{
                    "turn_id": "t1",
                    "user_message": "hey",
                    "mock_response": {
                        "response": "Hey, I am here.",
                        "response_workflow": {"stage_statuses": [{"stage": "gate_1_need_tool_access_menu", "status": "answered_no"}]},
                        "live_eval_monitor": {"chat_injection_allowed": false}
                    },
                    "expect": {
                        "required_substrings": ["Hey"],
                        "require_workflow_visibility": true,
                        "require_live_eval_monitor": true,
                        "simple_direct_conversation": true
                    }
                }]
            }]
        }),
    );
    let code = run_synthetic_user_chat_harness(&harness_args(&root, &cases, true));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(report.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        report
            .pointer("/transport_contract/normal_user_message_route_only")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        report
            .pointer("/turns/0/request_body_keys/0")
            .and_then(Value::as_str),
        Some("message")
    );
    assert_eq!(
        report
            .pointer("/turns/0/workflow_stage_count")
            .and_then(Value::as_u64),
        Some(1)
    );
}

#[test]
fn synthetic_user_harness_enforces_simple_direct_budgets() {
    let root = temp_path("synthetic_user_harness_budget");
    let cases = write_case_file(
        &root,
        &json!({
            "thresholds": {
                "min_cases": 1,
                "min_pass_rate": 1.0,
                "max_failures": 0,
                "simple_direct_max_response_tokens": 3,
                "simple_direct_max_stage_count": 2
            },
            "defaults": {"agent_id": "agent-budget"},
            "cases": [{
                "id": "slow_direct",
                "turns": [{
                    "turn_id": "t1",
                    "user_message": "hey",
                    "mock_response": {
                        "response": "This direct response is intentionally too verbose for the tiny budget.",
                        "tools": [{"name": "batch_query"}],
                        "response_workflow": {
                            "tool_gate": {"should_call_tools": true},
                            "stage_statuses": [
                                {"stage": "gate_1_need_tool_access_menu"},
                                {"stage": "gate_2_tool_family_menu"},
                                {"stage": "gate_3_tool_menu"}
                            ]
                        },
                        "live_eval_monitor": {"chat_injection_allowed": false}
                    },
                    "expect": {
                        "require_workflow_visibility": true,
                        "simple_direct_conversation": true
                    }
                }]
            }]
        }),
    );
    let code = run_synthetic_user_chat_harness(&harness_args(&root, &cases, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    let failures = report
        .pointer("/turns/0/failures")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    assert!(
        failures
            .iter()
            .any(|row| row.starts_with("simple_direct_response_tokens_over_budget")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|row| row.starts_with("simple_direct_stage_count_over_budget")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|row| row == "simple_direct_recorded_tool_calls"),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|row| row == "simple_direct_tool_gate_should_call_tools"),
        "{failures:?}"
    );
}

#[test]
fn synthetic_user_harness_flags_fallback_text_and_writes_attention() {
    let root = temp_path("synthetic_user_harness_failure");
    let cases = write_case_file(
        &root,
        &json!({
            "thresholds": {"min_cases": 1, "min_pass_rate": 1.0, "max_failures": 0},
            "defaults": {"agent_id": "agent-loop"},
            "cases": [{
                "id": "fallback_loop",
                "turns": [{
                    "turn_id": "t1",
                    "user_message": "what is going on?",
                    "mock_response": {
                        "response": "I hit a response finalization edge on that turn.",
                        "live_eval_monitor": {"chat_injection_allowed": false}
                    }
                }]
            }]
        }),
    );
    let code = run_synthetic_user_chat_harness(&harness_args(&root, &cases, false));
    assert_eq!(code, 0);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(report.get("ok").and_then(Value::as_bool), Some(false));
    assert!(root.join("attention/agent-loop.attention.jsonl").exists());
    let attention = fs::read_to_string(root.join("attention/agent-loop.attention.jsonl"))
        .expect("attention jsonl");
    let event: Value = serde_json::from_str(attention.lines().next().unwrap()).expect("event json");
    assert_eq!(
        event.get("owner_component").and_then(Value::as_str),
        Some("surface.orchestration.finalization")
    );
    let replay = event
        .get("replay_command")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        replay.contains("synthetic-user-chat-harness --live=0 --agent-id=agent-loop --strict=1"),
        "{replay}"
    );
    assert!(
        replay.contains(&format!("--cases={}", cases.display())),
        "{replay}"
    );
}

#[test]
fn synthetic_user_harness_blocks_remote_live_dashboard_by_default() {
    let root = temp_path("synthetic_user_harness_remote");
    let cases = write_case_file(
        &root,
        &json!({
            "thresholds": {"min_cases": 1},
            "cases": [{"id": "remote", "turns": [{"user_message": "hey"}]}]
        }),
    );
    let mut args = harness_args(&root, &cases, true);
    args.push("--live=1".to_string());
    args.push("--base-url=https://example.com".to_string());
    let code = run_synthetic_user_chat_harness(&args);
    assert_eq!(code, 1);
    let report = read_json(root.join("out.json").to_str().unwrap());
    assert_eq!(
        report.pointer("/setup_failures/0").and_then(Value::as_str),
        Some("remote_dashboard_url_requires_allow_remote")
    );
}

#[test]
fn synthetic_user_harness_live_monitor_freshness_contract() {
    let stale = live_monitor_freshness_report(
        true,
        true,
        Some("unix_ms:10"),
        Some("unix_ms:10"),
        &[],
        "local/state/ops/eval_live_monitor/latest.json",
    );
    assert_eq!(stale.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        stale.get("failure_reason").and_then(Value::as_str),
        Some("live_eval_monitor_timestamp_not_advanced")
    );

    let fresh = live_monitor_freshness_report(
        true,
        true,
        Some("unix_ms:10"),
        Some("unix_ms:11"),
        &[],
        "local/state/ops/eval_live_monitor/latest.json",
    );
    assert_eq!(fresh.get("ok").and_then(Value::as_bool), Some(true));
}

#[test]
fn misty_live_health_gate_command_requires_live_agent_and_strict() {
    let command = misty_live_health_gate_required_command("agent-5bc62b0875a9");
    assert!(
        command.contains(
            "synthetic-user-chat-harness --live=1 --agent-id=agent-5bc62b0875a9 --strict=1"
        ),
        "{command}"
    );
}
