// SPDX-License-Identifier: Apache-2.0
use crate::contract_lane_utils as lane_utils;
use crate::v8_kernel::{deterministic_merkle_root, write_receipt};
use crate::now_iso;
use base64::Engine as _;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const LANE_ID: &str = "autonomy_controller";
const REPLACEMENT: &str = "infring-ops autonomy-controller";
const STATE_DIR: &str = "local/state/ops/autonomy_controller";
const STATE_ENV: &str = "AUTONOMY_CONTROLLER_STATE_ROOT";
const STATE_SCOPE: &str = "autonomy_controller";

fn receipt_hash(v: &Value) -> String {
    deterministic_receipt_hash(v)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn usage() {
    println!("Usage:");
    println!("  infring-ops autonomy-controller status");
    println!("  infring-ops autonomy-controller run [--max-actions=<n>] [--objective=<id>]");
    println!("  infring-ops autonomy-controller hand-new [--hand-id=<id>] [--template=<id>] [--schedule=<cron>] [--provider=<id>] [--fallback=<id>] [--strict=1|0]");
    println!("  infring-ops autonomy-controller hand-cycle --hand-id=<id> [--goal=<text>] [--provider=<id>] [--fallback=<id>] [--strict=1|0]");
    println!("  infring-ops autonomy-controller hand-status [--hand-id=<id>] [--strict=1|0]");
    println!("  infring-ops autonomy-controller hand-memory-page --hand-id=<id> [--op=page-in|page-out|status] [--tier=core|archival|external] [--key=<id>] [--strict=1|0]");
    println!("  infring-ops autonomy-controller hand-wasm-task --hand-id=<id> [--task=<id>] [--fuel=<n>] [--epoch-ms=<n>] [--strict=1|0]");
    println!("  infring-ops autonomy-controller compact [<snip|micro|full|reactive>] [--hand-id=<id>] [--auto-compact-pct=<0..100>] [--pressure-ratio=<0..1>] [--strict=1|0]");
    println!("  infring-ops autonomy-controller dream [--hand-id=<id>] [--strict=1|0]");
    println!("  infring-ops autonomy-controller proactive_daemon|kairos [status|cycle|pause|resume] [--auto=1|0] [--force=1|0] [--tick-ms=<n>] [--jitter-ms=<n>] [--window-sec=<n>] [--max-proactive=<n>] [--block-budget-ms=<n>] [--dream-idle-ms=<n>] [--dream-max-without-ms=<n>] [--brief=1|0] [--strict=1|0]");
    println!("  infring-ops autonomy-controller speculate [run|status|merge|reject] [--spec-id=<id>] [--verify=1|0] [--input-json=<json>|--input-base64=<base64_json>] [--strict=1|0]");
    println!("  infring-ops autonomy-controller autoreason [run|status] [--task=<text>] [--run-id=<id>] [--convergence=<n>] [--max-iters=<n>] [--judges=<n>] [--strict=1|0]");
    println!("  infring-ops autonomy-controller ephemeral-run [--goal=<text>] [--domain=<id>] [--ui-leaf=1|0] [--strict=1|0]");
    println!("  infring-ops autonomy-controller trunk-status [--strict=1|0]");
    println!(
        "  infring-ops autonomy-controller pain-signal [--action=<status|emit|focus-start|focus-stop|focus-status>] [--source=<id>] [--code=<id>] [--severity=<low|medium|high|critical>] [--risk=<low|medium|high>]"
    );
    println!(
        "  infring-ops autonomy-controller multi-agent-debate <run|status> [--input-base64=<base64_json>|--input-json=<json>] [--policy=<path>] [--date=<YYYY-MM-DD>] [--persist=1|0]"
    );
    println!(
        "  infring-ops autonomy-controller ethical-reasoning <run|status> [--input-base64=<base64_json>|--policy=<path>] [--state-dir=<path>] [--persist=1|0]"
    );
    println!(
        "  infring-ops autonomy-controller autonomy-simulation-harness <run|status> [YYYY-MM-DD] [--days=N] [--write=1|0] [--strict=1|0]"
    );
    println!(
        "  infring-ops autonomy-controller runtime-stability-soak [--action=<start|check-now|status|report>] [flags]"
    );
    println!(
        "  infring-ops autonomy-controller self-documentation-closeout [--action=<run|status>] [flags]"
    );
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    lane_utils::parse_flag(argv, key, false)
}

fn parse_positional(argv: &[String], idx: usize) -> Option<String> {
    argv.iter()
        .filter(|arg| !arg.trim().starts_with("--"))
        .nth(idx)
        .map(|v| v.trim().to_string())
}

fn parse_bool(raw: Option<&str>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw, fallback)
}

fn parse_i64(raw: Option<&str>, fallback: i64, lo: i64, hi: i64) -> i64 {
    lane_utils::parse_i64_clamped(raw, fallback, lo, hi)
}

fn parse_u64(raw: Option<&str>, fallback: u64, lo: u64, hi: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn parse_payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = parse_flag(argv, "input-json") {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|e| format!("input_json_parse_failed:{e}"));
    }
    if let Some(raw) = parse_flag(argv, "input-base64") {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(raw.trim())
            .map_err(|e| format!("input_base64_decode_failed:{e}"))?;
        let text =
            String::from_utf8(decoded).map_err(|e| format!("input_base64_utf8_failed:{e}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|e| format!("input_base64_json_parse_failed:{e}"));
    }
    Ok(json!({}))
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
    if let Some(map) = out.as_object_mut() {
        map.remove("receipt_hash");
    }
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
    if let Some(map) = out.as_object_mut() {
        map.remove("receipt_hash");
    }
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

fn state_root(root: &Path) -> PathBuf {
    root.join(STATE_DIR)
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    lane_utils::write_json(path, value)
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, row)
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}

fn clean_id(raw: Option<String>, fallback: &str) -> String {
    let mut out = String::new();
    if let Some(v) = raw {
        for ch in v.trim().chars() {
            if out.len() >= 96 {
                break;
            }
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':') {
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push('-');
            }
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn hand_path(root: &Path, hand_id: &str) -> PathBuf {
    state_root(root)
        .join("hands")
        .join(format!("{hand_id}.json"))
}

fn hand_events_path(root: &Path, hand_id: &str) -> PathBuf {
    state_root(root)
        .join("hands")
        .join(format!("{hand_id}.events.jsonl"))
}

fn trunk_state_path(root: &Path) -> PathBuf {
    state_root(root).join("trunk").join("state.json")
}

fn trunk_events_path(root: &Path) -> PathBuf {
    state_root(root).join("trunk").join("events.jsonl")
}

fn autonomy_runs_dir(root: &Path) -> PathBuf {
    root.join("client")
        .join("runtime")
        .join("local")
        .join("state")
        .join("autonomy")
        .join("runs")
}

fn autonomy_runs_path(root: &Path, day: &str) -> PathBuf {
    autonomy_runs_dir(root).join(format!("{day}.jsonl"))
}

fn today_ymd(ts: &str) -> String {
    let day = ts.split('T').next().unwrap_or("").trim();
    if day.len() == 10 && day.chars().nth(4) == Some('-') && day.chars().nth(7) == Some('-') {
        day.to_string()
    } else {
        now_iso().chars().take(10).collect()
    }
}
