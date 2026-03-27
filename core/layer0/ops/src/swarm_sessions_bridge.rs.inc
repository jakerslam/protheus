// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::contract_lane_utils as lane_utils;
use serde_json::{json, Map, Value};
use std::path::Path;
use std::process::Command;

const DEFAULT_STATE_REL: &str = "local/state/ops/swarm_runtime/latest.json";

const DEFAULT_TOOL_ACCESS: &[&str] = &[
    "sessions_spawn",
    "sessions_send",
    "sessions_receive",
    "sessions_ack",
    "sessions_handoff",
    "sessions_context_put",
    "sessions_context_get",
    "sessions_query",
    "sessions_state",
    "sessions_tick",
    "tools_register_json_schema",
    "tools_invoke",
    "stream_emit",
    "stream_render",
    "turns_run",
    "turns_show",
    "networks_create",
    "networks_status",
];

#[derive(Debug)]
struct SwarmExec {
    status: i32,
    stdout: String,
    stderr: String,
    payload: Option<Value>,
}

fn usage() {
    println!("swarm-sessions-bridge commands:");
    println!("  protheus-ops swarm-sessions-bridge --command=<sessions_spawn|sessions_send|...> --options-json='<json>'");
}

fn parse_last_json(stdout: &str) -> Option<Value> {
    let mut lines = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    lines.reverse();
    for line in lines {
        if !line.starts_with('{') {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            return Some(value);
        }
    }
    None
}

fn options_json(argv: &[String]) -> Result<Map<String, Value>, String> {
    let Some(raw) = lane_utils::parse_flag(argv, "options-json", false) else {
        return Ok(Map::new());
    };
    let parsed = serde_json::from_str::<Value>(&raw)
        .map_err(|err| format!("swarm_sessions_bridge_options_decode_failed:{err}"))?;
    Ok(parsed.as_object().cloned().unwrap_or_default())
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(s) => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(if *b { "true" } else { "false" }.to_string()),
        _ => Some(value.to_string()),
    }
}

fn opt_string(options: &Map<String, Value>, keys: &[&str]) -> String {
    for key in keys {
        if let Some(value) = options.get(*key) {
            if let Some(out) = value_to_string(value) {
                if !out.is_empty() {
                    return out;
                }
            }
        }
    }
    String::new()
}

fn opt_value<'a>(options: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    for key in keys {
        if let Some(value) = options.get(*key) {
            return Some(value);
        }
    }
    None
}

fn opt_u64(options: &Map<String, Value>, keys: &[&str], fallback: u64, min: u64) -> u64 {
    for key in keys {
        if let Some(value) = options.get(*key) {
            let parsed = match value {
                Value::Number(n) => n.as_u64(),
                Value::String(s) => s.trim().parse::<u64>().ok(),
                _ => None,
            };
            if let Some(v) = parsed {
                return v.max(min);
            }
        }
    }
    fallback.max(min)
}

fn opt_f64_clamped(
    options: &Map<String, Value>,
    keys: &[&str],
    fallback: f64,
    min: f64,
    max: f64,
) -> f64 {
    for key in keys {
        if let Some(value) = options.get(*key) {
            let parsed = match value {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.trim().parse::<f64>().ok(),
                _ => None,
            };
            if let Some(v) = parsed {
                return v.clamp(min, max);
            }
        }
    }
    fallback.clamp(min, max)
}

fn opt_bool(options: &Map<String, Value>, keys: &[&str], fallback: bool) -> bool {
    for key in keys {
        if let Some(value) = options.get(*key) {
            let parsed = match value {
                Value::Bool(b) => Some(*b),
                Value::Number(n) => n.as_i64().map(|n| n != 0),
                Value::String(s) => Some(matches!(
                    s.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )),
                _ => None,
            };
            if let Some(v) = parsed {
                return v;
            }
        }
    }
    fallback
}

fn opt_has_key(options: &Map<String, Value>, keys: &[&str]) -> bool {
    keys.iter().any(|key| options.contains_key(*key))
}

fn opt_json_string(options: &Map<String, Value>, keys: &[&str], fallback: Value) -> String {
    if let Some(value) = opt_value(options, keys) {
        return if value.is_string() {
            value.as_str().unwrap_or_default().to_string()
        } else {
            value.to_string()
        };
    }
    fallback.to_string()
}

