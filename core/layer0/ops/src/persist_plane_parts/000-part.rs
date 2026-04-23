// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::persist_plane (authoritative)

use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_plane_conduit_enforcement, conduit_bypass_requested,
    emit_plane_receipt, load_json_or, parse_bool, parse_u64, plane_status, print_json, read_json,
    scoped_state_root, sha256_hex_str, write_json,
};
use crate::{clean, parse_args};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "PERSIST_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "persist_plane";

const SCHEDULE_CONTRACT_PATH: &str = "planes/contracts/persist/schedule_contract_v1.json";
const MOBILE_CONTRACT_PATH: &str = "planes/contracts/persist/mobile_cockpit_contract_v1.json";
const CONTINUITY_CONTRACT_PATH: &str = "planes/contracts/persist/continuity_contract_v1.json";
const CONNECTOR_CONTRACT_PATH: &str =
    "planes/contracts/persist/connector_onboarding_contract_v1.json";
const COWORK_CONTRACT_PATH: &str = "planes/contracts/persist/cowork_background_contract_v1.json";
const MOBILE_DAEMON_CONTRACT_PATH: &str =
    "planes/contracts/mobile/mobile_daemon_bitnet_contract_v1.json";

#[path = "persist_plane_connector.rs"]
mod persist_plane_connector;
#[path = "persist_plane_continuity.rs"]
mod persist_plane_continuity;
#[path = "persist_plane_cowork.rs"]
mod persist_plane_cowork;

use persist_plane_connector::run_connector;
use persist_plane_continuity::run_continuity;
use persist_plane_cowork::run_cowork;

fn usage() {
    println!("Usage:");
    println!("  infring-ops persist-plane status");
    println!(
        "  infring-ops persist-plane schedule --op=<upsert|list|kickoff> [--job=<id>] [--cron=<expr>] [--workflow=<id>] [--owner=<id>] [--strict=1|0]"
    );
    println!(
        "  infring-ops persist-plane mobile-cockpit --op=<publish|status|intervene> [--session-id=<id>] [--device=<id>] [--action=<pause|resume|abort>] [--strict=1|0]"
    );
    println!(
        "  infring-ops persist-plane continuity --op=<checkpoint|reconstruct|status|validate> [--session-id=<id>] [--context-json=<json>] [--strict=1|0]"
    );
    println!(
        "  infring-ops persist-plane connector --op=<add|list|status|remove> [--provider=<slack|gmail|drive>] [--policy-template=<id>] [--strict=1|0]"
    );
    println!(
        "  infring-ops persist-plane cowork --op=<delegate|tick|status|list> [--task=<text>] [--parent=<id>] [--child=<id>] [--mode=<co-work|sub-agent>] [--budget-ms=<n>] [--strict=1|0]"
    );
    println!(
        "  infring-ops persist-plane mobile-daemon --op=<enable|status|handoff> [--platform=<android|ios>] [--edge-backend=<bitnet>] [--sensor-lanes=<camera,mic,gps>] [--handoff=<edge|cloud>] [--strict=1|0]"
    );
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(root, STATE_ENV, STATE_SCOPE, "persist_plane_error", payload)
}

fn status(root: &Path) -> Value {
    plane_status(root, STATE_ENV, STATE_SCOPE, "persist_plane_status")
}

fn claim_ids_for_action(action: &str) -> Vec<&'static str> {
    match action {
        "schedule" => vec!["V6-PERSIST-001.1", "V6-PERSIST-001.6"],
        "mobile-cockpit" => vec!["V6-PERSIST-001.2", "V6-PERSIST-001.6"],
        "continuity" => vec!["V6-PERSIST-001.3", "V6-PERSIST-001.6"],
        "connector" => vec!["V6-PERSIST-001.4", "V6-PERSIST-001.6"],
        "cowork" | "co-work" => vec!["V6-PERSIST-001.5", "V6-PERSIST-001.6"],
        "mobile-daemon" => vec!["V7-MOBILE-001.1", "V6-PERSIST-001.6"],
        _ => vec!["V6-PERSIST-001.6"],
    }
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    let claim_ids = claim_ids_for_action(action);
    build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "persist_conduit_enforcement",
        "core/layer0/ops/persist_plane",
        bypass_requested,
        "persist_controls_route_through_layer0_conduit_with_fail_closed_denials",
        &claim_ids,
    )
}

fn persist_state_path(root: &Path, parts: &[&str]) -> PathBuf {
    let mut path = state_root(root);
    for part in parts {
        path.push(part);
    }
    path
}

fn schedules_path(root: &Path) -> PathBuf {
    persist_state_path(root, &["schedules", "registry.json"])
}

fn mobile_path(root: &Path) -> PathBuf {
    persist_state_path(root, &["mobile", "latest.json"])
}

fn continuity_dir(root: &Path) -> PathBuf {
    persist_state_path(root, &["continuity"])
}

fn continuity_snapshot_path(root: &Path, session_id: &str) -> PathBuf {
    continuity_dir(root)
        .join("snapshots")
        .join(format!("{session_id}.json"))
}

