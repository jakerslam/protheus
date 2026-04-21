// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/autonomy (authoritative).

use crate::{
    append_jsonl, clamp_int, clamp_num, clean_text, normalize_token, now_iso, parse_date_or_today,
    read_json, read_jsonl, resolve_runtime_path, round_to, write_json_atomic,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct RoleCfg {
    weight: f64,
    bias: String,
}

#[derive(Clone, Debug)]
struct DebatePolicy {
    version: String,
    enabled: bool,
    shadow_only: bool,
    rounds_max: i64,
    rounds_min_agents: i64,
    consensus_threshold: f64,
    confidence_floor: f64,
    disagreement_gap_threshold: f64,
    runoff_enabled: bool,
    max_runoff_rounds: i64,
    runoff_consensus_threshold: f64,
    require_distinct_roles_for_quorum: bool,
    roles: HashMap<String, RoleCfg>,
    latest_path: PathBuf,
    history_path: PathBuf,
    receipts_path: PathBuf,
}

#[derive(Clone, Debug)]
struct Candidate {
    id: String,
    score: f64,
    confidence: f64,
    risk: String,
}

#[derive(Clone, Debug)]
struct Agent {
    id: String,
    role: String,
}

fn debate_state_path(root: &Path, rel: &str) -> PathBuf {
    resolve_runtime_path(root, Some(rel), rel)
}

fn default_policy(root: &Path) -> DebatePolicy {
    let mut roles = HashMap::new();
    roles.insert(
        "soldier_guard".to_string(),
        RoleCfg {
            weight: 1.1,
            bias: "safety".to_string(),
        },
    );
    roles.insert(
        "creative_probe".to_string(),
        RoleCfg {
            weight: 1.0,
            bias: "growth".to_string(),
        },
    );
    roles.insert(
        "orderly_executor".to_string(),
        RoleCfg {
            weight: 1.15,
            bias: "delivery".to_string(),
        },
    );

    DebatePolicy {
        version: "1.0".to_string(),
        enabled: true,
        shadow_only: true,
        rounds_max: 2,
        rounds_min_agents: 3,
        consensus_threshold: 0.62,
        confidence_floor: 0.58,
        disagreement_gap_threshold: 0.08,
        runoff_enabled: true,
        max_runoff_rounds: 1,
        runoff_consensus_threshold: 0.57,
        require_distinct_roles_for_quorum: true,
        roles,
        latest_path: debate_state_path(root, "local/state/autonomy/multi_agent_debate/latest.json"),
        history_path: debate_state_path(
            root,
            "local/state/autonomy/multi_agent_debate/history.jsonl",
        ),
        receipts_path: debate_state_path(
            root,
            "local/state/autonomy/multi_agent_debate/receipts.jsonl",
        ),
    }
}

fn policy_path(root: &Path, explicit: Option<&Path>) -> PathBuf {
    explicit
        .map(|p| p.to_path_buf())
        .or_else(|| {
            std::env::var("MULTI_AGENT_DEBATE_POLICY_PATH")
                .ok()
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| {
            resolve_runtime_path(
                root,
                Some("config/multi_agent_debate_policy.json"),
                "config/multi_agent_debate_policy.json",
            )
        })
}
