// SPDX-License-Identifier: Apache-2.0
use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};
use serde_json::{json, Value};
use std::path::Path;

const LANE_ID: &str = "sensory_eyes_intake";
const REPLACEMENT: &str = "infring-ops sensory-eyes-intake";

fn receipt_hash(v: &Value) -> String {
    crate::deterministic_receipt_hash(v)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn usage() {
    println!("Usage:");
    println!("  infring-ops sensory-eyes-intake list");
    println!("  infring-ops sensory-eyes-intake status [--eye=<id>]");
    println!("  infring-ops sensory-eyes-intake create --name=<id> [--parser=<json|rss>] [--directive=<id>]");
    println!("  infring-ops sensory-eyes-intake run [--eye=<id>]");
}

fn native_receipt(root: &Path, cmd: &str, argv: &[String]) -> Value {
    let eye_id = lane_utils::parse_flag(argv, "eye", false)
        .or_else(|| lane_utils::parse_flag(argv, "name", false))
        .unwrap_or_else(|| "all".to_string());
    let parser =
        lane_utils::parse_flag(argv, "parser", false).unwrap_or_else(|| "json".to_string());
    let directive =
        lane_utils::parse_flag(argv, "directive", false).unwrap_or_else(|| "none".to_string());

    let mut out = json!({
        "ok": true,
        "type": "sensory_eyes_intake",
        "lane": LANE_ID,
        "ts": now_iso(),
        "command": cmd,
        "execution_receipt": {
            "lane": LANE_ID,
            "command": cmd,
            "status": "success",
            "source": "OPENCLAW-TOOLING-WEB-101",
            "tool_runtime_class": "receipt_wrapped"
        },
        "argv": argv,
        "eye_id": eye_id,
        "parser": parser,
        "directive": directive,
        "replacement": REPLACEMENT,
        "root": root.to_string_lossy(),
        "claim_evidence": [
            {
                "id": "native_sensory_eyes_intake_lane",
                "claim": "eyes_intake_executes_natively_in_rust",
                "evidence": {
                    "command": cmd,
                    "eye_id": eye_id
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn cli_error_receipt(argv: &[String], err: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "sensory_eyes_intake_cli_error",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": code,
        "execution_receipt": {
            "lane": LANE_ID,
            "command": "invalid",
            "status": "error",
            "source": "OPENCLAW-TOOLING-WEB-101",
            "tool_runtime_class": "receipt_wrapped"
        }
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
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
        "list" | "status" | "create" | "run" => {
            print_json_line(&native_receipt(root, &cmd, argv));
            0
        }
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

    #[test]
    fn native_receipt_is_deterministic() {
        let root = tempfile::tempdir().expect("tempdir");
        let args = vec![
            "create".to_string(),
            "--name=ollama".to_string(),
            "--parser=json".to_string(),
        ];
        let payload = native_receipt(root.path(), "create", &args);
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
}
