// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "local/state/ops/semantic_kernel_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/semantic_kernel_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/semantic_kernel_bridge/swarm_state.json";

fn usage() {
    println!("semantic-kernel-bridge commands:");
    println!("  protheus-ops semantic-kernel-bridge status [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge register-service [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge register-plugin [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge invoke-plugin [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge collaborate [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge plan [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge register-vector-connector [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge retrieve [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge register-llm-connector [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge route-llm [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge validate-structured-output [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge emit-enterprise-event [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge register-dotnet-bridge [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops semantic-kernel-bridge invoke-dotnet-bridge [--payload-base64=<json>] [--state-path=<path>]");
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
            .map_err(|err| format!("semantic_kernel_bridge_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("semantic_kernel_bridge_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("semantic_kernel_bridge_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("semantic_kernel_bridge_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn is_plain_object(value: &Value) -> bool {
    value.is_object()
}

fn repo_path(root: &Path, rel: &str) -> PathBuf {
    let trimmed = rel.trim();
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
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

fn semantic_swarm_state_path(
    root: &Path,
    argv: &[String],
    payload: &Map<String, Value>,
) -> PathBuf {
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
        "schema_version": "semantic_kernel_bridge_state_v1",
        "services": {},
        "plugins": {},
        "collaborations": {},
        "plans": {},
        "vector_connectors": {},
        "llm_connectors": {},
        "structured_processes": {},
        "enterprise_events": [],
        "dotnet_bridges": {},
        "last_receipt": null,
    })
}

fn ensure_state_shape(value: &mut Value) {
    if !value.is_object() {
        *value = default_state();
        return;
    }
    for key in [
        "services",
        "plugins",
        "collaborations",
        "plans",
        "vector_connectors",
        "llm_connectors",
        "structured_processes",
        "dotnet_bridges",
    ] {
        if !value.get(key).map(is_plain_object).unwrap_or(false) {
            value[key] = json!({});
        }
    }
    if !value
        .get("enterprise_events")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        value["enterprise_events"] = json!([]);
    }
    if value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_none()
    {
        value["schema_version"] = json!("semantic_kernel_bridge_state_v1");
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

fn as_array_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    if !value.get(key).map(Value::is_array).unwrap_or(false) {
        value[key] = json!([]);
    }
    value
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array")
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
    let basis_hash = deterministic_receipt_hash(basis);
    format!(
        "{}_{}_{}",
        prefix,
        to_base36(now_millis()),
        &basis_hash[..12]
    )
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    lane_utils::clean_text(raw, max_len)
}

fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    lane_utils::clean_token(raw, fallback)
}

fn rel(root: &Path, path: &Path) -> String {
    lane_utils::rel_path(root, path)
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

fn normalized_profile(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "tiny" | "tiny-max" | "embedded" => "tiny-max",
        "pure" => "pure",
        _ => "rich",
    }
}

fn approx_token_count(text: &str) -> u64 {
    let words = text.split_whitespace().count() as u64;
    let chars = text.chars().count() as u64;
    words.max(chars / 4).max(1)
}

fn has_token(input: &str, token: &str) -> bool {
    input
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|part| !part.is_empty() && part == token)
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

fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    let candidate = raw.trim();
    if candidate.is_empty() {
        return Err("bridge_path_required".to_string());
    }
    if candidate.contains("..") {
        return Err("unsafe_bridge_path_parent_reference".to_string());
    }
    let abs = repo_path(root, candidate);
    let rel_path = rel(root, &abs);
    if !safe_prefix_for_bridge(&rel_path) {
        return Err("unsupported_bridge_path".to_string());
    }
    Ok(rel_path)
}

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn read_swarm_state(path: &Path) -> Value {
    lane_utils::read_json(path).unwrap_or_else(
        || json!({ "sessions": {}, "handoff_registry": {}, "network_registry": {} }),
    )
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

fn find_swarm_network_id_by_name(state: &Value, name: &str) -> Option<String> {
    state
        .get("network_registry")
        .and_then(Value::as_object)
        .and_then(|rows| {
            rows.iter().find_map(|(network_id, row)| {
                (row.get("name").and_then(Value::as_str) == Some(name)).then(|| network_id.clone())
            })
        })
}

fn find_swarm_network_by_name(state: &Value, name: &str) -> Option<Value> {
    state
        .get("network_registry")
        .and_then(Value::as_object)
        .and_then(|rows| {
            rows.values()
                .find(|row| row.get("name").and_then(Value::as_str) == Some(name))
                .cloned()
        })
}

fn encode_json_arg(value: &Value) -> Result<String, String> {
    serde_json::to_string(value).map_err(|err| format!("json_encode_failed:{err}"))
}

fn semantic_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-008.1" => "kernel_service_registration_is_receipted_over_one_governed_orchestration_surface",
        "V6-WORKFLOW-008.2" => "plugin_assets_normalize_into_governed_manifests_with_fail_closed_invocation",
        "V6-WORKFLOW-008.3" => "semantic_kernel_style_agent_collaboration_reuses_authoritative_swarm_sessions_and_handoffs",
        "V6-WORKFLOW-008.4" => "planner_semantics_compile_into_deterministic_function_selection_receipts",
        "V6-WORKFLOW-008.5" => "vector_connector_retrieval_enforces_context_budget_and_explicit_profile_degradation",
        "V6-WORKFLOW-008.6" => "llm_connector_routes_and_multimodal_paths_are_policy_gated_and_receipted",
        "V6-WORKFLOW-008.7" => "structured_output_and_process_graphs_are_schema_validated_and_receipted",
        "V6-WORKFLOW-008.8" => "enterprise_observability_and_azure_events_emit_native_receipts_without_side_telemetry_stack",
        "V6-WORKFLOW-008.9" => "dotnet_parity_flows_route_through_governed_bridge_receipts",
        _ => "semantic_kernel_bridge_action_emits_deterministic_receipt",
    }
}

