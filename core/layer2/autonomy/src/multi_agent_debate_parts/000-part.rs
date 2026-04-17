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

fn load_policy(root: &Path, explicit: Option<&Path>) -> DebatePolicy {
    let p = policy_path(root, explicit);
    let mut policy = default_policy(root);
    let raw = read_json(&p);
    let obj = raw.as_object();

    if let Some(v) = obj
        .and_then(|m| m.get("version"))
        .and_then(Value::as_str)
        .map(|s| clean_text(s, 40))
    {
        if !v.is_empty() {
            policy.version = v;
        }
    }
    if let Some(v) = obj.and_then(|m| m.get("enabled")).and_then(Value::as_bool) {
        policy.enabled = v;
    }
    if let Some(v) = obj
        .and_then(|m| m.get("shadow_only"))
        .and_then(Value::as_bool)
    {
        policy.shadow_only = v;
    }

    if let Some(rounds) = obj.and_then(|m| m.get("rounds")).and_then(Value::as_object) {
        policy.rounds_max = clamp_int(
            rounds
                .get("max_rounds")
                .and_then(Value::as_i64)
                .unwrap_or(policy.rounds_max),
            1,
            8,
            policy.rounds_max,
        );
        policy.rounds_min_agents = clamp_int(
            rounds
                .get("min_agents")
                .and_then(Value::as_i64)
                .unwrap_or(policy.rounds_min_agents),
            1,
            16,
            policy.rounds_min_agents,
        );
        policy.consensus_threshold = clamp_num(
            rounds
                .get("consensus_threshold")
                .and_then(Value::as_f64)
                .unwrap_or(policy.consensus_threshold),
            0.0,
            1.0,
            policy.consensus_threshold,
        );
    }

    if let Some(res) = obj
        .and_then(|m| m.get("debate_resolution"))
        .and_then(Value::as_object)
    {
        policy.confidence_floor = clamp_num(
            res.get("confidence_floor")
                .and_then(Value::as_f64)
                .unwrap_or(policy.confidence_floor),
            0.0,
            1.0,
            policy.confidence_floor,
        );
        policy.disagreement_gap_threshold = clamp_num(
            res.get("disagreement_gap_threshold")
                .and_then(Value::as_f64)
                .unwrap_or(policy.disagreement_gap_threshold),
            0.0,
            1.0,
            policy.disagreement_gap_threshold,
        );
        if let Some(v) = res.get("runoff_enabled").and_then(Value::as_bool) {
            policy.runoff_enabled = v;
        }
        policy.max_runoff_rounds = clamp_int(
            res.get("max_runoff_rounds")
                .and_then(Value::as_i64)
                .unwrap_or(policy.max_runoff_rounds),
            0,
            3,
            policy.max_runoff_rounds,
        );
        policy.runoff_consensus_threshold = clamp_num(
            res.get("runoff_consensus_threshold")
                .and_then(Value::as_f64)
                .unwrap_or(policy.runoff_consensus_threshold),
            0.0,
            1.0,
            policy.runoff_consensus_threshold,
        );
        if let Some(v) = res
            .get("require_distinct_roles_for_quorum")
            .and_then(Value::as_bool)
        {
            policy.require_distinct_roles_for_quorum = v;
        }
    }

    if let Some(role_map) = obj
        .and_then(|m| m.get("agent_roles"))
        .and_then(Value::as_object)
    {
        let mut next = HashMap::new();
        for (k, row) in role_map {
            let role_key = normalize_token(k, 80);
            if role_key.is_empty() {
                continue;
            }
            let src = row.as_object();
            next.insert(
                role_key,
                RoleCfg {
                    weight: clamp_num(
                        src.and_then(|r| r.get("weight"))
                            .and_then(Value::as_f64)
                            .unwrap_or(1.0),
                        0.2,
                        5.0,
                        1.0,
                    ),
                    bias: {
                        let b = src
                            .and_then(|r| r.get("bias"))
                            .and_then(Value::as_str)
                            .map(|v| normalize_token(v, 40))
                            .unwrap_or_else(|| "delivery".to_string());
                        if b.is_empty() {
                            "delivery".to_string()
                        } else {
                            b
                        }
                    },
                },
            );
        }
        if !next.is_empty() {
            policy.roles = next;
        }
    }

    if let Some(outputs) = obj
        .and_then(|m| m.get("outputs"))
        .and_then(Value::as_object)
    {
        policy.latest_path = resolve_runtime_path(
            root,
            outputs.get("latest_path").and_then(Value::as_str),
            "local/state/autonomy/multi_agent_debate/latest.json",
        );
        policy.history_path = resolve_runtime_path(
            root,
            outputs.get("history_path").and_then(Value::as_str),
            "local/state/autonomy/multi_agent_debate/history.jsonl",
        );
        policy.receipts_path = resolve_runtime_path(
            root,
            outputs.get("receipts_path").and_then(Value::as_str),
            "local/state/autonomy/multi_agent_debate/receipts.jsonl",
        );
    }

    policy
}

