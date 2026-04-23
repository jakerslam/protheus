// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

const DEFAULT_STATE_REL: &str = "local/state/ops/camel_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/camel_bridge/history.jsonl";
const DEFAULT_SWARM_STATE_REL: &str = "local/state/ops/camel_bridge/swarm_state.json";
const DEFAULT_OUTPUT_DIR_REL: &str = "client/runtime/local/state/camel-shell";

fn usage() {
    println!("camel-bridge commands:");
    println!("  infring-ops camel-bridge status [--state-path=<path>]");
    println!("  infring-ops camel-bridge register-society [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops camel-bridge run-society [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops camel-bridge simulate-world [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops camel-bridge import-dataset [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops camel-bridge route-conversation [--payload-base64=<json>] [--state-path=<path>] [--swarm-state-path=<path>]");
    println!("  infring-ops camel-bridge record-crab-benchmark [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops camel-bridge register-tool-gateway [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops camel-bridge invoke-tool-gateway [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops camel-bridge record-scaling-observation [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops camel-bridge assimilate-intake [--payload-base64=<json>] [--state-path=<path>]");
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
    lane_utils::payload_json(argv, "camel_bridge")
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    lane_utils::payload_obj(value)
}

fn repo_path(root: &Path, rel: &str) -> PathBuf {
    lane_utils::repo_path(root, rel)
}

fn bridge_path_flag(
    root: &Path,
    argv: &[String],
    payload: &Map<String, Value>,
    flag_name: &str,
    payload_key: &str,
    default_rel: &str,
) -> PathBuf {
    lane_utils::path_flag(root, argv, payload, flag_name, payload_key, default_rel)
}

fn rel(root: &Path, path: &Path) -> String {
    lane_utils::rel_path(root, path)
}

fn state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path_flag(
        root,
        argv,
        payload,
        "state-path",
        "state_path",
        DEFAULT_STATE_REL,
    )
}

fn history_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path_flag(
        root,
        argv,
        payload,
        "history-path",
        "history_path",
        DEFAULT_HISTORY_REL,
    )
}

fn swarm_state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path_flag(
        root,
        argv,
        payload,
        "swarm-state-path",
        "swarm_state_path",
        DEFAULT_SWARM_STATE_REL,
    )
}

fn default_state() -> Value {
    json!({
        "schema_version": "camel_bridge_state_v1",
        "societies": {},
        "society_runs": {},
        "world_simulations": {},
        "datasets": {},
        "conversation_routes": {},
        "benchmarks": {},
        "tool_gateways": {},
        "tool_invocations": {},
        "scaling_observations": {},
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
        "societies",
        "society_runs",
        "world_simulations",
        "datasets",
        "conversation_routes",
        "benchmarks",
        "tool_gateways",
        "tool_invocations",
        "scaling_observations",
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
        value["schema_version"] = json!("camel_bridge_state_v1");
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

fn stable_id(prefix: &str, basis: &Value) -> String {
    lane_utils::stable_id(prefix, basis)
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    crate::contract_lane_utils::clean_text(raw, max_len)
}

fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    lane_utils::clean_token(raw, fallback)
}

fn parse_u64_value(value: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    lane_utils::json_u64_coerce(value, fallback, min, max)
}

fn parse_f64_value(value: Option<&Value>, fallback: f64, min: f64, max: f64) -> f64 {
    lane_utils::json_f64_coerce(value, fallback, min, max)
}

fn parse_bool_value(value: Option<&Value>, fallback: bool) -> bool {
    lane_utils::json_bool_coerce(value, fallback)
}

fn clean_profiles(value: Option<&Value>, fallback: &[&str]) -> Vec<String> {
    let mut rows = Vec::new();
    if let Some(array) = value.and_then(Value::as_array) {
        for row in array {
            let token = clean_token(row.as_str(), "");
            if !token.is_empty() && !rows.contains(&token) {
                rows.push(token);
            }
        }
    }
    if rows.is_empty() {
        return fallback.iter().map(|row| row.to_string()).collect();
    }
    rows
}

fn parse_profile(payload: &Map<String, Value>) -> String {
    clean_token(payload.get("profile").and_then(Value::as_str), "rich")
}

fn constrained_profile(profile: &str) -> bool {
    matches!(profile, "pure" | "tiny-max" | "tiny_max")
}

fn safe_bridge_path(path: &str) -> bool {
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

fn safe_output_prefix(path: &str) -> bool {
    path.starts_with("client/runtime/local/state/") || path.starts_with("apps/")
}

fn claim(id: &str, detail: &str) -> Value {
    json!([{
        "id": id,
        "detail": detail,
    }])
}

fn ok_with_claim(field: &str, value: Value, claim_id: &str, claim_detail: &str) -> Value {
    let mut row = Map::new();
    row.insert("ok".to_string(), Value::Bool(true));
    row.insert(field.to_string(), value);
    row.insert("claim_evidence".to_string(), claim(claim_id, claim_detail));
    Value::Object(row)
}

fn store_receipt(
    state_path: &Path,
    history_path: &Path,
    state: &mut Value,
    receipt: &Value,
) -> Result<(), String> {
    state["last_receipt"] = receipt.clone();
    save_state(state_path, state)?;
    append_history(history_path, receipt)
}

fn status_payload(state: &Value, state_path: &Path, history_path: &Path) -> Value {
    json!({
        "ok": true,
        "schema_version": state.get("schema_version").and_then(Value::as_str).unwrap_or("camel_bridge_state_v1"),
        "societies": state.get("societies").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "society_runs": state.get("society_runs").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "world_simulations": state.get("world_simulations").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "datasets": state.get("datasets").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "conversation_routes": state.get("conversation_routes").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "benchmarks": state.get("benchmarks").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "tool_gateways": state.get("tool_gateways").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "tool_invocations": state.get("tool_invocations").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "scaling_observations": state.get("scaling_observations").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "intakes": state.get("intakes").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "state_path": rel(Path::new("."), state_path),
        "history_path": rel(Path::new("."), history_path),
        "last_receipt_hash": state.get("last_receipt").and_then(|row| row.get("receipt_hash")).and_then(Value::as_str).unwrap_or(""),
    })
}
