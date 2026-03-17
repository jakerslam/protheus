// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const POLICY_REL: &str = "client/runtime/config/benchmark_autonomy_gate_policy.json";
const LATEST_REL: &str = "local/state/ops/benchmark_autonomy_gate/latest.json";
const RECEIPTS_REL: &str = "local/state/ops/benchmark_autonomy_gate/receipts.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Gates {
    cold_start_ms_max: f64,
    idle_memory_mb_max: f64,
    install_size_mb_max: f64,
    tasks_per_sec_min: f64,
}

impl Default for Gates {
    fn default() -> Self {
        Self {
            cold_start_ms_max: 250.0,
            idle_memory_mb_max: 64.0,
            install_size_mb_max: 128.0,
            tasks_per_sec_min: 3000.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Policy {
    enabled: bool,
    strict_default: bool,
    gates: Gates,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            enabled: true,
            strict_default: true,
            gates: Gates::default(),
        }
    }
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn parse_bool(raw: Option<&str>, fallback: bool) -> bool {
    match raw.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(value) if matches!(value.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

fn parse_last_json(raw: &str) -> Option<Value> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
        return Some(parsed);
    }
    let lines = trimmed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    for line in lines.iter().rev() {
        if let Ok(parsed) = serde_json::from_str::<Value>(line) {
            return Some(parsed);
        }
    }
    None
}

fn normalize_gates(raw: Option<&Value>) -> Gates {
    let mut gates = Gates::default();
    let Some(obj) = raw.and_then(Value::as_object) else {
        return gates;
    };
    if let Some(value) = obj.get("cold_start_ms_max").and_then(Value::as_f64) {
        gates.cold_start_ms_max = value;
    }
    if let Some(value) = obj.get("idle_memory_mb_max").and_then(Value::as_f64) {
        gates.idle_memory_mb_max = value;
    }
    if let Some(value) = obj.get("install_size_mb_max").and_then(Value::as_f64) {
        gates.install_size_mb_max = value;
    }
    if let Some(value) = obj.get("tasks_per_sec_min").and_then(Value::as_f64) {
        gates.tasks_per_sec_min = value;
    }
    gates
}

fn read_policy(root: &Path) -> Policy {
    let path = root.join(POLICY_REL);
    let Ok(raw) = fs::read_to_string(path) else {
        return Policy::default();
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return Policy::default();
    };
    let mut policy = Policy::default();
    if let Some(enabled) = value.get("enabled").and_then(Value::as_bool) {
        policy.enabled = enabled;
    }
    if let Some(strict_default) = value.get("strict_default").and_then(Value::as_bool) {
        policy.strict_default = strict_default;
    }
    policy.gates = normalize_gates(value.get("gates"));
    policy
}

fn evaluate_gate(payload: &Value, gates: &Gates) -> Value {
    let measured = payload
        .get("openclaw_measured")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let cold_start_ms = measured
        .get("cold_start_ms")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let idle_memory_mb = measured
        .get("idle_memory_mb")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let install_size_mb = measured
        .get("install_size_mb")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let tasks_per_sec = measured
        .get("tasks_per_sec")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    let checks = vec![
        json!({
            "id": "cold_start_ms_max",
            "ok": cold_start_ms > 0.0 && cold_start_ms <= gates.cold_start_ms_max,
            "value": cold_start_ms,
            "gate": gates.cold_start_ms_max,
        }),
        json!({
            "id": "idle_memory_mb_max",
            "ok": idle_memory_mb > 0.0 && idle_memory_mb <= gates.idle_memory_mb_max,
            "value": idle_memory_mb,
            "gate": gates.idle_memory_mb_max,
        }),
        json!({
            "id": "install_size_mb_max",
            "ok": install_size_mb > 0.0 && install_size_mb <= gates.install_size_mb_max,
            "value": install_size_mb,
            "gate": gates.install_size_mb_max,
        }),
        json!({
            "id": "tasks_per_sec_min",
            "ok": tasks_per_sec >= gates.tasks_per_sec_min,
            "value": tasks_per_sec,
            "gate": gates.tasks_per_sec_min,
        }),
    ];

    let failed = checks
        .iter()
        .filter(|row| !row.get("ok").and_then(Value::as_bool).unwrap_or(false))
        .filter_map(|row| row.get("id").and_then(Value::as_str).map(ToOwned::to_owned))
        .collect::<Vec<_>>();

    json!({
        "metrics": {
            "cold_start_ms": cold_start_ms,
            "idle_memory_mb": idle_memory_mb,
            "install_size_mb": install_size_mb,
            "tasks_per_sec": tasks_per_sec,
        },
        "checks": checks,
        "failed": failed,
    })
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("benchmark_autonomy_gate_create_dir_failed:{err}"))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("benchmark_autonomy_gate_open_jsonl_failed:{err}"))?;
    let line = serde_json::to_string(value)
        .map_err(|err| format!("benchmark_autonomy_gate_encode_jsonl_failed:{err}"))?;
    writeln!(file, "{line}")
        .map_err(|err| format!("benchmark_autonomy_gate_append_jsonl_failed:{err}"))
}

