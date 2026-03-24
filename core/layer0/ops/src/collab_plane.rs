// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::collab_plane (authoritative)

use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_plane_conduit_enforcement, conduit_bypass_requested,
    emit_plane_receipt, load_json_or, parse_bool, parse_u64, plane_status, print_json, read_json,
    scoped_state_root, sha256_hex_str, split_csv_clean, write_json,
};
use crate::{clean, parse_args};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "COLLAB_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "collab_plane";

const DASHBOARD_CONTRACT_PATH: &str = "planes/contracts/collab/team_dashboard_contract_v1.json";
const LAUNCHER_CONTRACT_PATH: &str = "planes/contracts/collab/role_launcher_contract_v1.json";
const TERMINATION_CONTRACT_PATH: &str = "planes/contracts/collab/role_termination_contract_v1.json";
const SCHEDULER_CONTRACT_PATH: &str = "planes/contracts/collab/team_schedule_contract_v1.json";
const THROTTLE_CONTRACT_PATH: &str = "planes/contracts/collab/team_throttle_contract_v1.json";
const CONTINUITY_CONTRACT_PATH: &str = "planes/contracts/collab/team_continuity_contract_v1.json";

const ROLE_DIRECTOR: &str = "director";
const ROLE_CELL_COORDINATOR: &str = "cell_coordinator";
const BASE_STABLE_MAX_ACTIVE_AGENTS: u64 = 512;
const DEFAULT_STABLE_MAX_ACTIVE_AGENTS: u64 = BASE_STABLE_MAX_ACTIVE_AGENTS * 2;
const DEFAULT_MAX_AGENTS_PER_CELL: u64 = 32;
const DEFAULT_DIRECTOR_FANOUT_CELLS: u64 = 16;
const DEFAULT_MAX_DIRECTORS: u64 = 256;
const DEFAULT_DECENTRALIZED_MIN_AGENTS: u64 = 24;
const DEFAULT_HANDOFF_RETAIN: usize = 2_000;

#[derive(Debug, Clone, Copy)]
struct LauncherLimits {
    base_max_active_agents: u64,
    max_active_agents: u64,
    max_agents_per_cell: u64,
    director_fanout_cells: u64,
    max_directors: u64,
    decentralized_min_agents: u64,
    auto_director_spawn: bool,
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops collab-plane status");
    println!(
        "  protheus-ops collab-plane dashboard [--team=<id>] [--refresh-ms=<n>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops collab-plane launch-role --role=<id> [--team=<id>] [--shadow=<id>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops collab-plane terminate-role --shadow=<id> [--team=<id>] [--reason=<id>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops collab-plane schedule --op=<upsert|kickoff|list> [--team=<id>] [--job=<id>] [--cron=<expr>] [--shadows=a,b] [--strict=1|0]"
    );
    println!(
        "  protheus-ops collab-plane throttle --plane=<id> [--team=<id>] [--max-depth=<n>] [--strategy=priority-shed] [--strict=1|0]"
    );
    println!(
        "  protheus-ops collab-plane continuity --op=<checkpoint|reconstruct|status> [--team=<id>] [--state-json=<json>] [--strict=1|0]"
    );
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn team_slug(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if out.len() >= 80 {
            break;
        }
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "default-team".to_string()
    } else {
        trimmed.to_string()
    }
}

fn default_team_state(team: &str) -> Value {
    json!({
        "version": "v1",
        "team": team,
        "agents": [],
        "tasks": [],
        "handoffs": [],
        "topology": {
            "version": "v2",
            "mode": "decentralized_hierarchy",
            "stable_agent_cap_base": BASE_STABLE_MAX_ACTIVE_AGENTS,
            "stable_agent_cap": DEFAULT_STABLE_MAX_ACTIVE_AGENTS,
            "max_agents_per_cell": DEFAULT_MAX_AGENTS_PER_CELL,
            "director_fanout_cells": DEFAULT_DIRECTOR_FANOUT_CELLS,
            "active_agents": 0,
            "cell_count": 0,
            "director_count": 0
        }
    })
}

fn value_u64_field(value: Option<&Value>, fallback: u64) -> u64 {
    value
        .and_then(Value::as_u64)
        .or_else(|| value.and_then(Value::as_i64).map(|v| v.max(0) as u64))
        .unwrap_or(fallback)
}

fn parse_launcher_limits(contract: &Value) -> LauncherLimits {
    let limits = contract
        .get("limits")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let base_max_active_agents = value_u64_field(
        limits.get("base_max_active_agents"),
        BASE_STABLE_MAX_ACTIVE_AGENTS,
    )
    .clamp(16, 2_000_000);
    let min_doubled = base_max_active_agents.saturating_mul(2);
    let max_active_agents = value_u64_field(
        limits.get("max_active_agents"),
        DEFAULT_STABLE_MAX_ACTIVE_AGENTS,
    )
    .clamp(min_doubled.max(16), 2_000_000);
    let max_agents_per_cell = value_u64_field(
        limits.get("max_agents_per_cell"),
        DEFAULT_MAX_AGENTS_PER_CELL,
    )
    .clamp(4, 1_000);
    let director_fanout_cells = value_u64_field(
        limits.get("director_fanout_cells"),
        DEFAULT_DIRECTOR_FANOUT_CELLS,
    )
    .clamp(1, 1_000);
    let max_directors =
        value_u64_field(limits.get("max_directors"), DEFAULT_MAX_DIRECTORS).clamp(1, 10_000);
    let decentralized_min_agents = value_u64_field(
        limits.get("decentralized_min_agents"),
        DEFAULT_DECENTRALIZED_MIN_AGENTS,
    )
    .clamp(1, 1_000_000);
    let auto_director_spawn = limits
        .get("auto_director_spawn")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    LauncherLimits {
        base_max_active_agents,
        max_active_agents,
        max_agents_per_cell,
        director_fanout_cells,
        max_directors,
        decentralized_min_agents,
        auto_director_spawn,
    }
}

fn role_from_agent(row: &Value) -> String {
    clean(
        row.get("role").and_then(Value::as_str).unwrap_or_default(),
        80,
    )
    .to_ascii_lowercase()
}

fn status_from_agent(row: &Value) -> String {
    clean(
        row.get("status")
            .and_then(Value::as_str)
            .unwrap_or("active"),
        40,
    )
    .to_ascii_lowercase()
}

