// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::observability_plane (authoritative)

use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_conduit_enforcement, conduit_bypass_requested,
    load_json_or, parse_bool, read_json, scoped_state_root, sha256_hex_str, write_json,
    write_receipt,
};
use crate::{clean, parse_args};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "OBSERVABILITY_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "observability_plane";

const MONITORING_CONTRACT_PATH: &str =
    "planes/contracts/observability/realtime_monitoring_contract_v1.json";
const WORKFLOW_CONTRACT_PATH: &str =
    "planes/contracts/observability/workflow_editor_contract_v1.json";
const INCIDENT_CONTRACT_PATH: &str =
    "planes/contracts/observability/incident_response_contract_v1.json";
const SELFHOST_CONTRACT_PATH: &str =
    "planes/contracts/observability/self_hosted_deploy_contract_v1.json";
const ACP_PROVENANCE_CONTRACT_PATH: &str =
    "planes/contracts/observability/acp_provenance_contract_v1.json";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops observability-plane status");
    println!(
        "  protheus-ops observability-plane monitor [--source=<id>] [--alert-class=<slo|security|runtime|cost>] [--severity=<low|medium|high|critical>] [--message=<text>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops observability-plane workflow --op=<upsert|list|run> [--workflow-id=<id>] [--trigger=<cron|event>] [--schedule=<expr>] [--steps-json=<json>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops observability-plane incident --op=<trigger|status|resolve> [--incident-id=<id>] [--runbook=<id>] [--action=<text>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops observability-plane selfhost --op=<deploy|status> [--profile=<docker-local|k8s-local>] [--telemetry-opt-in=0|1] [--strict=1|0]"
    );
    println!(
        "  protheus-ops observability-plane acp-provenance --op=<enable|status|trace|debug> [--source-agent=<id>] [--target-agent=<id>] [--intent=<text>] [--message=<text>] [--trace-id=<id>] [--visibility-mode=<off|meta|meta+receipt>] [--strict=1|0]"
    );
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn print_payload(payload: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn emit(root: &Path, payload: Value) -> i32 {
    match write_receipt(root, STATE_ENV, STATE_SCOPE, payload) {
        Ok(out) => {
            print_payload(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_payload(&json!({
                "ok": false,
                "type": "observability_plane_error",
                "error": clean(err, 240)
            }));
            1
        }
    }
}

fn status(root: &Path) -> Value {
    json!({
        "ok": true,
        "type": "observability_plane_status",
        "lane": "core/layer0/ops",
        "latest_path": latest_path(root).display().to_string(),
        "latest": read_json(&latest_path(root))
    })
}

fn claim_ids_for_action(action: &str) -> Vec<&'static str> {
    match action {
        "monitor" => vec!["V6-OBSERVABILITY-001.1", "V6-OBSERVABILITY-001.5"],
        "workflow" => vec!["V6-OBSERVABILITY-001.2", "V6-OBSERVABILITY-001.5"],
        "incident" => vec!["V6-OBSERVABILITY-001.3", "V6-OBSERVABILITY-001.5"],
        "selfhost" => vec!["V6-OBSERVABILITY-001.4", "V6-OBSERVABILITY-001.5"],
        "acp-provenance" => vec![
            "V6-OBSERVABILITY-005.7",
            "V6-OBSERVABILITY-005.8",
            "V6-OBSERVABILITY-005.9",
            "V6-OBSERVABILITY-005.10",
            "V6-OBSERVABILITY-005.11",
        ],
        _ => vec!["V6-OBSERVABILITY-001.5"],
    }
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    let claim_rows = claim_ids_for_action(action)
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "claim": "observability_controls_route_through_layer0_conduit_with_fail_closed_denials",
                "evidence": {
                    "action": clean(action, 120),
                    "bypass_requested": bypass_requested
                }
            })
        })
        .collect::<Vec<_>>();
    build_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "observability_conduit_enforcement",
        "core/layer0/ops/observability_plane",
        bypass_requested,
        claim_rows,
    )
}

fn alerts_state_path(root: &Path) -> PathBuf {
    state_root(root).join("alerts").join("latest.json")
}

fn workflows_state_path(root: &Path) -> PathBuf {
    state_root(root).join("workflows").join("registry.json")
}

fn incidents_state_path(root: &Path) -> PathBuf {
    state_root(root).join("incidents").join("active.json")
}

fn incident_artifacts_dir(root: &Path, incident_id: &str) -> PathBuf {
    state_root(root)
        .join("incidents")
        .join("artifacts")
        .join(incident_id)
}

fn selfhost_state_path(root: &Path) -> PathBuf {
    state_root(root).join("deploy").join("latest.json")
}

fn selfhost_health_path(root: &Path) -> PathBuf {
    state_root(root).join("deploy").join("health.json")
}

