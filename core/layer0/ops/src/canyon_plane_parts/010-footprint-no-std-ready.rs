// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::canyon_plane (authoritative)
#[path = "../canyon_plane_extensions.rs"]
mod canyon_plane_extensions;
use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_conduit_enforcement, conduit_bypass_requested,
    deterministic_merkle_root, history_path, latest_path, parse_bool, parse_u64, read_json,
    scoped_state_root, sha256_hex_str, write_json,
};
use crate::{clean, core_state_root, enterprise_hardening, now_iso, parse_args};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const LANE_ID: &str = "canyon_plane";
const ENV_KEY: &str = "INFRING_CANYON_PLANE_STATE_ROOT";

pub(crate) fn footprint_no_std_ready(default_empty: bool, source_body: &str) -> bool {
    if default_empty {
        return true;
    }
    let mut in_block_comment = false;
    for line in source_body.lines().take(80) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if in_block_comment {
            if trimmed.contains("*/") {
                in_block_comment = false;
            }
            continue;
        }
        if trimmed.starts_with("/*") {
            if !trimmed.contains("*/") {
                in_block_comment = true;
            }
            continue;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("#![no_std]") {
            return true;
        }
        if trimmed.starts_with("#![")
            && trimmed.contains("cfg_attr")
            && trimmed.contains("no_std")
        {
            return true;
        }
    }
    false
}

fn usage() {
    println!("Usage:");
    println!("  infring-ops canyon-plane efficiency [--strict=1|0] [--binary-path=<path>] [--idle-memory-mb=<n>] [--concurrent-agents=<n>]");
    println!("  infring-ops canyon-plane hands-army [--op=bootstrap|schedule|run|status] [--hand-id=<id>] [--cron=<expr>] [--trigger=cron|event|importance] [--strict=1|0]");
    println!("  infring-ops canyon-plane evolution [--op=propose|shadow-simulate|review|apply|rollback|status] [--proposal-id=<id>] [--kind=<id>] [--description=<text>] [--approved=1|0] [--strict=1|0]");
    println!("  infring-ops canyon-plane sandbox [--op=run|status|snapshot|resume] [--session-id=<id>] [--snapshot-id=<id>] [--tier=native|wasm|firecracker] [--language=python|ts|go|rust] [--fuel=<n>] [--epoch=<n>] [--logical-only=1|0] [--escape-attempt=1|0] [--strict=1|0]");
    println!("  infring-ops canyon-plane ecosystem [--op=bootstrap|status|init|marketplace-status|marketplace-publish|marketplace-install] [--target-dir=<path>] [--sdk=python|typescript|go|rust] [--template=<id>] [--workspace-mode=infring|pure] [--pure=1|0] [--tiny-max=1|0] [--dry-run=1|0] [--hand-id=<id>] [--receipt-file=<path>] [--version=<semver>] [--chaos-score=<n>] [--reputation=<n>] [--strict=1|0]");
    println!("  infring-ops canyon-plane workflow [--op=run|status] [--goal=<text>] [--workspace=<path>] [--strict=1|0]");
    println!("  infring-ops canyon-plane scheduler [--op=simulate|status] [--agents=<n>] [--nodes=<n>] [--modes=kubernetes,edge,distributed] [--strict=1|0]");
    println!("  infring-ops canyon-plane control-plane [--op=snapshot|status] [--rbac=1|0] [--sso=1|0] [--hitl=1|0] [--strict=1|0]");
    println!("  infring-ops canyon-plane adoption [--op=run-demo|status] [--tutorial=<id>] [--strict=1|0]");
    println!("  infring-ops canyon-plane benchmark-gate [--op=run|status] [--milestone=day90|day180] [--strict=1|0]");
    println!("  infring-ops canyon-plane footprint [--op=run|status] [--strict=1|0]");
    println!("  infring-ops canyon-plane lazy-substrate [--op=enable|load|status] [--feature-set=minimal|full-substrate] [--adapter=<id>] [--strict=1|0]");
    println!("  infring-ops canyon-plane release-pipeline [--op=run|status] [--binary=<id>] [--target=<triple>] [--profile=<id>] [--strict=1|0]");
    println!("  infring-ops canyon-plane receipt-batching [--op=flush|status] [--strict=1|0]");
    println!("  infring-ops canyon-plane package-release [--op=build|status] [--strict=1|0]");
    println!("  infring-ops canyon-plane size-trust [--strict=1|0]");
    println!("  infring-ops canyon-plane status");
}

fn lane_root(root: &Path) -> PathBuf {
    scoped_state_root(root, ENV_KEY, LANE_ID)
}

fn efficiency_path(root: &Path) -> PathBuf {
    lane_root(root).join("efficiency.json")
}