fn continuity_reconstruct_path(root: &Path, session_id: &str) -> PathBuf {
    continuity_dir(root)
        .join("reconstructed")
        .join(format!("{session_id}.json"))
}

fn connectors_path(root: &Path) -> PathBuf {
    persist_state_path(root, &["connectors", "registry.json"])
}

fn cowork_path(root: &Path) -> PathBuf {
    persist_state_path(root, &["cowork", "runs.json"])
}

fn mobile_daemon_path(root: &Path) -> PathBuf {
    persist_state_path(root, &["mobile", "daemon_profile.json"])
}

fn parse_json_flag(raw: Option<&String>) -> Option<Value> {
    raw.and_then(|text| serde_json::from_str::<Value>(text).ok())
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

fn run_schedule(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        SCHEDULE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "persist_schedule_contract",
            "allowed_ops": ["upsert", "list", "kickoff"]
        }),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "list".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let allowed_ops = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if strict
        && !allowed_ops
            .iter()
            .filter_map(Value::as_str)
            .any(|row| row == op)
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "persist_plane_schedule",
            "errors": ["persist_schedule_op_invalid"]
        });
    }

    let path = schedules_path(root);
    let mut state = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "jobs": {},
            "runs": []
        })
    });
    if !state.get("jobs").map(Value::is_object).unwrap_or(false) {
        state["jobs"] = Value::Object(serde_json::Map::new());
    }
    if !state.get("runs").map(Value::is_array).unwrap_or(false) {
        state["runs"] = Value::Array(Vec::new());
    }

    if op == "list" {
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "persist_plane_schedule",
            "lane": "core/layer0/ops",
            "op": "list",
            "state": state,
            "claim_evidence": [
                {
                    "id": "V6-PERSIST-001.1",
                    "claim": "scheduled_background_task_lane_surfaces_registered_jobs",
                    "evidence": {
                        "job_count": state
                            .get("jobs")
                            .and_then(Value::as_object)
                            .map(|m| m.len())
                            .unwrap_or(0)
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    let job_id = clean_id(
        parsed
            .flags
            .get("job")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("job-id").map(String::as_str)),
        "default-job",
    );
    if op == "upsert" {
        let cron = clean(
            parsed
                .flags
                .get("cron")
                .cloned()
                .unwrap_or_else(|| "*/5 * * * *".to_string()),
            160,
        );
        let workflow = clean(
            parsed
                .flags
                .get("workflow")
                .cloned()
                .unwrap_or_else(|| "default-workflow".to_string()),
            120,
        );
        let owner = clean(
            parsed
                .flags
                .get("owner")
                .cloned()
                .unwrap_or_else(|| "system".to_string()),
            120,
        );
        let job = json!({
            "job_id": job_id,
            "cron": cron,
            "workflow": workflow,
            "owner": owner,
            "updated_at": crate::now_iso()
        });
        state["jobs"][&job_id] = job.clone();
        state["updated_at"] = Value::String(crate::now_iso());
        let _ = write_json(&path, &state);
        let _ = append_jsonl(
            &state_root(root).join("schedules").join("history.jsonl"),
            &json!({"op":"upsert","job_id":job_id,"ts":crate::now_iso()}),
        );
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "persist_plane_schedule",
            "lane": "core/layer0/ops",
            "op": "upsert",
            "job": job,
            "artifact": {
                "path": path.display().to_string(),
                "sha256": sha256_hex_str(&state.to_string())
            },
            "claim_evidence": [
                {
                    "id": "V6-PERSIST-001.1",
                    "claim": "schedule_contract_supports_receipted_recurring_background_workflows",
                    "evidence": {
                        "job_id": job_id
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if strict && state["jobs"].get(&job_id).is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "persist_plane_schedule",
            "errors": ["persist_schedule_job_not_found"]
        });
    }
    let run_id = format!(
        "kickoff_{}",
        &sha256_hex_str(&format!("{job_id}:{}", crate::now_iso()))[..10]
    );
    let run = json!({
        "run_id": run_id,
        "job_id": job_id,
        "status": "started",
        "ts": crate::now_iso()
    });
    let mut runs = state
        .get("runs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    runs.push(run.clone());
    state["runs"] = Value::Array(runs);
    state["updated_at"] = Value::String(crate::now_iso());
    let _ = write_json(&path, &state);
    let _ = append_jsonl(
        &state_root(root).join("schedules").join("history.jsonl"),
        &json!({"op":"kickoff","run":run,"ts":crate::now_iso()}),
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "persist_plane_schedule",
        "lane": "core/layer0/ops",
        "op": "kickoff",
        "run": run,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-PERSIST-001.1",
                "claim": "scheduled_background_runtime_kickoff_is_receipted",
                "evidence": {
                    "run_id": run_id
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_mobile_cockpit(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        MOBILE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "persist_mobile_cockpit_contract",
            "allowed_ops": ["publish", "status", "intervene"],
            "allowed_actions": ["pause", "resume", "abort"]
        }),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let allowed_ops = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if strict
        && !allowed_ops
            .iter()
            .filter_map(Value::as_str)
            .any(|row| row == op)
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "persist_plane_mobile_cockpit",
            "errors": ["persist_mobile_cockpit_op_invalid"]
        });
