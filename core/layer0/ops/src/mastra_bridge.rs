// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "local/state/ops/mastra_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/mastra_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/mastra_bridge/swarm_state.json";
const DEFAULT_APPROVAL_QUEUE_REL: &str = "client/runtime/local/state/mastra_approvals.yaml";

fn usage() {
    println!("mastra-bridge commands:");
    println!("  protheus-ops mastra-bridge status [--state-path=<path>]");
    println!("  protheus-ops mastra-bridge register-graph [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops mastra-bridge execute-graph [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops mastra-bridge run-agent-loop [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops mastra-bridge memory-recall [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops mastra-bridge suspend-run [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  protheus-ops mastra-bridge resume-run [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>] [--approval-queue-path=<path>]");
    println!("  protheus-ops mastra-bridge register-mcp-bridge [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops mastra-bridge invoke-mcp-bridge [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  protheus-ops mastra-bridge record-eval-trace [--payload-base64=<json>] [--state-path=<path>]");
    println!(
        "  protheus-ops mastra-bridge deploy-shell [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  protheus-ops mastra-bridge register-runtime-bridge [--payload-base64=<json>] [--state-path=<path>]");
    println!(
        "  protheus-ops mastra-bridge route-model [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  protheus-ops mastra-bridge scaffold-intake [--payload-base64=<json>] [--state-path=<path>]");
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
            .map_err(|err| format!("mastra_bridge_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("mastra_bridge_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("mastra_bridge_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("mastra_bridge_payload_decode_failed:{err}"));
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

fn approval_queue_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    lane_utils::parse_flag(argv, "approval-queue-path", false)
        .or_else(|| {
            payload
                .get("approval_queue_path")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(DEFAULT_APPROVAL_QUEUE_REL))
}

fn default_state() -> Value {
    json!({
        "schema_version": "mastra_bridge_state_v1",
        "graphs": {},
        "graph_runs": {},
        "agent_loops": {},
        "memory_recalls": {},
        "suspended_runs": {},
        "mcp_bridges": {},
        "run_snapshots": {},
        "eval_traces": {},
        "deployments": {},
        "runtime_bridges": {},
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
        "graphs",
        "graph_runs",
        "agent_loops",
        "memory_recalls",
        "suspended_runs",
        "mcp_bridges",
        "run_snapshots",
        "eval_traces",
        "deployments",
        "runtime_bridges",
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
        value["schema_version"] = json!("mastra_bridge_state_v1");
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
        return Err("mastra_bridge_path_required".to_string());
    }
    if candidate.contains("..") {
        return Err("mastra_unsafe_bridge_path_parent_reference".to_string());
    }
    let abs = repo_path(root, candidate);
    let rel_path = rel(root, &abs);
    if !safe_prefix_for_bridge(&rel_path) {
        return Err("mastra_unsupported_bridge_path".to_string());
    }
    Ok(rel_path)
}

fn normalize_shell_path(root: &Path, raw: &str) -> Result<String, String> {
    let candidate = raw.trim();
    if candidate.is_empty() {
        return Err("mastra_shell_path_required".to_string());
    }
    if candidate.contains("..") {
        return Err("mastra_shell_path_parent_reference".to_string());
    }
    let abs = repo_path(root, candidate);
    let rel_path = rel(root, &abs);
    if !safe_shell_prefix(&rel_path) {
        return Err("mastra_shell_path_outside_client_or_apps".to_string());
    }
    Ok(rel_path)
}

fn encode_json_arg(value: &Value) -> Result<String, String> {
    serde_json::to_string(value).map_err(|err| format!("mastra_json_encode_failed:{err}"))
}

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn mastra_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-011.1" => "mastra_graph_workflows_register_and_execute_as_receipted_chain_branch_and_parallel_runs_over_authoritative_workflow_and_swarm_lanes",
        "V6-WORKFLOW-011.2" => "mastra_agent_tool_reasoning_reuses_authoritative_swarm_budgets_sessions_and_route_receipts",
        "V6-WORKFLOW-011.3" => "mastra_memory_recall_routes_through_existing_memory_runtime_and_budget_enforcement_with_profile_safe_degradation",
        "V6-WORKFLOW-011.4" => "mastra_suspend_resume_reuses_existing_receipt_backed_state_and_approval_gate_semantics",
        "V6-WORKFLOW-011.5" => "mastra_mcp_interoperability_is_adapter_owned_fail_closed_and_deterministically_receipted",
        "V6-WORKFLOW-011.6" => "mastra_evals_and_traces_emit_native_observability_receipts_without_a_parallel_telemetry_stack",
        "V6-WORKFLOW-011.7" => "mastra_multi_provider_model_routing_remains_adapter_owned_receipted_and_profile_safe",
        "V6-WORKFLOW-011.8" => "mastra_studio_and_full_stack_shells_remain_non_authoritative_and_delegate_back_to_core_receipts",
        "V6-WORKFLOW-011.9" => "mastra_ts_first_intake_scaffolds_thin_templates_without_forcing_node_dependency_into_sovereign_profiles",
        _ => "mastra_bridge_claim",
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

fn profile_supported(supported_profiles: &[String], profile: &str) -> bool {
    supported_profiles.is_empty() || supported_profiles.iter().any(|row| row == profile)
}

fn read_yaml_value(path: &Path) -> Value {
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    if raw.trim().is_empty() {
        return json!({});
    }
    serde_yaml::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}))
}

fn approval_status_from_queue(queue_path: &Path, action_id: &str) -> String {
    let queue = read_yaml_value(queue_path);
    for (status, key) in [
        ("pending", "pending"),
        ("approved", "approved"),
        ("denied", "denied"),
    ] {
        if queue
            .get(key)
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .any(|row| row.get("action_id").and_then(Value::as_str) == Some(action_id))
            })
            .unwrap_or(false)
        {
            return status.to_string();
        }
    }
    "unknown".to_string()
}

fn approval_is_approved(queue_path: &Path, action_id: &str) -> bool {
    approval_status_from_queue(queue_path, action_id) == "approved"
}

fn allowed_language(language: &str) -> bool {
    matches!(language, "python" | "ts" | "go" | "java" | "rust")
}