fn hands_registry_path(root: &Path) -> PathBuf {
    lane_root(root).join("hands_army").join("registry.json")
}

fn hands_runs_path(root: &Path) -> PathBuf {
    lane_root(root).join("hands_army").join("runs.jsonl")
}

fn evolution_state_path(root: &Path) -> PathBuf {
    lane_root(root).join("evolution").join("state.json")
}

fn sandbox_events_path(root: &Path) -> PathBuf {
    lane_root(root).join("sandbox").join("events.jsonl")
}

fn sandbox_sessions_path(root: &Path) -> PathBuf {
    lane_root(root).join("sandbox").join("sessions.json")
}

fn sandbox_snapshots_dir(root: &Path) -> PathBuf {
    lane_root(root).join("sandbox").join("snapshots")
}

fn ecosystem_inventory_path(root: &Path) -> PathBuf {
    lane_root(root).join("ecosystem").join("inventory.json")
}

fn ecosystem_marketplace_path(root: &Path) -> PathBuf {
    lane_root(root).join("ecosystem").join("marketplace.json")
}

fn workflow_history_path(root: &Path) -> PathBuf {
    lane_root(root).join("workflow").join("history.jsonl")
}

fn scheduler_state_path(root: &Path) -> PathBuf {
    lane_root(root).join("scheduler").join("latest.json")
}

fn control_snapshots_path(root: &Path) -> PathBuf {
    lane_root(root)
        .join("control_plane")
        .join("snapshots.jsonl")
}

fn adoption_history_path(root: &Path) -> PathBuf {
    lane_root(root).join("adoption").join("history.jsonl")
}

fn benchmark_state_path(root: &Path) -> PathBuf {
    lane_root(root).join("benchmark_gate").join("latest.json")
}

fn enterprise_state_root(root: &Path) -> PathBuf {
    core_state_root(root)
        .join("ops")
        .join("enterprise_hardening")
}

fn extract_first_f64(value: &Value, paths: &[&[&str]]) -> Option<f64> {
    for path in paths {
        let mut current = value;
        let mut found = true;
        for segment in *path {
            let Some(next) = current.get(*segment) else {
                found = false;
                break;
            };
            current = next;
        }
        if found {
            if let Some(number) = current.as_f64() {
                return Some(number);
            }
            if let Some(number) = current.as_u64() {
                return Some(number as f64);
            }
        }
    }
    None
}

fn top1_benchmark_paths(root: &Path) -> Vec<PathBuf> {
    vec![
        core_state_root(root)
            .join("ops")
            .join("top1_assurance")
            .join("benchmark_latest.json"),
        root.join("local/state/ops/top1_assurance/benchmark_latest.json"),
        root.join(
            "docs/client/reports/runtime_snapshots/ops/proof_pack/top1_benchmark_snapshot.json",
        ),
    ]
}

fn top1_benchmark_fallback(root: &Path) -> Option<(u64, f64, f64, String)> {
    for path in top1_benchmark_paths(root) {
        let Some(payload) = read_json(&path) else {
            continue;
        };
        let Some(cold_start_ms) = extract_first_f64(
            &payload,
            &[
                &["metrics", "cold_start_ms"],
                &["infring_measured", "cold_start_ms"],
            ],
        ) else {
            continue;
        };
        let Some(install_size_mb) = extract_first_f64(
            &payload,
            &[
                &["metrics", "install_size_mb"],
                &["infring_measured", "install_size_mb"],
            ],
        ) else {
            continue;
        };
        let tasks_per_sec = extract_first_f64(
            &payload,
            &[
                &["metrics", "tasks_per_sec"],
                &["infring_measured", "tasks_per_sec"],
            ],
        )
        .unwrap_or(0.0);
        return Some((
            cold_start_ms.round() as u64,
            install_size_mb,
            tasks_per_sec,
            path.to_string_lossy().to_string(),
        ));
    }
    None
}

fn top1_binary_size_paths(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join("target/x86_64-unknown-linux-musl/release/infringd"),
        root.join("target/release/infringd"),
        root.join("target/debug/infringd"),
    ]
}

fn top1_binary_size_fallback(root: &Path) -> Option<(f64, String)> {
    for path in top1_binary_size_paths(root) {
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        return Some((size_mb, path.to_string_lossy().to_string()));
    }
    top1_benchmark_fallback(root).map(|(_, size_mb, _, source)| (size_mb, source))
}

fn scheduler_agent_fallback(root: &Path) -> Option<(u64, String)> {
    let path = enterprise_state_root(root).join("f100/scale_ha_certification.json");
    let payload = read_json(&path)?;
    let agents = payload
        .get("airgap_agents")
        .and_then(Value::as_u64)
        .or_else(|| {
            payload
                .get("base")
                .and_then(|v| v.get("target_nodes"))
                .and_then(Value::as_u64)
        })?;
    Some((agents, path.to_string_lossy().to_string()))
}

