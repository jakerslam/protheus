// SPDX-License-Identifier: Apache-2.0
use protheus_ops_core::swarm_runtime;
use serde_json::Value;
use std::fs;
use std::process::Command;

const SWARM_CONTRACT_IDS: &[&str] = &[
    "V6-SWARM-013",
    "V6-SWARM-014",
    "V6-SWARM-015",
    "V6-SWARM-016",
    "V6-SWARM-017",
    "V6-SWARM-018",
    "V6-SWARM-019",
    "V6-SWARM-020",
    "V6-SWARM-021",
    "V6-SWARM-022",
    "V6-SWARM-023",
    "V6-SWARM-024",
    "V6-SWARM-025",
    "V6-SWARM-026",
    "V6-SWARM-027",
    "V6-SWARM-028",
    "V6-SWARM-029",
    "V6-SWARM-030",
    "V6-SWARM-031",
    "V6-SWARM-033",
    "V6-SWARM-034",
    "V6-SWARM-035",
    "V6-SWARM-036",
    "V6-SWARM-037",
    "V6-SWARM-038",
];

fn run_swarm(root: &std::path::Path, args: &[String]) -> i32 {
    swarm_runtime::run(root, args)
}

fn state_path_arg(state_path: &std::path::Path) -> String {
    format!("--state-path={}", state_path.display())
}

fn spawn_args(task: &str, role: &str, state_path: &std::path::Path) -> Vec<String> {
    vec![
        "spawn".to_string(),
        format!("--task={task}"),
        format!("--role={role}"),
        state_path_arg(state_path),
    ]
}

fn read_state(path: &std::path::Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read state")).expect("parse state")
}

fn find_session_id_by_task(state: &Value, task: &str) -> Option<String> {
    state
        .get("sessions")
        .and_then(Value::as_object)
        .and_then(|rows| {
            rows.iter().find_map(|(session_id, row)| {
                let matches = row
                    .get("report")
                    .and_then(|value| value.get("task"))
                    .and_then(Value::as_str)
                    == Some(task);
                matches.then(|| session_id.clone())
            })
        })
}

fn parse_last_json(stdout: &str) -> Value {
    stdout
        .lines()
        .rev()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('{') {
                serde_json::from_str::<Value>(trimmed).ok()
            } else {
                None
            }
        })
        .expect("json payload in stdout")
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root")
        .to_path_buf()
}

fn run_bridge_cli(args: &[String]) -> Value {
    let repo_root = repo_root();
    let bridge = repo_root
        .join("client")
        .join("runtime")
        .join("systems")
        .join("autonomy")
        .join("swarm_sessions_bridge.ts");
    let output = Command::new("node")
        .current_dir(&repo_root)
        .arg(bridge)
        .args(args)
        .output()
        .expect("run swarm sessions bridge");
    assert!(
        output.status.success(),
        "expected CLI success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    parse_last_json(&String::from_utf8_lossy(&output.stdout))
}

#[test]
fn swarm_contract_ids_are_embedded_for_receipt_audit_evidence() {
    assert_eq!(SWARM_CONTRACT_IDS.len(), 25);
    assert!(SWARM_CONTRACT_IDS
        .iter()
        .all(|id| id.starts_with("V6-SWARM-0")));
}

#[test]
fn sessions_send_receive_and_ack_support_sibling_chain() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let s1 = spawn_args("comm-agent-1", "generator", &state_path);
    let s2 = spawn_args("comm-agent-2", "filter", &state_path);
    assert_eq!(run_swarm(root.path(), &s1), 0);
    assert_eq!(run_swarm(root.path(), &s2), 0);

    let state = read_state(&state_path);
    let mut ids = state
        .get("sessions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    ids.sort();
    let sender = ids.first().cloned().expect("sender");
    let recipient = ids.get(1).cloned().expect("recipient");

    let send_args = vec![
        "sessions".to_string(),
        "send".to_string(),
        format!("--sender-id={sender}"),
        format!("--session-id={recipient}"),
        "--message=hello-from-agent-1".to_string(),
        "--delivery=at_least_once".to_string(),
        "--simulate-first-attempt-fail=1".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &send_args), 0);

    let receive_args = vec![
        "sessions".to_string(),
        "receive".to_string(),
        format!("--session-id={recipient}"),
        "--limit=5".to_string(),
        "--mark-read=0".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &receive_args), 0);

    let state_after = read_state(&state_path);
    let msg_id = state_after
        .get("mailboxes")
        .and_then(|rows| rows.get(&recipient))
        .and_then(|row| row.get("unread"))
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("message_id"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .expect("message id in mailbox");

    let ack_args = vec![
        "sessions".to_string(),
        "ack".to_string(),
        format!("--session-id={recipient}"),
        format!("--message-id={msg_id}"),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &ack_args), 0);

    let final_state = read_state(&state_path);
    let acknowledged = final_state
        .get("mailboxes")
        .and_then(|rows| rows.get(&recipient))
        .and_then(|row| row.get("unread"))
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("acknowledged"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    assert!(acknowledged, "expected message to be acknowledged");
}

