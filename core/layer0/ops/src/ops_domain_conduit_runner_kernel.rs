// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

fn usage() {
    println!("ops-domain-conduit-runner-kernel commands:");
    println!("  protheus-ops ops-domain-conduit-runner-kernel parse-argv --payload-base64=<json>");
    println!(
        "  protheus-ops ops-domain-conduit-runner-kernel build-pass-args --payload-base64=<json>"
    );
    println!("  protheus-ops ops-domain-conduit-runner-kernel build-run-options [--payload-base64=<json>]");
    println!(
        "  protheus-ops ops-domain-conduit-runner-kernel prepare-run [--payload-base64=<json>]"
    );
    println!("  protheus-ops ops-domain-conduit-runner-kernel run --payload-base64=<json>");
    println!(
        "  protheus-ops ops-domain-conduit-runner-kernel ipc-daemon [--queue-dir=<path>] [--poll-ms=<n>]"
    );
}

fn receipt_envelope(kind: &str, ok: bool) -> Value {
    let ts = now_iso();
    json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
    })
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw).map_err(|err| {
            format!("ops_domain_conduit_runner_kernel_payload_decode_failed:{err}")
        });
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD.decode(raw_b64.as_bytes()).map_err(|err| {
            format!("ops_domain_conduit_runner_kernel_payload_base64_decode_failed:{err}")
        })?;
        let text = String::from_utf8(bytes).map_err(|err| {
            format!("ops_domain_conduit_runner_kernel_payload_utf8_decode_failed:{err}")
        })?;
        return serde_json::from_str::<Value>(&text).map_err(|err| {
            format!("ops_domain_conduit_runner_kernel_payload_decode_failed:{err}")
        });
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_str(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = as_str(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn parse_bool_text(raw: &str, fallback: bool) -> bool {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn parse_i64_text(raw: &str, fallback: i64) -> i64 {
    raw.trim().parse::<i64>().ok().unwrap_or(fallback)
}

fn argv_list(payload: &Map<String, Value>) -> Vec<String> {
    payload
        .get("argv")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().map(|row| as_str(Some(row))).collect::<Vec<_>>())
        .unwrap_or_default()
}

fn parse_args_map(argv: &[String]) -> Map<String, Value> {
    let mut out = Map::new();
    out.insert("_".to_string(), Value::Array(Vec::new()));
    let mut index = 0usize;
    while index < argv.len() {
        let token = argv[index].trim().to_string();
        if !token.starts_with("--") {
            out.entry("_".to_string()).and_modify(|rows| {
                if let Value::Array(values) = rows {
                    values.push(Value::String(token.clone()));
                }
            });
            index += 1;
            continue;
        }
        if let Some((key, value)) = token.split_once('=') {
            out.insert(
                key.trim_start_matches("--").to_string(),
                Value::String(value.to_string()),
            );
            index += 1;
            continue;
        }
        let key = token.trim_start_matches("--").to_string();
        let next = argv.get(index + 1).cloned().unwrap_or_default();
        if !next.is_empty() && !next.starts_with("--") {
            out.insert(key, Value::String(next));
            index += 2;
            continue;
        }
        out.insert(key, Value::Bool(true));
        index += 1;
    }
    out
}

fn parsed_map(payload: &Map<String, Value>) -> Map<String, Value> {
    payload
        .get("parsed")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| parse_args_map(&argv_list(payload)))
}

fn parsed_positionals(parsed: &Map<String, Value>) -> Vec<String> {
    parsed
        .get("_")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().map(|row| as_str(Some(row))).collect::<Vec<_>>())
        .unwrap_or_default()
}

fn build_pass_args_vec(parsed: &Map<String, Value>) -> Vec<String> {
    let positional = parsed_positionals(parsed);
    let mut forwarded_flags = Vec::new();
    for (key, value) in parsed {
        if matches!(
            key.as_str(),
            "_" | "domain"
                | "run-context"
                | "skip-runtime-gate"
                | "stdio-timeout-ms"
                | "timeout-ms"
        ) {
            continue;
        }
        match value {
            Value::Bool(true) => forwarded_flags.push(format!("--{key}")),
            Value::Null | Value::Bool(false) => {}
            _ => forwarded_flags.push(format!("--{key}={}", as_str(Some(value)))),
        }
    }
    if !as_str(parsed.get("domain")).is_empty() {
        return positional.into_iter().chain(forwarded_flags).collect();
    }
    let mut args = if positional.is_empty() {
        Vec::new()
    } else {
        positional.into_iter().skip(1).collect::<Vec<_>>()
    };
    args.extend(forwarded_flags);
    args
}