fn is_director_role(role: &str) -> bool {
    matches!(role, ROLE_DIRECTOR | ROLE_CELL_COORDINATOR)
}

fn active_agents(agents: &[Value]) -> Vec<Value> {
    agents
        .iter()
        .filter(|row| status_from_agent(row) == "active")
        .cloned()
        .collect()
}

fn worker_cell_id(index: usize) -> String {
    format!("cell-{:04}", index + 1)
}

fn director_shadow_for_cell(team: &str, cell_index: usize, fanout_cells: u64) -> String {
    let fanout = fanout_cells.max(1) as usize;
    let director_index = (cell_index / fanout) + 1;
    format!("director-{team}-{director_index:03}")
}

fn refresh_team_topology(team_state: &mut Value, limits: LauncherLimits) {
    let team = team_slug(
        team_state
            .get("team")
            .and_then(Value::as_str)
            .unwrap_or("default-team"),
    );
    let mut agents = team_state
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let active = active_agents(&agents);
    let active_count = active.len() as u64;
    let director_count = active
        .iter()
        .filter(|row| is_director_role(role_from_agent(row).as_str()))
        .count() as u64;
    let mut cells: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();
    let mut workers_seen = 0usize;
    for row in &active {
        let role = role_from_agent(row);
        let shadow = clean(
            row.get("shadow")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            120,
        );
        if shadow.is_empty() || is_director_role(role.as_str()) {
            continue;
        }
        let default_cell_idx = workers_seen / (limits.max_agents_per_cell.max(1) as usize);
        let default_cell = worker_cell_id(default_cell_idx);
        let default_director =
            director_shadow_for_cell(&team, default_cell_idx, limits.director_fanout_cells);
        let coord = row
            .get("coordination")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let cell_id = clean(
            coord
                .get("cell_id")
                .and_then(Value::as_str)
                .unwrap_or(default_cell.as_str()),
            80,
        );
        let director_shadow = clean(
            coord
                .get("director_shadow")
                .and_then(Value::as_str)
                .unwrap_or(default_director.as_str()),
            120,
        );
        let key = if cell_id.is_empty() {
            default_cell
        } else {
            cell_id
        };
        let entry = cells
            .entry(key)
            .or_insert_with(|| (director_shadow.clone(), Vec::new()));
        if entry.0.is_empty() {
            entry.0 = director_shadow.clone();
        }
        entry.1.push(shadow);
        workers_seen += 1;
    }
    let cell_rows = cells
        .iter()
        .map(|(cell_id, (director_shadow, members))| {
            let active_in_cell = members.len() as u64;
            let status = if active_in_cell >= limits.max_agents_per_cell {
                "saturated"
            } else {
                "ok"
            };
            json!({
                "cell_id": cell_id,
                "director_shadow": director_shadow,
                "active_agents": active_in_cell,
                "status": status,
                "members": members
            })
        })
        .collect::<Vec<_>>();
    let cell_count = cell_rows.len() as u64;
    let director_target = if cell_count == 0 {
        0
    } else {
        cell_count
            .div_ceil(limits.director_fanout_cells.max(1))
            .min(limits.max_directors)
    };
    let utilization_pct = if limits.max_active_agents == 0 {
        0.0
    } else {
        ((active_count as f64 / limits.max_active_agents as f64) * 100.0).clamp(0.0, 1000.0)
    };
    let director_ring = active
        .iter()
        .filter_map(|row| {
            let role = role_from_agent(row);
            if !is_director_role(role.as_str()) {
                return None;
            }
            let shadow = clean(
                row.get("shadow")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                120,
            );
            if shadow.is_empty() {
                None
            } else {
                Some(Value::String(shadow))
            }
        })
        .collect::<Vec<_>>();
    team_state["topology"] = json!({
        "version": "v2",
        "mode": "decentralized_hierarchy",
        "stable_agent_cap_base": limits.base_max_active_agents,
        "stable_agent_cap": limits.max_active_agents,
        "active_agents": active_count,
        "available_capacity": limits.max_active_agents.saturating_sub(active_count),
        "utilization_pct": utilization_pct,
        "max_agents_per_cell": limits.max_agents_per_cell,
        "director_fanout_cells": limits.director_fanout_cells,
        "director_target": director_target,
        "max_directors": limits.max_directors,
        "decentralized_min_agents": limits.decentralized_min_agents,
        "decentralized_recommended": active_count >= limits.decentralized_min_agents,
        "cell_count": cell_count,
        "director_count": director_count,
        "director_ring": director_ring,
        "cells": cell_rows,
        "updated_at": crate::now_iso()
    });
    team_state["agents"] = Value::Array(std::mem::take(&mut agents));
}

fn prune_orphan_directors(agents: &mut Vec<Value>) -> usize {
    let active_rows = active_agents(agents);
    let mut needed_directors = std::collections::BTreeSet::<String>::new();
    for row in &active_rows {
        let role = role_from_agent(row);
        if is_director_role(role.as_str()) {
            continue;
        }
        let director = clean(
            row.get("coordination")
                .and_then(Value::as_object)
                .and_then(|coord| coord.get("director_shadow"))
                .and_then(Value::as_str)
                .unwrap_or_default(),
            120,
        );
        if !director.is_empty() {
            needed_directors.insert(director);
        }
    }
    let before = agents.len();
    agents.retain(|row| {
        let role = role_from_agent(row);
        if !is_director_role(role.as_str()) || status_from_agent(row) != "active" {
            return true;
        }
        let shadow = clean(
            row.get("shadow")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            120,
        );
        !shadow.is_empty() && needed_directors.contains(&shadow)
    });
    before.saturating_sub(agents.len())
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(root, STATE_ENV, STATE_SCOPE, "collab_plane_error", payload)
}

fn status(root: &Path) -> Value {
    plane_status(root, STATE_ENV, STATE_SCOPE, "collab_plane_status")
}

