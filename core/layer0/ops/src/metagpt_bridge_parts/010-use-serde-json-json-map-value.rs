// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils::{
    self as lane_utils, clean_text, clean_token, cli_error, cli_receipt,
    normalize_bridge_path_clean, payload_obj, print_json_line, rel_path as rel,
};
use crate::{deterministic_receipt_hash, now_iso};

const DEFAULT_STATE_REL: &str = "local/state/ops/metagpt_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/metagpt_bridge/history.jsonl";
const DEFAULT_APPROVAL_QUEUE_REL: &str = "client/runtime/local/state/metagpt_review_queue.yaml";
const DEFAULT_TRACE_REL: &str = "local/state/ops/metagpt_bridge/pipeline_trace.jsonl";

fn usage() {
    println!("metagpt-bridge commands:");
    println!("  infring-ops metagpt-bridge status [--state-path=<path>]");
    println!("  infring-ops metagpt-bridge register-company [--payload-base64=<json>] [--state-path=<path>]");
    println!(
        "  infring-ops metagpt-bridge run-sop [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!(
        "  infring-ops metagpt-bridge simulate-pr [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!(
        "  infring-ops metagpt-bridge run-debate [--payload-base64=<json>] [--state-path=<path>]"
    );
    println!("  infring-ops metagpt-bridge plan-requirements [--payload-base64=<json>] [--state-path=<path>]");
    println!("  infring-ops metagpt-bridge record-oversight [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  infring-ops metagpt-bridge record-pipeline-trace [--payload-base64=<json>] [--state-path=<path>] [--trace-path=<path>]");
    println!("  infring-ops metagpt-bridge ingest-config [--payload-base64=<json>] [--state-path=<path>]");
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    lane_utils::payload_json(argv, "metagpt_bridge")
}

fn bridge_path_flag(
    root: &Path,
    argv: &[String],
    payload: &Map<String, Value>,
    flag: &str,
    payload_key: &str,
    default_rel: &str,
) -> PathBuf {
    lane_utils::path_flag(root, argv, payload, flag, payload_key, default_rel)
}

fn state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path_flag(root, argv, payload, "state-path", "state_path", DEFAULT_STATE_REL)
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

fn approval_queue_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path_flag(
        root,
        argv,
        payload,
        "approval-queue-path",
        "approval_queue_path",
        DEFAULT_APPROVAL_QUEUE_REL,
    )
}

fn trace_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    bridge_path_flag(
        root,
        argv,
        payload,
        "trace-path",
        "trace_path",
        DEFAULT_TRACE_REL,
    )
}

fn default_state() -> Value {
    json!({
        "schema_version": "metagpt_bridge_state_v1",
        "companies": {},
        "sop_runs": {},
        "pr_simulations": {},
        "debates": {},
        "requirements": {},
        "oversight": {},
        "traces": [],
        "configs": {},
        "last_receipt": null,
    })
}

fn ensure_state_shape(value: &mut Value) {
    if !value.is_object() {
        *value = default_state();
        return;
    }
    for key in [
        "companies",
        "sop_runs",
        "pr_simulations",
        "debates",
        "requirements",
        "oversight",
        "configs",
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
        value["schema_version"] = json!("metagpt_bridge_state_v1");
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
        .map(|d| d.as_millis())
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
    let digest = crate::deterministic_receipt_hash(basis);
    format!("{prefix}_{}_{}", to_base36(now_millis()), &digest[..12])
}

fn claim(id: &str, claim: &str) -> Value {
    json!([{"id": id, "claim": claim}])
}
fn profile(raw: Option<&Value>) -> String {
    clean_token(raw.and_then(Value::as_str), "rich")
}

fn normalize_bridge_path(root: &Path, raw: &str) -> Result<String, String> {
    normalize_bridge_path_clean(root, raw, "metagpt_bridge_path_outside_allowed_surface")
}

fn register_company(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let company_name = clean_text(payload.get("company_name").and_then(Value::as_str), 120);
    if company_name.is_empty() {
        return Err("metagpt_company_name_required".to_string());
    }
    let roles = payload
        .get("roles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if roles.is_empty() {
        return Err("metagpt_roles_required".to_string());
    }
    let company = json!({
        "company_id": stable_id("mgcompany", &json!({"company_name": company_name, "roles": roles})),
        "company_name": company_name,
        "product_goal": clean_text(payload.get("product_goal").and_then(Value::as_str), 160),
        "roles": roles,
        "org_chart": payload.get("org_chart").cloned().unwrap_or_else(|| json!(["ceo", "cto", "pm", "engineer"])),
        "registered_at": now_iso(),
    });
    let id = company
        .get("company_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "companies").insert(id, company.clone());
    Ok(
        json!({"ok": true, "company": company, "claim_evidence": claim("V6-WORKFLOW-006.1", "metagpt_company_roles_are_registered_on_governed_workflow_swarm_and_persona_lanes")}),
    )
}