fn session_id_from_key(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if !trimmed.contains(':') {
        return trimmed.to_string();
    }
    trimmed
        .split(':')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .last()
        .unwrap_or(trimmed)
        .to_string()
}

fn as_session_key(session_id: &str) -> String {
    let id = session_id.trim();
    if id.is_empty() {
        String::new()
    } else {
        format!("agent:main:subagent:{id}")
    }
}

fn swarm_state_path(root: &Path, options: &Map<String, Value>) -> String {
    let explicit = opt_string(options, &["state-path", "state_path"]);
    if !explicit.is_empty() {
        return explicit;
    }
    root.join(DEFAULT_STATE_REL).to_string_lossy().to_string()
}

fn normalize_swarm_args(args: &[String]) -> Vec<String> {
    args.iter().filter(|arg| !arg.is_empty()).cloned().collect()
}

fn run_swarm_runtime(root: &Path, args: &[String]) -> Result<SwarmExec, String> {
    let exe = std::env::current_exe()
        .map_err(|err| format!("swarm_sessions_bridge_current_exe_failed:{err}"))?;
    let output = Command::new(exe)
        .arg("swarm-runtime")
        .args(normalize_swarm_args(args))
        .current_dir(root)
        .output()
        .map_err(|err| format!("swarm_sessions_bridge_spawn_failed:{err}"))?;

    let status = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let payload = parse_last_json(&stdout);
    Ok(SwarmExec {
        status,
        stdout,
        stderr,
        payload,
    })
}

fn require_swarm_ok(exec: SwarmExec, label: &str) -> Result<Value, String> {
    if exec.status != 0 {
        let reason = if !exec.stderr.trim().is_empty() {
            exec.stderr.trim().to_string()
        } else {
            exec.stdout.trim().to_string()
        };
        return Err(format!("{label}_failed:status={}:{}", exec.status, reason));
    }
    let Some(payload) = exec.payload else {
        return Err(format!("{label}_invalid_payload"));
    };
    if payload
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        Ok(payload)
    } else {
        Err(format!("{label}_invalid_payload"))
    }
}

