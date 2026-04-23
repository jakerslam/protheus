// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

const DEFAULT_QUEUE_REL: &str = "client/runtime/local/state/approvals_queue.yaml";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct ApprovalQueue {
    #[serde(default)]
    pending: Vec<ApprovalEntry>,
    #[serde(default)]
    approved: Vec<ApprovalEntry>,
    #[serde(default)]
    denied: Vec<ApprovalEntry>,
    #[serde(default)]
    history: Vec<ApprovalEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct ApprovalEntry {
    #[serde(default)]
    action_id: String,
    #[serde(default)]
    timestamp: String,
    #[serde(default)]
    directive_id: String,
    #[serde(rename = "type", default)]
    entry_type: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    payload_pointer: String,
    #[serde(default)]
    approved_at: String,
    #[serde(default)]
    denied_at: String,
    #[serde(default)]
    deny_reason: String,
    #[serde(default)]
    action: String,
    #[serde(default)]
    history_at: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct QueuePayload {
    #[serde(default)]
    action_envelope: Option<Value>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    queue: Option<ApprovalQueue>,
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn usage() {
    println!("approval-gate-kernel commands:");
    println!("  infring-ops approval-gate-kernel status [--queue-path=<path>]");
    println!("  infring-ops approval-gate-kernel queue --payload-base64=<base64_json> [--queue-path=<path>]");
    println!("  infring-ops approval-gate-kernel approve --action-id=<id> [--queue-path=<path>]");
    println!("  infring-ops approval-gate-kernel deny --action-id=<id> [--reason=<text>] [--queue-path=<path>]");
    println!(
        "  infring-ops approval-gate-kernel was-approved --action-id=<id> [--queue-path=<path>]"
    );
    println!("  infring-ops approval-gate-kernel parse-command --text-base64=<base64_text>");
    println!("  infring-ops approval-gate-kernel parse-yaml --text-base64=<base64_text>");
    println!("  infring-ops approval-gate-kernel replace --payload-base64=<base64_json> [--queue-path=<path>]");
}

fn resolve_queue_path(root: &Path, argv: &[String]) -> PathBuf {
    if let Some(explicit) = lane_utils::parse_flag(argv, "queue-path", false) {
        let cleaned = explicit.trim();
        if !cleaned.is_empty() {
            let candidate = PathBuf::from(cleaned);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }
    for env_name in [
        "APPROVAL_GATE_QUEUE_PATH",
        "INFRING_APPROVAL_GATE_QUEUE_PATH",
    ] {
        if let Ok(raw) = std::env::var(env_name) {
            let cleaned = raw.trim();
            if !cleaned.is_empty() {
                let candidate = PathBuf::from(cleaned);
                if candidate.is_absolute() {
                    return candidate;
                }
                return root.join(candidate);
            }
        }
    }
    root.join(DEFAULT_QUEUE_REL)
}

fn load_payload(argv: &[String]) -> Result<QueuePayload, String> {
    if let Some(payload) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<QueuePayload>(&payload)
            .map_err(|err| format!("approval_gate_kernel_payload_decode_failed:{err}"));
    }
    if let Some(payload_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(payload_b64.as_bytes())
            .map_err(|err| format!("approval_gate_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("approval_gate_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<QueuePayload>(&text)
            .map_err(|err| format!("approval_gate_kernel_payload_decode_failed:{err}"));
    }
    Err("approval_gate_kernel_missing_payload".to_string())
}

fn decode_text_flag(argv: &[String], flag: &str) -> Result<String, String> {
    let Some(encoded) = lane_utils::parse_flag(argv, flag, false) else {
        return Err(format!("approval_gate_kernel_missing_{flag}"));
    };
    let bytes = BASE64_STANDARD
        .decode(encoded.as_bytes())
        .map_err(|err| format!("approval_gate_kernel_{flag}_base64_decode_failed:{err}"))?;
    String::from_utf8(bytes)
        .map_err(|err| format!("approval_gate_kernel_{flag}_utf8_decode_failed:{err}"))
}

fn read_queue(path: &Path) -> Result<ApprovalQueue, String> {
    if !path.exists() {
        return Ok(ApprovalQueue::default());
    }
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("approval_gate_kernel_read_queue_failed:{err}"))?;
    if raw.trim().is_empty() {
        return Ok(ApprovalQueue::default());
    }
    serde_yaml::from_str::<ApprovalQueue>(&raw)
        .map_err(|err| format!("approval_gate_kernel_parse_queue_failed:{err}"))
}

fn write_queue(path: &Path, queue: &ApprovalQueue) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("approval_gate_kernel_create_dir_failed:{err}"))?;
    }
    let encoded = serde_yaml::to_string(queue)
        .map_err(|err| format!("approval_gate_kernel_encode_queue_failed:{err}"))?;
    fs::write(path, encoded).map_err(|err| format!("approval_gate_kernel_write_queue_failed:{err}"))
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .chars()
        .take(max_len)
        .collect()
}

fn generate_approval_message(entry: &ApprovalEntry) -> String {
    format!(
        "Action: {}\nType: {}\nDirective: {}\nWhy gated: {}\nAction ID: {}\n\nTo approve, reply: APPROVE {}\nTo deny, reply: DENY {}",
        entry.summary,
        entry.entry_type,
        entry.directive_id,
        entry.reason,
        entry.action_id,
        entry.action_id,
        entry.action_id
    )
}

fn queue_entry_from_payload(
    action_envelope: &Value,
    reason: &str,
) -> Result<ApprovalEntry, String> {
    let Some(obj) = action_envelope.as_object() else {
        return Err("approval_gate_kernel_action_envelope_invalid".to_string());
    };
    let action_id = clean_text(obj.get("action_id"), 160);
    if action_id.is_empty() {
        return Err("approval_gate_kernel_action_id_missing".to_string());
    }
    let directive_id = clean_text(obj.get("directive_id"), 160);
    let entry_type = clean_text(obj.get("type"), 120);
    let summary = clean_text(obj.get("summary"), 480);
    Ok(ApprovalEntry {
        action_id: action_id.clone(),
        timestamp: now_iso(),
        directive_id: if directive_id.is_empty() {
            "T0_invariants".to_string()
        } else {
            directive_id
        },
        entry_type,
        summary,
        reason: reason.trim().to_string(),
        status: "PENDING".to_string(),
        payload_pointer: action_id,
        ..ApprovalEntry::default()
    })
}

fn parse_approval_command(text: &str) -> Value {
    let trimmed = text.trim();
    let mut parts = trimmed.split_whitespace();
    let Some(action) = parts.next() else {
        return Value::Null;
    };
    let Some(action_id) = parts.next() else {
        return Value::Null;
    };
    if parts.next().is_some() {
        return Value::Null;
    }
    let normalized = action.trim().to_ascii_lowercase();
    if normalized == "approve" || normalized == "deny" {
        return json!({
            "action": normalized,
            "action_id": action_id
        });
    }
    Value::Null
}

fn command_status(root: &Path, argv: &[String]) -> Value {
    let queue_path = resolve_queue_path(root, argv);
    match read_queue(&queue_path) {
        Ok(queue) => cli_receipt(
            "approval_gate_kernel_status",
            json!({
                "ok": true,
                "queue_path": queue_path.to_string_lossy(),
                "queue": queue
            }),
        ),
        Err(error) => cli_error("approval_gate_kernel_status", &error),
    }
}

