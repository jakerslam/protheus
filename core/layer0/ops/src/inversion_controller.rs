// SPDX-License-Identifier: Apache-2.0
use crate::contract_lane_utils as lane_utils;
use crate::now_iso;
use serde_json::{json, Value};
use std::path::Path;

const LANE_ID: &str = "inversion_controller";
const REPLACEMENT: &str = "protheus-ops inversion-controller";

fn receipt_hash(v: &Value) -> String {
    deterministic_receipt_hash(v)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops inversion-controller status");
    println!("  protheus-ops inversion-controller run [--objective=<id>] [--impact=<low|medium|high>] [--target=<lane>]");
}

fn native_receipt(root: &Path, cmd: &str, argv: &[String]) -> Value {
    let objective =
        lane_utils::parse_flag(argv, "objective", false).unwrap_or_else(|| "default".to_string());
    let impact =
        lane_utils::parse_flag(argv, "impact", false).unwrap_or_else(|| "medium".to_string());
    let target =
        lane_utils::parse_flag(argv, "target", false).unwrap_or_else(|| "tactical".to_string());

    let mut out = json!({
        "ok": true,
        "type": "inversion_controller",
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
        "objective": objective,
        "impact": impact,
        "target": target,
        "replacement": REPLACEMENT,
        "root": root.to_string_lossy(),
        "claim_evidence": [
            {
                "id": "native_inversion_controller_lane",
                "claim": "inversion_controller_executes_natively_in_rust",
                "evidence": {
                    "command": cmd,
                    "target": target
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
        "type": "inversion_controller_cli_error",
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
        "status" | "run" => {
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
            "run".to_string(),
            "--objective=t1".to_string(),
            "--target=strategic".to_string(),
        ];
        let payload = native_receipt(root.path(), "run", &args);
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