fn normalize_spawn_payload(payload: &Value) -> Value {
    let session_id = payload
        .get("payload")
        .and_then(|row| row.get("session_id"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| payload.get("session_id").and_then(Value::as_str).unwrap_or(""));
    let tool_access = payload
        .get("payload")
        .and_then(|row| row.get("session_state"))
        .and_then(|row| row.get("tool_access"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| {
            DEFAULT_TOOL_ACCESS
                .iter()
                .map(|row| Value::String((*row).to_string()))
                .collect::<Vec<_>>()
        });
    let tool_manifest = payload
        .get("payload")
        .and_then(|row| row.get("session_state"))
        .and_then(|row| row.get("tool_manifest"))
        .cloned()
        .unwrap_or(Value::Null);
    let agent_bootstrap = tool_manifest
        .get("agent_bootstrap")
        .cloned()
        .unwrap_or(Value::Null);

    json!({
        "ok": true,
        "type": "sessions_spawn",
        "session_id": session_id,
        "session_key": as_session_key(session_id),
        "tool_access": tool_access,
        "tool_manifest": tool_manifest,
        "agent_bootstrap": agent_bootstrap,
        "payload": payload,
    })
}

fn normalize_receive_payload(payload: &Value, session_id: &str) -> Value {
    json!({
        "ok": true,
        "type": "sessions_receive",
        "session_id": session_id,
        "session_key": as_session_key(session_id),
        "message_count": payload.get("message_count").and_then(Value::as_u64).unwrap_or(0),
        "messages": payload.get("messages").cloned().unwrap_or_else(|| json!([])),
        "payload": payload,
    })
}

fn normalize_state_payload(payload: &Value, session_id: &str) -> Value {
    let agent_bootstrap = payload
        .get("session")
        .and_then(|row| row.get("tool_manifest"))
        .and_then(|row| row.get("agent_bootstrap"))
        .cloned()
        .unwrap_or(Value::Null);
    json!({
        "ok": true,
        "type": "sessions_state",
        "session_id": session_id,
        "session_key": as_session_key(session_id),
        "agent_bootstrap": agent_bootstrap,
        "payload": payload,
    })
}

fn normalize_bootstrap_payload(payload: &Value, session_id: &str) -> Value {
    json!({
        "ok": true,
        "type": "sessions_bootstrap",
        "session_id": session_id,
        "session_key": as_session_key(session_id),
        "bootstrap": payload.get("bootstrap").cloned().unwrap_or(Value::Null),
        "payload": payload,
    })
}

fn sessions_spawn(root: &Path, options: &Map<String, Value>) -> Result<Value, String> {
    let state_path = swarm_state_path(root, options);
    let task = {
        let raw = opt_string(options, &["task", "objective", "prompt", "message"]);
        if raw.is_empty() {
            "swarm-session-task".to_string()
        } else {
            raw
        }
    };
    let mut args = vec!["spawn".to_string(), format!("--task={task}")];

    let parent = session_id_from_key(&opt_string(
        options,
        &["session_id", "sessionId", "parent_session_id", "parentSessionId"],
    ));
    if !parent.is_empty() {
        args.push(format!("--session-id={parent}"));
    }

    if opt_bool(options, &["recursive"], false) {
        args.push("--recursive=1".to_string());
    }
    if opt_has_key(options, &["levels"]) {
        args.push(format!("--levels={}", opt_u64(options, &["levels"], 2, 1)));
    }
    if opt_has_key(options, &["max_depth", "maxDepth"]) {
        args.push(format!(
            "--max-depth={}",
            opt_u64(options, &["max_depth", "maxDepth"], 8, 1)
        ));
    }

    let token_budget = if opt_has_key(
        options,
        &["token_budget", "token-budget", "max_tokens", "maxTokens", "max-tokens"],
    ) {
        Some(opt_u64(
            options,
            &["token_budget", "token-budget", "max_tokens", "maxTokens", "max-tokens"],
            1,
            1,
        ))
    } else {
        None
    };
    if let Some(tokens) = token_budget {
        args.push(format!("--token-budget={tokens}"));
    }
    if opt_has_key(options, &["token_warning_at", "token-warning-at"]) {
        let warning = opt_f64_clamped(options, &["token_warning_at", "token-warning-at"], 0.8, 0.0, 1.0);
        args.push(format!("--token-warning-at={warning}"));
    }

    let budget_mode = opt_string(options, &["on_budget_exhausted", "on-budget-exhausted"]).to_ascii_lowercase();
    if matches!(budget_mode.as_str(), "fail" | "warn" | "compact") {
        args.push(format!("--on-budget-exhausted={budget_mode}"));
    } else if token_budget.is_some() {
        args.push("--on-budget-exhausted=fail".to_string());
    }

    if opt_has_key(options, &["adaptive_complexity", "adaptive-complexity"]) {
        let adaptive = opt_bool(options, &["adaptive_complexity", "adaptive-complexity"], false);
        args.push(format!("--adaptive-complexity={}", if adaptive { 1 } else { 0 }));
    }

    let role = opt_string(options, &["agentRole", "role"]);
    if !role.is_empty() {
        args.push(format!("--role={role}"));
    }
    let label = opt_string(options, &["agentLabel", "agent_label", "label"]);
    if !label.is_empty() {
        args.push(format!("--agent-label={label}"));
    }
    let capabilities = opt_string(options, &["capabilities"]);
    if !capabilities.is_empty() {
        args.push(format!("--capabilities={capabilities}"));
    }

    let session_type = opt_string(options, &["sessionType", "session_type"]).to_ascii_lowercase();
    if matches!(session_type.as_str(), "persistent" | "background") {
        args.push(format!("--execution-mode={session_type}"));
        let ttl_minutes = opt_u64(options, &["ttlMinutes", "ttl_minutes"], 60, 1);
        args.push(format!("--lifespan-sec={}", ttl_minutes * 60));
        let checkpoint_sec = opt_u64(options, &["checkpointInterval", "checkpoint_interval_sec"], 60, 1);
        args.push(format!("--check-in-interval-sec={checkpoint_sec}"));
    }

    if opt_has_key(options, &["auto_publish_results", "autoPublishResults"]) {
        let auto_publish = opt_bool(options, &["auto_publish_results", "autoPublishResults"], false);
        args.push(format!("--auto-publish-results={}", if auto_publish { 1 } else { 0 }));
    }

    let test_mode = opt_string(options, &["testMode", "test_mode"]).to_ascii_lowercase();
    if test_mode == "byzantine" || opt_bool(options, &["byzantine"], false) {
        let enable = run_swarm_runtime(root, &[
            "byzantine-test".to_string(),
            "enable".to_string(),
            format!("--state-path={state_path}"),
        ])?;
        let _ = require_swarm_ok(enable, "byzantine_test_enable")?;
        args.push("--byzantine=1".to_string());

        let mut corruption_type = opt_string(options, &["corruption_type", "corruptionType"]);
        if corruption_type.is_empty() {
            if let Some(pattern) = opt_value(options, &["faultPattern"]) {
                let parsed = if pattern.is_string() {
                    serde_json::from_str::<Value>(pattern.as_str().unwrap_or_default()).ok()
                } else {
                    Some(pattern.clone())
                };
                if let Some(obj) = parsed {
                    corruption_type = obj
                        .get("type")
                        .or_else(|| obj.get("value"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                }
            }
        }
        if !corruption_type.is_empty() {
            args.push(format!("--corruption-type={corruption_type}"));
        }
    }

    args.push(format!("--state-path={state_path}"));
    let payload = require_swarm_ok(run_swarm_runtime(root, &args)?, "sessions_spawn")?;
    Ok(normalize_spawn_payload(&payload))
}

fn sessions_send(root: &Path, options: &Map<String, Value>) -> Result<Value, String> {
    let state_path = swarm_state_path(root, options);
    let session_id = session_id_from_key(&opt_string(
        options,
        &["sessionKey", "session_key", "session_id", "sessionId"],
    ));
    let sender_id = {
        let parsed = session_id_from_key(&opt_string(
            options,
            &[
                "sender_session_key",
                "sender_session_id",
                "senderSessionKey",
                "senderSessionId",
                "sender",
            ],
        ));
        if parsed.is_empty() {
            "coordinator".to_string()
        } else {
            parsed
        }
    };
    let message = opt_string(options, &["message", "payload"]);
    let delivery = {
        let raw = opt_string(options, &["delivery"]);
        if raw.is_empty() {
            "at_least_once".to_string()
        } else {
            raw
        }
    };
    let ttl_ms = opt_u64(options, &["ttl_ms", "ttlMs"], 300_000, 1);

    let payload = require_swarm_ok(
        run_swarm_runtime(
            root,
            &[
                "sessions".to_string(),
                "send".to_string(),
                format!("--sender-id={sender_id}"),
                format!("--session-id={session_id}"),
                format!("--message={message}"),
                format!("--delivery={delivery}"),
                format!("--ttl-ms={ttl_ms}"),
                format!("--state-path={state_path}"),
            ],
        )?,
        "sessions_send",
    )?;

    Ok(json!({
        "ok": true,
        "type": "sessions_send",
        "message_id": payload.get("message_id").cloned().unwrap_or(Value::Null),
        "delivery": payload.get("delivery").cloned().unwrap_or(Value::Null),
        "attempts": payload.get("attempts").cloned().unwrap_or(Value::Null),
        "payload": payload,
    }))
}

fn sessions_simple(root: &Path, command: &str, options: &Map<String, Value>) -> Result<Value, String> {
    let state_path = swarm_state_path(root, options);
    let session_id = session_id_from_key(&opt_string(
        options,
        &["sessionKey", "session_key", "session_id", "sessionId"],
    ));
    let payload = require_swarm_ok(
        run_swarm_runtime(
            root,
            &[
                "sessions".to_string(),
                command.to_string(),
                format!("--session-id={session_id}"),
                format!("--state-path={state_path}"),
            ],
        )?,
        &format!("sessions_{}", command.replace('-', "_")),
    )?;

    Ok(match command {
        "resume" => json!({"ok": true, "type": "sessions_resume", "session_id": session_id, "payload": payload}),
        "bootstrap" => normalize_bootstrap_payload(&payload, &session_id),
        _ => json!({"ok": true, "type": format!("sessions_{}", command.replace('-', "_")), "session_id": session_id, "payload": payload}),
    })
}

fn execute(root: &Path, command: &str, options: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "sessions_spawn" | "spawn" => sessions_spawn(root, options),
        "sessions_send" | "send" => sessions_send(root, options),
        "sessions_receive" | "receive" => {
            let state_path = swarm_state_path(root, options);
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            let payload = require_swarm_ok(
                run_swarm_runtime(
                    root,
                    &[
                        "sessions".to_string(),
                        "receive".to_string(),
                        format!("--session-id={session_id}"),
                        format!("--limit={}", opt_u64(options, &["limit"], 10, 1)),
                        "--mark-read=0".to_string(),
                        format!("--state-path={state_path}"),
                    ],
                )?,
                "sessions_receive",
            )?;
            Ok(normalize_receive_payload(&payload, &session_id))
        }
        "sessions_ack" | "ack" => {
            let state_path = swarm_state_path(root, options);
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            let message_id = opt_string(options, &["message_id", "messageId"]);
            let payload = require_swarm_ok(
                run_swarm_runtime(
                    root,
                    &[
                        "sessions".to_string(),
                        "ack".to_string(),
                        format!("--session-id={session_id}"),
                        format!("--message-id={message_id}"),
                        format!("--state-path={state_path}"),
                    ],
                )?,
                "sessions_ack",
            )?;
            Ok(json!({"ok": true, "type": "sessions_ack", "session_id": session_id, "message_id": message_id, "payload": payload}))
        }
        "sessions_handoff" | "handoff" => {
            let state_path = swarm_state_path(root, options);
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            let target_session_id = session_id_from_key(&opt_string(
                options,
                &["targetSessionKey", "target_session_key", "target_session_id", "targetSessionId"],
            ));
            let reason = {
                let raw = opt_string(options, &["reason", "message"]);
                if raw.is_empty() { "handoff".to_string() } else { raw }
            };
            let mut args = vec![
                "sessions".to_string(),
                "handoff".to_string(),
                format!("--session-id={session_id}"),
                format!("--target-session-id={target_session_id}"),
                format!("--reason={reason}"),
                format!("--importance={}", opt_f64_clamped(options, &["importance"], 0.5, 0.0, 1.0)),
                format!("--state-path={state_path}"),
            ];
            if opt_has_key(options, &["context_json", "context"]) {
                args.push(format!(
                    "--context-json={}",
                    opt_json_string(options, &["context_json", "context"], json!({}))
                ));
            }
            let network_id = opt_string(options, &["network_id", "networkId"]);
            if !network_id.is_empty() {
                args.push(format!("--network-id={network_id}"));
            }
            let payload = require_swarm_ok(run_swarm_runtime(root, &args)?, "sessions_handoff")?;
            Ok(json!({
                "ok": true,
                "type": "sessions_handoff",
                "session_id": session_id,
                "target_session_id": target_session_id,
                "payload": payload,
            }))
        }
        "sessions_context_put" | "context-put" => {
            let state_path = swarm_state_path(root, options);
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            let context_json = opt_json_string(options, &["context_json", "context"], json!({}));
            let merge = opt_bool(options, &["merge"], true);
            let payload = require_swarm_ok(
                run_swarm_runtime(
                    root,
                    &[
                        "sessions".to_string(),
                        "context-put".to_string(),
                        format!("--session-id={session_id}"),
                        format!("--context-json={context_json}"),
                        format!("--merge={}", if merge { 1 } else { 0 }),
                        format!("--state-path={state_path}"),
                    ],
                )?,
                "sessions_context_put",
            )?;
            Ok(json!({"ok": true, "type": "sessions_context_put", "session_id": session_id, "payload": payload}))
        }
        "sessions_context_get" | "context-get" => {
            let state_path = swarm_state_path(root, options);
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            let payload = require_swarm_ok(
                run_swarm_runtime(
                    root,
                    &[
                        "sessions".to_string(),
                        "context-get".to_string(),
                        format!("--session-id={session_id}"),
                        format!("--state-path={state_path}"),
                    ],
                )?,
                "sessions_context_get",
            )?;
            Ok(json!({
                "ok": true,
                "type": "sessions_context_get",
                "session_id": session_id,
                "context": payload.get("context").cloned().unwrap_or_else(|| json!({})),
                "payload": payload,
            }))
        }
        "sessions_resume" | "resume" => sessions_simple(root, "resume", options),
        "sessions_bootstrap" | "bootstrap" => sessions_simple(root, "bootstrap", options),
        "sessions_dead_letter" | "dead-letter" => {
            let state_path = swarm_state_path(root, options);
            let mut args = vec!["sessions".to_string(), "dead-letter".to_string()];
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            if !session_id.is_empty() {
                args.push(format!("--session-id={session_id}"));
            }
            if opt_has_key(options, &["retryable"]) {
                args.push(format!("--retryable={}", if opt_bool(options, &["retryable"], false) { 1 } else { 0 }));
            }
            args.push(format!("--state-path={state_path}"));
            let payload = require_swarm_ok(run_swarm_runtime(root, &args)?, "sessions_dead_letter")?;
            Ok(json!({"ok": true, "type": "sessions_dead_letter", "payload": payload}))
        }
        "sessions_retry_dead_letter" | "retry-dead-letter" => {
            let state_path = swarm_state_path(root, options);
            let message_id = opt_string(options, &["message_id", "messageId"]);
            let payload = require_swarm_ok(
                run_swarm_runtime(
                    root,
                    &[
                        "sessions".to_string(),
                        "retry-dead-letter".to_string(),
                        format!("--message-id={message_id}"),
                        format!("--state-path={state_path}"),
                    ],
                )?,
                "sessions_retry_dead_letter",
            )?;
            Ok(json!({"ok": true, "type": "sessions_retry_dead_letter", "message_id": message_id, "payload": payload}))
        }
        "sessions_query" | "query" => {
            let state_path = swarm_state_path(root, options);
            let role = opt_string(options, &["agentRole", "role"]);
            let label = opt_string(options, &["agentLabel", "agent_label", "label"]);
            let task_id = opt_string(options, &["testId", "task_id", "taskId"]);
            let session_id = session_id_from_key(&opt_string(options, &["session_id", "sessionId"]));
            let wait = opt_bool(options, &["wait"], false);
            let mut args = vec![
                "results".to_string(),
                if wait { "wait".to_string() } else { "query".to_string() },
            ];
            if !role.is_empty() {
                args.push(format!("--role={role}"));
            }
            if !label.is_empty() {
                args.push(format!("--label-pattern={label}"));
            }
            if !task_id.is_empty() {
                args.push(format!("--task-id={task_id}"));
            }
            if !session_id.is_empty() {
                args.push(format!("--session-id={session_id}"));
            }
            if wait {
                args.push(format!("--min-count={}", opt_u64(options, &["min_count", "minCount"], 1, 1)));
                args.push(format!("--timeout-sec={}", opt_u64(options, &["timeout_sec", "timeoutSec"], 10, 1)));
            }
            args.push(format!("--state-path={state_path}"));
            let payload = require_swarm_ok(run_swarm_runtime(root, &args)?, "sessions_query")?;

            let discovery = if !role.is_empty() {
                let discovery_payload = run_swarm_runtime(
                    root,
                    &[
                        "sessions".to_string(),
                        "discover".to_string(),
                        format!("--role={role}"),
                        format!("--state-path={state_path}"),
                    ],
                )?;
                if discovery_payload.status == 0 {
                    discovery_payload.payload
                } else {
                    None
                }
            } else {
                None
            };

            Ok(json!({
                "ok": true,
                "type": "sessions_query",
                "result_count": payload.get("result_count").and_then(Value::as_u64).unwrap_or(0),
                "results": payload.get("results").cloned().unwrap_or_else(|| json!([])),
                "discovery": discovery,
                "payload": payload,
            }))
        }
        "sessions_state" | "state" => {
            let state_path = swarm_state_path(root, options);
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            let payload = require_swarm_ok(
                run_swarm_runtime(
                    root,
                    &[
                        "sessions".to_string(),
                        "state".to_string(),
                        format!("--session-id={session_id}"),
                        format!("--timeline={}", if opt_bool(options, &["timeline"], false) { 1 } else { 0 }),
                        format!(
                            "--tool-history-limit={}",
                            opt_u64(options, &["tool_history_limit", "toolHistoryLimit"], 32, 1)
                        ),
                        format!("--state-path={state_path}"),
                    ],
                )?,
                "sessions_state",
            )?;
            Ok(normalize_state_payload(&payload, &session_id))
        }
        "sessions_tick" | "tick" => {
            let state_path = swarm_state_path(root, options);
            let payload = require_swarm_ok(
                run_swarm_runtime(
                    root,
                    &[
                        "tick".to_string(),
                        format!("--advance-ms={}", opt_u64(options, &["advance_ms", "advanceMs"], 1000, 1)),
                        format!("--max-check-ins={}", opt_u64(options, &["max_check_ins", "maxCheckIns"], 32, 1)),
                        format!("--state-path={state_path}"),
                    ],
                )?,
                "sessions_tick",
            )?;
            Ok(json!({"ok": true, "type": "sessions_tick", "payload": payload}))
        }
        "tools_register_json_schema" | "register-json-schema" => {
            let state_path = swarm_state_path(root, options);
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            let tool_name = opt_string(options, &["toolName", "tool_name"]);
            let bridge_path = opt_string(options, &["bridgePath", "bridge_path"]);
            let entrypoint = {
                let raw = opt_string(options, &["entrypoint", "command"]);
                if raw.is_empty() { tool_name.clone() } else { raw }
            };
            let mut args = vec![
                "tools".to_string(),
                "register-json-schema".to_string(),
                format!("--session-id={session_id}"),
                format!("--tool-name={tool_name}"),
                format!("--schema-json={}", opt_json_string(options, &["schema_json", "schema"], json!({}))),
                format!("--bridge-path={bridge_path}"),
                format!("--entrypoint={entrypoint}"),
                format!("--state-path={state_path}"),
            ];
            let description = opt_string(options, &["description"]);
            if !description.is_empty() {
                args.push(format!("--description={description}"));
            }
            let payload = require_swarm_ok(run_swarm_runtime(root, &args)?, "tools_register_json_schema")?;
            Ok(json!({
                "ok": true,
                "type": "tools_register_json_schema",
                "session_id": session_id,
                "tool_name": tool_name,
                "payload": payload,
            }))
        }
        "tools_invoke" | "tool-invoke" => {
            let state_path = swarm_state_path(root, options);
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            let tool_name = opt_string(options, &["toolName", "tool_name"]);
            let payload = require_swarm_ok(
                run_swarm_runtime(
                    root,
                    &[
                        "tools".to_string(),
                        "invoke".to_string(),
                        format!("--session-id={session_id}"),
                        format!("--tool-name={tool_name}"),
                        format!("--args-json={}", opt_json_string(options, &["args_json", "args"], json!({}))),
                        format!("--state-path={state_path}"),
                    ],
                )?,
                "tools_invoke",
            )?;
            Ok(json!({
                "ok": true,
                "type": "tools_invoke",
                "session_id": session_id,
                "tool_name": tool_name,
                "payload": payload,
            }))
        }
        "stream_emit" | "stream-emit" => {
            let state_path = swarm_state_path(root, options);
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            let mut args = vec![
                "stream".to_string(),
                "emit".to_string(),
                format!("--session-id={session_id}"),
                format!("--chunks-json={}", opt_json_string(options, &["chunks_json", "chunks"], json!([]))),
                format!("--state-path={state_path}"),
            ];
            let turn_id = opt_string(options, &["turnId", "turn_id"]);
            if !turn_id.is_empty() {
                args.push(format!("--turn-id={turn_id}"));
            }
            let agent_label = opt_string(options, &["agentLabel", "agent_label"]);
            if !agent_label.is_empty() {
                args.push(format!("--agent-label={agent_label}"));
            }
            let payload = require_swarm_ok(run_swarm_runtime(root, &args)?, "stream_emit")?;
            Ok(json!({"ok": true, "type": "stream_emit", "session_id": session_id, "payload": payload}))
        }
        "stream_render" | "stream-render" => {
            let state_path = swarm_state_path(root, options);
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            let mut args = vec![
                "stream".to_string(),
                "render".to_string(),
                format!("--session-id={session_id}"),
                format!("--state-path={state_path}"),
            ];
            let turn_id = opt_string(options, &["turnId", "turn_id"]);
            if !turn_id.is_empty() {
                args.push(format!("--turn-id={turn_id}"));
            }
            let payload = require_swarm_ok(run_swarm_runtime(root, &args)?, "stream_render")?;
            Ok(json!({"ok": true, "type": "stream_render", "session_id": session_id, "payload": payload}))
        }
        "turns_run" | "turns-run" => {
            let state_path = swarm_state_path(root, options);
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            let mut args = vec![
                "turns".to_string(),
                "run".to_string(),
                format!("--session-id={session_id}"),
                format!("--turns-json={}", opt_json_string(options, &["turns_json", "turns"], json!([]))),
                format!("--state-path={state_path}"),
            ];
            let label = opt_string(options, &["label"]);
            if !label.is_empty() {
                args.push(format!("--label={label}"));
            }
            let payload = require_swarm_ok(run_swarm_runtime(root, &args)?, "turns_run")?;
            Ok(json!({"ok": true, "type": "turns_run", "session_id": session_id, "payload": payload}))
        }
        "turns_show" | "turns-show" => {
            let state_path = swarm_state_path(root, options);
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            let run_id = opt_string(options, &["runId", "run_id"]);
            let payload = require_swarm_ok(
                run_swarm_runtime(
                    root,
                    &[
                        "turns".to_string(),
                        "show".to_string(),
                        format!("--session-id={session_id}"),
                        format!("--run-id={run_id}"),
                        format!("--state-path={state_path}"),
                    ],
                )?,
                "turns_show",
            )?;
            Ok(json!({"ok": true, "type": "turns_show", "session_id": session_id, "run_id": run_id, "payload": payload}))
        }
        "networks_create" | "networks-create" => {
            let state_path = swarm_state_path(root, options);
            let mut args = vec![
                "networks".to_string(),
                "create".to_string(),
                format!("--spec-json={}", opt_json_string(options, &["spec_json", "spec"], json!({}))),
                format!("--state-path={state_path}"),
            ];
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            if !session_id.is_empty() {
                args.push(format!("--session-id={session_id}"));
            }
            let payload = require_swarm_ok(run_swarm_runtime(root, &args)?, "networks_create")?;
            Ok(json!({"ok": true, "type": "networks_create", "payload": payload}))
        }
        "networks_status" | "networks-status" => {
            let state_path = swarm_state_path(root, options);
            let network_id = opt_string(options, &["networkId", "network_id"]);
            let mut args = vec![
                "networks".to_string(),
                "status".to_string(),
                format!("--network-id={network_id}"),
                format!("--state-path={state_path}"),
            ];
            let session_id = session_id_from_key(&opt_string(
                options,
                &["sessionKey", "session_key", "session_id", "sessionId"],
            ));
            if !session_id.is_empty() {
                args.push(format!("--session-id={session_id}"));
            }
            let payload = require_swarm_ok(run_swarm_runtime(root, &args)?, "networks_status")?;
            Ok(json!({"ok": true, "type": "networks_status", "payload": payload}))
        }
        _ => Err(format!("unknown_command:{command}")),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = lane_utils::parse_flag(argv, "command", false)
        .or_else(|| argv.first().cloned())
        .unwrap_or_else(|| "sessions_spawn".to_string())
        .trim()
        .to_ascii_lowercase();

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let options = match options_json(argv) {
        Ok(value) => value,
        Err(err) => {
            let payload = json!({
                "ok": false,
                "type": "swarm_sessions_bridge_error",
                "command": command,
                "error": err,
            });
            println!("{}", serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()));
            return 2;
        }
    };

    match execute(root, &command, &options) {
        Ok(payload) => {
            println!("{}", serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()));
            0
        }
        Err(err) => {
            let payload = json!({
                "ok": false,
                "type": "swarm_sessions_bridge_error",
                "command": command,
                "error": err,
            });
            println!("{}", serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_key_normalization_extracts_session_id() {
        assert_eq!(session_id_from_key("agent:main:subagent:sess-123"), "sess-123");
        assert_eq!(session_id_from_key("sess-456"), "sess-456");
    }

    #[test]
    fn session_key_builder_round_trips_non_empty_ids() {
        assert_eq!(as_session_key("abc"), "agent:main:subagent:abc");
        assert_eq!(as_session_key(""), "");
    }
}