fn normalize_candidates(input: &Value) -> Vec<Candidate> {
    let rows = input
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut out = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        let obj = row.as_object();
        let mut candidate_id = obj
            .and_then(|m| m.get("candidate_id"))
            .and_then(Value::as_str)
            .map(|v| normalize_token(v, 120))
            .unwrap_or_default();
        if candidate_id.is_empty() {
            candidate_id = obj
                .and_then(|m| m.get("metric_id"))
                .and_then(Value::as_str)
                .map(|v| normalize_token(v, 120))
                .unwrap_or_else(|| format!("candidate_{}", idx + 1));
        }
        if candidate_id.is_empty() {
            continue;
        }
        out.push(Candidate {
            id: candidate_id,
            score: clamp_num(
                obj.and_then(|m| m.get("score"))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.5),
                0.0,
                1.0,
                0.5,
            ),
            confidence: clamp_num(
                obj.and_then(|m| m.get("confidence"))
                    .and_then(Value::as_f64)
                    .unwrap_or(0.5),
                0.0,
                1.0,
                0.5,
            ),
            risk: {
                let r = obj
                    .and_then(|m| m.get("risk"))
                    .and_then(Value::as_str)
                    .map(|v| normalize_token(v, 32))
                    .unwrap_or_else(|| "medium".to_string());
                if matches!(r.as_str(), "low" | "medium" | "high") {
                    r
                } else {
                    "medium".to_string()
                }
            },
        });
    }

    out
}

fn build_agents(policy: &DebatePolicy, input: &Value) -> Vec<Agent> {
    let explicit = input.get("agents").and_then(Value::as_array);
    if let Some(rows) = explicit {
        let mut out = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            let obj = row.as_object();
            let agent_id = obj
                .and_then(|m| m.get("agent_id"))
                .and_then(Value::as_str)
                .map(|v| normalize_token(v, 120))
                .unwrap_or_else(|| format!("agent_{}", idx + 1));
            if agent_id.is_empty() {
                continue;
            }
            let role = obj
                .and_then(|m| m.get("role"))
                .and_then(Value::as_str)
                .map(|v| normalize_token(v, 80))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "orderly_executor".to_string());
            out.push(Agent { id: agent_id, role });
        }
        if !out.is_empty() {
            return out;
        }
    }

    policy
        .roles
        .keys()
        .map(|k| Agent {
            id: k.clone(),
            role: k.clone(),
        })
        .collect()
}

fn score_candidate_for_role(role_cfg: &RoleCfg, candidate: &Candidate) -> f64 {
    let base = candidate.score * candidate.confidence;
    let bias_boost = match role_cfg.bias.as_str() {
        "safety" => match candidate.risk.as_str() {
            "low" => 0.25,
            "medium" => 0.08,
            _ => -0.15,
        },
        "growth" => match candidate.risk.as_str() {
            "high" => 0.18,
            "medium" => 0.10,
            _ => 0.02,
        },
        "delivery" => match candidate.risk.as_str() {
            "medium" => 0.14,
            "low" => 0.10,
            _ => -0.08,
        },
        _ => 0.0,
    };
    round_to(
        clamp_num((base + bias_boost) * role_cfg.weight, 0.0, 1.0, 0.0),
        6,
    )
}

pub fn run_multi_agent_debate(
    root: &Path,
    input: &Value,
    explicit_policy_path: Option<&Path>,
    persist: bool,
    date_override: Option<&str>,