fn allowed_service_surface(surface: &str) -> bool {
    matches!(
        surface,
        "workflow-executor"
            | "workflow-controller"
            | "swarm-runtime"
            | "mcp-plane"
            | "policy-runtime-kernel"
    )
}

fn register_service(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "semantic-kernel-service",
    );
    let role = clean_token(payload.get("role").and_then(Value::as_str), "orchestrator");
    let execution_surface = clean_token(
        payload.get("execution_surface").and_then(Value::as_str),
        "workflow-executor",
    );
    if !allowed_service_surface(&execution_surface) {
        return Err("semantic_kernel_service_surface_invalid".to_string());
    }
    let service = json!({
        "service_id": stable_id("sksvc", &json!({"name": name, "role": role, "surface": execution_surface})),
        "name": name,
        "role": role,
        "execution_surface": execution_surface,
        "description": clean_text(payload.get("description").and_then(Value::as_str), 240),
        "default_budget": parse_u64_value(payload.get("default_budget"), 512, 32, 8192),
        "capabilities": payload.get("capabilities").cloned().filter(Value::is_array).unwrap_or_else(|| json!([])),
        "registered_at": now_iso(),
    });
    let service_id = service
        .get("service_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "services").insert(service_id.clone(), service.clone());
    Ok(json!({
        "ok": true,
        "service": service,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.1", semantic_claim("V6-WORKFLOW-008.1")),
    }))
}

fn allowed_plugin_kind(kind: &str) -> bool {
    matches!(kind, "native" | "prompt" | "openapi" | "mcp")
}

fn register_plugin(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let service_id = clean_token(payload.get("service_id").and_then(Value::as_str), "");
    if service_id.is_empty() || !as_object_mut(state, "services").contains_key(&service_id) {
        return Err("semantic_kernel_plugin_service_not_found".to_string());
    }
    let plugin_name = clean_token(
        payload.get("plugin_name").and_then(Value::as_str),
        "semantic-kernel-plugin",
    );
    let plugin_kind = clean_token(payload.get("plugin_kind").and_then(Value::as_str), "native");
    if !allowed_plugin_kind(&plugin_kind) {
        return Err("semantic_kernel_plugin_kind_invalid".to_string());
    }
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/cognition/skills/mcp/mcp_gateway.ts"),
    )?;
    let entrypoint = clean_token(payload.get("entrypoint").and_then(Value::as_str), "invoke");
    let openapi_url = clean_text(payload.get("openapi_url").and_then(Value::as_str), 200);
    if plugin_kind == "openapi"
        && !(openapi_url.starts_with("https://") || openapi_url.ends_with("openapi.json"))
    {
        return Err("semantic_kernel_openapi_url_invalid".to_string());
    }
    let template = clean_text(payload.get("prompt_template").and_then(Value::as_str), 400);
    let plugin = json!({
        "plugin_id": stable_id("skplug", &json!({"service_id": service_id, "plugin_name": plugin_name, "plugin_kind": plugin_kind, "bridge_path": bridge_path})),
        "service_id": service_id,
        "plugin_name": plugin_name,
        "plugin_kind": plugin_kind,
        "bridge_path": bridge_path,
        "entrypoint": entrypoint,
        "openapi_url": openapi_url,
        "prompt_template": template,
        "schema": payload.get("schema").cloned().unwrap_or(Value::Null),
        "registered_at": now_iso(),
        "invocation_count": 0,
        "fail_closed": true,
    });
    let plugin_id = plugin
        .get("plugin_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "plugins").insert(plugin_id.clone(), plugin.clone());
    Ok(json!({
        "ok": true,
        "plugin": plugin,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.2", semantic_claim("V6-WORKFLOW-008.2")),
    }))
}