fn allowed_tool_kind(kind: &str) -> bool {
    matches!(kind, "native" | "mcp" | "openapi" | "custom")
}

fn allowed_workflow_mode(mode: &str) -> bool {
    matches!(mode, "sequential" | "parallel" | "loop")
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
        return Err("mastra_observability_enable_failed".to_string());
    }
    let exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=trace".to_string(),
            "--source-agent=mastra-bridge".to_string(),
            format!("--target-agent={}", clean_token(Some(intent), "workflow")),
            format!("--intent={}", clean_text(Some(intent), 80)),
            format!("--message={}", clean_text(Some(message), 160)),
            format!("--trace-id={trace_id}"),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if exit != 0 {
        return Err("mastra_observability_trace_failed".to_string());
    }
    Ok(())
}

fn ensure_session_for_task(
    root: &Path,
    swarm_state_path: &Path,
    task: &str,
    label: &str,
    role: Option<&str>,
    parent_session_id: Option<&str>,
    max_tokens: u64,
) -> Result<String, String> {
    let mut args = vec![
        "spawn".to_string(),
        format!("--task={task}"),
        format!("--agent-label={label}"),
        format!("--max-tokens={max_tokens}"),
        format!("--state-path={}", swarm_state_path.display()),
    ];
    if let Some(role) = role {
        args.push(format!("--role={role}"));
    }
    if let Some(parent) = parent_session_id {
        args.push(format!("--session-id={parent}"));
    }
    let exit = crate::swarm_runtime::run(root, &args);
    if exit != 0 {
        return Err(format!("mastra_swarm_spawn_failed:{label}"));
    }
    let swarm_state = read_swarm_state(swarm_state_path);
    find_swarm_session_id_by_task(&swarm_state, task)
        .ok_or_else(|| format!("mastra_swarm_session_missing:{label}"))
}

fn register_runtime_bridge(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "mastra-runtime",
    );
    let language = clean_token(payload.get("language").and_then(Value::as_str), "python");
    if !allowed_language(&language) {
        return Err("mastra_runtime_language_invalid".to_string());
    }
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/polyglot/mastra_runtime_bridge.ts"),
    )?;
    if !bridge_path.starts_with("adapters/") {
        return Err("mastra_runtime_bridge_must_be_adapter_owned".to_string());
    }
    let supported_profiles = parse_string_list(payload.get("supported_profiles"));
    let record = json!({
        "bridge_id": stable_id("mastrart", &json!({"name": name, "language": language, "bridge_path": bridge_path})),
        "name": name,
        "language": language,
        "provider": clean_token(payload.get("provider").and_then(Value::as_str), "openai-compatible"),
        "model_family": clean_token(payload.get("model_family").and_then(Value::as_str), "gemini"),
        "models": payload.get("models").cloned().unwrap_or_else(|| json!([])),
        "supported_profiles": supported_profiles,
        "bridge_path": bridge_path,
        "registered_at": now_iso(),
    });
    let bridge_id = record
        .get("bridge_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "runtime_bridges").insert(bridge_id, record.clone());
    Ok(json!({
        "ok": true,
        "runtime_bridge": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.7", mastra_claim("V6-WORKFLOW-011.7")),
    }))
}

fn select_runtime_bridge<'a>(
    bridges: &'a Map<String, Value>,
    bridge_id: &str,
    language: &str,
    provider: &str,
) -> Result<&'a Value, String> {
    if !bridge_id.is_empty() {
        return bridges
            .get(bridge_id)
            .ok_or_else(|| format!("unknown_mastra_runtime_bridge:{bridge_id}"));
    }
    bridges
        .values()
        .find(|row| {
            let language_match = row.get("language").and_then(Value::as_str) == Some(language);
            let provider_match = provider.is_empty()
                || row.get("provider").and_then(Value::as_str) == Some(provider);
            language_match && provider_match
        })
        .ok_or_else(|| format!("mastra_runtime_bridge_not_found:{language}:{provider}"))
}

fn route_model(state: &Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let bridge_id = clean_token(payload.get("bridge_id").and_then(Value::as_str), "");
    let language = clean_token(payload.get("language").and_then(Value::as_str), "python");
    let provider = clean_token(
        payload.get("provider").and_then(Value::as_str),
        "openai-compatible",
    );
    let model = clean_token(
        payload.get("model").and_then(Value::as_str),
        "gemini-2.0-flash",
    );
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let bridges = state
        .get("runtime_bridges")
        .and_then(Value::as_object)
        .ok_or_else(|| "mastra_runtime_bridges_missing".to_string())?;
    let bridge = select_runtime_bridge(bridges, &bridge_id, &language, &provider)?;
    let supported_profiles = parse_string_list(bridge.get("supported_profiles"));
    if !profile_supported(&supported_profiles, &profile) {
        return Err(format!("mastra_runtime_profile_unsupported:{profile}"));
    }
    let polyglot_requires_rich = matches!(language.as_str(), "python" | "go" | "java")
        && matches!(profile.as_str(), "pure" | "tiny-max");
    Ok(json!({
        "ok": true,
        "route": {
            "bridge_id": bridge.get("bridge_id").cloned().unwrap_or(Value::Null),
            "bridge_path": bridge.get("bridge_path").cloned().unwrap_or(Value::Null),
            "language": language,
            "provider": provider,
            "model": model,
            "profile": profile,
            "degraded": polyglot_requires_rich,
            "reason_code": if polyglot_requires_rich { "polyglot_runtime_requires_rich_profile" } else { "route_ok" },
            "invocation_mode": "adapter_owned_runtime_bridge"
        },
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.7", mastra_claim("V6-WORKFLOW-011.7")),
    }))
}

fn snapshot_record(state: &mut Value, session_id: &str, payload: Value) {
    as_object_mut(state, "run_snapshots").insert(session_id.to_string(), payload);
}

