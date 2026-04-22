// SPDX-License-Identifier: Apache-2.0
use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const LANE_ID: &str = "state_kernel";
const REPLACEMENT: &str = "protheus-ops state-kernel";
const SETUP_WIZARD_STATE_REL: &str = "local/state/ops/protheus_setup_wizard/latest.json";
const MAX_SETUP_WIZARD_PAYLOAD_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Deserialize, Default)]
struct SetupWizardPayload {
    #[serde(default)]
    command: String,
    #[serde(default)]
    force: bool,
    #[serde(default)]
    skip: bool,
    #[serde(default)]
    defaults: bool,
    #[serde(default)]
    yes: bool,
    #[serde(default)]
    interaction: String,
    #[serde(default)]
    notifications: String,
    #[serde(default)]
    covenant_acknowledged: Option<bool>,
}

fn receipt_hash(v: &Value) -> String {
    crate::deterministic_receipt_hash(v)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops state-kernel queue-enqueue --queue-name=<name> --payload-json=<json>");
    println!("  protheus-ops state-kernel setup-wizard --payload-base64=<base64_json_payload>");
    println!("  protheus-ops state-kernel status");
}

fn native_receipt(root: &Path, cmd: &str, argv: &[String]) -> Value {
    let queue_name =
        lane_utils::parse_flag(argv, "queue-name", false).unwrap_or_else(|| "autonomy".to_string());
    let payload_json = lane_utils::parse_flag(argv, "payload-json", false);

    let mut out = json!({
        "ok": true,
        "type": "state_kernel",
        "lane": LANE_ID,
        "ts": now_iso(),
        "command": cmd,
        "queue_name": queue_name,
        "payload_present": payload_json.is_some(),
        "argv": argv,
        "root": root.to_string_lossy(),
        "replacement": REPLACEMENT,
        "claim_evidence": [
            {
                "id": "native_state_kernel_lane",
                "claim": "state_kernel_executes_natively_in_rust",
                "evidence": {
                    "command": cmd,
                    "queue_name": queue_name
                }
            }
        ]
    });

    if let Some(payload) = payload_json {
        out["payload_json"] = Value::String(payload);
    }

    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn cli_error_receipt(argv: &[String], err: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "state_kernel_cli_error",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": code
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn setup_wizard_state_path(root: &Path) -> PathBuf {
    root.join(SETUP_WIZARD_STATE_REL)
}

fn normalize_setup_wizard_command(raw: &str) -> String {
    let normalized = clean_text(raw, 40).to_ascii_lowercase();
    match normalized.as_str() {
        "" | "run" | "start" => "run".to_string(),
        "status" | "state" => "status".to_string(),
        "reset" | "clear" => "reset".to_string(),
        "complete" => "complete".to_string(),
        "help" | "--help" | "-h" => "help".to_string(),
        _ => normalized,
    }
}

fn decode_setup_wizard_payload(raw: &str) -> Result<SetupWizardPayload, String> {
    if raw.len() > MAX_SETUP_WIZARD_PAYLOAD_BYTES {
        return Err(format!("setup_wizard_payload_too_large:{}", raw.len()));
    }
    let mut payload = serde_json::from_str::<SetupWizardPayload>(raw)
        .map_err(|err| format!("setup_wizard_payload_decode_failed:{err}"))?;
    payload.command = normalize_setup_wizard_command(&payload.command);
    Ok(payload)
}

fn parse_setup_wizard_payload(argv: &[String]) -> Result<SetupWizardPayload, String> {
    if let Some(payload) = lane_utils::parse_flag(argv, "payload", false) {
        return decode_setup_wizard_payload(&payload);
    }
    if let Some(payload_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(payload_b64.as_bytes())
            .map_err(|err| format!("setup_wizard_payload_base64_decode_failed:{err}"))?;
        if bytes.len() > MAX_SETUP_WIZARD_PAYLOAD_BYTES {
            return Err(format!("setup_wizard_payload_too_large:{}", bytes.len()));
        }
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("setup_wizard_payload_utf8_decode_failed:{err}"))?;
        return decode_setup_wizard_payload(&text);
    }
    let mut payload = SetupWizardPayload::default();
    payload.command = normalize_setup_wizard_command(&payload.command);
    Ok(payload)
}

fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("state_parent_create_failed:{err}"))?;
    }
    let encoded =
        serde_json::to_string_pretty(value).map_err(|err| format!("state_encode_failed:{err}"))?;
    fs::write(path, format!("{encoded}\n")).map_err(|err| format!("state_write_failed:{err}"))
}

