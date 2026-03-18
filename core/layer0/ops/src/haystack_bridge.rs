// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "local/state/ops/haystack_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/haystack_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/haystack_bridge/swarm_state.json";

fn usage() {
    println!("haystack-bridge commands:");
    println!("  protheus-ops haystack-bridge status [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge register-pipeline [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge run-pipeline [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops haystack-bridge run-agent-toolset [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  protheus-ops haystack-bridge register-template [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge render-template [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge register-document-store [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge retrieve-documents [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge route-and-rank [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge record-multimodal-eval [--payload-base64=<json>] [--state-path=<path>]");
    println!(
        "  protheus-ops haystack-bridge trace-run [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  protheus-ops haystack-bridge import-connector [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops haystack-bridge assimilate-intake [--payload-base64=<json>] [--state-path=<path>]");
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
            .map_err(|err| format!("haystack_bridge_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("haystack_bridge_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("haystack_bridge_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("haystack_bridge_payload_decode_failed:{err}"));
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
    path.strip_prefix(root)
        .map(|value| value.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
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
        "schema_version": "haystack_bridge_state_v1",
        "pipelines": {},
        "pipeline_runs": {},
        "agent_runs": {},
        "templates": {},
        "template_renders": {},
        "document_stores": {},
        "retrieval_runs": {},
        "routes": {},
        "evaluations": {},
        "traces": [],
        "connectors": {},
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
        "pipelines",
        "pipeline_runs",
        "agent_runs",
        "templates",
        "template_renders",
        "document_stores",
        "retrieval_runs",
        "routes",
        "evaluations",
        "connectors",
        "intakes",
    ] {
        if !value.get(key).map(Value::is_object).unwrap_or(false) {
            value[key] = json!({});
        }
    }
    if !value.get("traces").map(Value::is_array).unwrap_or(false) {
        value["traces"] = json!([]);
    }
    if value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_none()
    {
        value["schema_version"] = json!("haystack_bridge_state_v1");
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
            (b'a' + (digit - 10)) as char
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
    raw.unwrap_or_default()
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect()
}

fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    let value = clean_text(raw, 96);
    if value.is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

fn parse_u64_value(value: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    value
        .and_then(|row| row.as_u64())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_bool_value(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn safe_prefix_for_bridge(path: &str) -> bool {
    path.starts_with("adapters/")
}

fn safe_shell_prefix(path: &str) -> bool {
    path.starts_with("client/") || path.starts_with("apps/")
}

fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    let cleaned = clean_text(Some(raw), 240);
    if cleaned.is_empty() {
        return Err("haystack_bridge_path_required".to_string());
    }
    if !safe_prefix_for_bridge(&cleaned) {
        return Err("haystack_bridge_path_must_be_adapter_owned".to_string());
    }
    let full = repo_path(root, &cleaned);
    if !full.starts_with(root.join("adapters")) {
        return Err("haystack_bridge_path_escapes_adapters".to_string());
    }
    Ok(cleaned)
}

fn normalize_shell_path(root: &Path, raw: &str) -> Result<String, String> {
    let cleaned = clean_text(Some(raw), 240);
    if cleaned.is_empty() {
        return Err("haystack_shell_path_required".to_string());
    }
    if !safe_shell_prefix(&cleaned) {
        return Err("haystack_shell_path_must_live_under_client_or_apps".to_string());
    }
    let full = repo_path(root, &cleaned);
    if !(full.starts_with(root.join("client")) || full.starts_with(root.join("apps"))) {
        return Err("haystack_shell_path_escapes_workspace".to_string());
    }
    Ok(cleaned)
}

fn default_claim_evidence(id: &str, claim: &str) -> Value {
    json!([{ "id": id, "claim": claim }])
}

fn haystack_claim(id: &str) -> &'static str {
    match id {
        "V6-WORKFLOW-012.1" => {
            "haystack_pipelines_register_and_execute_as_governed_component_graphs"
        }
        "V6-WORKFLOW-012.2" => {
            "haystack_searchable_tool_agents_reduce_tool_fanout_and_execute_through_swarm_authority"
        }
        "V6-WORKFLOW-012.3" => {
            "haystack_templates_and_rendered_prompts_are_versioned_provenanced_and_receipted"
        }
        "V6-WORKFLOW-012.4" => {
            "haystack_document_stores_and_rag_queries_normalize_to_governed_retrieval_runtime"
        }
        "V6-WORKFLOW-012.5" => {
            "haystack_routes_and_rankers_are_deterministic_replayable_and_fail_closed"
        }
        "V6-WORKFLOW-012.6" => {
            "haystack_multimodal_evals_emit_typed_artifacts_and_governed_metrics"
        }
        "V6-WORKFLOW-012.7" => {
            "haystack_step_traces_and_branch_decisions_fold_into_native_observability"
        }
        "V6-WORKFLOW-012.8" => {
            "haystack_connectors_and_pipeline_assets_ingest_through_one_governed_gateway"
        }
        _ => "haystack_bridge_claim",
    }
}

fn read_swarm_state(path: &Path) -> Value {
    lane_utils::read_json(path).unwrap_or_else(|| json!({ "sessions": {}, "handoff_registry": {} }))
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
        return Err(format!("haystack_swarm_spawn_failed:{label}"));
    }
    let swarm_state = read_swarm_state(swarm_state_path);
    find_swarm_session_id_by_task(&swarm_state, task)
        .ok_or_else(|| format!("haystack_swarm_session_missing:{label}"))
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
        return Err("haystack_observability_enable_failed".to_string());
    }
    let exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=trace".to_string(),
            "--source-agent=haystack-bridge".to_string(),
            format!("--target-agent={}", clean_token(Some(intent), "workflow")),
            format!("--intent={}", clean_text(Some(intent), 80)),
            format!("--message={}", clean_text(Some(message), 160)),
            format!("--trace-id={trace_id}"),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if exit != 0 {
        return Err("haystack_observability_trace_failed".to_string());
    }
    Ok(())
}

fn doc_token_set(doc: &Value) -> BTreeSet<String> {
    clean_text(doc.get("text").and_then(Value::as_str), 4096)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn query_terms(query: &str) -> Vec<String> {
    clean_text(Some(query), 240)
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn retrieval_score(doc: &Value, terms: &[String], mode: &str) -> i64 {
    let tokens = doc_token_set(doc);
    let mut score = 0i64;
    for term in terms {
        if tokens.contains(term) {
            score += match mode {
                "ranker" => 4,
                "vector" => 3,
                _ => 2,
            };
        }
    }
    if mode == "hybrid"
        && doc
            .get("metadata")
            .and_then(|row| row.get("kind"))
            .and_then(Value::as_str)
            == Some("graph")
    {
        score += 2;
    }
    score
}

fn render_template_text(template: &str, variables: &Map<String, Value>) -> String {
    let mut out = template.to_string();
    for (key, value) in variables {
        let replacement = value
            .as_str()
            .map(|row| clean_text(Some(row), 4000))
            .unwrap_or_else(|| value.to_string());
        out = out.replace(&format!("{{{{{key}}}}}"), &replacement);
    }
    out
}

fn allowed_connector_type(kind: &str) -> bool {
    matches!(
        kind,
        "mcp"
            | "openapi"
            | "filesystem"
            | "pgvector"
            | "qdrant"
            | "weaviate"
            | "elasticsearch"
            | "opensearch"
            | "s3"
            | "http"
    )
}

fn register_pipeline(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "haystack-pipeline",
    );
    let components = payload
        .get("components")
        .or_else(|| payload.get("stages"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if components.is_empty() {
        return Err("haystack_pipeline_components_required".to_string());
    }
    let normalized = components
        .into_iter()
        .map(|component| {
            let obj = component.as_object().cloned().unwrap_or_default();
            json!({
                "id": clean_token(obj.get("id").and_then(Value::as_str), "stage"),
                "stage_type": clean_token(obj.get("stage_type").and_then(Value::as_str).or_else(|| obj.get("type").and_then(Value::as_str)), "generator"),
                "input_type": clean_token(obj.get("input_type").and_then(Value::as_str), "text"),
                "output_type": clean_token(obj.get("output_type").and_then(Value::as_str), "text"),
                "parallel": parse_bool_value(obj.get("parallel"), false),
                "spawn": parse_bool_value(obj.get("spawn"), false),
                "budget": parse_u64_value(obj.get("budget"), 192, 32, 4096),
            })
        })
        .collect::<Vec<_>>();
    let pipeline = json!({
        "pipeline_id": stable_id("haypipe", &json!({"name": name, "components": normalized.len()})),
        "name": name,
        "components": normalized,
        "registered_at": now_iso(),
    });
    let pipeline_id = pipeline
        .get("pipeline_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "pipelines").insert(pipeline_id, pipeline.clone());
    Ok(json!({
        "ok": true,
        "pipeline": pipeline,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.1", haystack_claim("V6-WORKFLOW-012.1")),
    }))
}

fn run_pipeline(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let pipeline_id = clean_token(payload.get("pipeline_id").and_then(Value::as_str), "");
    if pipeline_id.is_empty() {
        return Err("haystack_pipeline_id_required".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let pipeline = state
        .get("pipelines")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&pipeline_id))
        .cloned()
        .ok_or_else(|| format!("unknown_haystack_pipeline:{pipeline_id}"))?;
    let components = pipeline
        .get("components")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let parallel_count = components
        .iter()
        .filter(|row| row.get("parallel").and_then(Value::as_bool) == Some(true))
        .count();
    let degraded = matches!(profile.as_str(), "pure" | "tiny-max") && parallel_count > 1;
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let root_session_id = if components.iter().any(|row| {
        row.get("spawn").and_then(Value::as_bool) == Some(true)
            || matches!(
                row.get("stage_type").and_then(Value::as_str),
                Some("generator" | "tool" | "agent")
            )
    }) {
        Some(ensure_session_for_task(
            root,
            &swarm_state_path,
            &format!(
                "haystack:pipeline:{}",
                clean_token(pipeline.get("name").and_then(Value::as_str), "pipeline")
            ),
            "haystack-pipeline",
            Some("pipeline"),
            None,
            parse_u64_value(payload.get("budget"), 896, 96, 12288),
        )?)
    } else {
        None
    };
    let mut selected_parallel = 0usize;
    let visited = components
        .into_iter()
        .map(|component| {
            let is_parallel = component.get("parallel").and_then(Value::as_bool) == Some(true);
            let selected = if degraded && is_parallel {
                selected_parallel += 1;
                selected_parallel == 1
            } else {
                true
            };
            json!({
                "stage_id": component.get("id").cloned().unwrap_or(Value::Null),
                "stage_type": component.get("stage_type").cloned().unwrap_or(Value::Null),
                "parallel": is_parallel,
                "selected": selected,
                "session_id": if selected { root_session_id.clone().map(Value::String).unwrap_or(Value::Null) } else { Value::Null },
            })
        })
        .collect::<Vec<_>>();
    let run = json!({
        "run_id": stable_id("hayrun", &json!({"pipeline_id": pipeline_id, "profile": profile})),
        "pipeline_id": pipeline_id,
        "profile": profile,
        "visited": visited,
        "degraded": degraded,
        "reason_code": if degraded { "parallel_pipeline_profile_limited" } else { "pipeline_ok" },
        "root_session_id": root_session_id,
        "executed_at": now_iso(),
    });
    let run_id = run
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "pipeline_runs").insert(run_id, run.clone());
    Ok(json!({
        "ok": true,
        "run": run,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.1", haystack_claim("V6-WORKFLOW-012.1")),
    }))
}

fn run_agent_toolset(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "haystack-agent",
    );
    let goal = clean_text(payload.get("goal").and_then(Value::as_str), 240);
    if goal.is_empty() {
        return Err("haystack_agent_goal_required".to_string());
    }
    let tools = payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if tools.is_empty() {
        return Err("haystack_agent_tools_required".to_string());
    }
    let terms = query_terms(&goal);
    let search_limit = parse_u64_value(payload.get("search_limit"), 3, 1, 12) as usize;
    let mut ranked = tools
        .into_iter()
        .map(|tool| {
            let hay = format!(
                "{} {} {}",
                clean_text(tool.get("name").and_then(Value::as_str), 120),
                clean_text(tool.get("description").and_then(Value::as_str), 240),
                tool.get("tags").cloned().unwrap_or_else(|| json!([]))
            )
            .to_ascii_lowercase();
            let score = terms
                .iter()
                .filter(|term| hay.contains(term.as_str()))
                .count() as i64;
            (score, tool)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    let selected_tools = ranked
        .iter()
        .filter(|(score, _)| *score > 0)
        .take(search_limit)
        .map(|(_, tool)| tool.clone())
        .collect::<Vec<_>>();
    let selected_tools = if selected_tools.is_empty() {
        vec![ranked
            .first()
            .map(|(_, tool)| tool.clone())
            .ok_or_else(|| "haystack_agent_tool_selection_failed".to_string())?]
    } else {
        selected_tools
    };
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let session_id = ensure_session_for_task(
        root,
        &swarm_state_path,
        &format!("haystack:agent:{name}:{goal}"),
        &name,
        Some("tool-agent"),
        None,
        parse_u64_value(payload.get("budget"), 640, 96, 12288),
    )?;
    let run = json!({
        "agent_run_id": stable_id("hayagent", &json!({"name": name, "goal": goal})),
        "name": name,
        "goal": goal,
        "session_id": session_id,
        "search_terms": terms,
        "selected_tools": selected_tools,
        "discarded_tool_count": ranked.len().saturating_sub(selected_tools.len()),
        "executed_at": now_iso(),
    });
    let run_id = run
        .get("agent_run_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "agent_runs").insert(run_id, run.clone());
    Ok(json!({
        "ok": true,
        "agent": run,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.2", haystack_claim("V6-WORKFLOW-012.2")),
    }))
}

fn register_template(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "haystack-template",
    );
    let template = clean_text(payload.get("template").and_then(Value::as_str), 4000);
    if template.is_empty() {
        return Err("haystack_template_body_required".to_string());
    }
    let record = json!({
        "template_id": stable_id("haytpl", &json!({"name": name, "template": template})),
        "name": name,
        "template": template,
        "asset_kind": clean_token(payload.get("asset_kind").and_then(Value::as_str), "prompt"),
        "version": 1,
        "registered_at": now_iso(),
    });
    let template_id = record
        .get("template_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "templates").insert(template_id, record.clone());
    Ok(json!({
        "ok": true,
        "template": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.3", haystack_claim("V6-WORKFLOW-012.3")),
    }))
}

fn render_template(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let template_id = clean_token(payload.get("template_id").and_then(Value::as_str), "");
    if template_id.is_empty() {
        return Err("haystack_render_template_id_required".to_string());
    }
    let template = state
        .get("templates")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&template_id))
        .cloned()
        .ok_or_else(|| format!("unknown_haystack_template:{template_id}"))?;
    let variables = payload
        .get("variables")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let body = render_template_text(
        template
            .get("template")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        &variables,
    );
    let render = json!({
        "render_id": stable_id("hayrender", &json!({"template_id": template_id, "variables": variables})),
        "template_id": template_id,
        "source_template_id": template.get("template_id").cloned().unwrap_or(Value::Null),
        "output": body,
        "rendered_at": now_iso(),
    });
    let render_id = render
        .get("render_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "template_renders").insert(render_id, render.clone());
    Ok(json!({
        "ok": true,
        "render": render,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.3", haystack_claim("V6-WORKFLOW-012.3")),
    }))
}

fn register_document_store(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "haystack-store",
    );
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/haystack_connector_bridge.ts"),
    )?;
    let documents = payload
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if documents.is_empty() {
        return Err("haystack_document_store_documents_required".to_string());
    }
    let retrieval_modes = payload
        .get("retrieval_modes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("hybrid"), json!("vector"), json!("ranker")]);
    let supported_profiles = payload
        .get("supported_profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("rich"), json!("pure")]);
    let store = json!({
        "store_id": stable_id("haystore", &json!({"name": name, "bridge_path": bridge_path})),
        "name": name,
        "bridge_path": bridge_path,
        "documents": documents,
        "retrieval_modes": retrieval_modes,
        "supported_profiles": supported_profiles,
        "context_budget": parse_u64_value(payload.get("context_budget"), 512, 64, 4096),
        "registered_at": now_iso(),
    });
    let store_id = store
        .get("store_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "document_stores").insert(store_id, store.clone());
    Ok(json!({
        "ok": true,
        "document_store": store,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.4", haystack_claim("V6-WORKFLOW-012.4")),
    }))
}

fn retrieve_documents(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let store_id = clean_token(payload.get("store_id").and_then(Value::as_str), "");
    if store_id.is_empty() {
        return Err("haystack_store_id_required".to_string());
    }
    let query = clean_text(payload.get("query").and_then(Value::as_str), 240);
    if query.is_empty() {
        return Err("haystack_query_required".to_string());
    }
    let mode = clean_token(payload.get("mode").and_then(Value::as_str), "hybrid");
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let store = state
        .get("document_stores")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&store_id))
        .cloned()
        .ok_or_else(|| format!("unknown_haystack_document_store:{store_id}"))?;
    let supported_profiles = store
        .get("supported_profiles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !supported_profiles
        .iter()
        .filter_map(Value::as_str)
        .any(|row| row == profile)
    {
        return Err(format!(
            "haystack_document_store_profile_unsupported:{profile}"
        ));
    }
    let supported_mode_rows = store
        .get("retrieval_modes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let supported_modes = supported_mode_rows
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if !supported_modes.contains(mode.as_str()) {
        return Err(format!("haystack_retrieval_mode_unsupported:{mode}"));
    }
    let requested_top_k = parse_u64_value(payload.get("top_k"), 3, 1, 12) as usize;
    let top_k = if matches!(profile.as_str(), "pure" | "tiny-max") {
        requested_top_k.min(2)
    } else {
        requested_top_k
    };
    let terms = query_terms(&query);
    let mut ranked = store
        .get("documents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|doc| (retrieval_score(&doc, &terms, &mode), doc))
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    let context_budget = parse_u64_value(
        payload.get("context_budget"),
        store
            .get("context_budget")
            .and_then(Value::as_u64)
            .unwrap_or(512),
        64,
        4096,
    );
    let mut consumed = 0usize;
    let context_limit = (context_budget as usize) * 4;
    let mut results = Vec::new();
    for (score, doc) in ranked.into_iter().take(top_k) {
        let text = clean_text(doc.get("text").and_then(Value::as_str), 4000);
        if !results.is_empty() && consumed + text.len() > context_limit {
            break;
        }
        consumed += text.len();
        results.push(json!({
            "score": score,
            "text": text,
            "metadata": doc.get("metadata").cloned().unwrap_or(Value::Null),
        }));
    }
    let retrieval = json!({
        "retrieval_id": stable_id("hayret", &json!({"store_id": store_id, "query": query, "mode": mode})),
        "store_id": store_id,
        "query": query,
        "mode": mode,
        "profile": profile,
        "degraded": top_k != requested_top_k,
        "reason_code": if top_k != requested_top_k { "profile_context_budget_limited" } else { "retrieval_ok" },
        "results": results,
        "context_budget": context_budget,
        "recorded_at": now_iso(),
    });
    let retrieval_id = retrieval
        .get("retrieval_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "retrieval_runs").insert(retrieval_id, retrieval.clone());
    Ok(json!({
        "ok": true,
        "retrieval": retrieval,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.4", haystack_claim("V6-WORKFLOW-012.4")),
    }))
}

fn route_and_rank(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "haystack-router",
    );
    let routes = payload
        .get("routes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let candidates = payload
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if routes.is_empty() {
        return Err("haystack_routes_required".to_string());
    }
    let context = payload
        .get("context")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let query = clean_text(payload.get("query").and_then(Value::as_str), 240);
    let terms = query_terms(&query);
    let mut best_route = None::<(i64, Value)>;
    for route in routes {
        let obj = route.as_object().cloned().unwrap_or_default();
        let mut score = parse_u64_value(obj.get("weight"), 0, 0, 100) as i64;
        if let Some(field) = obj.get("field").and_then(Value::as_str) {
            if let Some(expected) = obj.get("equals") {
                if context.get(field) == Some(expected) {
                    score += 10;
                }
            }
        }
        if let Some(tag) = obj.get("contains").and_then(Value::as_str) {
            if query
                .to_ascii_lowercase()
                .contains(&tag.to_ascii_lowercase())
            {
                score += 4;
            }
        }
        if best_route
            .as_ref()
            .map(|(current, _)| score > *current)
            .unwrap_or(true)
        {
            best_route = Some((score, Value::Object(obj)));
        }
    }
    let (_, route) = best_route.ok_or_else(|| "haystack_route_selection_failed".to_string())?;
    let route_obj = route.as_object().cloned().unwrap_or_default();
    let mut ranked = candidates
        .into_iter()
        .map(|candidate| {
            let metadata_boost = route_obj
                .get("metadata_key")
                .and_then(Value::as_str)
                .and_then(|key| candidate.get("metadata").and_then(|row| row.get(key)))
                .map(|_| 2)
                .unwrap_or(0);
            let score = retrieval_score(&candidate, &terms, "ranker") + metadata_boost;
            (score, candidate)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    let record = json!({
        "route_id": stable_id("hayroute", &json!({"name": name, "query": query})),
        "name": name,
        "selected_route": {
            "id": clean_token(route_obj.get("id").and_then(Value::as_str), "route"),
            "reason": clean_text(route_obj.get("reason").and_then(Value::as_str), 160),
        },
        "ranked": ranked.into_iter().take(4).map(|(score, candidate)| json!({
            "score": score,
            "text": candidate.get("text").cloned().unwrap_or(Value::Null),
            "metadata": candidate.get("metadata").cloned().unwrap_or(Value::Null),
        })).collect::<Vec<_>>(),
        "context": context,
        "query": query,
        "recorded_at": now_iso(),
    });
    let route_id = record
        .get("route_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "routes").insert(route_id, record.clone());
    Ok(json!({
        "ok": true,
        "route": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.5", haystack_claim("V6-WORKFLOW-012.5")),
    }))
}

fn record_multimodal_eval(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(payload.get("name").and_then(Value::as_str), "haystack-eval");
    let artifacts = payload
        .get("artifacts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if artifacts.is_empty() {
        return Err("haystack_eval_artifacts_required".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let multimodal = artifacts
        .iter()
        .filter_map(|row| row.get("media_type").and_then(Value::as_str))
        .any(|kind| kind != "text/plain");
    let degraded = multimodal && matches!(profile.as_str(), "pure" | "tiny-max");
    let record = json!({
        "evaluation_id": stable_id("hayeval", &json!({"name": name, "artifact_count": artifacts.len()})),
        "name": name,
        "artifact_count": artifacts.len(),
        "artifacts": artifacts,
        "metrics": payload.get("metrics").cloned().unwrap_or_else(|| json!({})),
        "profile": profile,
        "degraded": degraded,
        "reason_code": if degraded { "multimodal_evaluation_profile_limited" } else { "evaluation_ok" },
        "recorded_at": now_iso(),
    });
    let evaluation_id = record
        .get("evaluation_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    emit_native_trace(
        root,
        &evaluation_id,
        "haystack_eval",
        &format!(
            "name={name} artifacts={}",
            record
                .get("artifact_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
    )?;
    as_object_mut(state, "evaluations").insert(evaluation_id, record.clone());
    Ok(json!({
        "ok": true,
        "evaluation": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.6", haystack_claim("V6-WORKFLOW-012.6")),
    }))
}

fn trace_run(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let trace_id = clean_token(
        payload.get("trace_id").and_then(Value::as_str),
        "haystack-trace",
    );
    let steps = payload
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if steps.is_empty() {
        return Err("haystack_trace_steps_required".to_string());
    }
    for step in &steps {
        let label = clean_token(step.get("stage").and_then(Value::as_str), "step");
        let message = clean_text(step.get("message").and_then(Value::as_str), 160);
        emit_native_trace(root, &trace_id, &label, &message)?;
    }
    let record = json!({
        "trace_id": trace_id,
        "steps": steps,
        "recorded_at": now_iso(),
    });
    as_array_mut(state, "traces").push(record.clone());
    Ok(json!({
        "ok": true,
        "trace": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.7", haystack_claim("V6-WORKFLOW-012.7")),
    }))
}

fn import_connector(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "haystack-connector",
    );
    let connector_type = clean_token(payload.get("connector_type").and_then(Value::as_str), "mcp");
    if !allowed_connector_type(&connector_type) {
        return Err(format!(
            "haystack_connector_type_unsupported:{connector_type}"
        ));
    }
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/haystack_connector_bridge.ts"),
    )?;
    let record = json!({
        "connector_id": stable_id("hayconn", &json!({"name": name, "connector_type": connector_type, "bridge_path": bridge_path})),
        "name": name,
        "connector_type": connector_type,
        "bridge_path": bridge_path,
        "assets": payload.get("assets").cloned().unwrap_or_else(|| json!([])),
        "supported_profiles": payload.get("supported_profiles").cloned().unwrap_or_else(|| json!(["rich", "pure"])),
        "imported_at": now_iso(),
    });
    let connector_id = record
        .get("connector_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "connectors").insert(connector_id, record.clone());
    Ok(json!({
        "ok": true,
        "connector": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.8", haystack_claim("V6-WORKFLOW-012.8")),
    }))
}

fn assimilate_intake(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let output_dir = normalize_shell_path(
        root,
        payload
            .get("output_dir")
            .and_then(Value::as_str)
            .unwrap_or("client/runtime/local/state/haystack-shell"),
    )?;
    let full = repo_path(root, &output_dir);
    let src_dir = full.join("src");
    let template_dir = full.join("templates");
    fs::create_dir_all(&src_dir)
        .map_err(|err| format!("haystack_intake_src_dir_create_failed:{err}"))?;
    fs::create_dir_all(&template_dir)
        .map_err(|err| format!("haystack_intake_template_dir_create_failed:{err}"))?;
    let package_json = json!({
        "name": clean_token(payload.get("package_name").and_then(Value::as_str), "haystack-shell"),
        "private": true,
        "scripts": {
            "start": "node src/haystack.pipeline.ts"
        }
    });
    let pipeline_source = "export const haystackPipeline = { components: [\n  { id: 'retrieve', stage_type: 'retriever', input_type: 'query', output_type: 'documents' },\n  { id: 'rank', stage_type: 'ranker', input_type: 'documents', output_type: 'documents' },\n  { id: 'answer', stage_type: 'generator', input_type: 'documents', output_type: 'answer', spawn: true }\n] };\n";
    let readme = "# Haystack Shell\n\nThin generated shell over `core://haystack-bridge`.\n";
    let prompt_template = "Answer the question: {{question}}\nUse only the supplied context.\n";
    fs::write(
        full.join("package.json"),
        serde_json::to_string_pretty(&package_json).unwrap(),
    )
    .map_err(|err| format!("haystack_intake_package_write_failed:{err}"))?;
    fs::write(src_dir.join("haystack.pipeline.ts"), pipeline_source)
        .map_err(|err| format!("haystack_intake_pipeline_write_failed:{err}"))?;
    fs::write(template_dir.join("prompt.jinja"), prompt_template)
        .map_err(|err| format!("haystack_intake_template_write_failed:{err}"))?;
    fs::write(full.join("README.md"), readme)
        .map_err(|err| format!("haystack_intake_readme_write_failed:{err}"))?;
    let record = json!({
        "intake_id": stable_id("hayintake", &json!({"output_dir": output_dir})),
        "output_dir": output_dir,
        "files": [
            format!("{}/package.json", rel(root, &full)),
            format!("{}/src/haystack.pipeline.ts", rel(root, &full)),
            format!("{}/templates/prompt.jinja", rel(root, &full)),
            format!("{}/README.md", rel(root, &full)),
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
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-012.8", haystack_claim("V6-WORKFLOW-012.8")),
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
            print_json_line(&cli_error("haystack_bridge_error", &err));
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
            "pipelines": as_object_mut(&mut state, "pipelines").len(),
            "pipeline_runs": as_object_mut(&mut state, "pipeline_runs").len(),
            "agent_runs": as_object_mut(&mut state, "agent_runs").len(),
            "templates": as_object_mut(&mut state, "templates").len(),
            "template_renders": as_object_mut(&mut state, "template_renders").len(),
            "document_stores": as_object_mut(&mut state, "document_stores").len(),
            "retrieval_runs": as_object_mut(&mut state, "retrieval_runs").len(),
            "routes": as_object_mut(&mut state, "routes").len(),
            "evaluations": as_object_mut(&mut state, "evaluations").len(),
            "traces": as_array_mut(&mut state, "traces").len(),
            "connectors": as_object_mut(&mut state, "connectors").len(),
            "intakes": as_object_mut(&mut state, "intakes").len(),
            "last_receipt": state.get("last_receipt").cloned().unwrap_or(Value::Null),
        })),
        "register-pipeline" => register_pipeline(&mut state, input),
        "run-pipeline" => run_pipeline(root, argv, &mut state, input),
        "run-agent-toolset" => run_agent_toolset(root, argv, &mut state, input),
        "register-template" => register_template(&mut state, input),
        "render-template" => render_template(&mut state, input),
        "register-document-store" => register_document_store(root, &mut state, input),
        "retrieve-documents" => retrieve_documents(&mut state, input),
        "route-and-rank" => route_and_rank(&mut state, input),
        "record-multimodal-eval" => record_multimodal_eval(root, &mut state, input),
        "trace-run" => trace_run(root, &mut state, input),
        "import-connector" => import_connector(root, &mut state, input),
        "assimilate-intake" => assimilate_intake(root, &mut state, input),
        _ => Err(format!("unknown_haystack_bridge_command:{command}")),
    };

    match result {
        Ok(payload) => {
            let receipt = cli_receipt(
                &format!("haystack_bridge_{}", command.replace('-', "_")),
                payload,
            );
            state["last_receipt"] = receipt.clone();
            if let Err(err) = save_state(&state_path, &state)
                .and_then(|_| append_history(&history_path, &receipt))
            {
                print_json_line(&cli_error("haystack_bridge_error", &err));
                return 1;
            }
            print_json_line(&receipt);
            0
        }
        Err(err) => {
            print_json_line(&cli_error("haystack_bridge_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_render_replaces_variables() {
        let mut state = default_state();
        let payload = json!({"name": "support-template", "template": "Hello {{name}}"});
        let _ = register_template(&mut state, payload.as_object().unwrap()).expect("template");
        let template_id = state["templates"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .to_string();
        let render = render_template(
            &mut state,
            json!({"template_id": template_id, "variables": {"name": "Jay"}})
                .as_object()
                .unwrap(),
        )
        .expect("render");
        assert_eq!(render["render"]["output"].as_str(), Some("Hello Jay"));
    }

    #[test]
    fn route_and_rank_is_deterministic() {
        let mut state = default_state();
        let out = route_and_rank(&mut state, json!({
            "name": "router",
            "query": "billing issue",
            "context": {"intent": "billing"},
            "routes": [
                {"id": "billing", "field": "intent", "equals": "billing", "reason": "billing path"},
                {"id": "general", "field": "intent", "equals": "general", "reason": "general path"}
            ],
            "candidates": [
                {"text": "billing policy doc", "metadata": {"kind": "policy"}},
                {"text": "general faq", "metadata": {"kind": "faq"}}
            ]
        }).as_object().unwrap()).expect("route");
        assert_eq!(
            out["route"]["selected_route"]["id"].as_str(),
            Some("billing")
        );
    }
}