fn run_llm_agent(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "mastra-llm-agent",
    );
    let instruction = clean_text(payload.get("instruction").and_then(Value::as_str), 240);
    if instruction.is_empty() {
        return Err("mastra_llm_agent_instruction_required".to_string());
    }
    let mode = clean_token(payload.get("mode").and_then(Value::as_str), "sequential");
    if !allowed_workflow_mode(&mode) {
        return Err("mastra_llm_agent_mode_invalid".to_string());
    }
    let steps = payload
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if steps.is_empty() {
        return Err("mastra_llm_agent_steps_required".to_string());
    }
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let route = route_model(
        state,
        &Map::from_iter([
            (
                "bridge_id".to_string(),
                payload
                    .get("runtime_bridge_id")
                    .cloned()
                    .unwrap_or(Value::String(String::new())),
            ),
            (
                "language".to_string(),
                payload
                    .get("language")
                    .cloned()
                    .unwrap_or_else(|| json!("python")),
            ),
            (
                "provider".to_string(),
                payload
                    .get("provider")
                    .cloned()
                    .unwrap_or_else(|| json!("openai-compatible")),
            ),
            (
                "model".to_string(),
                payload
                    .get("model")
                    .cloned()
                    .unwrap_or_else(|| json!("gemini-2.0-flash")),
            ),
            ("profile".to_string(), json!(profile.clone())),
        ]),
    )?;
    let primary_task = format!("mastra:llm:{}:{}", name, instruction);
    let primary_session_id = ensure_session_for_task(
        root,
        &swarm_state_path,
        &primary_task,
        &name,
        Some("llm-agent"),
        None,
        parse_u64_value(payload.get("budget"), 640, 64, 8192),
    )?;

    let mut step_reports = Vec::new();
    let mut child_sessions = Vec::new();
    match mode.as_str() {
        "sequential" => {
            for (idx, step) in steps.iter().enumerate() {
                let step_id = clean_token(
                    step.get("id").and_then(Value::as_str),
                    &format!("step-{}", idx + 1),
                );
                step_reports.push(json!({
                    "step_id": step_id,
                    "mode": "sequential",
                    "budget": parse_u64_value(step.get("budget"), 96, 16, 2048),
                }));
            }
        }
        "parallel" => {
            for (idx, step) in steps.iter().enumerate() {
                let step_id = clean_token(
                    step.get("id").and_then(Value::as_str),
                    &format!("parallel-{}", idx + 1),
                );
                let task = format!("mastra:parallel:{name}:{step_id}");
                let child = ensure_session_for_task(
                    root,
                    &swarm_state_path,
                    &task,
                    &step_id,
                    Some("llm-worker"),
                    Some(&primary_session_id),
                    parse_u64_value(step.get("budget"), 128, 16, 2048),
                )?;
                child_sessions.push(child.clone());
                step_reports
                    .push(json!({"step_id": step_id, "mode": "parallel", "session_id": child}));
            }
        }
        "loop" => {
            let max_iterations = parse_u64_value(payload.get("max_iterations"), 2, 1, 6);
            for iter in 0..max_iterations {
                for (idx, step) in steps.iter().enumerate() {
                    let step_id = clean_token(
                        step.get("id").and_then(Value::as_str),
                        &format!("loop-{}", idx + 1),
                    );
                    step_reports.push(json!({
                        "step_id": step_id,
                        "mode": "loop",
                        "iteration": iter + 1,
                        "budget": parse_u64_value(step.get("budget"), 64, 16, 1024),
                    }));
                }
            }
        }
        _ => unreachable!(),
    }

    let agent = json!({
        "agent_id": stable_id("mastraagent", &json!({"name": name, "instruction": instruction, "mode": mode})),
        "name": name,
        "instruction": instruction,
        "mode": mode,
        "profile": profile,
        "route": route.get("route").cloned().unwrap_or(Value::Null),
        "primary_session_id": primary_session_id,
        "child_sessions": child_sessions,
        "steps": step_reports,
        "executed_at": now_iso(),
    });
    let agent_id = agent
        .get("agent_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    snapshot_record(
        state,
        agent
            .get("primary_session_id")
            .and_then(Value::as_str)
            .unwrap_or("mastra-session"),
        json!({
            "snapshot_id": stable_id("mastrasnap", &json!({"agent_id": agent_id})),
            "agent_id": agent_id,
            "context_payload": {"instruction": instruction, "mode": mode, "profile": profile},
            "route": route.get("route").cloned().unwrap_or(Value::Null),
            "recorded_at": now_iso(),
        }),
    );
    as_object_mut(state, "agent_loops").insert(agent_id, agent.clone());
    Ok(json!({
        "ok": true,
        "agent": agent,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.2", mastra_claim("V6-WORKFLOW-011.2")),
    }))
}

fn register_graph(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(payload.get("name").and_then(Value::as_str), "mastra-graph");
    let nodes = payload
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if nodes.is_empty() {
        return Err("mastra_graph_nodes_required".to_string());
    }
    let edges = payload.get("edges").cloned().unwrap_or_else(|| json!([]));
    let record = json!({
        "graph_id": stable_id("mastragraph", &json!({"name": name, "nodes": nodes})),
        "name": name,
        "nodes": nodes,
        "edges": edges,
        "entrypoint": clean_token(payload.get("entrypoint").and_then(Value::as_str), "start"),
        "parallel_branches": payload
            .get("nodes")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().filter(|row| parse_bool_value(row.get("parallel"), false)).count())
            .unwrap_or(0),
        "registered_at": now_iso(),
    });
    let graph_id = record
        .get("graph_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "graphs").insert(graph_id, record.clone());
    Ok(json!({
        "ok": true,
        "graph": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.1", mastra_claim("V6-WORKFLOW-011.1")),
    }))
}

