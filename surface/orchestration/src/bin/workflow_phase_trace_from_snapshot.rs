use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_SNAPSHOT_PATH: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/latest_snapshot.json";
const DEFAULT_OUT_PATH: &str = "local/state/ops/orchestration/workflow_phase_trace_latest.json";

fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline) {
            return Some(value.to_string());
        }
        if arg == &format!("--{key}") {
            return args.get(idx + 1).cloned();
        }
    }
    None
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn ensure_parent(path: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn read_json(path: &str) -> io::Result<Value> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str::<Value>(&raw)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn bool_field(value: &Value, key: &str, default: bool) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn collectors() -> Value {
    json!([
        {
            "collector_id": "kernel_runtime_receipts",
            "role": "authoritative_runtime_facts",
            "path_hint": "core/local/artifacts/** and local/state/attention/receipts.jsonl"
        },
        {
            "collector_id": "dashboard_troubleshooting_snapshot",
            "role": "shell_display_snapshot",
            "path_hint": "client/runtime/local/state/ui/infring_dashboard/troubleshooting/**"
        },
        {
            "collector_id": "attention_passive_memory",
            "role": "chat_turn_event_stream",
            "path_hint": "local/state/attention/queue.jsonl"
        }
    ])
}

fn phase(name: &str, status: &str, note: String) -> Value {
    json!({
        "phase": name,
        "status": status,
        "owner": "surface_orchestration_control_plane",
        "note": note,
        "eval_visible": true
    })
}

fn receipt_hashes(entries: &[Value]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|entry| entry.get("receipt_hash").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect()
}

fn first_user_intent(entries: &[Value]) -> String {
    entries
        .iter()
        .filter_map(|entry| {
            entry
                .get("exchange")
                .and_then(|exchange| exchange.get("user"))
                .and_then(Value::as_str)
        })
        .find(|text| !text.trim().is_empty())
        .unwrap_or("unavailable:snapshot_lacks_user_intent")
        .to_string()
}

fn normalized_failure_codes(entries: &[Value]) -> Vec<String> {
    let mut codes = entries
        .iter()
        .filter_map(|entry| entry.get("error_code").and_then(Value::as_str))
        .filter(|code| !code.trim().is_empty())
        .map(|code| code.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    codes.sort();
    codes.dedup();
    codes
}

fn tool_family(entries: &[Value]) -> &'static str {
    if entries.iter().any(|entry| {
        entry
            .get("exchange")
            .and_then(|exchange| exchange.get("tool_receipts"))
            .and_then(Value::as_array)
            .is_some_and(|receipts| !receipts.is_empty())
    }) {
        "tool_route"
    } else {
        "none"
    }
}

fn tool_result_summary(entries: &[Value]) -> String {
    let tool_receipt_count = entries
        .iter()
        .filter_map(|entry| {
            entry
                .get("exchange")
                .and_then(|exchange| exchange.get("tool_receipts"))
                .and_then(Value::as_array)
        })
        .map(Vec::len)
        .sum::<usize>();
    format!(
        "entries={};tool_receipts={};receipt_hashes={}",
        entries.len(),
        tool_receipt_count,
        receipt_hashes(entries).len()
    )
}

fn issue_signals(entries: &[Value]) -> Vec<Value> {
    entries
        .iter()
        .filter(|entry| !bool_field(entry, "lane_ok", true))
        .map(|entry| {
            json!({
                "signal_id": "collector_workflow_lane_failed",
                "severity_hint": "high",
                "phase": "coordination_sequencing",
                "summary": format!(
                    "workflow_id={} error_code={}",
                    str_field(entry, "workflow_id"),
                    str_field(entry, "error_code")
                )
            })
        })
        .collect()
}

fn hash_trace(trace: &Value) -> String {
    let mut canonical = trace.clone();
    if let Some(obj) = canonical.as_object_mut() {
        obj.insert("receipt_hash".to_string(), Value::String(String::new()));
    }
    let payload = serde_json::to_vec(&canonical).unwrap_or_default();
    format!("{:x}", Sha256::digest(payload))
}

fn build_trace(snapshot: &Value, generated_at_ms: u64) -> Value {
    let entries = snapshot
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let failure_count = snapshot
        .get("failure_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let trace_id = str_field(snapshot, "snapshot_id");
    let active_stage = if failure_count > 0 {
        "recovery_escalation"
    } else {
        "verification_closure"
    };
    let mut trace = json!({
        "type": "orchestration_workflow_phase_trace",
        "schema_version": 1,
        "generated_at_ms": generated_at_ms,
        "owner": "surface_orchestration_control_plane",
        "trace_id": if trace_id.is_empty() { str_field(snapshot, "receipt_hash") } else { trace_id },
        "user_intent": first_user_intent(&entries),
        "selected_workflow": "diagnose_retry_escalate",
        "selected_model": null,
        "tool_decision": if entries.is_empty() { "no_collector_entries" } else { "normalize_collector_tool_receipts" },
        "tool_family": tool_family(&entries),
        "tool_result_summary": tool_result_summary(&entries),
        "finalization_status": if failure_count > 0 { "blocked" } else { "completed" },
        "fallback_path": if failure_count > 0 { "recovery_escalation" } else { "none" },
        "latency_ms": null,
        "workflow_template": "diagnose_retry_escalate",
        "active_stage": active_stage,
        "phases": [
            phase("intake_normalization", "completed", "collector snapshot accepted for orchestration normalization".to_string()),
            phase("decomposition_planning", "completed", format!("collector entries={}", entries.len())),
            phase("coordination_sequencing", if failure_count > 0 { "blocked" } else { "completed" }, format!("failure_count={failure_count}")),
            phase("recovery_escalation", if failure_count > 0 { "ready" } else { "skipped" }, "collector failure signals mapped for eval".to_string()),
            phase("result_packaging", "completed", "canonical orchestration trace packaged".to_string()),
            phase("verification_closure", "completed", "receipt hash generated for trace integrity".to_string())
        ],
        "collectors": collectors(),
        "decision_trace": {
            "chosen": "normalize_dashboard_collector_snapshot",
            "alternatives_rejected": [],
            "confidence": 0.91,
            "rationale": ["dashboard_snapshot_is_collector_input", "orchestration_owns_phase_trace_normalization"]
        },
        "observed_kernel_receipt_ids": receipt_hashes(&entries),
        "observed_kernel_outcome_refs": [],
        "expected_kernel_contract_ids": [],
        "normalized_failure_codes": normalized_failure_codes(&entries),
        "issue_signals": issue_signals(&entries),
        "receipt_hash": ""
    });
    let receipt_hash = hash_trace(&trace);
    trace["receipt_hash"] = Value::String(receipt_hash);
    trace
}

fn run() -> io::Result<Value> {
    let args: Vec<String> = env::args().skip(1).collect();
    let snapshot_path =
        parse_flag(&args, "snapshot").unwrap_or_else(|| DEFAULT_SNAPSHOT_PATH.to_string());
    let out_path = parse_flag(&args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let snapshot = read_json(&snapshot_path)?;
    let trace = build_trace(&snapshot, now_ms());
    ensure_parent(&out_path)?;
    fs::write(
        &out_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&trace).unwrap_or_default()
        ),
    )?;
    Ok(trace)
}

fn main() -> ExitCode {
    match run() {
        Ok(report) => {
            let _ = writeln!(
                io::stdout(),
                "{}",
                serde_json::to_string(&report).unwrap_or_default()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            let _ = writeln!(
                io::stderr(),
                "workflow phase trace generation failed: {err}"
            );
            ExitCode::from(1)
        }
    }
}
