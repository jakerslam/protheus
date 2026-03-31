// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "local/state/ops/dspy_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/dspy_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/dspy_bridge/swarm_state.json";

fn usage() {
    println!("dspy-bridge commands:");
    println!("  protheus-ops dspy-bridge status [--state-path=<path>]");
    println!("  protheus-ops dspy-bridge register-signature [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dspy-bridge compile-program [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dspy-bridge optimize-program [--payload-base64=<json>] [--state-path=<path>]");
    println!(
        "  protheus-ops dspy-bridge assert-program [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  protheus-ops dspy-bridge import-integration [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dspy-bridge execute-multihop [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops dspy-bridge record-benchmark [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dspy-bridge record-optimization-trace [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops dspy-bridge assimilate-intake [--payload-base64=<json>] [--state-path=<path>]");
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

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("dspy_bridge_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("dspy_bridge_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("dspy_bridge_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("dspy_bridge_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn repo_path(root: &Path, rel: &str) -> PathBuf {
    let candidate = PathBuf::from(rel.trim());
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn rel(root: &Path, path: &Path) -> String {
    lane_utils::rel_path(root, path)
}

fn state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::parse_flag(argv, "state-path", false)
        .or_else(|| {
            payload
                .get("state_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(DEFAULT_STATE_REL))
}

fn history_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::parse_flag(argv, "history-path", false)
        .or_else(|| {
            payload
                .get("history_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(DEFAULT_HISTORY_REL))
}

fn swarm_state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::parse_flag(argv, "swarm-state-path", false)
        .or_else(|| {
            payload
                .get("swarm_state_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(DEFAULT_SWARM_STATE_REL))
}

fn default_state() -> Value {
    json!({
        "schema_version": "dspy_bridge_state_v1",
        "signatures": {},
        "compiled_programs": {},
        "optimization_runs": {},
        "assertion_runs": {},
        "integrations": {},
        "multihop_runs": {},
        "benchmarks": {},
        "optimization_traces": {},
        "intakes": {},
        "last_receipt": null,
    })
}

fn ensure_state_shape(value: &mut Value) {
    if !value.is_object() {
        *value = default_state();
        return;
    }
    for key in [
        "signatures",
        "compiled_programs",
        "optimization_runs",
        "assertion_runs",
        "integrations",
        "multihop_runs",
        "benchmarks",
        "optimization_traces",
        "intakes",
    ] {
        if !value.get(key).map(Value::is_object).unwrap_or(false) {
            value[key] = json!({});
        }
    }
    if value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_none()
    {
        value["schema_version"] = json!("dspy_bridge_state_v1");
    }
}

fn load_state(path: &Path) -> Value {
    let mut state = lane_utils::read_json(path).unwrap_or_else(default_state);
    ensure_state_shape(&mut state);
    state
}

fn save_state(path: &Path, state: &Value) -> Result<(), String> {
    lane_utils::write_json(path, state)
}

fn append_history(path: &Path, row: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, row)
}

fn as_object_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !value.get(key).map(Value::is_object).unwrap_or(false) {
        value[key] = json!({});
    }
    value
        .get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object")
}

fn now_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|row| row.as_millis())
        .unwrap_or(0)
}

fn to_base36(mut value: u128) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while value > 0 {
        let digit = (value % 36) as u8;
        out.push(if digit < 10 {
            (b'0' + digit) as char
        } else {
            (b'a' + digit - 10) as char
        });
        value /= 36;
    }
    out.iter().rev().collect()
}

fn stable_id(prefix: &str, basis: &Value) -> String {
    let digest = deterministic_receipt_hash(basis);
    format!("{prefix}_{}_{}", to_base36(now_millis()), &digest[..12])
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    lane_utils::clean_text(raw, max_len)
}

fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    lane_utils::clean_token(raw, fallback)
}

fn parse_u64_value(value: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    value
        .and_then(|row| match row {
            Value::Number(n) => n.as_u64(),
            Value::String(s) => s.trim().parse::<u64>().ok(),
            _ => None,
        })
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_f64_value(value: Option<&Value>, fallback: f64, min: f64, max: f64) -> f64 {
    value
        .and_then(|row| match row {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        })
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_bool_value(value: Option<&Value>, fallback: bool) -> bool {
    value
        .and_then(|row| match row {
            Value::Bool(v) => Some(*v),
            Value::String(s) => {
                let lower = s.trim().to_ascii_lowercase();
                match lower.as_str() {
                    "1" | "true" | "yes" | "on" => Some(true),
                    "0" | "false" | "no" | "off" => Some(false),
                    _ => None,
                }
            }
            _ => None,
        })
        .unwrap_or(fallback)
}

fn parse_string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|s| clean_token(Some(s), "")))
        .filter(|row| !row.is_empty())
        .collect()
}

fn safe_prefix_for_bridge(path: &str) -> bool {
    [
        "adapters/",
        "client/runtime/systems/",
        "client/runtime/lib/",
        "client/lib/",
        "planes/contracts/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

fn safe_shell_prefix(path: &str) -> bool {
    ["client/", "apps/"]
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    let candidate = raw.trim();
    if candidate.is_empty() {
        return Err("dspy_bridge_path_required".to_string());
    }
    if candidate.contains("..") {
        return Err("dspy_unsafe_bridge_path_parent_reference".to_string());
    }
    let abs = repo_path(root, candidate);
    let rel_path = rel(root, &abs);
    if !safe_prefix_for_bridge(&rel_path) {
        return Err("dspy_unsupported_bridge_path".to_string());
    }
    Ok(rel_path)
}

fn normalize_shell_path(root: &Path, raw: &str) -> Result<String, String> {
    let candidate = raw.trim();
    if candidate.is_empty() {
        return Err("dspy_shell_path_required".to_string());
    }
    if candidate.contains("..") {
        return Err("dspy_shell_path_parent_reference".to_string());
    }
    let abs = repo_path(root, candidate);
    let rel_path = rel(root, &abs);
    if !safe_shell_prefix(&rel_path) {
        return Err("dspy_shell_path_outside_client_or_apps".to_string());
    }
    Ok(rel_path)
}

fn encode_json_arg(value: &Value) -> Result<String, String> {
    serde_json::to_string(value).map_err(|err| format!("dspy_json_encode_failed:{err}"))
}

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn dspy_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-017.1" => "dspy_signatures_and_typed_predictors_register_over_authoritative_workflow_and_swarm_lanes",
        "V6-WORKFLOW-017.2" => "dspy_modules_and_compiler_runs_normalize_to_the_authoritative_workflow_engine",
        "V6-WORKFLOW-017.3" => "dspy_optimizer_and_teleprompter_runs_remain_receipted_policy_bounded_and_profile_safe",
        "V6-WORKFLOW-017.4" => "dspy_assertions_retry_or_reject_fail_closed_with_deterministic_receipts",
        "V6-WORKFLOW-017.5" => "dspy_multihop_rag_and_agent_loops_reuse_memory_skill_and_swarm_primitives",
        "V6-WORKFLOW-017.6" => "dspy_metrics_evaluators_and_benchmarks_stream_through_native_observability_and_evidence",
        "V6-WORKFLOW-017.7" => "dspy_integrations_normalize_through_governed_intake_and_adapter_bridges",
        "V6-WORKFLOW-017.8" => "dspy_optimization_and_reproducibility_traces_flow_through_native_observability",
        _ => "dspy_bridge_claim",
    }
}

fn read_swarm_state(path: &Path) -> Value {
    lane_utils::read_json(path)
        .unwrap_or_else(|| json!({ "sessions": {}, "handoff_registry": {}, "message_queues": {} }))
}

fn find_swarm_session_id_by_task(state: &Value, task: &str) -> Option<String> {
    state
        .get("sessions")
        .and_then(Value::as_object)
        .and_then(|rows| {
            rows.iter().find_map(|(session_id, row)| {
                let row_task = row.get("task").and_then(Value::as_str);
                let report_task = row
                    .get("report")
                    .and_then(|value| value.get("task"))
                    .and_then(Value::as_str);
                (row_task == Some(task) || report_task == Some(task)).then(|| session_id.clone())
            })
        })
}

fn emit_native_trace(
    root: &Path,
    trace_id: &str,
    intent: &str,
    message: &str,
) -> Result<(), String> {
    let enable_exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=enable".to_string(),
            "--enabled=1".to_string(),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if enable_exit != 0 {
        return Err("dspy_observability_enable_failed".to_string());
    }
    let exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=trace".to_string(),
            "--source-agent=dspy-bridge".to_string(),
            format!("--target-agent={}", clean_token(Some(intent), "workflow")),
            format!("--intent={}", clean_text(Some(intent), 80)),
            format!("--message={}", clean_text(Some(message), 160)),
            format!("--trace-id={trace_id}"),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if exit != 0 {
        return Err("dspy_observability_trace_failed".to_string());
    }
    Ok(())
}

