// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

const DEFAULT_POLICY_REL: &str = "client/runtime/config/system_health_audit_runner_policy.json";
const DEFAULT_LATEST_REL: &str = "local/state/ops/system_health_audit/latest.json";
const DEFAULT_RECEIPTS_REL: &str = "local/state/ops/system_health_audit/receipts.jsonl";
const RUST_SOURCE_OF_TRUTH_POLICY_REL: &str = "client/runtime/config/rust_source_of_truth_policy.json";
const WEB_PROVIDER_CONTRACT_TARGETS: &[&str] = &[
    "brave",
    "duckduckgo",
    "exa",
    "firecrawl",
    "google",
    "minimax",
    "moonshot",
    "perplexity",
    "tavily",
    "xai",
];

#[derive(Clone, Debug)]
struct HealthPolicy {
    enabled: bool,
    check_timeout_ms: u64,
    latest_path: String,
    receipts_path: String,
}

#[derive(Clone, Debug)]
struct CheckRun {
    status: i32,
    payload_type: Option<String>,
    stderr: String,
}

fn usage() {
    println!("system-health-audit-runner-kernel commands:");
    println!(
        "  infring-ops system-health-audit-runner-kernel run [--strict=1|0] [--policy=<path>]"
    );
    println!("  infring-ops system-health-audit-runner-kernel status [--policy=<path>]");
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

fn workspace_root(root: &Path) -> PathBuf {
    if let Some(raw) = std::env::var_os("INFRING_WORKSPACE") {
        let value = PathBuf::from(raw);
        if value.is_absolute() {
            return value;
        }
    }
    root.to_path_buf()
}

fn resolve_path(root: &Path, raw: &str, fallback_rel: &str) -> PathBuf {
    let workspace = workspace_root(root);
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return workspace.join(fallback_rel);
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        workspace.join(trimmed)
    }
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    lane_utils::write_json(path, value)
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, value)
}

fn parse_last_json(text: &str) -> Option<Value> {
    let raw = text.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        return Some(value);
    }
    let first_brace = raw.find('{')?;
    let last_brace = raw.rfind('}')?;
    if last_brace > first_brace {
        if let Ok(value) = serde_json::from_str::<Value>(&raw[first_brace..=last_brace]) {
            return Some(value);
        }
    }
    for line in raw.lines().rev() {
        let line = line.trim();
        if !(line.starts_with('{') && line.ends_with('}')) {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            return Some(value);
        }
    }
    None
}

fn load_policy(root: &Path, argv: &[String]) -> HealthPolicy {
    let policy_path = resolve_path(
        root,
        lane_utils::parse_flag(argv, "policy", false)
            .as_deref()
            .unwrap_or(DEFAULT_POLICY_REL),
        DEFAULT_POLICY_REL,
    );
    let parsed = read_json(&policy_path).unwrap_or_else(|| json!({}));
    HealthPolicy {
        enabled: parsed
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        check_timeout_ms: parsed
            .get("check_timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(300_000),
        latest_path: parsed
            .get("latest_path")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_LATEST_REL)
            .to_string(),
        receipts_path: parsed
            .get("receipts_path")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_RECEIPTS_REL)
            .to_string(),
    }
}

fn run_ops_capture(domain: &str, args: &[&str], timeout_ms: u64) -> CheckRun {
    let command = std::env::var("INFRING_OPS_BIN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe().unwrap_or_else(|_| PathBuf::from("infring-ops"))
        });
    let output = Command::new(command).arg(domain).args(args).output();
    let Ok(output) = output else {
        return CheckRun {
            status: 1,
            payload_type: None,
            stderr: "spawn_failed".to_string(),
        };
    };
    let status = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let payload_type = parse_last_json(&stdout).and_then(|value| {
        value
            .get("type")
            .and_then(Value::as_str)
            .map(|v| v.to_string())
    });
    let stderr_tail = if stderr.len() > 300 {
        stderr[stderr.len() - 300..].to_string()
    } else {
        stderr
    };
    let _ = timeout_ms;
    CheckRun {
        status,
        payload_type,
        stderr: stderr_tail,
    }
}