fn execute_graph(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let graph_id = clean_token(payload.get("graph_id").and_then(Value::as_str), "");
    if graph_id.is_empty() {
        return Err("mastra_execute_graph_id_required".to_string());
    }
    let graph = state
        .get("graphs")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&graph_id))
        .cloned()
        .ok_or_else(|| format!("unknown_mastra_graph:{graph_id}"))?;
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let name = clean_token(graph.get("name").and_then(Value::as_str), "mastra-graph");
    let swarm_state = swarm_state_path(root, argv, payload);
    let root_session_id = ensure_session_for_task(
        root,
        &swarm_state,
        &format!("mastra:graph:{name}"),
        &name,
        Some("coordinator"),
        None,
        parse_u64_value(payload.get("budget"), 768, 64, 8192),
    )?;
    let nodes = graph
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let low_profile = matches!(profile.as_str(), "pure" | "tiny-max");
    let mut child_sessions = Vec::new();
    let mut node_reports = Vec::new();
    let mut consumed_parallel = false;
    for (idx, node) in nodes.iter().enumerate() {
        let obj = node
            .as_object()
            .ok_or_else(|| "mastra_graph_node_object_required".to_string())?;
        let node_id = clean_token(
            obj.get("id").and_then(Value::as_str),
            &format!("node-{}", idx + 1),
        );
        let is_parallel = parse_bool_value(obj.get("parallel"), false);
        let selected = !(low_profile && is_parallel && consumed_parallel);
        if is_parallel && selected {
            consumed_parallel = true;
        }
        let session_id = if selected && (is_parallel || parse_bool_value(obj.get("spawn"), false)) {
            child_sessions.push(root_session_id.clone());
            Some(root_session_id.clone())
        } else {
            None
        };
        node_reports.push(json!({
            "node_id": node_id,
            "parallel": is_parallel,
            "selected": selected,
            "branch": clean_token(obj.get("branch").and_then(Value::as_str), "default"),
            "session_id": session_id,
        }));
    }
    let run = json!({
        "run_id": stable_id("mastragraphrun", &json!({"graph_id": graph_id, "profile": profile})),
        "graph_id": graph_id,
        "profile": profile,
        "root_session_id": root_session_id,
        "child_sessions": child_sessions,
        "nodes": node_reports,
        "degraded": low_profile && graph.get("parallel_branches").and_then(Value::as_u64).unwrap_or(0) > 1,
        "reason_code": if low_profile && graph.get("parallel_branches").and_then(Value::as_u64).unwrap_or(0) > 1 {
            "graph_parallelism_profile_limited"
        } else {
            "graph_execution_ok"
        },
        "executed_at": now_iso(),
    });
    let run_id = run
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    snapshot_record(
        state,
        &root_session_id,
        json!({
            "snapshot_id": stable_id("mastragraphsnap", &json!({"run_id": run_id})),
            "run_id": run_id,
            "context_payload": {"graph_id": graph_id, "profile": profile},
            "recorded_at": now_iso(),
        }),
    );
    as_object_mut(state, "graph_runs").insert(run_id, run.clone());
    Ok(json!({
        "ok": true,
        "run": run,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.1", mastra_claim("V6-WORKFLOW-011.1")),
    }))
}