fn replace_template(template: &str, args: &Map<String, Value>) -> String {
    let mut out = template.to_string();
    for (key, value) in args {
        let replacement = value
            .as_str()
            .map(ToString::to_string)
            .unwrap_or_else(|| value.to_string());
        out = out.replace(&format!("{{{{{key}}}}}"), &replacement);
    }
    out
}

fn invoke_plugin(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let plugin_id = clean_token(payload.get("plugin_id").and_then(Value::as_str), "");
    let args = payload
        .get("args")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if plugin_id.is_empty() {
        return Err("semantic_kernel_plugin_id_required".to_string());
    }
    let plugins = as_object_mut(state, "plugins");
    let plugin = plugins
        .get_mut(&plugin_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "semantic_kernel_plugin_not_found".to_string())?;
    let plugin_kind = plugin
        .get("plugin_kind")
        .and_then(Value::as_str)
        .unwrap_or("native");
    let rendered = if plugin_kind == "prompt" {
        replace_template(
            plugin
                .get("prompt_template")
                .and_then(Value::as_str)
                .unwrap_or(""),
            &args,
        )
    } else {
        String::new()
    };
    let invocation = match plugin_kind {
        "prompt" => json!({
            "mode": "prompt_render",
            "rendered": rendered,
        }),
        "openapi" => json!({
            "mode": "openapi_request",
            "target": plugin.get("openapi_url").cloned().unwrap_or(Value::Null),
            "method": payload.get("method").cloned().unwrap_or_else(|| json!("POST")),
            "path": payload.get("path").cloned().unwrap_or_else(|| json!("/invoke")),
            "body": Value::Object(args.clone()),
        }),
        "mcp" => json!({
            "mode": "mcp_tool_call",
            "tool": payload.get("tool").cloned().unwrap_or_else(|| json!(plugin.get("plugin_name").cloned().unwrap_or_else(|| json!("tool")))),
            "arguments": Value::Object(args.clone()),
        }),
        _ => json!({
            "mode": "native_function",
            "entrypoint": plugin.get("entrypoint").cloned().unwrap_or_else(|| json!("invoke")),
            "arguments": Value::Object(args.clone()),
        }),
    };
    let invocation_count = plugin
        .get("invocation_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(1);
    plugin.insert("invocation_count".to_string(), json!(invocation_count));
    plugin.insert("last_invoked_at".to_string(), json!(now_iso()));
    Ok(json!({
        "ok": true,
        "plugin_id": plugin_id,
        "invocation": invocation,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.2", semantic_claim("V6-WORKFLOW-008.2")),
    }))
}

fn collaborate(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let collaboration_name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "semantic-kernel-collaboration",
    );
    let agents = payload
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if agents.is_empty() {
        return Err("semantic_kernel_collaboration_agents_required".to_string());
    }
    let swarm_state_path = semantic_swarm_state_path(root, argv, payload);
    let mut session_ids = BTreeMap::new();
    let coordinator_task = format!("semantic-kernel:{}:coordinator", collaboration_name);
    let child_budget_sum = agents
        .iter()
        .filter_map(|row| row.get("budget").and_then(Value::as_u64))
        .sum::<u64>();
    let edge_count = payload
        .get("edges")
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0);
    let max_budget = child_budget_sum
        .saturating_add(edge_count.saturating_mul(96))
        .saturating_add(2048)
        .clamp(2048, 16384);
    let coordinator_exit = crate::swarm_runtime::run(
        root,
        &[
            "spawn".to_string(),
            format!("--task={coordinator_task}"),
            format!("--max-tokens={max_budget}"),
            "--agent-label=semantic-kernel-coordinator".to_string(),
            format!("--state-path={}", swarm_state_path.display()),
        ],
    );
    if coordinator_exit != 0 {
        return Err("semantic_kernel_collaboration_coordinator_spawn_failed".to_string());
    }
    let swarm_state = read_swarm_state(&swarm_state_path);
    let coordinator_id = find_swarm_session_id_by_task(&swarm_state, &coordinator_task)
        .ok_or_else(|| "semantic_kernel_collaboration_coordinator_missing".to_string())?;
    session_ids.insert("coordinator".to_string(), coordinator_id.clone());

    let mut node_specs = Vec::new();
    for agent in &agents {
        let label = clean_token(agent.get("label").and_then(Value::as_str), "agent");
        let role = clean_token(agent.get("role").and_then(Value::as_str), "specialist");
        let task = format!(
            "semantic-kernel:{}:{}:{}",
            collaboration_name,
            label,
            clean_text(agent.get("task").and_then(Value::as_str), 80)
        );
        let budget = parse_u64_value(agent.get("budget"), 256, 32, 4096);
        node_specs.push(json!({
            "label": label,
            "role": role,
            "task": task,
            "token_budget": budget,
            "context": {
                "semantic_kernel_collaboration": collaboration_name,
                "semantic_kernel_role": role,
            }
        }));
    }

    let mut edge_specs = Vec::new();
    for edge in payload
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let from = clean_token(edge.get("from").and_then(Value::as_str), "");
        let to = clean_token(edge.get("to").and_then(Value::as_str), "");
        if from.is_empty() || to.is_empty() {
            continue;
        }
        let reason = clean_text(edge.get("reason").and_then(Value::as_str), 120);
        edge_specs.push(json!({
            "from": from,
            "to": to,
            "relation": edge.get("relation").cloned().unwrap_or_else(|| json!("handoff")),
            "importance": parse_f64_value(edge.get("importance"), 0.8, 0.0, 1.0),
            "auto_handoff": true,
            "reason": reason,
            "context": {
                "semantic_kernel_reason": reason,
                "semantic_kernel_collaboration": collaboration_name,
            },
        }));
    }

    let network_name = format!("semantic-kernel-{}", collaboration_name);
    let network_spec = json!({
        "name": network_name,
        "nodes": node_specs,
        "edges": edge_specs,
    });
    let network_exit = crate::swarm_runtime::run(
        root,
        &[
            "networks".to_string(),
            "create".to_string(),
            format!("--session-id={coordinator_id}"),
            format!("--spec-json={}", encode_json_arg(&network_spec)?),
            format!("--state-path={}", swarm_state_path.display()),
        ],
    );
    if network_exit != 0 {
        return Err("semantic_kernel_network_create_failed".to_string());
    }
    let final_swarm_state = read_swarm_state(&swarm_state_path);
    let network_id = find_swarm_network_id_by_name(&final_swarm_state, &network_name)
        .ok_or_else(|| "semantic_kernel_network_missing".to_string())?;
    let network = find_swarm_network_by_name(&final_swarm_state, &network_name)
        .ok_or_else(|| "semantic_kernel_network_receipt_missing".to_string())?;
    if let Some(nodes) = network.get("nodes").and_then(Value::as_array) {
        for node in nodes {
            let Some(label) = node.get("label").and_then(Value::as_str) else {
                continue;
            };
            let Some(session_id) = node.get("session_id").and_then(Value::as_str) else {
                continue;
            };
            session_ids.insert(label.to_string(), session_id.to_string());
        }
    }
    let collaboration = json!({
        "collaboration_id": stable_id("skcollab", &json!({"name": collaboration_name, "network": network_id})),
        "name": collaboration_name,
        "coordinator_session_id": coordinator_id,
        "session_ids": session_ids,
        "swarm_state_path": rel(root, &swarm_state_path),
        "network_id": network_id,
        "registered_at": now_iso(),
    });
    let collaboration_id = collaboration
        .get("collaboration_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "collaborations").insert(collaboration_id, collaboration.clone());
    Ok(json!({
        "ok": true,
        "collaboration": collaboration,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.3", semantic_claim("V6-WORKFLOW-008.3")),
    }))
}