fn pick_interaction(raw: &str) -> &'static str {
    let normalized = clean_text(raw, 40).to_ascii_lowercase();
    match normalized.as_str() {
        "silent" | "quiet" => "silent",
        _ => "proactive",
    }
}

fn pick_notifications(raw: &str) -> &'static str {
    let normalized = clean_text(raw, 40).to_ascii_lowercase();
    match normalized.as_str() {
        "all" => "all",
        "none" => "none",
        _ => "critical",
    }
}

fn run_setup_wizard(root: &Path, payload: &SetupWizardPayload) -> Result<Value, String> {
    let state_path = setup_wizard_state_path(root);
    let existing = read_json(&state_path);
    let force = payload.force;

    if !force
        && existing
            .as_ref()
            .and_then(|row| row.get("completed").and_then(Value::as_bool))
            .unwrap_or(false)
    {
        return Ok(json!({
            "ok": true,
            "type": "protheus_setup_wizard",
            "command": "run",
            "skipped": true,
            "reason": "already_completed",
            "state_path": state_path.to_string_lossy().to_string(),
            "state": existing.unwrap_or_else(|| json!({}))
        }));
    }

    let non_interactive = payload.yes || payload.defaults;
    let skip = payload.skip;
    let interaction = if skip {
        "silent"
    } else {
        pick_interaction(if payload.interaction.is_empty() {
            "proactive"
        } else {
            payload.interaction.as_str()
        })
    };
    let notifications = if skip {
        "none"
    } else {
        pick_notifications(if payload.notifications.is_empty() {
            "critical"
        } else {
            payload.notifications.as_str()
        })
    };
    let covenant_acknowledged = if skip {
        false
    } else {
        payload.covenant_acknowledged.unwrap_or(true)
    };
    let completion_mode = if skip {
        "skipped"
    } else if non_interactive {
        "defaults"
    } else {
        "interactive"
    };
    let saved = json!({
        "type": "protheus_setup_wizard_state",
        "completed": true,
        "completed_at": now_iso(),
        "completion_mode": completion_mode,
        "covenant_acknowledged": covenant_acknowledged,
        "interaction_style": interaction,
        "notifications": notifications,
        "profile": {
            "interaction_style": interaction,
            "notifications": notifications
        },
        "version": 1
    });
    write_json(&state_path, &saved)?;
    Ok(json!({
        "ok": true,
        "type": "protheus_setup_wizard",
        "command": "run",
        "state_path": state_path.to_string_lossy().to_string(),
        "state": saved
    }))
}

fn setup_wizard_status(root: &Path) -> Value {
    let state_path = setup_wizard_state_path(root);
    let existing = read_json(&state_path);
    json!({
        "ok": true,
        "type": "protheus_setup_wizard",
        "command": "status",
        "state_path": state_path.to_string_lossy().to_string(),
        "state": existing.unwrap_or_else(|| json!({
            "type": "protheus_setup_wizard_state",
            "completed": false,
            "version": 1
        }))
    })
}

fn setup_wizard_reset(root: &Path) -> Value {
    let state_path = setup_wizard_state_path(root);
    let removed = fs::remove_file(&state_path).is_ok();
    json!({
        "ok": true,
        "type": "protheus_setup_wizard",
        "command": "reset",
        "state_path": state_path.to_string_lossy().to_string(),
        "removed": removed
    })
}

fn setup_wizard_help() -> Value {
    json!({
        "ok": true,
        "type": "protheus_setup_wizard_help",
        "usage": [
            "protheus setup [run|status|reset] [--json]",
            "protheus setup run [--force] [--yes] [--defaults] [--interaction=<proactive|silent>] [--notifications=<all|critical|none>]",
            "protheus setup run --skip",
            "protheus setup status",
            "protheus setup reset"
        ]
    })
}

