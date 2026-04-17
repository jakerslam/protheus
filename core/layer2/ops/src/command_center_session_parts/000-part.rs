// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_epoch_ms, parse_cli_flag, print_json_line};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_STATE_PATH: &str = "local/state/ops/command_center/session_registry.json";
const USAGE: &[&str] = &[
    "Usage:",
    "  protheus-ops command-center-session status [--session-id=<id>] [--state-path=<path>]",
    "  protheus-ops command-center-session list [--state-path=<path>]",
    "  protheus-ops command-center-session register --session-id=<id> [--lineage-id=<id>] [--status=<running|paused|terminated>] [--task=<text>] [--state-path=<path>]",
    "  protheus-ops command-center-session resume <id> [--state-path=<path>]",
    "  protheus-ops command-center-session send <id> --message=<text> [--state-path=<path>]",
    "  protheus-ops command-center-session kill <id> [--state-path=<path>]",
    "  protheus-ops command-center-session tail <id> [--lines=<n>] [--state-path=<path>]",
    "  protheus-ops command-center-session inspect <id> [--state-path=<path>]",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SessionRegistry {
    #[serde(default)]
    sessions: BTreeMap<String, SessionState>,
    #[serde(default)]
    updated_epoch_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionState {
    session_id: String,
    lineage_id: String,
    status: String,
    started_epoch_ms: u64,
    #[serde(default)]
    last_attach_epoch_ms: Option<u64>,
    #[serde(default)]
    terminated_epoch_ms: Option<u64>,
    #[serde(default)]
    attach_count: u64,
    #[serde(default)]
    steering_count: u64,
    #[serde(default)]
    token_count: u64,
    #[serde(default)]
    cost_usd: f64,
    #[serde(default)]
    health: String,
    #[serde(default)]
    last_steering_hash: Option<String>,
    #[serde(default)]
    recent_steering: Vec<SteeringEvent>,
    #[serde(default)]
    events: Vec<SessionEvent>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SteeringEvent {
    ts_epoch_ms: u64,
    message: String,
    message_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionEvent {
    ts_epoch_ms: u64,
    kind: String,
    detail: Value,
}

fn first_free_positional(argv: &[String], skip: usize) -> Option<String> {
    argv.iter()
        .skip(skip)
        .find(|token| !token.trim_start().starts_with('-'))
        .cloned()
}

fn state_path(root: &Path, argv: &[String]) -> PathBuf {
    parse_cli_flag(argv, "state-path")
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(DEFAULT_STATE_PATH))
}

fn session_id_from_args(cmd: &str, argv: &[String]) -> Option<String> {
    parse_cli_flag(argv, "session-id")
        .or_else(|| first_free_positional(argv, 1))
        .filter(|v| !v.trim().is_empty())
        .map(|v| {
            if cmd == "send" || cmd == "steer" {
                v
            } else {
                v.trim().to_string()
            }
        })
}

fn load_registry(path: &Path) -> Result<SessionRegistry, String> {
    if !path.exists() {
        return Ok(SessionRegistry::default());
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("state_read_failed:{e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("state_parse_failed:{e}"))
}

fn save_registry(path: &Path, registry: &SessionRegistry) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("state_dir_create_failed:{e}"))?;
    }
    let encoded =
        serde_json::to_string_pretty(registry).map_err(|e| format!("state_encode_failed:{e}"))?;
    fs::write(path, encoded).map_err(|e| format!("state_write_failed:{e}"))
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn lineage_seed(session_id: &str, now_ms: u64) -> String {
    let digest = sha256_hex(&format!("{session_id}:{now_ms}"));
    format!("lineage-{}", &digest[..12])
}

fn normalized_health(status: &str) -> String {
    match status.trim().to_ascii_lowercase().as_str() {
        "running" => "healthy".to_string(),
        "paused" => "paused".to_string(),
        "terminated" => "terminated".to_string(),
        _ => "degraded".to_string(),
    }
}

fn push_event(session: &mut SessionState, now_ms: u64, kind: &str, detail: Value) {
    session.events.push(SessionEvent {
        ts_epoch_ms: now_ms,
        kind: kind.to_string(),
        detail,
    });
    if session.events.len() > 100 {
        let excess = session.events.len() - 100;
        session.events.drain(0..excess);
    }
}

fn session_view(session: &SessionState, now_ms: u64) -> Value {
    json!({
      "session_id": session.session_id,
      "lineage_id": session.lineage_id,
      "status": session.status,
      "health": if session.health.is_empty() { normalized_health(&session.status) } else { session.health.clone() },
      "started_epoch_ms": session.started_epoch_ms,
      "terminated_epoch_ms": session.terminated_epoch_ms,
      "uptime_seconds": if now_ms >= session.started_epoch_ms { (now_ms - session.started_epoch_ms) / 1000 } else { 0 },
      "last_attach_epoch_ms": session.last_attach_epoch_ms,
      "attach_count": session.attach_count,
      "steering_count": session.steering_count,
      "token_count": session.token_count,
      "cost_usd": session.cost_usd,
      "last_steering_hash": session.last_steering_hash,
      "task": session.metadata.get("task").cloned().unwrap_or(Value::Null),
      "event_count": session.events.len()
    })
}

fn with_hash(mut payload: Value) -> Value {
    payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));
    payload
}

fn error_receipt(
    code: &str,
    message: &str,
    cmd: &str,
    argv: &[String],
    state_path: &Path,
    exit_code: i32,
) -> Value {
    with_hash(json!({
        "ok": false,
        "type": "command_center_session_error",
        "code": code,
        "message": message,
        "command": cmd,
        "argv": argv,
        "state_path": state_path.to_string_lossy(),
        "exit_code": exit_code
    }))
}

fn success_receipt(
    lane_type: &str,
    cmd: &str,
    argv: &[String],
    state_path: &Path,
    payload: Value,
) -> Value {
    with_hash(json!({
        "ok": true,
        "type": lane_type,
        "lane": "command_center_session",
        "command": cmd,
        "argv": argv,
        "ts_epoch_ms": now_epoch_ms(),
        "state_path": state_path.to_string_lossy(),
        "payload": payload,
        "claim_evidence": [
            {
                "id": "v6_cockpit_025_2",
                "claim": "session_resume_and_live_steering_are_core_authoritative",
                "evidence": {
                    "layer": "core/layer2/ops",
                    "surface": "command_center_session"
                }
            }
        ]
    }))
}

fn usage() {
    for row in USAGE {
        println!("{row}");
    }
}