fn write_artifacts(root: &Path, payload: &Value) -> Result<(), String> {
    let latest = root.join(LATEST_REL);
    let receipts = root.join(RECEIPTS_REL);
    if let Some(parent) = latest.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("benchmark_autonomy_gate_create_dir_failed:{err}"))?;
    }
    let encoded = serde_json::to_string_pretty(payload)
        .map_err(|err| format!("benchmark_autonomy_gate_encode_failed:{err}"))?;
    fs::write(&latest, format!("{encoded}\n"))
        .map_err(|err| format!("benchmark_autonomy_gate_write_latest_failed:{err}"))?;
    append_jsonl(&receipts, payload)
}

fn run_benchmark_matrix(root: &Path) -> Result<Value, String> {
    let current_exe = std::env::current_exe()
        .map_err(|err| format!("benchmark_autonomy_gate_current_exe_failed:{err}"))?;
    let output = Command::new(current_exe)
        .current_dir(root)
        .arg("benchmark-matrix")
        .arg("run")
        .arg("--refresh-runtime=1")
        .output()
        .map_err(|err| format!("benchmark_autonomy_gate_spawn_failed:{err}"))?;
    let status = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let payload = parse_last_json(&stdout)
        .ok_or_else(|| "benchmark_autonomy_gate_missing_benchmark_payload".to_string())?;
    if status != 0 || payload.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(format!(
            "benchmark_autonomy_gate_benchmark_matrix_run_failed:{}:{}",
            status,
            stderr.trim()
        ));
    }
    Ok(payload)
}

fn run_gate(root: &Path, gates: &Gates) -> Value {
    match run_benchmark_matrix(root) {
        Ok(payload) => {
            let evaluated = evaluate_gate(&payload, gates);
            let failed = evaluated
                .get("failed")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            json!({
                "ok": failed.is_empty(),
                "type": "benchmark_autonomy_gate",
                "generated_at": now_iso(),
                "gates": gates,
                "metrics": evaluated.get("metrics").cloned().unwrap_or_else(|| json!({})),
                "checks": evaluated.get("checks").cloned().unwrap_or_else(|| json!([])),
                "failed": failed,
                "benchmark_receipt_hash": payload.get("receipt_hash").cloned().unwrap_or(Value::Null),
            })
        }
        Err(err) => json!({
            "ok": false,
            "type": "benchmark_autonomy_gate",
            "generated_at": now_iso(),
            "error": "benchmark_matrix_run_failed",
            "reason": err,
        }),
    }
}

fn status_value(root: &Path) -> Result<Value, String> {
    let latest = root.join(LATEST_REL);
    let raw = fs::read_to_string(&latest)
        .map_err(|_| "missing_latest_benchmark_autonomy_gate".to_string())?;
    serde_json::from_str::<Value>(&raw)
        .map_err(|err| format!("benchmark_autonomy_gate_status_decode_failed:{err}"))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "run".to_string());
    let strict = parse_bool(lane_utils::parse_flag(argv, "strict", false).as_deref(), true);
    let policy = read_policy(root);
    if !policy.enabled {
        let payload = json!({
            "ok": false,
            "type": "benchmark_autonomy_gate",
            "generated_at": now_iso(),
            "error": "lane_disabled_by_policy",
        });
        print_json_line(&cli_receipt("benchmark_autonomy_gate_run", payload));
        return 1;
    }

    let payload = match command.as_str() {
        "status" => status_value(root),
        "run" => {
            let mut out = run_gate(root, &policy.gates);
            out["strict"] = Value::Bool(strict && policy.strict_default);
            if out.get("error").is_none() {
                if let Err(err) = write_artifacts(root, &out) {
                    return {
                        print_json_line(&cli_error("benchmark_autonomy_gate_error", &err));
                        1
                    };
                }
            }
            Ok(out)
        }
        "help" | "--help" | "-h" => {
            println!("benchmark-autonomy-gate commands:");
            println!("  protheus-ops benchmark-autonomy-gate run [--strict=1|0]");
            println!("  protheus-ops benchmark-autonomy-gate status");
            return 0;
        }
        _ => Err(format!("benchmark_autonomy_gate_unknown_command:{command}")),
    };

    match payload {
        Ok(payload) => {
            let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&cli_receipt(
                &format!("benchmark_autonomy_gate_{}", command.replace('-', "_")),
                payload,
            ));
            if ok || !(strict && policy.strict_default) {
                0
            } else {
                2
            }
        }
        Err(err) => {
            print_json_line(&cli_error("benchmark_autonomy_gate_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_gate_detects_failures() {
        let payload = json!({
            "openclaw_measured": {
                "cold_start_ms": 500.0,
                "idle_memory_mb": 9.0,
                "install_size_mb": 10.0,
                "tasks_per_sec": 1000.0
            }
        });
        let evaluated = evaluate_gate(&payload, &Gates::default());
        let failed = evaluated
            .get("failed")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(failed.iter().any(|row| row.as_str() == Some("cold_start_ms_max")));
        assert!(failed.iter().any(|row| row.as_str() == Some("tasks_per_sec_min")));
    }

    #[test]
    fn read_policy_uses_defaults_when_missing() {
        let tmp = tempfile::tempdir().expect("tmp");
        let policy = read_policy(tmp.path());
        assert!(policy.enabled);
        assert!(policy.strict_default);
        assert_eq!(policy.gates, Gates::default());
    }
}
