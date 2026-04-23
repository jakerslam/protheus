// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_iso};
use serde_json::{json, Value};
use std::path::Path;

const LANE_ID: &str = "personas_cli";
const REPLACEMENT: &str = "infring-ops personas-cli";

fn receipt_hash(v: &Value) -> String {
    crate::deterministic_receipt_hash(v)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn usage() {
    println!("Usage:");
    println!("  infring-ops personas-cli lens <name> [--schema=json]");
    println!("  infring-ops personas-cli status");
}

fn native_receipt(root: &Path, cmd: &str, argv: &[String]) -> Value {
    let lens = if cmd == "lens" {
        argv.get(1)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "default".to_string())
    } else {
        "status".to_string()
    };

    let mut out = json!({
        "ok": true,
        "type": "personas_cli",
        "lane": LANE_ID,
        "ts": now_iso(),
        "command": cmd,
        "execution_receipt": {
            "lane": LANE_ID,
            "command": cmd,
            "status": "success",
            "source": "OPENCLAW-TOOLING-WEB-100",
            "tool_runtime_class": "receipt_wrapped"
        },
        "lens": lens,
        "argv": argv,
        "root": root.to_string_lossy(),
        "replacement": REPLACEMENT,
        "claim_evidence": [
            {
                "id": "native_personas_cli_lane",
                "claim": "personas_cli_executes_natively_in_rust",
                "evidence": {
                    "command": cmd,
                    "lens": lens
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
        "type": "personas_cli_error",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": code,
        "execution_receipt": {
            "lane": LANE_ID,
            "command": "invalid",
            "status": "error",
            "source": "OPENCLAW-TOOLING-WEB-100",
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
        "status" | "lens" => {
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
    fn lens_command_populates_lens_name() {
        let root = tempfile::tempdir().expect("tempdir");
        let payload = native_receipt(
            root.path(),
            "lens",
            &["lens".to_string(), "vikram_menon".to_string()],
        );
        assert_eq!(
            payload.get("lens").and_then(Value::as_str),
            Some("vikram_menon")
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
}