fn parse_function_specs(payload: &Map<String, Value>) -> Vec<Map<String, Value>> {
    payload
        .get("functions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_object().cloned())
        .collect()
}

fn plan(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let objective = clean_text(payload.get("objective").and_then(Value::as_str), 200);
    if objective.is_empty() {
        return Err("semantic_kernel_planner_objective_required".to_string());
    }
    let service_id = clean_token(payload.get("service_id").and_then(Value::as_str), "");
    if service_id.is_empty() || !as_object_mut(state, "services").contains_key(&service_id) {
        return Err("semantic_kernel_planner_service_not_found".to_string());
    }
    let mut functions = parse_function_specs(payload);
    let objective_lc = objective.to_ascii_lowercase();
    functions.sort_by(|a, b| {
        let a_name = clean_token(a.get("name").and_then(Value::as_str), "fn");
        let b_name = clean_token(b.get("name").and_then(Value::as_str), "fn");
        let a_score = parse_f64_value(a.get("score"), 0.5, 0.0, 1.0)
            + if has_token(&objective_lc, &a_name) {
                0.25
            } else {
                0.0
            };
        let b_score = parse_f64_value(b.get("score"), 0.5, 0.0, 1.0)
            + if has_token(&objective_lc, &b_name) {
                0.25
            } else {
                0.0
            };
        b_score
            .partial_cmp(&a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a_name.cmp(&b_name))
    });
    let max_steps = parse_u64_value(payload.get("max_steps"), 4, 1, 16) as usize;
    let selected = functions.into_iter().take(max_steps).collect::<Vec<_>>();
    let plan_steps = selected
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let name = clean_token(row.get("name").and_then(Value::as_str), "function");
            json!({
                "step_id": format!("step-{}", index + 1),
                "function_name": name,
                "description": clean_text(row.get("description").and_then(Value::as_str), 160),
                "checkpoint_key": format!("workflow.{}.{}", service_id, name),
                "execution_surface": "workflow-executor",
                "function_selection_score": parse_f64_value(row.get("score"), 0.5, 0.0, 1.0),
            })
        })
        .collect::<Vec<_>>();
    let plan = json!({
        "plan_id": stable_id("skplan", &json!({"service_id": service_id, "objective": objective, "steps": plan_steps})),
        "service_id": service_id,
        "objective": objective,
        "routing_mode": clean_token(payload.get("routing_mode").and_then(Value::as_str), "sequential"),
        "steps": plan_steps,
        "registered_at": now_iso(),
    });
    let plan_id = plan
        .get("plan_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "plans").insert(plan_id.clone(), plan.clone());
    Ok(json!({
        "ok": true,
        "plan": plan,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.4", semantic_claim("V6-WORKFLOW-008.4")),
    }))
}