fn claim_ids_for_action(action: &str) -> Vec<&'static str> {
    match action {
        "dashboard" => vec!["V6-COLLAB-001.1", "V6-COLLAB-001.4"],
        "launch-role" => vec!["V6-COLLAB-001.2", "V6-COLLAB-001.4"],
        "terminate-role" | "remove-role" | "revoke-role" | "stop-role" | "archive-role" => {
            vec![
                "V6-COLLAB-001.2",
                "V6-COLLAB-001.4",
                "V6-AGENT-LIFECYCLE-001.2",
            ]
        }
        "schedule" => vec!["V6-COLLAB-001.3", "V6-COLLAB-001.4"],
        "throttle" => vec!["V6-COLLAB-001.3", "V6-COLLAB-001.4"],
        "continuity" => vec!["V6-COLLAB-001.5", "V6-COLLAB-001.4"],
        _ => vec!["V6-COLLAB-001.4"],
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
        "collab_conduit_enforcement",
        "core/layer0/ops/collab_plane",
        bypass_requested,
        "collaboration_controls_route_through_layer0_conduit_with_fail_closed_denials",
        &claim_ids,
    )
}

fn team_state_path(root: &Path, team: &str) -> PathBuf {
    state_root(root).join("teams").join(format!("{team}.json"))
}

fn schedule_state_path(root: &Path, team: &str) -> PathBuf {
    state_root(root)
        .join("schedules")
        .join(format!("{team}.json"))
}

fn continuity_checkpoint_path(root: &Path, team: &str) -> PathBuf {
    state_root(root)
        .join("continuity")
        .join("checkpoint")
        .join(format!("{team}.json"))
}

fn continuity_reconstruct_path(root: &Path, team: &str) -> PathBuf {
    state_root(root)
        .join("continuity")
        .join("reconstructed")
        .join(format!("{team}.json"))
}

fn throttle_state_path(root: &Path, team: &str) -> PathBuf {
    state_root(root)
        .join("throttle")
        .join(format!("{team}.json"))
}