fn setup_wizard_command(root: &Path, argv: &[String]) -> Result<Value, String> {
    let mut payload = parse_setup_wizard_payload(argv)?;
    let command = normalize_setup_wizard_command(&payload.command);
    match command.as_str() {
        "help" | "--help" | "-h" => Ok(setup_wizard_help()),
        "status" => Ok(setup_wizard_status(root)),
        "reset" => Ok(setup_wizard_reset(root)),
        "complete" => {
            payload.yes = true;
            payload.defaults = true;
            run_setup_wizard(root, &payload)
        }
        "run" => run_setup_wizard(root, &payload),
        _ => Err(format!("setup_wizard_unknown_command:{command}")),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    match cmd.as_str() {
        "status" | "queue-enqueue" => {
            print_json_line(&native_receipt(root, &cmd, argv));
            0
        }
        "setup-wizard" => match setup_wizard_command(root, argv) {
            Ok(payload) => {
                print_json_line(&cli_receipt("state_kernel_setup_wizard", payload));
                0
            }
            Err(err) => {
                print_json_line(&cli_error_receipt(argv, &err, 2));
                2
            }
        },
        _ => {
            usage();
            print_json_line(&cli_error_receipt(argv, "unknown_command", 2));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload_b64(value: Value) -> String {
        BASE64_STANDARD.encode(
            serde_json::to_string(&value)
                .unwrap_or_else(|_| "{}".to_string())
                .as_bytes(),
        )
    }

    #[test]
    fn queue_enqeue_receipt_contains_queue_name() {
        let root = tempfile::tempdir().expect("tempdir");
        let payload = native_receipt(
            root.path(),
            "queue-enqueue",
            &[
                "queue-enqueue".to_string(),
                "--queue-name=autonomy".to_string(),
                "--payload-json={\"job\":\"x\"}".to_string(),
            ],
        );
        assert_eq!(
            payload.get("queue_name").and_then(Value::as_str),
            Some("autonomy")
        );
        assert_eq!(
            payload.get("payload_present").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn status_receipt_is_hashed() {
        let root = tempfile::tempdir().expect("tempdir");
        let payload = native_receipt(root.path(), "status", &[]);
        let hash = payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .expect("hash")
            .to_string();
        let mut unhashed = payload.clone();
        unhashed
            .as_object_mut()
            .expect("obj")
            .remove("receipt_hash");
        assert_eq!(receipt_hash(&unhashed), hash);
    }

    #[test]
    fn setup_wizard_run_writes_state_and_status_reads_it() {
        let root = tempfile::tempdir().expect("tempdir");
        let run = setup_wizard_command(
            root.path(),
            &[format!(
                "--payload-base64={}",
                payload_b64(json!({
                    "command": "run",
                    "defaults": true,
                    "yes": true,
                    "interaction": "proactive",
                    "notifications": "critical",
                    "covenant_acknowledged": true
                }))
            )],
        )
        .expect("run");
        assert_eq!(run.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(run.get("command").and_then(Value::as_str), Some("run"));
        assert_eq!(
            run.pointer("/state/completed").and_then(Value::as_bool),
            Some(true)
        );
        let status = setup_wizard_command(
            root.path(),
            &[format!(
                "--payload-base64={}",
                payload_b64(json!({"command": "status"}))
            )],
        )
        .expect("status");
        assert_eq!(status.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            status.get("command").and_then(Value::as_str),
            Some("status")
        );
        assert_eq!(
            status.pointer("/state/completed").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn setup_wizard_reset_removes_state_file() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = setup_wizard_command(
            root.path(),
            &[format!(
                "--payload-base64={}",
                payload_b64(json!({"command":"run","yes":true,"defaults":true}))
            )],
        )
        .expect("run");
        let reset = setup_wizard_command(
            root.path(),
            &[format!(
                "--payload-base64={}",
                payload_b64(json!({"command":"reset"}))
            )],
        )
        .expect("reset");
        assert_eq!(reset.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(reset.get("command").and_then(Value::as_str), Some("reset"));
        assert_eq!(reset.get("removed").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn setup_wizard_payload_rejects_oversized_payload() {
        let root = tempfile::tempdir().expect("tempdir");
        let huge = "x".repeat(MAX_SETUP_WIZARD_PAYLOAD_BYTES + 32);
        let payload = json!({
            "command": "run",
            "interaction": huge
        })
        .to_string();
        let err = setup_wizard_command(root.path(), &[format!("--payload={payload}")]).expect_err("reject");
        assert!(err.contains("setup_wizard_payload_too_large"));
    }

    #[test]
    fn setup_wizard_command_aliases_normalize_to_supported_commands() {
        let root = tempfile::tempdir().expect("tempdir");
        let run = setup_wizard_command(
            root.path(),
            &[format!(
                "--payload-base64={}",
                payload_b64(json!({"command":"start","yes":true,"defaults":true}))
            )],
        )
        .expect("run");
        assert_eq!(run.get("command").and_then(Value::as_str), Some("run"));

        let reset = setup_wizard_command(
            root.path(),
            &[format!(
                "--payload-base64={}",
                payload_b64(json!({"command":"clear"}))
            )],
        )
        .expect("reset");
        assert_eq!(reset.get("command").and_then(Value::as_str), Some("reset"));
    }
}