fn supported_vector_provider(provider: &str) -> bool {
    matches!(
        provider,
        "azure-ai-search" | "chroma" | "elasticsearch" | "memory-plane"
    )
}

fn register_vector_connector(
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "semantic-vector",
    );
    let provider = clean_token(
        payload.get("provider").and_then(Value::as_str),
        "memory-plane",
    );
    if !supported_vector_provider(&provider) {
        return Err("semantic_kernel_vector_provider_invalid".to_string());
    }
    let connector = json!({
        "connector_id": stable_id("skvec", &json!({"name": name, "provider": provider})),
        "name": name,
        "provider": provider,
        "context_budget_tokens": parse_u64_value(payload.get("context_budget_tokens"), 512, 32, 4096),
        "min_profile": if provider == "memory-plane" { "tiny-max" } else { "rich" },
        "documents": payload.get("documents").cloned().filter(Value::is_array).unwrap_or_else(|| json!([])),
        "registered_at": now_iso(),
    });
    let connector_id = connector
        .get("connector_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "vector_connectors").insert(connector_id.clone(), connector.clone());
    Ok(json!({
        "ok": true,
        "connector": connector,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.5", semantic_claim("V6-WORKFLOW-008.5")),
    }))
}

fn lexical_score(query: &str, text: &str) -> u64 {
    let query_lc = query.to_ascii_lowercase();
    let query_tokens = query_lc
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<BTreeSet<_>>();
    let text_lc = text.to_ascii_lowercase();
    query_tokens
        .into_iter()
        .map(|token| text_lc.matches(token).count() as u64)
        .sum()
}

fn retrieve(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let connector_id = clean_token(payload.get("connector_id").and_then(Value::as_str), "");
    let query = clean_text(payload.get("query").and_then(Value::as_str), 240);
    if connector_id.is_empty() || query.is_empty() {
        return Err("semantic_kernel_retrieve_connector_and_query_required".to_string());
    }
    let connectors = as_object_mut(state, "vector_connectors");
    let connector = connectors
        .get(&connector_id)
        .and_then(Value::as_object)
        .ok_or_else(|| "semantic_kernel_vector_connector_not_found".to_string())?;
    let provider = connector
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("memory-plane");
    let profile = normalized_profile(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("rich"),
    );
    let min_profile = connector
        .get("min_profile")
        .and_then(Value::as_str)
        .unwrap_or("rich");
    if min_profile == "rich" && profile != "rich" {
        return Err(format!(
            "semantic_kernel_vector_connector_degraded_profile:{provider}:{profile}"
        ));
    }
    let top_k = parse_u64_value(payload.get("top_k"), 3, 1, 12) as usize;
    let budget = connector
        .get("context_budget_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(512);
    let docs = connector
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut ranked = docs
        .into_iter()
        .filter_map(|row| {
            let text = row
                .get("text")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| row.to_string());
            let score = lexical_score(&query, &text);
            (score > 0).then(|| {
                json!({
                    "text": text,
                    "score": score,
                    "token_estimate": approx_token_count(&text),
                    "metadata": row.get("metadata").cloned().unwrap_or(Value::Null),
                })
            })
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        b.get("score")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .cmp(&a.get("score").and_then(Value::as_u64).unwrap_or(0))
    });
    let mut used = 0_u64;
    let mut results = Vec::new();
    for row in ranked.into_iter().take(top_k) {
        let tokens = row
            .get("token_estimate")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if used.saturating_add(tokens) > budget {
            break;
        }
        used = used.saturating_add(tokens);
        results.push(row);
    }
    Ok(json!({
        "ok": true,
        "connector_id": connector_id,
        "provider": provider,
        "profile": profile,
        "results": results,
        "used_tokens": used,
        "budget_tokens": budget,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.5", semantic_claim("V6-WORKFLOW-008.5")),
    }))
}

fn supported_llm_provider(provider: &str) -> bool {
    matches!(
        provider,
        "azure-openai" | "ollama" | "hugging-face" | "nvidia" | "openai-compatible"
    )
}