fn run_dashboard(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        DASHBOARD_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_team_dashboard_contract",
            "max_refresh_ms": 2000,
            "default_refresh_ms": 1000
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("collab_dashboard_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "collab_team_dashboard_contract"
    {
        errors.push("collab_dashboard_contract_kind_invalid".to_string());
    }
    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let default_refresh = contract
        .get("default_refresh_ms")
        .and_then(Value::as_u64)
        .unwrap_or(1000);
    let max_refresh = contract
        .get("max_refresh_ms")
        .and_then(Value::as_u64)
        .unwrap_or(2000);
    let refresh_ms = parse_u64(parsed.flags.get("refresh-ms"), default_refresh);
    if strict && refresh_ms > max_refresh {
        errors.push("collab_dashboard_refresh_exceeds_contract".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_dashboard",
            "errors": errors
        });
    }

    let launch_contract = load_json_or(
        root,
        LAUNCHER_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_role_launcher_contract",
            "limits": {
                "base_max_active_agents": BASE_STABLE_MAX_ACTIVE_AGENTS,
                "max_active_agents": DEFAULT_STABLE_MAX_ACTIVE_AGENTS,
                "max_agents_per_cell": DEFAULT_MAX_AGENTS_PER_CELL,
                "director_fanout_cells": DEFAULT_DIRECTOR_FANOUT_CELLS,
                "max_directors": DEFAULT_MAX_DIRECTORS,
                "decentralized_min_agents": DEFAULT_DECENTRALIZED_MIN_AGENTS,
                "auto_director_spawn": true
            }
        }),
    );
    let limits = parse_launcher_limits(&launch_contract);
    let team_state_path = team_state_path(root, &team);
    let mut team_state = read_json(&team_state_path).unwrap_or_else(|| default_team_state(&team));
    refresh_team_topology(&mut team_state, limits);
    let _ = write_json(&team_state_path, &team_state);
    let handoffs = team_state
        .get("handoffs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tasks = team_state
        .get("tasks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let agents = team_state
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let topology = team_state.get("topology").cloned().unwrap_or_else(|| {
        json!({
            "version": "v2",
            "mode": "decentralized_hierarchy"
        })
    });
    let receipt_drilldown = vec![
        json!({
            "lane": "collab_plane",
            "latest_path": latest_path(root).display().to_string()
        }),
        json!({
            "lane": "agency_plane",
            "latest_path": state_root(root)
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("agency_plane")
                .join("latest.json")
                .display()
                .to_string()
        }),
    ];
    let dashboard = json!({
        "version": "v1",
        "team": team,
        "refresh_ms": refresh_ms,
        "target_refresh_ms": max_refresh,
        "agents": agents,
        "tasks": tasks,
        "handoff_history": handoffs,
        "topology": topology,
        "receipt_drilldown": receipt_drilldown,
        "rendered_at": crate::now_iso()
    });
    let path = state_root(root)
        .join("dashboard")
        .join(format!("{team}.json"));
    let _ = write_json(&path, &dashboard);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "collab_plane_dashboard",
        "lane": "core/layer0/ops",
        "dashboard": dashboard,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&dashboard.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-COLLAB-001.1",
                "claim": "team_dashboard_exposes_agent_status_tasks_handoffs_with_receipt_drilldown_and_sub_two_second_refresh",
                "evidence": {
                    "team": team,
                    "refresh_ms": refresh_ms,
                    "agent_count": agents.len(),
                    "task_count": tasks.len(),
                    "handoff_count": handoffs.len(),
                    "hierarchy_mode": dashboard
                        .get("topology")
                        .and_then(|row| row.get("mode"))
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_launch_role(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        LAUNCHER_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_role_launcher_contract",
            "limits": {
                "base_max_active_agents": BASE_STABLE_MAX_ACTIVE_AGENTS,
                "max_active_agents": DEFAULT_STABLE_MAX_ACTIVE_AGENTS,
                "max_agents_per_cell": DEFAULT_MAX_AGENTS_PER_CELL,
                "director_fanout_cells": DEFAULT_DIRECTOR_FANOUT_CELLS,
                "max_directors": DEFAULT_MAX_DIRECTORS,
                "decentralized_min_agents": DEFAULT_DECENTRALIZED_MIN_AGENTS,
                "auto_director_spawn": true
            },
            "roles": {
                "director": {"default_tools": ["plan", "route", "govern"], "policy_mode": "safe"},
                "cell_coordinator": {"default_tools": ["route", "handoff"], "policy_mode": "safe"},
                "coordinator": {"default_tools": ["plan", "route"], "policy_mode": "safe"},
                "researcher": {"default_tools": ["search", "extract"], "policy_mode": "safe"},
                "builder": {"default_tools": ["compile", "verify"], "policy_mode": "safe"},
                "reviewer": {"default_tools": ["audit", "report"], "policy_mode": "safe"},
                "analyst": {"default_tools": ["summarize", "score"], "policy_mode": "safe"}
            }
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("collab_role_launcher_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "collab_role_launcher_contract"
    {
        errors.push("collab_role_launcher_contract_kind_invalid".to_string());
    }
    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let role = clean(
        parsed
            .flags
            .get("role")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        80,
    );
    if role.is_empty() {
        errors.push("collab_role_required".to_string());
    }
    let role_table = contract
        .get("roles")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let limits = parse_launcher_limits(&contract);
    if strict && limits.max_active_agents < limits.base_max_active_agents.saturating_mul(2) {
        errors.push("collab_role_launcher_max_agents_not_doubled".to_string());
    }
    let role_cfg = role_table.get(&role).cloned().unwrap_or(Value::Null);
    if strict && role_cfg.is_null() {
        errors.push("collab_role_unknown".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_launch_role",
            "errors": errors
        });
    }

    let shadow = clean(
        parsed
            .flags
            .get("shadow")
            .cloned()
            .unwrap_or_else(|| format!("{}-{}", role, &sha256_hex_str(&team)[..8])),
        120,
    );
    let activation = json!({
        "team": team,
        "shadow": shadow,
        "role": role,
        "policy_mode": role_cfg
            .get("policy_mode")
            .cloned()
            .unwrap_or(json!("safe")),
        "default_tools": role_cfg
            .get("default_tools")
            .cloned()
            .unwrap_or_else(|| json!(["plan"])),
        "activated_at": crate::now_iso(),
        "activation_hash": sha256_hex_str(&format!("{}:{}:{}", team, shadow, role))
    });

    let team_path = team_state_path(root, &team);
    let mut team_state = read_json(&team_path).unwrap_or_else(|| default_team_state(&team));
    let mut agents = team_state
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let already_active = agents.iter().any(|row| {
        row.get("shadow").and_then(Value::as_str) == Some(shadow.as_str())
            && status_from_agent(row) == "active"
    });
    let active_count = active_agents(&agents).len() as u64;
    if strict && !already_active && active_count >= limits.max_active_agents {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_launch_role",
            "errors": ["collab_role_team_at_capacity"],
            "capacity": {
                "active_agents": active_count,
                "stable_agent_cap": limits.max_active_agents,
                "stable_agent_cap_base": limits.base_max_active_agents
            }
        });
    }

    let mut assigned_cell = String::new();
    let mut director_shadow = String::new();
    let mut auto_spawned_director = false;
    if !already_active {
        let is_director_launch = is_director_role(role.as_str());
        if !is_director_launch {
            let worker_active_count = active_agents(&agents)
                .iter()
                .filter(|row| !is_director_role(role_from_agent(row).as_str()))
                .count();
            let cell_index = worker_active_count / limits.max_agents_per_cell.max(1) as usize;
            assigned_cell = worker_cell_id(cell_index);
            director_shadow =
                director_shadow_for_cell(&team, cell_index, limits.director_fanout_cells);
            let director_exists = agents.iter().any(|row| {
                row.get("shadow").and_then(Value::as_str) == Some(director_shadow.as_str())
                    && status_from_agent(row) == "active"
            });
            let active_directors = active_agents(&agents)
                .iter()
                .filter(|row| is_director_role(role_from_agent(row).as_str()))
                .count() as u64;
            if limits.auto_director_spawn && !director_exists {
                if strict && active_directors >= limits.max_directors {
                    return json!({
                        "ok": false,
                        "strict": strict,
                        "type": "collab_plane_launch_role",
                        "errors": ["collab_director_capacity_reached"],
                        "director_capacity": {
                            "active_directors": active_directors,
                            "max_directors": limits.max_directors
                        }
                    });
                }
                agents.push(json!({
                    "shadow": director_shadow,
                    "role": ROLE_DIRECTOR,
                    "status": "active",
                    "activated_at": crate::now_iso(),
                    "auto_managed": true,
                    "coordination": {
                        "tier": "director",
                        "decentralized": true
                    }
                }));
                auto_spawned_director = true;
            }
        } else {
            director_shadow = shadow.clone();
        }
        let coordination = if is_director_role(role.as_str()) {
            json!({
                "tier": "director",
                "decentralized": true
            })
        } else {
            json!({
                "tier": "worker",
                "decentralized": true,
                "cell_id": assigned_cell,
                "director_shadow": director_shadow,
                "routing_ring": format!("ring-{}", clean(director_shadow.clone(), 120))
            })
        };
        agents.push(json!({
            "shadow": shadow,
            "role": role,
            "status": "active",
            "activated_at": crate::now_iso(),
            "coordination": coordination
        }));
    }

    team_state["agents"] = Value::Array(agents.clone());
    refresh_team_topology(&mut team_state, limits);
    let _ = write_json(&team_path, &team_state);
    let _ = append_jsonl(
        &state_root(root).join("launch").join("history.jsonl"),
        &activation,
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "collab_plane_launch_role",
        "lane": "core/layer0/ops",
        "activation": activation,
        "topology": team_state.get("topology").cloned().unwrap_or_else(|| json!({})),
        "artifact": {
            "path": team_path.display().to_string(),
            "sha256": sha256_hex_str(&team_state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-COLLAB-001.2",
                "claim": "instant_role_launcher_starts_policy_safe_shadows_with_deterministic_activation_receipts",
                "evidence": {
                    "team": team,
                    "role": role,
                    "shadow": shadow,
                    "stable_agent_cap_base": limits.base_max_active_agents,
                    "stable_agent_cap": limits.max_active_agents,
                    "auto_spawned_director": auto_spawned_director,
                    "director_shadow": director_shadow,
                    "cell_id": assigned_cell
                }
            },
            {
                "id": "V6-COLLAB-002.1",
                "claim": "decentralized_director_cell_hierarchy_assigns_agents_without_single_coordinator_chokepoint",
                "evidence": {
                    "team": team,
                    "auto_spawned_director": auto_spawned_director,
                    "director_shadow": director_shadow,
                    "cell_id": assigned_cell,
                    "max_agents_per_cell": limits.max_agents_per_cell,
                    "director_fanout_cells": limits.director_fanout_cells
                }
            },
            {
                "id": "V6-COLLAB-002.2",
                "claim": "stable_agent_capacity_is_explicitly_doubled_with_fail_closed_admission_control",
                "evidence": {
                    "stable_agent_cap_base": limits.base_max_active_agents,
                    "stable_agent_cap": limits.max_active_agents
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_terminate_role(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let contract = load_json_or(
        root,
        TERMINATION_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_role_termination_contract",
            "allowed_actions": ["terminate-role", "remove-role", "revoke-role", "stop-role", "archive-role"],
            "require_shadow": true
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("collab_role_termination_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "collab_role_termination_contract"
    {
        errors.push("collab_role_termination_contract_kind_invalid".to_string());
    }

    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let action_clean = clean(action, 40).to_ascii_lowercase();
    let allowed_actions = contract
        .get("allowed_actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("terminate-role")])
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 40).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if strict && !allowed_actions.iter().any(|row| row == &action_clean) {
        errors.push("collab_role_termination_action_invalid".to_string());
    }

    let shadow = clean(
        parsed
            .flags
            .get("shadow")
            .cloned()
            .or_else(|| parsed.flags.get("agent").cloned())
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        120,
    );
    let require_shadow = contract
        .get("require_shadow")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if strict && require_shadow && shadow.is_empty() {
        errors.push("collab_role_termination_shadow_required".to_string());
    }

    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_terminate_role",
            "errors": errors
        });
    }

    let reason = clean(
        parsed
            .flags
            .get("reason")
            .cloned()
            .unwrap_or_else(|| action_clean.clone()),
        120,
    );
    let limits = parse_launcher_limits(&load_json_or(
        root,
        LAUNCHER_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_role_launcher_contract",
            "limits": {
                "base_max_active_agents": BASE_STABLE_MAX_ACTIVE_AGENTS,
                "max_active_agents": DEFAULT_STABLE_MAX_ACTIVE_AGENTS,
                "max_agents_per_cell": DEFAULT_MAX_AGENTS_PER_CELL,
                "director_fanout_cells": DEFAULT_DIRECTOR_FANOUT_CELLS,
                "max_directors": DEFAULT_MAX_DIRECTORS,
                "decentralized_min_agents": DEFAULT_DECENTRALIZED_MIN_AGENTS,
                "auto_director_spawn": true
            }
        }),
    ));
    let team_path = team_state_path(root, &team);
    let mut team_state = read_json(&team_path).unwrap_or_else(|| default_team_state(&team));

    let mut agents = team_state
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let before_agents = agents.len();
    agents.retain(|row| {
        if shadow.is_empty() {
            return true;
        }
        row.get("shadow").and_then(Value::as_str) != Some(shadow.as_str())
    });
    let removed_count = before_agents.saturating_sub(agents.len());
    let orphaned_director_gc = prune_orphan_directors(&mut agents);
    team_state["agents"] = Value::Array(agents.clone());

    let mut tasks = team_state
        .get("tasks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let before_tasks = tasks.len();
    tasks.retain(|row| {
        if shadow.is_empty() {
            return true;
        }
        let assigned = ["shadow", "agent", "assignee", "owner"]
            .iter()
            .any(|key| row.get(key).and_then(Value::as_str) == Some(shadow.as_str()));
        !assigned
    });
    let released_task_count = before_tasks.saturating_sub(tasks.len());
    team_state["tasks"] = Value::Array(tasks.clone());

    let mut handoffs = team_state
        .get("handoffs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if removed_count > 0 || released_task_count > 0 {
        handoffs.push(json!({
            "type": "agent_terminated",
            "team": team,
            "shadow": shadow,
            "action": action_clean,
            "reason": reason,
            "terminated_at": crate::now_iso(),
            "removed_count": removed_count,
            "released_task_count": released_task_count,
            "orphaned_director_gc": orphaned_director_gc,
            "termination_hash": sha256_hex_str(&format!("{}:{}:{}:{}", team, shadow, action_clean, reason))
        }));
        if handoffs.len() > DEFAULT_HANDOFF_RETAIN {
            handoffs = handoffs[handoffs.len().saturating_sub(DEFAULT_HANDOFF_RETAIN)..].to_vec();
        }
    }
    team_state["handoffs"] = Value::Array(handoffs.clone());
    refresh_team_topology(&mut team_state, limits);
    let _ = write_json(&team_path, &team_state);

    if removed_count > 0 || released_task_count > 0 {
        let _ = append_jsonl(
            &state_root(root).join("terminate").join("history.jsonl"),
            &json!({
                "type": "collab_role_termination",
                "team": team,
                "shadow": shadow,
                "action": action_clean,
                "reason": reason,
                "removed_count": removed_count,
                "released_task_count": released_task_count,
                "orphaned_director_gc": orphaned_director_gc,
                "ts": crate::now_iso()
            }),
        );
    }

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "collab_plane_terminate_role",
        "lane": "core/layer0/ops",
        "team": team,
        "shadow": shadow,
        "action": action_clean,
        "reason": reason,
        "removed_count": removed_count,
        "orphaned_director_gc": orphaned_director_gc,
        "released_task_count": released_task_count,
        "team_state": {
            "agent_count": agents.len(),
            "task_count": tasks.len(),
            "handoff_count": handoffs.len(),
            "topology": team_state.get("topology").cloned().unwrap_or_else(|| json!({}))
        },
        "artifact": {
            "path": team_path.display().to_string(),
            "sha256": sha256_hex_str(&team_state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-COLLAB-001.2",
                "claim": "role_lifecycle_supports_deterministic_termination_with_receipts_and_team_state_cleanup",
                "evidence": {
                    "team": team,
                    "shadow": shadow,
                    "removed_count": removed_count,
                    "released_task_count": released_task_count,
                    "orphaned_director_gc": orphaned_director_gc
                }
            },
            {
                "id": "V6-AGENT-LIFECYCLE-001.2",
                "claim": "auto_termination_path_removes_idle_agents_from_authority_state",
                "evidence": {
                    "team": team,
                    "shadow": shadow,
                    "removed_count": removed_count
                }
            },
            {
                "id": "V6-COLLAB-002.3",
                "claim": "decentralized_role_gc_prunes_orphaned_directors_when_worker_cells_empty",
                "evidence": {
                    "team": team,
                    "shadow": shadow,
                    "orphaned_director_gc": orphaned_director_gc
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_schedule(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        SCHEDULER_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_team_scheduler_contract",
            "allowed_ops": ["upsert", "kickoff", "list"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("collab_scheduler_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "collab_team_scheduler_contract"
    {
        errors.push("collab_scheduler_contract_kind_invalid".to_string());
    }
    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "list".to_string()),
        40,
    )
    .to_ascii_lowercase();
    let allowed_ops = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("upsert"), json!("kickoff"), json!("list")])
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 30).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if strict && !allowed_ops.iter().any(|v| v == &op) {
        errors.push("collab_scheduler_op_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_schedule",
            "errors": errors
        });
    }

    let path = schedule_state_path(root, &team);
    let mut schedule_state = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "team": team,
            "jobs": []
        })
    });
    if !schedule_state
        .get("jobs")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        schedule_state["jobs"] = Value::Array(Vec::new());
    }

    let job_id = clean(
        parsed
            .flags
            .get("job")
            .cloned()
            .unwrap_or_else(|| "default-job".to_string()),
        120,
    );
    let cron = clean(
        parsed
            .flags
            .get("cron")
            .cloned()
            .unwrap_or_else(|| "*/30 * * * *".to_string()),
        120,
    );
    let shadows = parsed
        .flags
        .get("shadows")
        .map(|raw| split_csv_clean(raw, 80))
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| vec!["default-shadow".to_string()]);

    let mut kickoff_receipts = Vec::<Value>::new();
    match op.as_str() {
        "upsert" => {
            let mut jobs = schedule_state
                .get("jobs")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut replaced = false;
            for row in &mut jobs {
                if row.get("job_id").and_then(Value::as_str) == Some(job_id.as_str()) {
                    *row = json!({
                        "job_id": job_id,
                        "cron": cron,
                        "shadows": shadows,
                        "updated_at": crate::now_iso()
                    });
                    replaced = true;
                }
            }
            if !replaced {
                jobs.push(json!({
                    "job_id": job_id,
                    "cron": cron,
                    "shadows": shadows,
                    "updated_at": crate::now_iso()
                }));
            }
            schedule_state["jobs"] = Value::Array(jobs);
        }
        "kickoff" => {
            kickoff_receipts = shadows
                .iter()
                .enumerate()
                .map(|(idx, shadow)| {
                    json!({
                        "index": idx + 1,
                        "job_id": job_id,
                        "shadow": shadow,
                        "kickoff_ts": crate::now_iso(),
                        "handoff_hash": sha256_hex_str(&format!("{}:{}:{}:{}", team, job_id, shadow, idx + 1))
                    })
                })
                .collect::<Vec<_>>();
            let mut team_state = read_json(&team_state_path(root, &team))
                .unwrap_or_else(|| default_team_state(&team));
            let mut handoffs = team_state
                .get("handoffs")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            handoffs.extend(kickoff_receipts.clone());
            team_state["handoffs"] = Value::Array(handoffs);
            let limits = parse_launcher_limits(&load_json_or(
                root,
                LAUNCHER_CONTRACT_PATH,
                json!({
                    "version": "v1",
                    "kind": "collab_role_launcher_contract",
                    "limits": {
                        "base_max_active_agents": BASE_STABLE_MAX_ACTIVE_AGENTS,
                        "max_active_agents": DEFAULT_STABLE_MAX_ACTIVE_AGENTS,
                        "max_agents_per_cell": DEFAULT_MAX_AGENTS_PER_CELL,
                        "director_fanout_cells": DEFAULT_DIRECTOR_FANOUT_CELLS,
                        "max_directors": DEFAULT_MAX_DIRECTORS,
                        "decentralized_min_agents": DEFAULT_DECENTRALIZED_MIN_AGENTS,
                        "auto_director_spawn": true
                    }
                }),
            ));
            refresh_team_topology(&mut team_state, limits);
            let _ = write_json(&team_state_path(root, &team), &team_state);
        }
        _ => {}
    }
    schedule_state["updated_at"] = Value::String(crate::now_iso());
    let _ = write_json(&path, &schedule_state);
    let _ = append_jsonl(
        &state_root(root).join("schedules").join("history.jsonl"),
        &json!({
            "type": "collab_schedule",
            "team": team,
            "op": op,
            "job_id": job_id,
            "cron": cron,
            "shadows": shadows,
            "kickoff_count": kickoff_receipts.len(),
            "ts": crate::now_iso()
        }),
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "collab_plane_schedule",
        "lane": "core/layer0/ops",
        "op": op,
        "team": team,
        "job_id": job_id,
        "schedule": schedule_state,
        "kickoff_receipts": kickoff_receipts,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&schedule_state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-COLLAB-001.3",
                "claim": "team_scheduler_supports_deterministic_kickoff_and_handoff_receipts",
                "evidence": {
                    "team": team,
                    "op": op,
                    "kickoff_count": kickoff_receipts.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_throttle(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        THROTTLE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_team_throttle_contract",
            "min_depth": 1,
            "max_depth": 1000000,
            "allowed_strategies": ["priority-shed", "pause-noncritical", "batch-sync"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("collab_throttle_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "collab_team_throttle_contract"
    {
        errors.push("collab_throttle_contract_kind_invalid".to_string());
    }

    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let plane = clean(
        parsed
            .flags
            .get("plane")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        120,
    );
    if plane.is_empty() {
        errors.push("collab_throttle_plane_required".to_string());
    }
    let min_depth = contract
        .get("min_depth")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    let max_depth_allowed = contract
        .get("max_depth")
        .and_then(Value::as_u64)
        .unwrap_or(1_000_000);
    let max_depth = parse_u64(parsed.flags.get("max-depth"), 75);
    if strict && (max_depth < min_depth || max_depth > max_depth_allowed) {
        errors.push("collab_throttle_max_depth_out_of_range".to_string());
    }
    let strategy = clean(
        parsed
            .flags
            .get("strategy")
            .cloned()
            .unwrap_or_else(|| "priority-shed".to_string()),
        80,
    )
    .to_ascii_lowercase();
    let allowed_strategies = contract
        .get("allowed_strategies")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("priority-shed")])
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 80).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if strict && !allowed_strategies.iter().any(|v| v == &strategy) {
        errors.push("collab_throttle_strategy_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_throttle",
            "errors": errors
        });
    }

    let state_path = throttle_state_path(root, &team);
    let mut throttle_state = read_json(&state_path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "team": team,
            "policies": []
        })
    });
    let mut policies = throttle_state
        .get("policies")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let policy = json!({
        "plane": plane,
        "max_depth": max_depth,
        "strategy": strategy,
        "active": true,
        "updated_at": crate::now_iso()
    });
    let mut replaced = false;
    for row in &mut policies {
        if row.get("plane").and_then(Value::as_str) == Some(plane.as_str()) {
            *row = policy.clone();
            replaced = true;
        }
    }
    if !replaced {
        policies.push(policy.clone());
    }
    throttle_state["policies"] = Value::Array(policies.clone());
    throttle_state["updated_at"] = Value::String(crate::now_iso());
    let _ = write_json(&state_path, &throttle_state);
    let _ = append_jsonl(
        &state_root(root).join("throttle").join("history.jsonl"),
        &json!({
            "type": "collab_throttle",
            "team": team,
            "plane": plane,
            "max_depth": max_depth,
            "strategy": strategy,
            "ts": crate::now_iso()
        }),
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "collab_plane_throttle",
        "lane": "core/layer0/ops",
        "team": team,
        "plane": plane,
        "max_depth": max_depth,
        "strategy": strategy,
        "policy": policy,
        "state": throttle_state,
        "artifact": {
            "path": state_path.display().to_string(),
            "sha256": sha256_hex_str(&Value::Array(policies.clone()).to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-COLLAB-001.3",
                "claim": "team_scheduler_and_throttle_policies_support_deterministic_backpressure_controls",
                "evidence": {
                    "team": team,
                    "plane": plane,
                    "strategy": strategy,
                    "max_depth": max_depth
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_continuity(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CONTINUITY_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_team_continuity_contract",
            "required_keys": ["team", "agents", "tasks", "handoffs"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("collab_continuity_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "collab_team_continuity_contract"
    {
        errors.push("collab_continuity_contract_kind_invalid".to_string());
    }
    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        30,
    )
    .to_ascii_lowercase();
    if !matches!(op.as_str(), "checkpoint" | "reconstruct" | "status") {
        errors.push("collab_continuity_op_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_continuity",
            "errors": errors
        });
    }

    match op.as_str() {
        "status" => {
            let checkpoint_path = continuity_checkpoint_path(root, &team);
            let reconstruct_path = continuity_reconstruct_path(root, &team);
            let checkpoint = read_json(&checkpoint_path);
            let reconstructed = read_json(&reconstruct_path);
            let mut out = json!({
                "ok": true,
                "strict": strict,
                "type": "collab_plane_continuity",
                "op": "status",
                "team": team,
                "checkpoint_present": checkpoint.is_some(),
                "reconstructed_present": reconstructed.is_some(),
                "checkpoint_path": checkpoint_path.display().to_string(),
                "reconstruct_path": reconstruct_path.display().to_string(),
                "claim_evidence": [
                    {
                        "id": "V6-COLLAB-001.5",
                        "claim": "team_state_continuity_supports_restart_reconstruction_with_deterministic_audit_receipts",
                        "evidence": {
                            "team": team,
                            "checkpoint_present": checkpoint.is_some(),
                            "reconstructed_present": reconstructed.is_some()
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            out
        }
        "checkpoint" => {
            let mut state = parsed
                .flags
                .get("state-json")
                .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
                .unwrap_or_else(|| default_team_state(&team));
            for key in contract
                .get("required_keys")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
            {
                if !state.get(key).is_some() {
                    state[key] = Value::Null;
                }
            }
            state["checkpoint_ts"] = Value::String(crate::now_iso());
            state["checkpoint_hash"] = Value::String(sha256_hex_str(&state.to_string()));
            let path = continuity_checkpoint_path(root, &team);
            let _ = write_json(&path, &state);
            let _ = append_jsonl(
                &state_root(root).join("continuity").join("history.jsonl"),
                &json!({
                    "type": "collab_checkpoint",
                    "team": team,
                    "path": path.display().to_string(),
                    "ts": crate::now_iso()
                }),
            );
            let mut out = json!({
                "ok": true,
                "strict": strict,
                "type": "collab_plane_continuity",
                "op": "checkpoint",
                "team": team,
                "checkpoint": state,
                "artifact": {
                    "path": path.display().to_string(),
                    "sha256": sha256_hex_str(&state.to_string())
                },
                "claim_evidence": [
                    {
                        "id": "V6-COLLAB-001.5",
                        "claim": "team_state_continuity_persists_checkpoint_for_recovery_audits",
                        "evidence": {
                            "team": team
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            out
        }
        "reconstruct" => {
            let checkpoint_path = continuity_checkpoint_path(root, &team);
            let Some(checkpoint) = read_json(&checkpoint_path) else {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "collab_plane_continuity",
                    "op": "reconstruct",
                    "errors": [format!("checkpoint_missing:{}", checkpoint_path.display())]
                });
            };
            let mut restored = checkpoint.clone();
            restored["reconstructed_ts"] = Value::String(crate::now_iso());
            restored["daemon_restart_simulated"] = Value::Bool(true);
            restored["reattach_simulated"] = Value::Bool(true);
            let path = continuity_reconstruct_path(root, &team);
            let _ = write_json(&path, &restored);
            let mut out = json!({
                "ok": true,
                "strict": strict,
                "type": "collab_plane_continuity",
                "op": "reconstruct",
                "team": team,
                "restored": restored,
                "artifact": {
                    "path": path.display().to_string(),
                    "sha256": sha256_hex_str(&restored.to_string())
                },
                "claim_evidence": [
                    {
                        "id": "V6-COLLAB-001.5",
                        "claim": "team_state_reconstruction_restores_auditable_collaboration_state_after_restart",
                        "evidence": {
                            "team": team,
                            "daemon_restart_simulated": true,
                            "reattach_simulated": true
                        }
                    }
                ]
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            out
        }
        _ => json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_continuity",
            "errors": ["collab_continuity_op_invalid"]
        }),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let strict = parse_bool(parsed.flags.get("strict"), true);
    let conduit = if command != "status" {
        Some(conduit_enforcement(root, &parsed, strict, &command))
    } else {
        None
    };
    if strict
        && conduit
            .as_ref()
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": "collab_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "dashboard" => run_dashboard(root, &parsed, strict),
        "launch-role" | "launch" => run_launch_role(root, &parsed, strict),
        "terminate-role" | "remove-role" | "revoke-role" | "stop-role" | "archive-role" => {
            run_terminate_role(root, &parsed, strict, command.as_str())
        }
        "schedule" => run_schedule(root, &parsed, strict),
        "throttle" => run_throttle(root, &parsed, strict),
        "continuity" => run_continuity(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "collab_plane_error",
            "error": "unknown_command",
            "command": command
        }),
    };
    if command == "status" {
        print_json(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["dashboard".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "dashboard");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn throttle_persists_plane_policy() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&[
            "throttle".to_string(),
            "--team=ops".to_string(),
            "--plane=backlog_delivery_plane".to_string(),
            "--max-depth=75".to_string(),
            "--strategy=priority-shed".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_throttle(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("policy")
                .and_then(|v| v.get("plane"))
                .and_then(Value::as_str),
            Some("backlog_delivery_plane")
        );
        assert_eq!(
            out.get("policy")
                .and_then(|v| v.get("max_depth"))
                .and_then(Value::as_u64),
            Some(75)
        );
    }

    #[test]
    fn terminate_role_removes_shadow_and_is_idempotent() {
        let root = tempfile::tempdir().expect("tempdir");
        let team_path = team_state_path(root.path(), "ops");
        let _ = write_json(
            &team_path,
            &json!({
                "version": "v1",
                "team": "ops",
                "agents": [
                    {"shadow": "ops-a", "role": "analyst", "status": "active", "activated_at": "2026-03-22T00:00:00Z"},
                    {"shadow": "ops-b", "role": "researcher", "status": "active", "activated_at": "2026-03-22T00:00:00Z"}
                ],
                "tasks": [
                    {"id": "task-1", "assignee": "ops-a"},
                    {"id": "task-2", "assignee": "ops-b"}
                ],
                "handoffs": []
            }),
        );

        let parsed = crate::parse_args(&[
            "terminate-role".to_string(),
            "--team=ops".to_string(),
            "--shadow=ops-a".to_string(),
            "--reason=test".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_terminate_role(root.path(), &parsed, true, "terminate-role");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("removed_count").and_then(Value::as_u64), Some(1));
        assert_eq!(
            out.get("released_task_count").and_then(Value::as_u64),
            Some(1)
        );

        let team_state = read_json(&team_path).expect("team state");
        let agents = team_state
            .get("agents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(agents.len(), 1);
        assert_eq!(
            agents[0].get("shadow").and_then(Value::as_str),
            Some("ops-b")
        );

        let out_second = run_terminate_role(root.path(), &parsed, true, "terminate-role");
        assert_eq!(out_second.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out_second.get("removed_count").and_then(Value::as_u64),
            Some(0)
        );
    }

    #[test]
    fn launch_role_auto_spawns_director_and_updates_topology() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&[
            "launch-role".to_string(),
            "--team=ops".to_string(),
            "--role=analyst".to_string(),
            "--shadow=ops-a".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_launch_role(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("topology")
                .and_then(|v| v.get("director_count"))
                .and_then(Value::as_u64),
            Some(1)
        );

        let team_state = read_json(&team_state_path(root.path(), "ops")).expect("team state");
        let agents = team_state
            .get("agents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(agents
            .iter()
            .any(|row| row.get("role").and_then(Value::as_str) == Some("director")));
        assert!(agents
            .iter()
            .any(|row| row.get("shadow").and_then(Value::as_str) == Some("ops-a")));
    }

    #[test]
    fn launch_role_enforces_configured_stable_capacity() {
        let root = tempfile::tempdir().expect("tempdir");
        let contract_path = root
            .path()
            .join("planes")
            .join("contracts")
            .join("collab")
            .join("role_launcher_contract_v1.json");
        fs::create_dir_all(contract_path.parent().expect("contract parent"))
            .expect("mkdir contract");
        fs::write(
            &contract_path,
            serde_json::to_string_pretty(&json!({
                "version": "v1",
                "kind": "collab_role_launcher_contract",
                "limits": {
                    "base_max_active_agents": 16,
                    "max_active_agents": 32,
                    "max_agents_per_cell": 8,
                    "director_fanout_cells": 4,
                    "max_directors": 2,
                    "decentralized_min_agents": 4,
                    "auto_director_spawn": false
                },
                "roles": {
                    "analyst": {"default_tools": ["summarize", "score"], "policy_mode": "safe"}
                }
            }))
            .expect("encode contract"),
        )
        .expect("write contract");
        let loaded = load_json_or(root.path(), LAUNCHER_CONTRACT_PATH, json!({}));
        assert_eq!(
            loaded
                .get("limits")
                .and_then(|row| row.get("max_active_agents"))
                .and_then(Value::as_u64),
            Some(32)
        );

        for idx in 0..32 {
            let parsed = crate::parse_args(&[
                "launch-role".to_string(),
                "--team=ops".to_string(),
                "--role=analyst".to_string(),
                format!("--shadow=ops-{idx}"),
                "--strict=1".to_string(),
            ]);
            let out = run_launch_role(root.path(), &parsed, true);
            assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        }

        let overflow = crate::parse_args(&[
            "launch-role".to_string(),
            "--team=ops".to_string(),
            "--role=analyst".to_string(),
            "--shadow=ops-over".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_launch_role(root.path(), &overflow, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert!(out
            .get("errors")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .any(|row| row == "collab_role_team_at_capacity"));
    }

    #[test]
    fn terminate_role_prunes_orphaned_director_cells() {
        let root = tempfile::tempdir().expect("tempdir");
        let launch = crate::parse_args(&[
            "launch-role".to_string(),
            "--team=ops".to_string(),
            "--role=researcher".to_string(),
            "--shadow=ops-worker".to_string(),
            "--strict=1".to_string(),
        ]);
        let launched = run_launch_role(root.path(), &launch, true);
        assert_eq!(launched.get("ok").and_then(Value::as_bool), Some(true));

        let terminate = crate::parse_args(&[
            "terminate-role".to_string(),
            "--team=ops".to_string(),
            "--shadow=ops-worker".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_terminate_role(root.path(), &terminate, true, "terminate-role");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("orphaned_director_gc").and_then(Value::as_u64),
            Some(1)
        );

        let team_state = read_json(&team_state_path(root.path(), "ops")).expect("team state");
        let agents = team_state
            .get("agents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            agents.is_empty(),
            "expected worker and director to be fully removed"
        );
    }
}