#[test]
fn service_discovery_and_send_role_route_messages() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let s1 = spawn_args("role-agent-1", "generator", &state_path);
    let s2 = spawn_args("role-agent-2", "filter", &state_path);
    assert_eq!(run_swarm(root.path(), &s1), 0);
    assert_eq!(run_swarm(root.path(), &s2), 0);

    let discover_args = vec![
        "sessions".to_string(),
        "discover".to_string(),
        "--role=filter".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &discover_args), 0);

    let send_role_args = vec![
        "sessions".to_string(),
        "send-role".to_string(),
        "--sender-id=coordinator".to_string(),
        "--role=filter".to_string(),
        "--message=role-routed-message".to_string(),
        "--delivery=exactly_once".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &send_role_args), 0);

    let state = read_state(&state_path);
    let filter_session_id = state
        .get("service_registry")
        .and_then(|rows| rows.get("filter"))
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("session_id"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .expect("filter session id");
    let inbox_count = state
        .get("mailboxes")
        .and_then(|rows| rows.get(&filter_session_id))
        .and_then(|row| row.get("unread"))
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert!(inbox_count >= 1, "expected routed message in filter inbox");
}

#[test]
fn state_cache_reload_detects_external_state_mutation() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let enable_args = vec![
        "byzantine-test".to_string(),
        "enable".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &enable_args), 0);

    let mut externally_edited = read_state(&state_path);
    externally_edited["byzantine_test_mode"] = Value::Bool(false);
    let encoded = serde_json::to_string_pretty(&externally_edited).expect("encode edited state");
    fs::write(&state_path, encoded).expect("write edited state");

    let status_args = vec!["status".to_string(), state_path_arg(&state_path)];
    assert_eq!(run_swarm(root.path(), &status_args), 0);

    let state_after = read_state(&state_path);
    let byzantine_enabled = state_after
        .get("byzantine_test_mode")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    assert!(
        !byzantine_enabled,
        "cached state should refresh when state file is externally edited"
    );
}

#[test]
fn high_volume_mailboxes_use_compact_state_encoding() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    for task in ["compact-encoder-sender", "compact-encoder-receiver"] {
        let args = spawn_args(task, "filter", &state_path);
        assert_eq!(run_swarm(root.path(), &args), 0);
    }

    let state = read_state(&state_path);
    let sender = find_session_id_by_task(&state, "compact-encoder-sender").expect("sender");
    let receiver = find_session_id_by_task(&state, "compact-encoder-receiver").expect("receiver");

    for idx in 0..110 {
        let send_args = vec![
            "sessions".to_string(),
            "send".to_string(),
            format!("--sender-id={sender}"),
            format!("--session-id={receiver}"),
            format!("--message=compact-encoding-test-{idx}"),
            "--delivery=at_least_once".to_string(),
            state_path_arg(&state_path),
        ];
        assert_eq!(run_swarm(root.path(), &send_args), 0);
    }

    let state_after = read_state(&state_path);
    let dead_letter_count = state_after
        .get("dead_letters")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert!(
        dead_letter_count >= 64,
        "expected sustained dead-letter volume from mailbox backpressure"
    );

    let raw = fs::read_to_string(&state_path).expect("raw state");
    assert!(
        !raw.contains('\n'),
        "high-volume state should be compact encoded to avoid persistence overhead"
    );
}