fn register_llm_connector(
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(payload.get("name").and_then(Value::as_str), "semantic-llm");
    let provider = clean_token(
        payload.get("provider").and_then(Value::as_str),
        "openai-compatible",
    );
    if !supported_llm_provider(&provider) {
        return Err("semantic_kernel_llm_provider_invalid".to_string());
    }
    let modalities = payload
        .get("modalities")
        .cloned()
        .filter(Value::is_array)
        .unwrap_or_else(|| json!(["text"]));
    let connector = json!({
        "connector_id": stable_id("skllm", &json!({"name": name, "provider": provider, "modalities": modalities})),
        "name": name,
        "provider": provider,
        "model": clean_text(payload.get("model").and_then(Value::as_str), 120),
        "modalities": modalities,
        "registered_at": now_iso(),
    });
    let connector_id = connector
        .get("connector_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "llm_connectors").insert(connector_id.clone(), connector.clone());
    Ok(json!({
        "ok": true,
        "connector": connector,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.6", semantic_claim("V6-WORKFLOW-008.6")),
    }))
}

fn route_llm(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let connector_id = clean_token(payload.get("connector_id").and_then(Value::as_str), "");
    if connector_id.is_empty() {
        return Err("semantic_kernel_llm_connector_required".to_string());
    }
    let connectors = as_object_mut(state, "llm_connectors");
    let connector = connectors
        .get(&connector_id)
        .and_then(Value::as_object)
        .ok_or_else(|| "semantic_kernel_llm_connector_not_found".to_string())?;
    let modality = clean_token(payload.get("modality").and_then(Value::as_str), "text");
    let connector_modalities = connector
        .get("modalities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let supports_modality = connector_modalities
        .iter()
        .any(|row| row.as_str() == Some(modality.as_str()));
    if !supports_modality {
        return Err(format!(
            "semantic_kernel_llm_modality_unsupported:{modality}"
        ));
    }
    let profile = normalized_profile(
        payload
            .get("profile")
            .and_then(Value::as_str)
            .unwrap_or("rich"),
    );
    if modality != "text" && profile != "rich" {
        return Err(format!(
            "semantic_kernel_llm_multimodal_profile_blocked:{profile}:{modality}"
        ));
    }
    Ok(json!({
        "ok": true,
        "route": {
            "connector_id": connector_id,
            "provider": connector.get("provider").cloned().unwrap_or(Value::Null),
            "model": connector.get("model").cloned().unwrap_or(Value::Null),
            "modality": modality,
            "prompt_tokens_estimate": approx_token_count(payload.get("prompt").and_then(Value::as_str).unwrap_or("")),
            "profile": profile,
        },
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.6", semantic_claim("V6-WORKFLOW-008.6")),
    }))
}

fn schema_type_matches(expected: &str, value: &Value) -> bool {
    match expected {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => true,
    }
}

fn validate_json_schema(schema: &Value, value: &Value, path: &str, violations: &mut Vec<String>) {
    let expected_type = schema.get("type").and_then(Value::as_str).unwrap_or("");
    if !expected_type.is_empty() && !schema_type_matches(expected_type, value) {
        violations.push(format!("type_mismatch:{}:{}", path, expected_type));
        return;
    }
    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        if let Some(map) = value.as_object() {
            for field in required.iter().filter_map(Value::as_str) {
                if !map.contains_key(field) {
                    violations.push(format!("missing_required:{}:{}", path, field));
                }
            }
        }
    }
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        if let Some(map) = value.as_object() {
            for (key, child_schema) in properties {
                if let Some(child_value) = map.get(key) {
                    let child_path = if path == "$" {
                        format!("$.{}", key)
                    } else {
                        format!("{}.{}", path, key)
                    };
                    validate_json_schema(child_schema, child_value, &child_path, violations);
                }
            }
        }
    }
    if let Some(items) = schema.get("items") {
        if let Some(rows) = value.as_array() {
            for (index, row) in rows.iter().enumerate() {
                validate_json_schema(items, row, &format!("{}[{}]", path, index), violations);
            }
        }
    }
    if let Some(options) = schema.get("enum").and_then(Value::as_array) {
        if !options.iter().any(|row| row == value) {
            violations.push(format!("enum_violation:{}", path));
        }
    }
}

fn validate_process_graph(process: &Value) -> Result<Value, String> {
    let steps = process
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if steps.is_empty() {
        return Err("semantic_kernel_process_steps_required".to_string());
    }
    let mut ids = BTreeSet::new();
    for step in &steps {
        let step_id = clean_token(step.get("id").and_then(Value::as_str), "");
        if step_id.is_empty() {
            return Err("semantic_kernel_process_step_id_required".to_string());
        }
        ids.insert(step_id);
    }
    for step in &steps {
        if let Some(next) = step.get("next").and_then(Value::as_str) {
            let next_id = clean_token(Some(next), "");
            if !next_id.is_empty() && !ids.contains(&next_id) {
                return Err(format!("semantic_kernel_process_missing_next:{next_id}"));
            }
        }
    }
    Ok(json!({
        "step_count": steps.len(),
        "validated": true,
    }))
}

