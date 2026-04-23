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
    println!("  infring-ops collab-plane status");
    println!(
        "  infring-ops collab-plane dashboard [--team=<id>] [--refresh-ms=<n>] [--strict=1|0]"
    );
    println!(
        "  infring-ops collab-plane launch-role --role=<id> [--team=<id>] [--shadow=<id>] [--strict=1|0]"
    );
    println!(
        "  infring-ops collab-plane terminate-role --shadow=<id> [--team=<id>] [--reason=<id>] [--strict=1|0]"
    );
    println!(
        "  infring-ops collab-plane schedule --op=<upsert|kickoff|list> [--team=<id>] [--job=<id>] [--cron=<expr>] [--shadows=a,b] [--strict=1|0]"
    );
    println!(
        "  infring-ops collab-plane throttle --plane=<id> [--team=<id>] [--max-depth=<n>] [--strategy=priority-shed] [--strict=1|0]"
    );
    println!(
        "  infring-ops collab-plane continuity --op=<checkpoint|reconstruct|status> [--team=<id>] [--state-json=<json>] [--strict=1|0]"
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
    let mut prev_dash = false;
    for ch in raw.trim().chars() {
        if out.len() >= 80 {
            break;
        }
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
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
