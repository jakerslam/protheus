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

const DEFAULT_STATE_REL: &str = "local/state/ops/shannon_bridge/latest.json";
const DEFAULT_HISTORY_REL: &str = "local/state/ops/shannon_bridge/history.jsonl";
const DEFAULT_APPROVAL_QUEUE_REL: &str = "client/runtime/local/state/shannon_approvals.yaml";
const DEFAULT_REPLAY_DIR_REL: &str = "local/state/ops/shannon_bridge/replays";
const DEFAULT_OBSERVABILITY_TRACE_REL: &str = "local/state/ops/shannon_bridge/observability.jsonl";
const DEFAULT_OBSERVABILITY_METRICS_REL: &str = "local/state/ops/shannon_bridge/metrics.prom";
const DEFAULT_DESKTOP_HISTORY_REL: &str = "client/runtime/local/state/shannon_desktop_shell.json";

fn usage() {
    println!("shannon-bridge commands:");
    println!("  protheus-ops shannon-bridge status [--state-path=<path>]");
    println!("  protheus-ops shannon-bridge register-pattern [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops shannon-bridge guard-budget [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops shannon-bridge memory-bridge [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops shannon-bridge replay-run [--payload-base64=<json>] [--state-path=<path>] [--replay-dir=<path>]");
    println!("  protheus-ops shannon-bridge approval-checkpoint [--payload-base64=<json>] [--state-path=<path>] [--approval-queue-path=<path>]");
    println!("  protheus-ops shannon-bridge sandbox-execute [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops shannon-bridge record-observability [--payload-base64=<json>] [--state-path=<path>] [--observability-trace-path=<path>] [--observability-metrics-path=<path>]");
    println!("  protheus-ops shannon-bridge gateway-route [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops shannon-bridge register-tooling [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops shannon-bridge schedule-run [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops shannon-bridge desktop-shell [--payload-base64=<json>] [--state-path=<path>] [--desktop-history-path=<path>]");
    println!("  protheus-ops shannon-bridge p2p-reliability [--payload-base64=<json>] [--state-path=<path>]");
    println!("  protheus-ops shannon-bridge assimilate-intake [--payload-base64=<json>] [--state-path=<path>]");
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
            .map_err(|err| format!("shannon_bridge_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("shannon_bridge_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("shannon_bridge_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("shannon_bridge_payload_decode_failed:{err}"));
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

fn path_flag(
    root: &Path,
    argv: &[String],
    payload: &Map<String, Value>,
    flag: &str,
    payload_key: &str,
    default_rel: &str,
) -> PathBuf {
    lane_utils::parse_flag(argv, flag, false)
        .or_else(|| {
            payload
                .get(payload_key)
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|raw| repo_path(root, &raw))
        .unwrap_or_else(|| root.join(default_rel))
}

fn state_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    path_flag(
        root,
        argv,
        payload,
        "state-path",
        "state_path",
        DEFAULT_STATE_REL,
    )
}

fn history_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    path_flag(
        root,
        argv,
        payload,
        "history-path",
        "history_path",
        DEFAULT_HISTORY_REL,
    )
}

fn approval_queue_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    path_flag(
        root,
        argv,
        payload,
        "approval-queue-path",
        "approval_queue_path",
        DEFAULT_APPROVAL_QUEUE_REL,
    )
}

fn replay_dir(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    path_flag(
        root,
        argv,
        payload,
        "replay-dir",
        "replay_dir",
        DEFAULT_REPLAY_DIR_REL,
    )
}

fn observability_trace_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    path_flag(
        root,
        argv,
        payload,
        "observability-trace-path",
        "observability_trace_path",
        DEFAULT_OBSERVABILITY_TRACE_REL,
    )
}

fn observability_metrics_path(
    root: &Path,
    argv: &[String],
    payload: &Map<String, Value>,
) -> PathBuf {
    path_flag(
        root,
        argv,
        payload,
        "observability-metrics-path",
        "observability_metrics_path",
        DEFAULT_OBSERVABILITY_METRICS_REL,
    )
}

fn desktop_history_path(root: &Path, argv: &[String], payload: &Map<String, Value>) -> PathBuf {
    path_flag(
        root,
        argv,
        payload,
        "desktop-history-path",
        "desktop_history_path",
        DEFAULT_DESKTOP_HISTORY_REL,
    )
}

fn default_state() -> Value {
    json!({
        "schema_version": "shannon_bridge_state_v1",
        "patterns": {},
        "budget_guards": {},
        "memory_workspaces": {},
        "replays": {},
        "approvals": {},
        "sandbox_runs": {},
        "observability": {},
        "gateway_routes": {},
        "tool_registrations": {},
        "schedules": {},
        "desktop_events": {},
        "p2p_reliability": {},
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
        "patterns",
        "budget_guards",
        "memory_workspaces",
        "replays",
        "approvals",
        "sandbox_runs",
        "observability",
        "gateway_routes",
        "tool_registrations",
        "schedules",
        "desktop_events",
        "p2p_reliability",
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
        value["schema_version"] = json!("shannon_bridge_state_v1");
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
    let digest = deterministic_receipt_hash(basis);
    format!("{prefix}_{}_{}", to_base36(now_millis()), &digest[..12])
}

fn clean_text(raw: Option<&str>, max_len: usize) -> String {
    lane_utils::clean_text(raw, max_len)
}
fn clean_token(raw: Option<&str>, fallback: &str) -> String {
    lane_utils::clean_token(raw, fallback)
}
fn profile(raw: Option<&Value>) -> String {
    clean_token(raw.and_then(Value::as_str), "rich")
}
fn claim(id: &str, claim: &str) -> Value {
    json!([{"id": id, "claim": claim}])
}

fn parse_u64(raw: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    raw.and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_bool(raw: Option<&Value>, fallback: bool) -> bool {
    raw.and_then(Value::as_bool).unwrap_or(fallback)
}

fn normalize_surface_path(
    root: &Path,
    raw: &str,
    allowed_prefixes: &[&str],
) -> Result<String, String> {
    let clean = clean_text(Some(raw), 260);
    if !allowed_prefixes
        .iter()
        .any(|prefix| clean.starts_with(prefix))
    {
        return Err("shannon_bridge_path_outside_allowed_surface".to_string());
    }
    Ok(rel(root, &repo_path(root, &clean)))
}

fn looks_like_cron(expr: &str) -> bool {
    let clean = expr.trim();
    if clean.is_empty() {
        return false;
    }
    if matches!(clean, "@hourly" | "@daily" | "@weekly") {
        return true;
    }
    clean.split_whitespace().count() == 5
}

fn record_pattern(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let pattern_name = clean_text(payload.get("pattern_name").and_then(Value::as_str), 120);
    if pattern_name.is_empty() {
        return Err("shannon_pattern_name_required".to_string());
    }
    let strategies = payload
        .get("strategies")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if strategies.is_empty() {
        return Err("shannon_pattern_strategies_required".to_string());
    }
    let pattern_profile = profile(payload.get("profile"));
    let allowed_parallelism = match pattern_profile.as_str() {
        "tiny-max" => 1,
        "pure" => 2,
        _ => parse_u64(payload.get("max_parallelism"), 4, 1, 16),
    };
    let record = json!({
        "pattern_id": stable_id("shpattern", &json!({"pattern_name": pattern_name, "strategies": strategies})),
        "pattern_name": pattern_name,
        "strategies": strategies,
        "stages": payload.get("stages").cloned().unwrap_or_else(|| json!(["plan", "route", "execute", "review"])),
        "handoff_graph": payload.get("handoff_graph").cloned().unwrap_or_else(|| json!([])),
        "profile": pattern_profile,
        "allowed_parallelism": allowed_parallelism,
        "registered_at": now_iso(),
    });
    let id = record
        .get("pattern_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "patterns").insert(id, record.clone());
    Ok(json!({
        "ok": true,
        "pattern": record,
        "claim_evidence": claim("V6-WORKFLOW-001.1", "shannon_orchestration_patterns_register_on_governed_workflow_and_swarm_lanes")
    }))
}