fn run_enterprise_lane(root: &Path, argv: &[&str]) -> bool {
    let owned = argv.iter().map(|v| (*v).to_string()).collect::<Vec<_>>();
    enterprise_hardening::run(root, &owned) == 0
}

fn ensure_enterprise_zero_trust_profile(root: &Path) -> bool {
    let profile_path = enterprise_state_root(root).join("f100/zero_trust_profile.json");
    if profile_path.exists() {
        return true;
    }
    run_enterprise_lane(root, &["zero-trust-profile", "--strict=1"])
}

fn ensure_benchmark_audit_evidence(root: &Path) -> Option<String> {
    let candidates = [
        enterprise_state_root(root).join("f100/ops_bridge.json"),
        enterprise_state_root(root).join("moat/explorer/index.json"),
        enterprise_state_root(root).join("f100/super_gate.json"),
        root.join("docs/client/reports/proof_pack_latest.json"),
        root.join("docs/client/reports/runtime_snapshots/ops/proof_pack/latest.json"),
    ];
    if let Some(found) = evidence_exists(&candidates) {
        return Some(found);
    }
    let _ = ensure_enterprise_zero_trust_profile(root);
    if run_enterprise_lane(root, &["explore", "--strict=1"]) {
        return evidence_exists(&[
            enterprise_state_root(root).join("moat/explorer/index.json"),
            root.join("docs/client/reports/proof_pack_latest.json"),
            root.join("docs/client/reports/runtime_snapshots/ops/proof_pack/latest.json"),
        ]);
    }
    None
}

fn ensure_benchmark_adoption_evidence(root: &Path) -> Option<String> {
    let candidates = [
        enterprise_state_root(root).join("f100/adoption_bootstrap/bootstrap.json"),
        enterprise_state_root(root).join("f100/adoption_bootstrap/openapi.json"),
    ];
    if let Some(found) = evidence_exists(&candidates) {
        return Some(found);
    }
    let _ = ensure_enterprise_zero_trust_profile(root);
    if run_enterprise_lane(root, &["adoption-bootstrap", "--strict=1"]) {
        return evidence_exists(&candidates);
    }
    None
}

fn evidence_exists(candidates: &[PathBuf]) -> Option<String> {
    candidates
        .iter()
        .find(|path| path.exists())
        .map(|path| path.to_string_lossy().to_string())
}

fn read_object(path: &Path) -> Map<String, Value> {
    read_json(path)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn stringify_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

fn read_array(path: &Path) -> Vec<Value> {
    read_json(path)
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
}

fn upsert_marketplace_entry(entries: &mut Vec<Value>, hand_id: &str, row: Value) {
    if let Some(existing) = entries
        .iter_mut()
        .find(|existing| existing.get("hand_id").and_then(Value::as_str) == Some(hand_id))
    {
        *existing = row;
    } else {
        entries.push(row);
    }
}

fn sandbox_session_map(root: &Path) -> Map<String, Value> {
    read_object(&sandbox_sessions_path(root))
}

fn sandbox_session_snapshot(state: &Value) -> Value {
    json!({
        "session_id": state.get("session_id").cloned().unwrap_or_else(|| Value::String("sandbox".to_string())),
        "tier": state.get("tier").cloned().unwrap_or_else(|| Value::String("native".to_string())),
        "language": state.get("language").cloned().unwrap_or_else(|| Value::String("rust".to_string())),
        "fuel": state.get("fuel").cloned().unwrap_or_else(|| json!(0)),
        "epoch": state.get("epoch").cloned().unwrap_or_else(|| json!(0)),
        "logical_only": state.get("logical_only").cloned().unwrap_or_else(|| Value::Bool(false)),
        "overhead_mb": state.get("overhead_mb").cloned().unwrap_or_else(|| json!(0.0)),
        "last_event_hash": state.get("last_event_hash").cloned().unwrap_or_else(|| Value::String(String::new())),
        "updated_at": state.get("updated_at").cloned().unwrap_or_else(|| Value::String(now_iso()))
    })
}

fn emit(
    root: &Path,
    _command: &str,
    _strict: bool,
    payload: Value,
    conduit: Option<&Value>,
) -> i32 {
    let out = attach_conduit(payload, conduit);
    let _ = write_json(&latest_path(root, ENV_KEY, LANE_ID), &out);
    let _ = append_jsonl(&history_path(root, ENV_KEY, LANE_ID), &out);
    println!(
        "{}",
        serde_json::to_string_pretty(&out)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
    if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}