fn build_run_options_value(parsed: &Map<String, Value>) -> Value {
    let skip_runtime_gate = parse_bool_text(
        as_str(parsed.get("skip-runtime-gate")).as_str(),
        parse_bool_text(
            std::env::var("PROTHEUS_OPS_DOMAIN_SKIP_RUNTIME_GATE")
                .unwrap_or_else(|_| "true".to_string())
                .as_str(),
            true,
        ),
    );
    let stdio_timeout_ms = parse_i64_text(
        as_str(parsed.get("stdio-timeout-ms")).as_str(),
        parse_i64_text(
            std::env::var("PROTHEUS_OPS_DOMAIN_STDIO_TIMEOUT_MS")
                .or_else(|_| std::env::var("PROTHEUS_CONDUIT_STDIO_TIMEOUT_MS"))
                .unwrap_or_else(|_| "120000".to_string())
                .as_str(),
            120000,
        ),
    );
    let timeout_ms = parse_i64_text(
        as_str(parsed.get("timeout-ms")).as_str(),
        parse_i64_text(
            std::env::var("PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS")
                .or_else(|_| std::env::var("PROTHEUS_CONDUIT_BRIDGE_TIMEOUT_MS"))
                .unwrap_or_else(|_| (stdio_timeout_ms + 1000).max(125000).to_string())
                .as_str(),
            (stdio_timeout_ms + 1000).max(125000),
        ),
    );
    let run_context = clean_text(parsed.get("run-context"), 120);
    json!({
        "runContext": if run_context.is_empty() { Value::Null } else { Value::String(run_context) },
        "skipRuntimeGate": skip_runtime_gate,
        "stdioTimeoutMs": stdio_timeout_ms,
        "timeoutMs": timeout_ms
    })
}

fn run_parse_argv(payload: &Map<String, Value>) -> Value {
    json!({
        "ok": true,
        "parsed": Value::Object(parse_args_map(&argv_list(payload)))
    })
}

fn run_build_pass_args(payload: &Map<String, Value>) -> Value {
    let parsed = parsed_map(payload);
    json!({
        "ok": true,
        "args": Value::Array(build_pass_args_vec(&parsed).into_iter().map(Value::String).collect())
    })
}

fn run_build_run_options(payload: &Map<String, Value>) -> Value {
    let parsed = parsed_map(payload);
    json!({
        "ok": true,
        "options": build_run_options_value(&parsed)
    })
}

fn run_prepare_run(payload: &Map<String, Value>) -> Value {
    let parsed = parsed_map(payload);
    let positionals = parsed_positionals(&parsed);
    let domain = if let Some(value) = parsed.get("domain") {
        clean_text(Some(value), 120)
    } else {
        positionals
            .first()
            .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect::<String>()
    };
    json!({
        "ok": !domain.is_empty(),
        "domain": domain,
        "args": Value::Array(build_pass_args_vec(&parsed).into_iter().map(Value::String).collect()),
        "options": build_run_options_value(&parsed)
    })
}

fn resolve_command_and_args(domain: &str) -> (String, Vec<String>) {
    let explicit = std::env::var("PROTHEUS_OPS_BIN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    if let Some(cmd) = explicit {
        return (cmd, vec![domain.to_string()]);
    }
    if let Ok(current) = std::env::current_exe() {
        return (
            current.to_string_lossy().to_string(),
            vec![domain.to_string()],
        );
    }
    (
        "cargo".to_string(),
        vec![
            "run".to_string(),
            "--quiet".to_string(),
            "-p".to_string(),
            "protheus-ops-core".to_string(),
            "--bin".to_string(),
            "protheus-ops".to_string(),
            "--".to_string(),
            domain.to_string(),
        ],
    )
}