fn validate_structured_output(
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let schema = payload.get("schema").cloned().unwrap_or_else(|| json!({}));
    let output = payload.get("output").cloned().unwrap_or(Value::Null);
    let mut violations = Vec::new();
    validate_json_schema(&schema, &output, "$", &mut violations);
    let process_report = if let Some(process) = payload.get("process") {
        Some(validate_process_graph(process)?)
    } else {
        None
    };
    if !violations.is_empty() {
        return Err(format!(
            "semantic_kernel_structured_output_invalid:{}",
            violations.join(",")
        ));
    }
    let record = json!({
        "record_id": stable_id("skproc", &json!({"schema": schema, "output": output, "process": payload.get("process")})),
        "schema": schema,
        "output": output,
        "process_report": process_report,
        "validated_at": now_iso(),
    });
    let record_id = record
        .get("record_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "structured_processes").insert(record_id.clone(), record.clone());
    Ok(json!({
        "ok": true,
        "record": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.7", semantic_claim("V6-WORKFLOW-008.7")),
    }))
}

fn emit_enterprise_event(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let sink = clean_token(payload.get("sink").and_then(Value::as_str), "otel");
    let cloud = clean_token(payload.get("cloud").and_then(Value::as_str), "azure");
    let endpoint = clean_text(payload.get("endpoint").and_then(Value::as_str), 200);
    if !endpoint.is_empty() && !endpoint.starts_with("https://") {
        return Err("semantic_kernel_enterprise_endpoint_must_be_https".to_string());
    }
    let event = json!({
        "event_id": stable_id("skevt", &json!({"sink": sink, "cloud": cloud, "event_type": payload.get("event_type")})),
        "event_type": clean_token(payload.get("event_type").and_then(Value::as_str), "semantic-kernel-observability"),
        "sink": sink,
        "cloud": cloud,
        "endpoint": endpoint,
        "tags": payload.get("tags").cloned().filter(Value::is_object).unwrap_or_else(|| json!({})),
        "deployment": payload.get("deployment").cloned().filter(Value::is_object).unwrap_or_else(|| json!({})),
        "recorded_at": now_iso(),
    });
    as_array_mut(state, "enterprise_events").push(event.clone());
    Ok(json!({
        "ok": true,
        "event": event,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.8", semantic_claim("V6-WORKFLOW-008.8")),
    }))
}

fn register_dotnet_bridge(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "semantic-kernel-dotnet",
    );
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/polyglot/semantic_kernel_dotnet_bridge.ts"),
    )?;
    if !bridge_path.starts_with("adapters/") {
        return Err("semantic_kernel_dotnet_bridge_must_live_in_adapters".to_string());
    }
    let bridge = json!({
        "bridge_id": stable_id("skdotnet", &json!({"name": name, "bridge_path": bridge_path})),
        "name": name,
        "bridge_path": bridge_path,
        "command": clean_text(payload.get("command").and_then(Value::as_str), 160),
        "command_args": payload.get("command_args").cloned().filter(Value::is_array).unwrap_or_else(|| json!([])),
        "capabilities": payload.get("capabilities").cloned().filter(Value::is_array).unwrap_or_else(|| json!([])),
        "registered_at": now_iso(),
    });
    let bridge_id = bridge
        .get("bridge_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "dotnet_bridges").insert(bridge_id.clone(), bridge.clone());
    Ok(json!({
        "ok": true,
        "bridge": bridge,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.9", semantic_claim("V6-WORKFLOW-008.9")),
    }))
}