fn run_agent_loop(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let mut llm_payload = payload.clone();
    if llm_payload
        .get("instruction")
        .and_then(Value::as_str)
        .is_none()
    {
        return Err("mastra_agent_loop_instruction_required".to_string());
    }
    if llm_payload.get("steps").is_none() {
        let tool_rows = payload
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let synthesized = if tool_rows.is_empty() {
            vec![json!({"id": "reason", "budget": 96})]
        } else {
            tool_rows
                .iter()
                .enumerate()
                .map(|(idx, row)| {
                    json!({
                        "id": clean_token(row.get("tool_id").and_then(Value::as_str), &format!("tool-{}", idx + 1)),
                        "budget": parse_u64_value(row.get("budget"), 96, 16, 2048),
                    })
                })
                .collect::<Vec<_>>()
        };
        llm_payload.insert("steps".to_string(), json!(synthesized));
    }
    llm_payload
        .entry("mode".to_string())
        .or_insert_with(|| json!("loop"));
    llm_payload
        .entry("max_iterations".to_string())
        .or_insert_with(|| json!(parse_u64_value(payload.get("max_iterations"), 2, 1, 6)));
    let tool_ids = payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            row.get("tool_id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();
    let mut result = run_llm_agent(root, argv, state, &llm_payload)?;
    if let Some(agent) = result.get_mut("agent").and_then(Value::as_object_mut) {
        agent.insert("selected_tools".to_string(), json!(tool_ids));
        agent.insert(
            "reasoning_mode".to_string(),
            json!(clean_token(
                payload.get("reasoning_mode").and_then(Value::as_str),
                "bounded"
            )),
        );
    }
    Ok(result)
}

fn memory_recall(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let query = clean_text(payload.get("query").and_then(Value::as_str), 240);
    if query.is_empty() {
        return Err("mastra_memory_query_required".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let top = parse_u64_value(payload.get("top"), 5, 1, 25);
    let ambient_exit = crate::memory_ambient::run(
        root,
        &[
            "run".to_string(),
            "--memory-command=recall".to_string(),
            format!("--memory-arg=--query={query}"),
            format!("--memory-arg=--limit={top}"),
            "--run-context=mastra".to_string(),
        ],
    );
    let ambient_path = root.join("local/state/client/memory/ambient/latest.json");
    let ambient_receipt = lane_utils::read_json(&ambient_path).unwrap_or_else(|| json!({}));
    let degraded = ambient_exit != 0 || (matches!(profile.as_str(), "tiny-max") && top > 3);
    let record = json!({
        "recall_id": stable_id("mastrarecall", &json!({"query": query, "top": top, "profile": profile})),
        "query": query,
        "top": top,
        "profile": profile,
        "degraded": degraded,
        "reason_code": if ambient_exit != 0 {
            "memory_runtime_unavailable_degraded"
        } else if matches!(profile.as_str(), "tiny-max") && top > 3 {
            "memory_recall_top_trimmed_for_profile"
        } else {
            "memory_recall_ok"
        },
        "ambient_exit_code": ambient_exit,
        "ambient_receipt_path": rel(root, &ambient_path),
        "ambient_receipt": ambient_receipt,
        "recalled_at": now_iso(),
    });
    let recall_id = record
        .get("recall_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "memory_recalls").insert(recall_id, record.clone());
    Ok(json!({
        "ok": true,
        "memory_recall": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.3", mastra_claim("V6-WORKFLOW-011.3")),
    }))
}

fn register_mcp_bridge(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "mastra-mcp-bridge",
    );
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/mastra_mcp_bridge.ts"),
    )?;
    let record = json!({
        "tool_id": stable_id("mastramcp", &json!({"name": name, "bridge_path": bridge_path})),
        "name": name,
        "kind": "mcp",
        "bridge_path": bridge_path,
        "entrypoint": clean_token(payload.get("entrypoint").and_then(Value::as_str), "invoke"),
        "requires_approval": parse_bool_value(payload.get("requires_approval"), false),
        "supported_profiles": parse_string_list(payload.get("supported_profiles")),
        "capabilities": payload.get("capabilities").cloned().unwrap_or_else(|| json!(["tools", "resources"])),
        "registered_at": now_iso(),
        "invocation_count": 0,
        "fail_closed": true,
    });
    let tool_id = record
        .get("tool_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "mcp_bridges").insert(tool_id, record.clone());
    Ok(json!({
        "ok": true,
        "mcp_bridge": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.5", mastra_claim("V6-WORKFLOW-011.5")),
    }))
}

fn invoke_mcp_bridge(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let bridge_id = clean_token(
        payload
            .get("bridge_id")
            .and_then(Value::as_str)
            .or_else(|| payload.get("tool_id").and_then(Value::as_str)),
        "",
    );
    if bridge_id.is_empty() {
        return Err("mastra_mcp_bridge_id_required".to_string());
    }
    let mut invoke_payload = payload.clone();
    invoke_payload.insert("tool_id".to_string(), json!(bridge_id));
    let out = invoke_tool_manifest(root, argv, state, &invoke_payload)?;
    Ok(json!({
        "ok": true,
        "mcp_invocation": out.get("invocation").cloned().unwrap_or(Value::Null),
        "tool_id": out.get("tool_id").cloned().unwrap_or(Value::Null),
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.5", mastra_claim("V6-WORKFLOW-011.5")),
    }))
}

fn find_run_record(state: &Value, run_id: &str) -> Option<Value> {
    for key in ["graph_runs", "agent_loops"] {
        if let Some(row) = state
            .get(key)
            .and_then(Value::as_object)
            .and_then(|rows| rows.get(run_id))
        {
            return Some(row.clone());
        }
    }
    None
}

fn run_session_id(run: &Value) -> String {
    clean_token(
        run.get("root_session_id")
            .and_then(Value::as_str)
            .or_else(|| run.get("primary_session_id").and_then(Value::as_str)),
        "",
    )
}

fn suspend_run(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let run_id = clean_token(payload.get("run_id").and_then(Value::as_str), "");
    if run_id.is_empty() {
        return Err("mastra_suspend_run_id_required".to_string());
    }
    let run =
        find_run_record(state, &run_id).ok_or_else(|| format!("unknown_mastra_run:{run_id}"))?;
    let session_id = run_session_id(&run);
    let mut approval: Option<Value> = None;
    if parse_bool_value(payload.get("require_approval"), true) {
        let approval_out = approval_checkpoint(
            root,
            argv,
            state,
            &Map::from_iter([
                (
                    "summary".to_string(),
                    json!(clean_text(
                        payload.get("summary").and_then(Value::as_str),
                        200
                    )),
                ),
                (
                    "reason".to_string(),
                    json!(clean_text(
                        payload.get("reason").and_then(Value::as_str),
                        200
                    )),
                ),
                (
                    "action_id".to_string(),
                    payload.get("action_id").cloned().unwrap_or(Value::Null),
                ),
                (
                    "decision".to_string(),
                    payload.get("decision").cloned().unwrap_or(Value::Null),
                ),
            ]),
        )?;
        approval = approval_out.get("approval").cloned();
    }
    let record = json!({
        "run_id": run_id,
        "session_id": session_id,
        "resume_token": stable_id("mastraresume", &run),
        "approval": approval,
        "status": "suspended",
        "suspended_at": now_iso(),
    });
    snapshot_record(
        state,
        &session_id,
        json!({
            "snapshot_id": stable_id("mastrasuspend", &record),
            "run_id": run_id,
            "context_payload": {"status": "suspended"},
            "recorded_at": now_iso(),
        }),
    );
    as_object_mut(state, "suspended_runs")
        .insert(clean_token(Some(&run_id), &run_id), record.clone());
    Ok(json!({
        "ok": true,
        "suspension": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.4", mastra_claim("V6-WORKFLOW-011.4")),
    }))
}

fn resume_run(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let run_id = clean_token(payload.get("run_id").and_then(Value::as_str), "");
    if run_id.is_empty() {
        return Err("mastra_resume_run_id_required".to_string());
    }
    let record = as_object_mut(state, "suspended_runs")
        .get_mut(&run_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("unknown_mastra_suspended_run:{run_id}"))?;
    if let Some(action_id) = record
        .get("approval")
        .and_then(|value| value.get("action_id"))
        .and_then(Value::as_str)
    {
        let queue_path = approval_queue_path(root, argv, payload);
        if !approval_is_approved(&queue_path, action_id) {
            return Err("mastra_resume_requires_approved_checkpoint".to_string());
        }
    }
    let session_id = clean_token(record.get("session_id").and_then(Value::as_str), "");
    if !session_id.is_empty() {
        let swarm_state = swarm_state_path(root, argv, payload);
        let context_json = encode_json_arg(&json!({"status": "resumed", "run_id": run_id}))?;
        let exit = crate::swarm_runtime::run(
            root,
            &[
                "sessions".to_string(),
                "context-put".to_string(),
                format!("--session-id={session_id}"),
                format!("--context-json={context_json}"),
                "--merge=1".to_string(),
                format!("--state-path={}", swarm_state.display()),
            ],
        );
        if exit != 0 {
            return Err("mastra_resume_context_restore_failed".to_string());
        }
    }
    record.insert("status".to_string(), json!("resumed"));
    record.insert("resumed_at".to_string(), json!(now_iso()));
    Ok(json!({
        "ok": true,
        "resume": Value::Object(record.clone()),
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.4", mastra_claim("V6-WORKFLOW-011.4")),
    }))
}

fn record_eval_trace(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let base = record_evaluation(root, state, payload)?;
    let mut evaluation = base.get("evaluation").cloned().unwrap_or_else(|| json!({}));
    let evaluation_id = clean_token(
        evaluation.get("evaluation_id").and_then(Value::as_str),
        "mastra-eval",
    );
    if let Some(obj) = evaluation.as_object_mut() {
        obj.insert(
            "trace".to_string(),
            payload.get("trace").cloned().unwrap_or_else(|| json!([])),
        );
        obj.insert(
            "token_telemetry".to_string(),
            payload
                .get("token_telemetry")
                .cloned()
                .unwrap_or_else(|| json!({"prompt_tokens": 0, "completion_tokens": 0})),
        );
        obj.insert(
            "log_summary".to_string(),
            json!(clean_text(
                payload.get("log_summary").and_then(Value::as_str),
                240
            )),
        );
    }
    as_object_mut(state, "eval_traces").insert(evaluation_id.clone(), evaluation.clone());
    emit_native_trace(
        root,
        &evaluation_id,
        "mastra_eval_trace",
        &format!(
            "session_id={} score={:.2}",
            clean_token(
                payload.get("session_id").and_then(Value::as_str),
                "mastra-session"
            ),
            evaluation
                .get("score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
    )?;
    Ok(json!({
        "ok": true,
        "evaluation": evaluation,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.6", mastra_claim("V6-WORKFLOW-011.6")),
    }))
}

fn scaffold_intake(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let output_dir = normalize_shell_path(
        root,
        payload
            .get("output_dir")
            .and_then(Value::as_str)
            .unwrap_or("apps/mastra-studio"),
    )?;
    let abs_dir = repo_path(root, &output_dir);
    let src_dir = abs_dir.join("src");
    std::fs::create_dir_all(&src_dir)
        .map_err(|err| format!("mastra_scaffold_create_dir_failed:{err}"))?;
    let package_name = clean_token(
        payload.get("package_name").and_then(Value::as_str),
        "mastra-assimilation-shell",
    );
    std::fs::write(
        abs_dir.join("package.json"),
        format!(
            "{{\n  \"name\": \"{package_name}\",\n  \"private\": true,\n  \"version\": \"0.1.0\",\n  \"scripts\": {{\n    \"bridge\": \"node ../../client/runtime/systems/workflow/mastra_bridge.ts\"\n  }}\n}}\n"
        ),
    )
    .map_err(|err| format!("mastra_scaffold_package_write_failed:{err}"))?;
    std::fs::write(
        src_dir.join("mastra.graph.ts"),
        "export const mastraGraph = {\n  name: 'mastra-assimilated-graph',\n  bridge: 'client/runtime/systems/workflow/mastra_bridge.ts',\n  steps: [{ id: 'reason', budget: 96 }],\n};\n",
    )
    .map_err(|err| format!("mastra_scaffold_graph_write_failed:{err}"))?;
    std::fs::write(
        abs_dir.join("README.md"),
        "# Mastra Assimilated Shell\n\nThis shell is non-authoritative. All execution delegates to `core://mastra-bridge`.\n",
    )
    .map_err(|err| format!("mastra_scaffold_readme_write_failed:{err}"))?;
    let record = json!({
        "intake_id": stable_id("mastraintake", &json!({"output_dir": output_dir, "package_name": package_name})),
        "output_dir": output_dir,
        "files": [
            format!("{}/package.json", rel(root, &abs_dir)),
            format!("{}/src/mastra.graph.ts", rel(root, &abs_dir)),
            format!("{}/README.md", rel(root, &abs_dir)),
        ],
        "created_at": now_iso(),
    });
    let intake_id = record
        .get("intake_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "intakes").insert(intake_id, record.clone());
    Ok(json!({
        "ok": true,
        "intake": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.9", mastra_claim("V6-WORKFLOW-011.9")),
    }))
}

fn register_tool_manifest(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(payload.get("name").and_then(Value::as_str), "mastra-tool");
    let kind = clean_token(payload.get("kind").and_then(Value::as_str), "custom");
    if !allowed_tool_kind(&kind) {
        return Err("mastra_tool_kind_invalid".to_string());
    }
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/mastra_mcp_bridge.ts"),
    )?;
    let supported_profiles = parse_string_list(payload.get("supported_profiles"));
    let openapi_url = clean_text(payload.get("openapi_url").and_then(Value::as_str), 200);
    if kind == "openapi"
        && !(openapi_url.starts_with("https://") || openapi_url.ends_with("openapi.json"))
    {
        return Err("mastra_tool_openapi_url_invalid".to_string());
    }
    if kind == "mcp" {
        let exit = crate::mcp_plane::run(
            root,
            &[
                "capability-matrix".to_string(),
                "--server-capabilities=tools,resources,prompts".to_string(),
                "--strict=1".to_string(),
            ],
        );
        if exit != 0 {
            return Err("mastra_tool_mcp_capability_validation_failed".to_string());
        }
    }
    let record = json!({
        "tool_id": stable_id("mastratool", &json!({"name": name, "kind": kind, "bridge_path": bridge_path})),
        "name": name,
        "kind": kind,
        "bridge_path": bridge_path,
        "entrypoint": clean_token(payload.get("entrypoint").and_then(Value::as_str), "invoke"),
        "openapi_url": openapi_url,
        "requires_approval": parse_bool_value(payload.get("requires_approval"), false),
        "supported_profiles": supported_profiles,
        "schema": payload.get("schema").cloned().unwrap_or(Value::Null),
        "capabilities": payload.get("capabilities").cloned().unwrap_or_else(|| json!([])),
        "registered_at": now_iso(),
        "invocation_count": 0,
        "fail_closed": true,
    });
    let tool_id = record
        .get("tool_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "mcp_bridges").insert(tool_id, record.clone());
    Ok(json!({
        "ok": true,
        "tool": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.5", mastra_claim("V6-WORKFLOW-011.5")),
    }))
}

fn invoke_tool_manifest(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let tool_id = clean_token(payload.get("tool_id").and_then(Value::as_str), "");
    if tool_id.is_empty() {
        return Err("mastra_tool_id_required".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let queue_path = approval_queue_path(root, argv, payload);
    let tools = as_object_mut(state, "mcp_bridges");
    let tool = tools
        .get_mut(&tool_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("unknown_mastra_tool:{tool_id}"))?;
    let supported_profiles = parse_string_list(tool.get("supported_profiles"));
    if !profile_supported(&supported_profiles, &profile) {
        return Err(format!("mastra_tool_profile_unsupported:{profile}"));
    }
    let requires_approval = parse_bool_value(tool.get("requires_approval"), false)
        || parse_bool_value(payload.get("requires_approval"), false);
    if requires_approval {
        let approval_action_id = clean_token(
            payload.get("approval_action_id").and_then(Value::as_str),
            "",
        );
        if approval_action_id.is_empty() {
            return Err("mastra_tool_requires_approval".to_string());
        }
        if !approval_is_approved(&queue_path, &approval_action_id) {
            return Err("mastra_tool_approval_not_granted".to_string());
        }
    }
    let kind = clean_token(tool.get("kind").and_then(Value::as_str), "custom");
    let args = payload.get("args").cloned().unwrap_or_else(|| json!({}));
    let invocation = match kind.as_str() {
        "openapi" => json!({
            "mode": "openapi_request",
            "target": tool.get("openapi_url").cloned().unwrap_or(Value::Null),
            "method": payload.get("method").cloned().unwrap_or_else(|| json!("POST")),
            "path": payload.get("path").cloned().unwrap_or_else(|| json!("/invoke")),
            "body": args,
        }),
        "mcp" => json!({
            "mode": "mcp_tool_call",
            "tool": tool.get("name").cloned().unwrap_or_else(|| json!("tool")),
            "arguments": args,
        }),
        "native" => json!({
            "mode": "native_call",
            "entrypoint": tool.get("entrypoint").cloned().unwrap_or_else(|| json!("invoke")),
            "arguments": args,
        }),
        _ => json!({
            "mode": "custom_function",
            "entrypoint": tool.get("entrypoint").cloned().unwrap_or_else(|| json!("invoke")),
            "arguments": args,
        }),
    };
    let invocation_count = tool
        .get("invocation_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(1);
    tool.insert("invocation_count".to_string(), json!(invocation_count));
    tool.insert("last_invoked_at".to_string(), json!(now_iso()));
    Ok(json!({
        "ok": true,
        "tool_id": tool_id,
        "invocation": invocation,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.5", mastra_claim("V6-WORKFLOW-011.5")),
    }))
}

fn approval_checkpoint(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let queue_path = approval_queue_path(root, argv, payload);
    let action_id = clean_token(payload.get("action_id").and_then(Value::as_str), "");
    let decision = clean_token(payload.get("decision").and_then(Value::as_str), "pending");
    let result = if action_id.is_empty() || decision == "pending" {
        let new_action_id = if action_id.is_empty() {
            stable_id(
                "mastraapproval",
                &json!({
                    "tool_id": payload.get("tool_id"),
                    "summary": payload.get("summary"),
                    "risk": payload.get("risk")
                }),
            )
        } else {
            action_id.clone()
        };
        let action_envelope = json!({
            "action_id": new_action_id,
            "directive_id": "mastra-bridge",
            "type": "tool_invocation",
            "summary": clean_text(payload.get("summary").and_then(Value::as_str), 200),
            "payload_pointer": clean_text(payload.get("tool_id").and_then(Value::as_str), 160),
        });
        let queue_payload = json!({
            "action_envelope": action_envelope,
            "reason": clean_text(payload.get("reason").and_then(Value::as_str), 200),
        });
        let encoded = BASE64_STANDARD.encode(encode_json_arg(&queue_payload)?.as_bytes());
        let exit = crate::approval_gate_kernel::run(
            root,
            &[
                "queue".to_string(),
                format!("--payload-base64={encoded}"),
                format!("--queue-path={}", queue_path.display()),
            ],
        );
        if exit != 0 {
            return Err("mastra_approval_queue_failed".to_string());
        }
        json!({
            "action_id": new_action_id,
            "decision": "pending",
            "status": approval_status_from_queue(&queue_path, &new_action_id),
        })
    } else {
        let args = if decision == "approve" {
            vec![
                "approve".to_string(),
                format!("--action-id={action_id}"),
                format!("--queue-path={}", queue_path.display()),
            ]
        } else {
            vec![
                "deny".to_string(),
                format!("--action-id={action_id}"),
                format!(
                    "--reason={}",
                    clean_text(payload.get("reason").and_then(Value::as_str), 120)
                ),
                format!("--queue-path={}", queue_path.display()),
            ]
        };
        let exit = crate::approval_gate_kernel::run(root, &args);
        if exit != 0 {
            return Err(format!("mastra_approval_{}_failed", decision));
        }
        json!({
            "action_id": action_id,
            "decision": decision,
            "status": approval_status_from_queue(&queue_path, &action_id),
        })
    };
    let action_id = result
        .get("action_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "approval_records").insert(
        action_id.clone(),
        json!({
            "action_id": action_id,
            "tool_id": payload.get("tool_id").cloned().unwrap_or(Value::Null),
            "queue_path": rel(root, &queue_path),
            "status": result.get("status").cloned().unwrap_or(Value::Null),
            "updated_at": now_iso(),
        }),
    );
    Ok(json!({
        "ok": true,
        "approval": result,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.4", mastra_claim("V6-WORKFLOW-011.4")),
    }))
}

fn rewind_session(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let session_id = clean_token(payload.get("session_id").and_then(Value::as_str), "");
    if session_id.is_empty() {
        return Err("mastra_rewind_session_id_required".to_string());
    }
    let snapshot = state
        .get("run_snapshots")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&session_id))
        .cloned()
        .ok_or_else(|| format!("mastra_snapshot_missing:{session_id}"))?;
    let context_payload = snapshot
        .get("context_payload")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let context_json = encode_json_arg(&context_payload)?;
    let exit = crate::swarm_runtime::run(
        root,
        &[
            "sessions".to_string(),
            "context-put".to_string(),
            format!("--session-id={session_id}"),
            format!("--context-json={context_json}"),
            "--merge=0".to_string(),
            format!("--state-path={}", swarm_state_path.display()),
        ],
    );
    if exit != 0 {
        return Err("mastra_rewind_context_restore_failed".to_string());
    }
    emit_native_trace(
        root,
        &clean_token(
            snapshot.get("snapshot_id").and_then(Value::as_str),
            "mastra-rewind",
        ),
        "mastra_rewind",
        &format!("rewound session {session_id}"),
    )?;
    Ok(json!({
        "ok": true,
        "restored": {
            "session_id": session_id,
            "snapshot": snapshot,
            "swarm_state_path": rel(root, &swarm_state_path),
        },
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.4", mastra_claim("V6-WORKFLOW-011.4")),
    }))
}

fn record_evaluation(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let session_id = clean_token(
        payload.get("session_id").and_then(Value::as_str),
        "mastra-session",
    );
    let metrics = payload.get("metrics").cloned().unwrap_or_else(|| json!({}));
    let evaluation = json!({
        "evaluation_id": stable_id("mastraeval", &json!({"session_id": session_id, "metrics": metrics})),
        "session_id": session_id,
        "metrics": metrics,
        "score": parse_f64_value(payload.get("score"), 0.0, 0.0, 1.0),
        "profile": clean_token(payload.get("profile").and_then(Value::as_str), "rich"),
        "evaluated_at": now_iso(),
    });
    let evaluation_id = evaluation
        .get("evaluation_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "eval_traces").insert(evaluation_id.clone(), evaluation.clone());
    emit_native_trace(
        root,
        &evaluation_id,
        "mastra_eval",
        &format!(
            "session_id={} score={:.2}",
            session_id,
            evaluation
                .get("score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
    )?;
    Ok(json!({
        "ok": true,
        "evaluation": evaluation,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.6", mastra_claim("V6-WORKFLOW-011.6")),
    }))
}

fn deploy_shell(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let shell_path = normalize_shell_path(
        root,
        payload
            .get("shell_path")
            .and_then(Value::as_str)
            .unwrap_or("client/runtime/systems/workflow/mastra_bridge.ts"),
    )?;
    let target = clean_token(payload.get("target").and_then(Value::as_str), "local");
    let record = json!({
        "deployment_id": stable_id("mastradep", &json!({"shell_path": shell_path, "target": target})),
        "shell_name": clean_token(payload.get("shell_name").and_then(Value::as_str), "mastra-shell"),
        "shell_path": shell_path,
        "target": target,
        "deletable": true,
        "authority_delegate": "core://mastra-bridge",
        "artifact_path": clean_text(payload.get("artifact_path").and_then(Value::as_str), 240),
        "deployed_at": now_iso(),
    });
    let deployment_id = record
        .get("deployment_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "deployments").insert(deployment_id, record.clone());
    Ok(json!({
        "ok": true,
        "deployment": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.8", mastra_claim("V6-WORKFLOW-011.8")),
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].as_str();
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("mastra_bridge_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let state_path = state_path(root, argv, input);
    let history_path = history_path(root, argv, input);
    let mut state = load_state(&state_path);

    let result = match command {
        "status" => Ok(json!({
            "ok": true,
            "state_path": rel(root, &state_path),
            "history_path": rel(root, &history_path),
            "graphs": as_object_mut(&mut state, "graphs").len(),
            "graph_runs": as_object_mut(&mut state, "graph_runs").len(),
            "agent_loops": as_object_mut(&mut state, "agent_loops").len(),
            "memory_recalls": as_object_mut(&mut state, "memory_recalls").len(),
            "suspended_runs": as_object_mut(&mut state, "suspended_runs").len(),
            "mcp_bridges": as_object_mut(&mut state, "mcp_bridges").len(),
            "run_snapshots": as_object_mut(&mut state, "run_snapshots").len(),
            "eval_traces": as_object_mut(&mut state, "eval_traces").len(),
            "deployments": as_object_mut(&mut state, "deployments").len(),
            "runtime_bridges": as_object_mut(&mut state, "runtime_bridges").len(),
            "intakes": as_object_mut(&mut state, "intakes").len(),
            "last_receipt": state.get("last_receipt").cloned().unwrap_or(Value::Null),
        })),
        "register-graph" => register_graph(&mut state, input),
        "execute-graph" => execute_graph(root, argv, &mut state, input),
        "run-agent-loop" => run_agent_loop(root, argv, &mut state, input),
        "memory-recall" => memory_recall(root, &mut state, input),
        "suspend-run" => suspend_run(root, argv, &mut state, input),
        "resume-run" => resume_run(root, argv, &mut state, input),
        "register-mcp-bridge" => register_mcp_bridge(root, &mut state, input),
        "invoke-mcp-bridge" => invoke_mcp_bridge(root, argv, &mut state, input),
        "record-eval-trace" => record_eval_trace(root, &mut state, input),
        "register-runtime-bridge" => register_runtime_bridge(root, &mut state, input),
        "route-model" => route_model(&state, input),
        "deploy-shell" => deploy_shell(root, &mut state, input),
        "scaffold-intake" => scaffold_intake(root, &mut state, input),
        "run-llm-agent" => run_llm_agent(root, argv, &mut state, input),
        "register-tool-manifest" => register_tool_manifest(root, &mut state, input),
        "invoke-tool-manifest" => invoke_tool_manifest(root, argv, &mut state, input),
        "approval-checkpoint" => approval_checkpoint(root, argv, &mut state, input),
        "rewind-session" => rewind_session(root, argv, &mut state, input),
        "record-evaluation" => record_evaluation(root, &mut state, input),
        _ => Err(format!("unknown_mastra_bridge_command:{command}")),
    };

    match result {
        Ok(payload) => {
            let receipt = cli_receipt(
                &format!("mastra_bridge_{}", command.replace('-', "_")),
                payload,
            );
            state["last_receipt"] = receipt.clone();
            if let Err(err) = save_state(&state_path, &state)
                .and_then(|_| append_history(&history_path, &receipt))
            {
                print_json_line(&cli_error("mastra_bridge_error", &err));
                return 1;
            }
            print_json_line(&receipt);
            0
        }
        Err(err) => {
            print_json_line(&cli_error("mastra_bridge_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_bridge_route_degrades_polyglot_in_pure_mode() {
        let mut state = default_state();
        let payload = json!({
            "name": "python-gateway",
            "language": "python",
            "provider": "google",
            "bridge_path": "adapters/polyglot/mastra_runtime_bridge.ts",
            "supported_profiles": ["rich", "pure"]
        });
        let _ = register_runtime_bridge(Path::new("."), &mut state, payload.as_object().unwrap())
            .expect("register");
        let out = route_model(
            &state,
            &Map::from_iter([
                ("language".to_string(), json!("python")),
                ("provider".to_string(), json!("google")),
                ("model".to_string(), json!("gemini-2.0-flash")),
                ("profile".to_string(), json!("pure")),
            ]),
        )
        .expect("route");
        assert_eq!(out["route"]["degraded"].as_bool(), Some(true));
    }
}