fn parse_json_payload(stdout: &str) -> Option<Value> {
    let raw = stdout.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(raw) {
        return Some(parsed);
    }
    for line in raw.lines().rev() {
        let candidate = line.trim();
        if !candidate.starts_with('{') {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<Value>(candidate) {
            return Some(parsed);
        }
    }
    None
}

fn run_domain_once(root: &Path, domain: &str, args: &[String]) -> Result<(i32, Value), String> {
    let clean_domain = domain
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(120)
        .collect::<String>();
    if clean_domain.is_empty() {
        return Ok((
            2,
            json!({
                "ok": false,
                "type": "ops_domain_conduit_bridge_error",
                "reason": "missing_domain",
                "routed_via": "core_local"
            }),
        ));
    }

    let (command, mut command_args) = resolve_command_and_args(&clean_domain);
    command_args.extend(args.iter().cloned());
    let run = Command::new(&command)
        .args(&command_args)
        .current_dir(root)
        .output()
        .map_err(|err| format!("ops_domain_conduit_runner_kernel_spawn_failed:{err}"))?;

    if !run.stdout.is_empty() {
        std::io::stdout()
            .write_all(&run.stdout)
            .map_err(|err| format!("ops_domain_conduit_runner_kernel_stdout_write_failed:{err}"))?;
    }
    if !run.stderr.is_empty() {
        std::io::stderr()
            .write_all(&run.stderr)
            .map_err(|err| format!("ops_domain_conduit_runner_kernel_stderr_write_failed:{err}"))?;
    }

    let status = run.status.code().unwrap_or(1);
    let parsed = parse_json_payload(String::from_utf8_lossy(&run.stdout).as_ref());
    let payload = if let Some(object) = parsed.and_then(|value| value.as_object().cloned()) {
        let mut owned = Value::Object(object);
        if owned.get("routed_via").is_none() {
            owned["routed_via"] = Value::String("core_local".to_string());
        }
        if status != 0 && owned.get("ok").is_none() {
            owned["ok"] = Value::Bool(false);
        }
        owned
    } else {
        let stderr = String::from_utf8_lossy(&run.stderr);
        let reason = if status == 0 {
            "ok".to_string()
        } else {
            lane_utils::clean_text(Some(stderr.as_ref()), 320)
        };
        json!({
            "ok": status == 0,
            "type": if status == 0 { "ops_domain_conduit_bridge_result" } else { "ops_domain_conduit_bridge_error" },
            "reason": reason,
            "routed_via": "core_local"
        })
    };
    Ok((status, payload))
}

fn run_execute(root: &Path, payload: &Map<String, Value>) -> Value {
    let parsed = parsed_map(payload);
    let positionals = parsed_positionals(&parsed);
    let domain = if let Some(value) = parsed.get("domain") {
        clean_text(Some(value), 120)
    } else {
        positionals
            .first()
            .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect::<String>()
    };
    if domain.is_empty() {
        return json!({
            "ok": false,
            "status": 2,
            "payload": {
                "ok": false,
                "type": "ops_domain_conduit_bridge_error",
                "reason": "missing_domain",
                "routed_via": "core_local"
            }
        });
    }
    let args = build_pass_args_vec(&parsed);
    match run_domain_once(root, &domain, &args) {
        Ok((status, payload)) => json!({
            "ok": status == 0 && payload.get("ok").and_then(Value::as_bool).unwrap_or(true),
            "status": status,
            "payload": payload
        }),
        Err(err) => json!({
            "ok": false,
            "status": 1,
            "payload": {
                "ok": false,
                "type": "ops_domain_conduit_bridge_error",
                "reason": err,
                "routed_via": "core_local"
            }
        }),
    }
}

fn queue_dir_from_argv(root: &Path, argv: &[String]) -> std::path::PathBuf {
    let raw = lane_utils::parse_flag(argv, "queue-dir", false)
        .unwrap_or_else(|| "local/state/tools/ops_bridge_ipc".to_string());
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return root.join("local/state/tools/ops_bridge_ipc");
    }
    let candidate = std::path::PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn poll_ms_from_argv(argv: &[String]) -> u64 {
    let raw = lane_utils::parse_flag(argv, "poll-ms", false).unwrap_or_else(|| "20".to_string());
    parse_i64_text(raw.as_str(), 20).clamp(5, 1000) as u64
}

fn write_json_atomic(path: &Path, payload: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("ops_domain_conduit_runner_kernel_queue_mkdir_failed:{err}"))?;
    }
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_string(payload)
        .map_err(|err| format!("ops_domain_conduit_runner_kernel_queue_encode_failed:{err}"))?;
    fs::write(&tmp, format!("{body}\n"))
        .map_err(|err| format!("ops_domain_conduit_runner_kernel_queue_write_failed:{err}"))?;
    fs::rename(&tmp, path)
        .map_err(|err| format!("ops_domain_conduit_runner_kernel_queue_rename_failed:{err}"))?;
    Ok(())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn write_ipc_heartbeat(path: &Path, poll_ms: u64) -> Result<(), String> {
    write_json_atomic(
        path,
        &json!({
            "ok": true,
            "type": "ops_domain_ipc_daemon_heartbeat",
            "pid": std::process::id(),
            "ts_ms": now_ms(),
            "poll_ms": poll_ms
        }),
    )
}