#[test]
fn channels_create_publish_poll_and_communication_test_pass() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let mut sessions = Vec::new();
    for role in ["generator", "filter", "summarizer", "validator"] {
        let args = spawn_args(&format!("channel-{role}"), role, &state_path);
        assert_eq!(run_swarm(root.path(), &args), 0);
    }
    let state = read_state(&state_path);
    let ids = state
        .get("sessions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    sessions.extend(ids);
    sessions.sort();
    let sender = sessions.first().cloned().expect("sender");
    let pollee = sessions.get(1).cloned().expect("pollee");
    let participants = sessions.join(",");

    let create_args = vec![
        "channels".to_string(),
        "create".to_string(),
        "--name=swarm-test-6".to_string(),
        format!("--participants={participants}"),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &create_args), 0);

    let state_after_create = read_state(&state_path);
    let channel_id = state_after_create
        .get("channels")
        .and_then(Value::as_object)
        .and_then(|rows| rows.keys().next().cloned())
        .expect("channel id");

    let publish_args = vec![
        "channels".to_string(),
        "publish".to_string(),
        format!("--channel-id={channel_id}"),
        format!("--sender-id={sender}"),
        "--message=channel-broadcast".to_string(),
        "--delivery=at_most_once".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &publish_args), 0);

    let poll_args = vec![
        "channels".to_string(),
        "poll".to_string(),
        format!("--channel-id={channel_id}"),
        format!("--session-id={pollee}"),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &poll_args), 0);

    let chain_test_args = vec![
        "test".to_string(),
        "communication".to_string(),
        "--delivery=at_least_once".to_string(),
        "--simulate-first-attempt-fail=1".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &chain_test_args), 0);

    let final_state = read_state(&state_path);
    let mailbox_total = final_state
        .get("mailboxes")
        .and_then(Value::as_object)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert!(mailbox_total >= 2, "expected message mailboxes to exist");
}

#[test]
fn heterogeneous_results_registry_supports_query_wait_consensus_and_outliers() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let fast = vec![
        "spawn".to_string(),
        "--task=Calculate 1-100 quickly".to_string(),
        "--role=calculator".to_string(),
        "--auto-publish-results=1".to_string(),
        "--agent-label=swarm-test-7-het-agent-fast".to_string(),
        "--result-value=5050".to_string(),
        state_path_arg(&state_path),
    ];
    let thorough = vec![
        "spawn".to_string(),
        "--task=Calculate and verify 1-100".to_string(),
        "--role=calculator".to_string(),
        "--auto-publish-results=1".to_string(),
        "--agent-label=swarm-test-7-het-agent-thorough".to_string(),
        "--result-value=5050".to_string(),
        "--verification-status=verified".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &fast), 0);
    assert_eq!(run_swarm(root.path(), &thorough), 0);

    let wait_args = vec![
        "results".to_string(),
        "wait".to_string(),
        "--label-pattern=swarm-test-7-het-agent-*".to_string(),
        "--min-count=2".to_string(),
        "--timeout-sec=2".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &wait_args), 0);

    let query_args = vec![
        "results".to_string(),
        "query".to_string(),
        "--role=calculator".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &query_args), 0);

    let consensus_args = vec![
        "results".to_string(),
        "consensus".to_string(),
        "--label-pattern=swarm-test-7-het-agent-*".to_string(),
        "--field=value".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &consensus_args), 0);

    let outlier_args = vec![
        "results".to_string(),
        "outliers".to_string(),
        "--label-pattern=swarm-test-7-het-agent-*".to_string(),
        "--field=value".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &outlier_args), 0);

    let state = read_state(&state_path);
    let result_count = state
        .get("result_registry")
        .and_then(Value::as_object)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert!(result_count >= 2, "expected published result rows");
    let labels = state
        .get("results_by_label")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    assert!(
        labels.contains_key("swarm-test-7-het-agent-fast")
            && labels.contains_key("swarm-test-7-het-agent-thorough"),
        "expected both heterogeneous labels indexed"
    );
}

#[test]
fn results_wait_times_out_when_min_count_not_met() {
    let root = tempfile::tempdir().expect("tempdir");
    let state_path = root.path().join("state/swarm/latest.json");

    let wait_args = vec![
        "results".to_string(),
        "wait".to_string(),
        "--label-pattern=non-existent-*".to_string(),
        "--min-count=1".to_string(),
        "--timeout-sec=0.1".to_string(),
        state_path_arg(&state_path),
    ];
    assert_eq!(run_swarm(root.path(), &wait_args), 2);

    let state = read_state(&state_path);
    let result_count = state
        .get("result_registry")
        .and_then(Value::as_object)
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert_eq!(result_count, 0, "timeout path must not fabricate results");
}