fn invoke_dotnet_bridge(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let bridge_id = clean_token(payload.get("bridge_id").and_then(Value::as_str), "");
    if bridge_id.is_empty() {
        return Err("semantic_kernel_dotnet_bridge_required".to_string());
    }
    let bridges = as_object_mut(state, "dotnet_bridges");
    let bridge = bridges
        .get_mut(&bridge_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "semantic_kernel_dotnet_bridge_not_found".to_string())?;
    let dry_run = payload
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let invocation = if dry_run
        || bridge
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
    {
        json!({
            "mode": "dry_run",
            "operation": clean_token(payload.get("operation").and_then(Value::as_str), "invoke"),
            "arguments": payload.get("args").cloned().unwrap_or_else(|| json!({})),
            "simulated": true,
        })
    } else {
        let command = bridge.get("command").and_then(Value::as_str).unwrap_or("");
        let command_args = bridge
            .get("command_args")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.as_str().map(ToString::to_string))
            .collect::<Vec<_>>();
        let operation = clean_token(payload.get("operation").and_then(Value::as_str), "invoke");
        let args_json = payload
            .get("args")
            .cloned()
            .unwrap_or_else(|| json!({}))
            .to_string();
        let run = Command::new(command)
            .args(command_args)
            .arg(operation)
            .env("PROTHEUS_SK_DOTNET_ARGS", args_json)
            .output()
            .map_err(|err| format!("semantic_kernel_dotnet_exec_failed:{err}"))?;
        if !run.status.success() {
            return Err(format!(
                "semantic_kernel_dotnet_exec_nonzero:{}",
                String::from_utf8_lossy(&run.stderr)
            ));
        }
        json!({
            "mode": "process_exec",
            "stdout": String::from_utf8_lossy(&run.stdout).trim().to_string(),
            "stderr": String::from_utf8_lossy(&run.stderr).trim().to_string(),
            "exit_code": run.status.code().unwrap_or(0),
        })
    };
    bridge.insert("last_invoked_at".to_string(), json!(now_iso()));
    Ok(json!({
        "ok": true,
        "bridge_id": bridge_id,
        "invocation": invocation,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.9", semantic_claim("V6-WORKFLOW-008.9")),
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("semantic_kernel_bridge_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let state_path = state_path(root, argv, input);
    let history_path = history_path(root, argv, input);
    let mut state = load_state(&state_path);

    let result = match command.as_str() {
        "status" => Ok(json!({
            "ok": true,
            "state_path": rel(root, &state_path),
            "history_path": rel(root, &history_path),
            "services": as_object_mut(&mut state, "services").len(),
            "plugins": as_object_mut(&mut state, "plugins").len(),
            "collaborations": as_object_mut(&mut state, "collaborations").len(),
            "plans": as_object_mut(&mut state, "plans").len(),
            "vector_connectors": as_object_mut(&mut state, "vector_connectors").len(),
            "llm_connectors": as_object_mut(&mut state, "llm_connectors").len(),
            "structured_processes": as_object_mut(&mut state, "structured_processes").len(),
            "enterprise_events": as_array_mut(&mut state, "enterprise_events").len(),
            "dotnet_bridges": as_object_mut(&mut state, "dotnet_bridges").len(),
            "last_receipt": state.get("last_receipt").cloned().unwrap_or(Value::Null),
        })),
        "register-service" => register_service(&mut state, input),
        "register-plugin" => register_plugin(root, &mut state, input),
        "invoke-plugin" => invoke_plugin(&mut state, input),
        "collaborate" => collaborate(root, argv, &mut state, input),
        "plan" => plan(&mut state, input),
        "register-vector-connector" => register_vector_connector(&mut state, input),
        "retrieve" => retrieve(&mut state, input),
        "register-llm-connector" => register_llm_connector(&mut state, input),
        "route-llm" => route_llm(&mut state, input),
        "validate-structured-output" => validate_structured_output(&mut state, input),
        "emit-enterprise-event" => emit_enterprise_event(&mut state, input),
        "register-dotnet-bridge" => register_dotnet_bridge(root, &mut state, input),
        "invoke-dotnet-bridge" => invoke_dotnet_bridge(&mut state, input),
        _ => Err(format!("unknown_command:{command}")),
    };

    match result {
        Ok(payload_out) => {
            let receipt = cli_receipt(
                &format!("semantic_kernel_bridge_{}", command.replace('-', "_")),
                payload_out,
            );
            state["last_receipt"] = receipt.clone();
            if let Err(err) = save_state(&state_path, &state) {
                print_json_line(&cli_error("semantic_kernel_bridge_error", &err));
                return 1;
            }
            if let Err(err) = append_history(&history_path, &receipt) {
                print_json_line(&cli_error("semantic_kernel_bridge_error", &err));
                return 1;
            }
            print_json_line(&receipt);
            0
        }
        Err(err) => {
            let receipt = cli_error("semantic_kernel_bridge_error", &err);
            print_json_line(&receipt);
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_output_validator_catches_missing_required() {
        let schema = json!({
            "type": "object",
            "required": ["answer"],
            "properties": {
                "answer": { "type": "string" }
            }
        });
        let output = json!({"other": true});
        let mut violations = Vec::new();
        validate_json_schema(&schema, &output, "$", &mut violations);
        assert!(violations
            .iter()
            .any(|row| row.contains("missing_required")));
    }

    #[test]
    fn planner_prefers_matching_functions() {
        let mut state = default_state();
        let service = register_service(
            &mut state,
            &json!({"name":"planner-service","execution_surface":"workflow-executor"})
                .as_object()
                .unwrap()
                .clone(),
        )
        .expect("service");
        let service_id = service["service"]["service_id"]
            .as_str()
            .unwrap()
            .to_string();
        let result = plan(
            &mut state,
            &json!({
                "service_id": service_id,
                "objective": "summarize then route the case",
                "functions": [
                    {"name":"route","score":0.6},
                    {"name":"summarize","score":0.4}
                ]
            })
            .as_object()
            .unwrap()
            .clone(),
        )
        .expect("plan");
        let steps = result["plan"]["steps"].as_array().expect("steps");
        assert_eq!(
            steps
                .first()
                .and_then(|row| row.get("function_name"))
                .and_then(Value::as_str),
            Some("route")
        );
    }
}
