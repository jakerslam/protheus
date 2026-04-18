mod crdt_merge;
mod econ_crypto;
mod execution_replay;
mod hybrid_envelope;
mod hybrid_plan;
mod memory_hotpath;
mod red_chaos;
mod security_vault;
mod telemetry_emit;
mod wasm_bridge;

use serde_json::{json, Value};
use std::env;
use std::path::Path;
use std::time::Instant;

fn parse_arg<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    let prefix = format!("--{}=", key);
    let flag = format!("--{key}");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&prefix) {
            return Some(value);
        }
        if arg == &flag {
            if let Some(next) = args.get(idx + 1) {
                if !next.starts_with("--") {
                    return Some(next.as_str());
                }
            }
        }
    }
    None
}

fn normalize_command_token(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in raw.trim().chars() {
        if ch.is_control() {
            continue;
        }
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };
        if mapped == '-' {
            if last_dash {
                continue;
            }
            last_dash = true;
            out.push(mapped);
            continue;
        }
        last_dash = false;
        out.push(mapped);
    }
    out.trim_matches('-').to_string()
}

fn parse_bool(v: Option<&str>, fallback: bool) -> bool {
    match v.unwrap_or("").trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn parse_u32(v: Option<&str>, fallback: u32) -> u32 {
    v.and_then(|raw| raw.trim().parse::<u32>().ok())
        .unwrap_or(fallback)
}

fn parse_usize(v: Option<&str>, fallback: usize) -> usize {
    v.and_then(|raw| raw.trim().parse::<usize>().ok())
        .unwrap_or(fallback)
}

fn parse_f64(v: Option<&str>, fallback: f64) -> f64 {
    v.and_then(|raw| raw.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
}

fn events_from_arg(v: Option<&str>) -> Vec<String> {
    let raw = v.unwrap_or("start,hydrate,execute,receipt,commit");
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn print_json(v: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(v)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn help() -> Value {
    json!({
        "ok": true,
        "commands": [
            "hybrid-plan --root=. --min=15 --max=25",
            "memory-hotpath",
            "execution-replay --events=a,b,c",
            "security-vault --tampered=0|1",
            "crdt-merge",
            "econ-crypto",
            "red-chaos --cycles=100000",
            "telemetry-emit",
            "wasm-bridge",
            "hybrid-envelope --within-target=0|1 --completed=9"
        ]
    })
}

fn wrap_command_receipt(command: &str, payload: Value, started: Instant) -> Value {
    let ok = payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let duration_ms = started.elapsed().as_secs_f64() * 1000.0;
    json!({
        "ok": ok,
        "type": "hybrid_runtime_command_receipt",
        "command": command,
        "status": if ok { "success" } else { "error" },
        "payload": payload,
        "telemetry": {
            "duration_ms": duration_ms
        }
    })
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let raw_cmd = args.first().map(String::as_str).unwrap_or("help");
    let normalized_cmd = normalize_command_token(raw_cmd);
    let cmd = if normalized_cmd.is_empty() {
        "help".to_string()
    } else {
        normalized_cmd
    };
    let started = Instant::now();

    let payload = match cmd.as_str() {
        "help" | "--help" | "-h" => help(),
        "hybrid-plan" => {
            let root = hybrid_plan::resolve_root_with_status(parse_arg(&args, "root"));
            let min = parse_f64(parse_arg(&args, "min"), 15.0);
            let max = parse_f64(parse_arg(&args, "max"), 25.0);
            let mut report = hybrid_plan::scan_language_share(Path::new(&root.resolved), min, max);
            if let Some(obj) = report.as_object_mut() {
                obj.insert(
                    "root_resolution".to_string(),
                    serde_json::to_value(root)
                        .unwrap_or_else(|_| json!({"accepted": false, "reason": "root_serialize_failed"})),
                );
            }
            report
        }
        "memory-hotpath" => memory_hotpath::sample_report(),
        "execution-replay" => {
            let events = events_from_arg(parse_arg(&args, "events"));
            execution_replay::replay_report(&events)
        }
        "security-vault" => {
            let tampered = parse_bool(parse_arg(&args, "tampered"), false);
            let mut report = security_vault::sample_report();
            let allowed = security_vault::fail_closed_attestation(tampered);
            if let Some(obj) = report.as_object_mut() {
                obj.insert("ok".to_string(), Value::Bool(allowed));
                obj.insert(
                    "attestation".to_string(),
                    json!({"tamper_detected": tampered, "allowed": allowed, "mode": "fail_closed"}),
                );
            }
            report
        }
        "crdt-merge" => crdt_merge::sample_report(),
        "econ-crypto" => econ_crypto::sample_report(),
        "red-chaos" => {
            let cycles = parse_u32(parse_arg(&args, "cycles"), 50_000);
            red_chaos::sample_report(cycles)
        }
        "telemetry-emit" => telemetry_emit::sample_report(),
        "wasm-bridge" => wasm_bridge::sample_report(),
        "hybrid-envelope" => {
            let within_target = parse_bool(parse_arg(&args, "within-target"), false);
            let completed = parse_usize(parse_arg(&args, "completed"), 9);
            hybrid_envelope::build_envelope(within_target, completed)
        }
        _ => json!({"ok": false, "error": "unknown_command", "command": raw_cmd, "normalized_command": cmd}),
    };
    let out = wrap_command_receipt(&cmd, payload, started);

    print_json(&out);
}
