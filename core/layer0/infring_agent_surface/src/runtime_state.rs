use crate::scheduler::{ScheduleEntry, SchedulePlan};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RuntimeReleaseGateCounters {
    pub denied_actions_total: u64,
    pub pause_reasons_total: u64,
    pub pause_reason_counts: BTreeMap<String, u64>,
    pub merkle_chain_continuity_failures_total: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RuntimeLaneDurableState {
    pub merkle_roots: BTreeMap<String, String>,
    pub scheduler_entries: BTreeMap<String, ScheduleEntry>,
    pub release_gate_counters: RuntimeReleaseGateCounters,
}

pub fn runtime_lane_state_path(metadata: &Value) -> PathBuf {
    if let Some(path) = metadata
        .get("state_path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        return PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("INFRING_AGENT_SURFACE_STATE_PATH") {
        let normalized = path.trim();
        if !normalized.is_empty() {
            return PathBuf::from(normalized);
        }
    }
    PathBuf::from("local/state/infring_agent_surface/runtime_lane_state.json")
}

pub fn runtime_lane_state_load(path: &Path) -> RuntimeLaneDurableState {
    let Ok(raw) = fs::read(path) else {
        return RuntimeLaneDurableState::default();
    };
    serde_json::from_slice::<RuntimeLaneDurableState>(&raw).unwrap_or_default()
}

pub fn runtime_lane_state_save(path: &Path, state: &RuntimeLaneDurableState) -> Option<String> {
    let Some(parent) = path.parent() else {
        return Some("runtime_lane_state_parent_missing".to_string());
    };
    if fs::create_dir_all(parent).is_err() {
        return Some("runtime_lane_state_mkdir_failed".to_string());
    }
    let Ok(bytes) = serde_json::to_vec_pretty(state) else {
        return Some("runtime_lane_state_encode_failed".to_string());
    };
    if fs::write(path, bytes).is_err() {
        return Some("runtime_lane_state_write_failed".to_string());
    }
    None
}

pub fn runtime_lane_state_record_denied_action(
    state: &mut RuntimeLaneDurableState,
    reason: &str,
) {
    state.release_gate_counters.denied_actions_total =
        state.release_gate_counters.denied_actions_total.saturating_add(1);
    runtime_lane_state_record_pause_reason(state, reason);
}

pub fn runtime_lane_state_record_pause_reason(
    state: &mut RuntimeLaneDurableState,
    reason: &str,
) {
    let normalized = reason.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return;
    }
    state.release_gate_counters.pause_reasons_total =
        state.release_gate_counters.pause_reasons_total.saturating_add(1);
    let entry = state
        .release_gate_counters
        .pause_reason_counts
        .entry(normalized)
        .or_insert(0);
    *entry = entry.saturating_add(1);
}

pub fn runtime_lane_state_record_merkle_continuity_failure(state: &mut RuntimeLaneDurableState) {
    state.release_gate_counters.merkle_chain_continuity_failures_total = state
        .release_gate_counters
        .merkle_chain_continuity_failures_total
        .saturating_add(1);
}

pub fn runtime_lane_state_release_gate_counters(state: &RuntimeLaneDurableState) -> Value {
    json!({
        "denied_actions_total": state.release_gate_counters.denied_actions_total,
        "pause_reasons_total": state.release_gate_counters.pause_reasons_total,
        "pause_reason_counts": state.release_gate_counters.pause_reason_counts,
        "merkle_chain_continuity_failures_total": state.release_gate_counters.merkle_chain_continuity_failures_total,
    })
}

pub fn runtime_lane_state_mark_schedule_success(
    state: &mut RuntimeLaneDurableState,
    agent_name: &str,
    capability_pack: &str,
    plan: &SchedulePlan,
) {
    let entry_id = format!("{}::{}", agent_name.trim(), capability_pack.trim());
    let now_unix = now_unix();
    let entry = state
        .scheduler_entries
        .entry(entry_id.clone())
        .or_insert_with(|| ScheduleEntry {
            entry_id: entry_id.clone(),
            agent_name: agent_name.to_string(),
            capability_pack: capability_pack.to_string(),
            plan: plan.clone(),
            next_due_unix: now_unix.saturating_add(plan.interval_seconds),
            last_run_unix: None,
            run_count: 0,
            paused: false,
            pause_reason: None,
            last_status: None,
        });

    entry.plan = plan.clone();
    entry.last_run_unix = Some(now_unix);
    entry.run_count = entry.run_count.saturating_add(1);
    entry.last_status = Some("ok".to_string());
    let jitter = if plan.jitter_seconds == 0 {
        0
    } else {
        (entry.run_count as u64) % plan.jitter_seconds
    };
    entry.next_due_unix = now_unix
        .saturating_add(plan.interval_seconds)
        .saturating_add(jitter);

    if let Some(max_runs) = plan.max_runs {
        if entry.run_count >= max_runs {
            entry.paused = true;
            if entry.pause_reason.as_deref() != Some("max_runs_reached") {
                entry.pause_reason = Some("max_runs_reached".to_string());
                runtime_lane_state_record_pause_reason(state, "max_runs_reached");
            }
        }
    }
}

pub fn runtime_lane_state_mark_schedule_failure(
    state: &mut RuntimeLaneDurableState,
    agent_name: &str,
    capability_pack: &str,
    plan: &SchedulePlan,
    error_code: &str,
) {
    let entry_id = format!("{}::{}", agent_name.trim(), capability_pack.trim());
    let now_unix = now_unix();
    let entry = state
        .scheduler_entries
        .entry(entry_id.clone())
        .or_insert_with(|| ScheduleEntry {
            entry_id: entry_id.clone(),
            agent_name: agent_name.to_string(),
            capability_pack: capability_pack.to_string(),
            plan: plan.clone(),
            next_due_unix: now_unix.saturating_add(plan.interval_seconds),
            last_run_unix: None,
            run_count: 0,
            paused: false,
            pause_reason: None,
            last_status: None,
        });

    entry.plan = plan.clone();
    entry.last_run_unix = Some(now_unix);
    entry.last_status = Some(format!("error:{error_code}"));
    entry.next_due_unix = now_unix.saturating_add(plan.interval_seconds.min(60));
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