fn web_tooling_contract_check(root: &Path) -> Value {
    let policy_path = resolve_path(root, RUST_SOURCE_OF_TRUTH_POLICY_REL, RUST_SOURCE_OF_TRUTH_POLICY_REL);
    let policy = read_json(&policy_path).unwrap_or_else(|| json!({}));
    let entries = policy
        .pointer("/web_tooling_contract_targets_gate/entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut collected = Vec::<String>::new();
    for entry in &entries {
        for provider in entry
            .get("provider_targets")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(|row| row.trim().to_ascii_lowercase())
            .filter(|row| !row.is_empty())
        {
            if !collected.iter().any(|existing| existing == &provider) {
                collected.push(provider);
            }
        }
    }
    let missing_targets = WEB_PROVIDER_CONTRACT_TARGETS
        .iter()
        .filter(|target| !collected.iter().any(|row| row == *target))
        .map(|row| row.to_string())
        .collect::<Vec<_>>();
    json!({
        "id": "web_tooling_contract_targets_gate",
        "ok": !entries.is_empty() && missing_targets.is_empty(),
        "policy_path": policy_path.display().to_string(),
        "entry_count": entries.len(),
        "provider_targets_found": collected,
        "provider_targets_expected": WEB_PROVIDER_CONTRACT_TARGETS,
        "missing_provider_targets": missing_targets
    })
}

fn build_health_snapshot_with_runner<F>(root: &Path, strict: bool, mut runner: F) -> Value
where
    F: FnMut(&str, &[&str]) -> CheckRun,
{
    let mut checks = [
        (
            "control_plane",
            "infring-control-plane",
            vec!["status", if strict { "--strict=1" } else { "--strict=0" }],
        ),
        ("alpha_readiness", "alpha-readiness", vec!["status"]),
        ("swarm_runtime", "swarm-runtime", vec!["status"]),
        (
            "supply_chain_provenance",
            "supply-chain-provenance-v2",
            vec!["status"],
        ),
    ]
    .into_iter()
    .map(|(id, domain, args)| {
        let run = runner(domain, &args);
        json!({
            "id": id,
            "ok": run.status == 0,
            "status_code": run.status,
            "payload_type": run.payload_type,
            "stderr_tail": run.stderr
        })
    })
    .collect::<Vec<_>>();
    checks.push(web_tooling_contract_check(root));
    let failed = checks
        .iter()
        .filter(|row| row.get("ok").and_then(Value::as_bool) != Some(true))
        .filter_map(|row| row.get("id").and_then(Value::as_str).map(|v| v.to_string()))
        .collect::<Vec<_>>();
    json!({
        "ok": failed.is_empty(),
        "type": "system_health_audit_runner",
        "generated_at": now_iso(),
        "strict": strict,
        "checks": checks,
        "failed": failed
    })
}

fn build_health_snapshot(root: &Path, policy: &HealthPolicy, strict: bool) -> Value {
    build_health_snapshot_with_runner(root, strict, |domain, args| {
        run_ops_capture(domain, args, policy.check_timeout_ms)
    })
}

fn status_payload(root: &Path, policy: &HealthPolicy) -> Result<Value, String> {
    let latest_path = resolve_path(root, &policy.latest_path, DEFAULT_LATEST_REL);
    let latest =
        fs::read_to_string(&latest_path).map_err(|_| "missing_latest_health_audit".to_string())?;
    serde_json::from_str::<Value>(&latest)
        .map_err(|err| format!("system_health_audit_runner_kernel_decode_latest_failed:{err}"))
}

fn run_command(root: &Path, argv: &[String]) -> Result<(Value, i32), String> {
    let command = argv.first().map(|value| value.as_str()).unwrap_or("run");
    let strict = lane_utils::parse_bool(
        lane_utils::parse_flag(argv, "strict", false).as_deref(),
        true,
    );
    let policy = load_policy(root, argv);

    if !policy.enabled {
        let payload = json!({
            "ok": false,
            "type": "system_health_audit_runner",
            "generated_at": now_iso(),
            "error": "lane_disabled_by_policy"
        });
        return Ok((payload, 1));
    }

    match command {
        "status" => Ok((status_payload(root, &policy)?, 0)),
        "run" => {
            let out = build_health_snapshot(root, &policy, strict);
            let latest_path = resolve_path(root, &policy.latest_path, DEFAULT_LATEST_REL);
            let receipts_path = resolve_path(root, &policy.receipts_path, DEFAULT_RECEIPTS_REL);
            write_json(&latest_path, &out)?;
            append_jsonl(&receipts_path, &out)?;
            let exit_code = if out.get("ok").and_then(Value::as_bool) == Some(true) {
                0
            } else {
                2
            };
            Ok((out, exit_code))
        }
        _ => Err("system_health_audit_runner_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|value| value.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    match run_command(root, argv) {
        Ok((payload, exit_code)) => {
            print_json_line(&cli_receipt("system_health_audit_runner_kernel", payload));
            exit_code
        }
        Err(err) => {
            print_json_line(&cli_error("system_health_audit_runner_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_health_snapshot_marks_failed_checks() {
        let snapshot =
            build_health_snapshot_with_runner(Path::new("."), true, |domain, _args| CheckRun {
            status: if domain == "swarm-runtime" { 1 } else { 0 },
            payload_type: Some(format!("{domain}_status")),
            stderr: String::new(),
        });
        assert_eq!(snapshot.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            snapshot.pointer("/failed/0").and_then(Value::as_str),
            Some("swarm_runtime")
        );
    }
}