fn run_ipc_daemon(root: &Path, argv: &[String]) -> Result<(), String> {
    let queue_dir = queue_dir_from_argv(root, argv);
    let poll_ms = poll_ms_from_argv(argv);
    let requests_dir = queue_dir.join("requests");
    let responses_dir = queue_dir.join("responses");
    let heartbeat_path = queue_dir.join("daemon.heartbeat.json");
    fs::create_dir_all(&requests_dir)
        .map_err(|err| format!("ops_domain_conduit_runner_kernel_requests_dir_failed:{err}"))?;
    fs::create_dir_all(&responses_dir)
        .map_err(|err| format!("ops_domain_conduit_runner_kernel_responses_dir_failed:{err}"))?;
    let _ = write_ipc_heartbeat(&heartbeat_path, poll_ms);
    let heartbeat_ticks = ((250 + poll_ms.saturating_sub(1)) / poll_ms.max(1)).max(1);
    let mut tick: u64 = 0;

    loop {
        if tick % heartbeat_ticks == 0 {
            let _ = write_ipc_heartbeat(&heartbeat_path, poll_ms);
        }
        tick = tick.wrapping_add(1);
        let mut request_files = fs::read_dir(&requests_dir)
            .map_err(|err| format!("ops_domain_conduit_runner_kernel_read_dir_failed:{err}"))?
            .filter_map(|entry| entry.ok().map(|row| row.path()))
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        request_files.sort();

        for request_path in request_files {
            let raw = fs::read_to_string(&request_path).unwrap_or_default();
            let request = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}));
            let request_id = clean_text(request.get("id"), 120);
            let domain = clean_text(request.get("domain"), 120);
            let args = request
                .get("args")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().map(|row| as_str(Some(row))).collect::<Vec<_>>())
                .unwrap_or_default();

            let response = if request_id.is_empty() {
                json!({
                    "ok": false,
                    "status": 2,
                    "payload": {
                        "ok": false,
                        "type": "ops_domain_ipc_request_invalid",
                        "reason": "missing_request_id"
                    }
                })
            } else if domain.is_empty() {
                json!({
                    "ok": false,
                    "status": 2,
                    "payload": {
                        "ok": false,
                        "type": "ops_domain_ipc_request_invalid",
                        "reason": "missing_domain"
                    }
                })
            } else {
                match run_domain_once(root, &domain, &args) {
                    Ok((status, payload)) => json!({
                        "ok": status == 0 && payload.get("ok").and_then(Value::as_bool).unwrap_or(true),
                        "status": status,
                        "payload": payload
                    }),
                    Err(err) => json!({
                        "ok": false,
                        "status": 1,
                        "payload": {
                            "ok": false,
                            "type": "ops_domain_conduit_bridge_error",
                            "reason": err
                        }
                    }),
                }
            };

            if !request_id.is_empty() {
                let response_path = responses_dir.join(format!("{request_id}.json"));
                let envelope = json!({
                    "ok": response.get("ok").and_then(Value::as_bool).unwrap_or(false),
                    "request_id": request_id,
                    "response": response
                });
                let _ = write_json_atomic(&response_path, &envelope);
            }
            let _ = fs::remove_file(&request_path);
        }

        thread::sleep(Duration::from_millis(poll_ms));
    }
}

pub fn run(root: &std::path::Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error(
                "ops_domain_conduit_runner_kernel_error",
                err.as_str(),
            ));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let result = match command.as_str() {
        "parse-argv" => Ok(run_parse_argv(payload)),
        "build-pass-args" => Ok(run_build_pass_args(payload)),
        "build-run-options" => Ok(run_build_run_options(payload)),
        "prepare-run" => Ok(run_prepare_run(payload)),
        "run" => Ok(run_execute(root, payload)),
        "ipc-daemon" => match run_ipc_daemon(root, argv) {
            Ok(()) => {
                Ok(json!({"ok": true, "type": "ops_domain_conduit_runner_kernel_ipc_daemon"}))
            }
            Err(err) => Err(err),
        },
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => Err(format!(
            "ops_domain_conduit_runner_kernel_unknown_command:{command}"
        )),
    };

    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt("ops_domain_conduit_runner_kernel", payload));
            0
        }
        Err(err) => {
            print_json_line(&cli_error(
                "ops_domain_conduit_runner_kernel_error",
                err.as_str(),
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_pass_args_respects_flag_domain() {
        let parsed = parse_args_map(&[
            "--domain".to_string(),
            "legacy-retired-lane".to_string(),
            "build".to_string(),
            "--lane-id=FOO-1".to_string(),
        ]);
        let args = build_pass_args_vec(&parsed);
        assert_eq!(
            args,
            vec!["build".to_string(), "--lane-id=FOO-1".to_string()]
        );
    }

    #[test]
    fn build_pass_args_strips_positional_domain() {
        let parsed = parse_args_map(&[
            "legacy-retired-lane".to_string(),
            "build".to_string(),
            "--lane-id=FOO-2".to_string(),
        ]);
        let args = build_pass_args_vec(&parsed);
        assert_eq!(
            args,
            vec!["build".to_string(), "--lane-id=FOO-2".to_string()]
        );
    }

    #[test]
    fn run_execute_missing_domain_returns_status_2() {
        let root = std::path::Path::new(".");
        let payload = Map::new();
        let out = run_execute(root, &payload);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("status").and_then(Value::as_i64), Some(2));
        let reason = out
            .get("payload")
            .and_then(Value::as_object)
            .and_then(|value| value.get("reason"))
            .and_then(Value::as_str);
        assert_eq!(reason, Some("missing_domain"));
    }
}
