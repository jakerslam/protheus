// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_iso};
use serde_json::{json, Value};
use std::path::Path;

const LANE_ID: &str = "autonomy_controller";
const REPLACEMENT: &str = "protheus-ops autonomy-controller";

fn receipt_hash(v: &Value) -> String {
    deterministic_receipt_hash(v)
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops autonomy-controller status");
    println!("  protheus-ops autonomy-controller run [--max-actions=<n>] [--objective=<id>]");
    println!(
        "  protheus-ops autonomy-controller pain-signal [--action=<status|emit|focus-start|focus-stop|focus-status>] [--source=<id>] [--code=<id>] [--severity=<low|medium|high|critical>] [--risk=<low|medium|high>]"
    );
    println!(
        "  protheus-ops autonomy-controller runtime-stability-soak [--action=<start|check-now|status|report>] [flags]"
    );
    println!(
        "  protheus-ops autonomy-controller self-documentation-closeout [--action=<run|status>] [flags]"
    );
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let pref = format!("--{key}=");
    argv.iter().find_map(|arg| {
        let t = arg.trim();
        t.strip_prefix(&pref).map(|v| v.to_string())
    })
}

fn parse_positional(argv: &[String], idx: usize) -> Option<String> {
    argv.iter()
        .filter(|arg| !arg.trim().starts_with("--"))
        .nth(idx)
        .map(|v| v.trim().to_string())
}

fn native_receipt(root: &Path, cmd: &str, argv: &[String]) -> Value {
    let max_actions = parse_flag(argv, "max-actions")
        .and_then(|v| v.parse::<i64>().ok())
        .map(|v| v.clamp(1, 100))
        .unwrap_or(1);
    let objective = parse_flag(argv, "objective").unwrap_or_else(|| "default".to_string());

    let mut out = protheus_autonomy_core_v1::autonomy_receipt(cmd, Some(&objective));
    out["lane"] = Value::String(LANE_ID.to_string());
    out["ts"] = Value::String(now_iso());
    out["argv"] = json!(argv);
    out["max_actions"] = json!(max_actions);
    out["replacement"] = Value::String(REPLACEMENT.to_string());
    out["root"] = Value::String(root.to_string_lossy().to_string());
    out["claim_evidence"] = json!([
        {
            "id": "native_autonomy_controller_lane",
            "claim": "autonomy_controller_executes_natively_in_rust",
            "evidence": {
                "command": cmd,
                "max_actions": max_actions
            }
        }
    ]);
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn native_pain_signal_receipt(root: &Path, argv: &[String]) -> Value {
    let action = parse_flag(argv, "action")
        .or_else(|| parse_positional(argv, 1))
        .unwrap_or_else(|| "status".to_string());
    let source = parse_flag(argv, "source");
    let code = parse_flag(argv, "code");
    let severity = parse_flag(argv, "severity");
    let risk = parse_flag(argv, "risk");

    let mut out = protheus_autonomy_core_v1::pain_signal_receipt(
        action.as_str(),
        source.as_deref(),
        code.as_deref(),
        severity.as_deref(),
        risk.as_deref(),
    );
    out["lane"] = Value::String(LANE_ID.to_string());
    out["ts"] = Value::String(now_iso());
    out["argv"] = json!(argv);
    out["replacement"] = Value::String(REPLACEMENT.to_string());
    out["root"] = Value::String(root.to_string_lossy().to_string());
    out["claim_evidence"] = json!([
        {
            "id": "native_autonomy_pain_signal_lane",
            "claim": "pain_signal_contract_executes_natively_in_rust",
            "evidence": {
                "action": action,
                "source": source,
                "code": code
            }
        }
    ]);
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

fn cli_error_receipt(argv: &[String], err: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "autonomy_controller_cli_error",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": code
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
        "status" | "run" | "runtime-stability-soak" | "self-documentation-closeout" => {
            print_json_line(&native_receipt(root, &cmd, argv));
            0
        }
        "pain-signal" => {
            print_json_line(&native_pain_signal_receipt(root, argv));
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
        let args = vec!["run".to_string(), "--objective=t1".to_string()];
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