fn provenance_config_path(root: &Path) -> PathBuf {
    state_root(root).join("provenance").join("config.json")
}

fn provenance_history_path(root: &Path) -> PathBuf {
    state_root(root).join("provenance").join("traces.jsonl")
}

fn provenance_latest_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("provenance")
        .join("latest_trace.json")
}

fn parse_visibility_mode(raw: Option<String>) -> String {
    let mode = clean(raw.unwrap_or_else(|| "meta+receipt".to_string()), 32).to_ascii_lowercase();
    match mode.as_str() {
        "off" | "meta" | "meta+receipt" => mode,
        _ => "meta+receipt".to_string(),
    }
}

fn visible_trace_payload(entry: &Value, mode: &str) -> Value {
    if mode == "off" {
        return json!({
            "trace_id": entry.get("trace_id").cloned().unwrap_or(Value::Null),
            "hop_index": entry.get("hop_index").cloned().unwrap_or(Value::Null),
            "visibility_mode": mode
        });
    }
    if mode == "meta" {
        return json!({
            "trace_id": entry.get("trace_id").cloned().unwrap_or(Value::Null),
            "hop_index": entry.get("hop_index").cloned().unwrap_or(Value::Null),
            "source_agent": entry.get("source_agent").cloned().unwrap_or(Value::Null),
            "target_agent": entry.get("target_agent").cloned().unwrap_or(Value::Null),
            "intent": entry.get("intent").cloned().unwrap_or(Value::Null),
            "ts": entry.get("ts").cloned().unwrap_or(Value::Null),
            "visibility_mode": mode
        });
    }
    json!({
        "trace_id": entry.get("trace_id").cloned().unwrap_or(Value::Null),
        "hop_index": entry.get("hop_index").cloned().unwrap_or(Value::Null),
        "source_agent": entry.get("source_agent").cloned().unwrap_or(Value::Null),
        "target_agent": entry.get("target_agent").cloned().unwrap_or(Value::Null),
        "intent": entry.get("intent").cloned().unwrap_or(Value::Null),
        "message": entry.get("message").cloned().unwrap_or(Value::Null),
        "ts": entry.get("ts").cloned().unwrap_or(Value::Null),
        "hop_hash": entry.get("hop_hash").cloned().unwrap_or(Value::Null),
        "previous_hop_hash": entry.get("previous_hop_hash").cloned().unwrap_or(Value::Null),
        "receipt_hash": entry.get("receipt_hash").cloned().unwrap_or(Value::Null),
        "visibility_mode": mode
    })
}

fn clean_id(raw: Option<&str>, fallback: &str) -> String {
    let mut out = String::new();
    if let Some(v) = raw {
        for ch in v.trim().chars() {
            if out.len() >= 96 {
                break;
            }
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push('-');
            }
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_json_flag(raw: Option<&String>, fallback: Value) -> Value {
    if let Some(value) = raw {
        if let Ok(parsed) = serde_json::from_str::<Value>(value) {
            return parsed;
        }
    }
    fallback
}

fn looks_like_cron(expr: &str) -> bool {
    expr.split_whitespace().count() == 5
}

fn split_actions(raw: &str) -> Vec<String> {
    raw.split(['+', ','])
        .map(|row| clean(row, 80).to_ascii_lowercase())
        .filter(|row| !row.is_empty())
        .collect()
}

fn compile_steps_graph(step_names: &[String]) -> Value {
    let nodes = step_names
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            json!({
                "id": format!("step-{idx}"),
                "name": clean(name, 120),
                "kind": "workflow_step"
            })
        })
        .collect::<Vec<_>>();
    let edges = (1..step_names.len())
        .map(|idx| {
            json!({
                "from": format!("step-{}", idx - 1),
                "to": format!("step-{idx}")
            })
        })
        .collect::<Vec<_>>();
    json!({
        "nodes": nodes,
        "edges": edges
    })
}

fn intelligent_context(root: &Path) -> Value {
    let company_feed = read_json(
        &root
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("company_plane")
            .join("heartbeat")
            .join("remote_feed.json"),
    );
    let substrate_latest = read_json(
        &root
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("substrate_plane")
            .join("latest.json"),
    );
    let persist_mobile = read_json(
        &root
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("persist_plane")
            .join("mobile")
            .join("latest.json"),
    );
    json!({
        "company_heartbeat": company_feed
            .and_then(|v| v.get("teams").cloned())
            .unwrap_or_else(|| json!({})),
        "substrate_feedback_mode": substrate_latest
            .as_ref()
            .and_then(|v| v.get("feedback"))
            .and_then(|v| v.get("mode"))
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        "persist_mobile_connected": persist_mobile
            .as_ref()
            .and_then(|v| v.get("connected"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "snapshot_at": crate::now_iso()
    })
}

