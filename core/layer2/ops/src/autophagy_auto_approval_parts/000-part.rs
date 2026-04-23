// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, parse_cli_flag, print_json_line};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_POLICY_PATH: &str = "client/runtime/config/autophagy_auto_approval_policy.json";
const DEFAULT_STATE_PATH: &str = "local/state/autonomy/autophagy_auto_approval/state.json";
const DEFAULT_LATEST_PATH: &str = "local/state/autonomy/autophagy_auto_approval/latest.json";
const DEFAULT_RECEIPTS_PATH: &str = "local/state/autonomy/autophagy_auto_approval/receipts.jsonl";
const DEFAULT_REGRETS_PATH: &str = "local/state/autonomy/autophagy_auto_approval/regrets.jsonl";

const USAGE: &[&str] = &[
    "Usage:",
    "  infring-ops autophagy-auto-approval evaluate --proposal-json=<json>|--proposal-file=<path> [--apply=1|0] [--policy=<path>] [--state-path=<path>] [--latest-path=<path>] [--receipts-path=<path>] [--regrets-path=<path>]",
    "  infring-ops autophagy-auto-approval monitor --proposal-id=<id> [--drift=<float>] [--yield-drop=<float>] [--apply=1|0] [--policy=<path>] [--state-path=<path>] [--latest-path=<path>] [--receipts-path=<path>] [--regrets-path=<path>]",
    "  infring-ops autophagy-auto-approval commit --proposal-id=<id> [--reason=<text>] [--policy=<path>] [--state-path=<path>] [--latest-path=<path>] [--receipts-path=<path>] [--regrets-path=<path>]",
    "  infring-ops autophagy-auto-approval rollback --proposal-id=<id> [--reason=<text>] [--policy=<path>] [--state-path=<path>] [--latest-path=<path>] [--receipts-path=<path>] [--regrets-path=<path>]",
    "  infring-ops autophagy-auto-approval status [--policy=<path>] [--state-path=<path>]",
];

#[derive(Clone, Debug)]
struct Policy {
    enabled: bool,
    min_confidence: f64,
    min_historical_success_rate: f64,
    max_impact_score: f64,
    excluded_types: Vec<String>,
    auto_rollback_on_degradation: bool,
    max_drift_delta: f64,
    max_yield_drop: f64,
    rollback_window_minutes: i64,
    regret_issue_label: String,
    state_path: PathBuf,
    latest_path: PathBuf,
    receipts_path: PathBuf,
    regrets_path: PathBuf,
}

#[derive(Clone, Debug)]
struct ProposalSummary {
    id: String,
    title: String,
    proposal_type: String,
    confidence: f64,
    historical_success_rate: f64,
    impact_score: f64,
    raw: Value,
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn usage() {
    for line in USAGE {
        println!("{line}");
    }
}

fn parse_bool(raw: Option<&str>, fallback: bool) -> bool {
    let Some(v) = raw else {
        return fallback;
    };
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn parse_f64(raw: Option<&str>) -> Option<f64> {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
}

fn resolve_path(root: &Path, raw: Option<String>, fallback: &Path) -> PathBuf {
    let path = raw
        .map(PathBuf::from)
        .unwrap_or_else(|| fallback.to_path_buf());
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("missing_parent_for_path:{}", path.display()))?;
    fs::create_dir_all(parent).map_err(|e| format!("create_dir_all_failed:{e}"))
}

fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|e| format!("encode_json_failed:{e}"))?,
    )
    .map_err(|e| format!("write_json_failed:{e}"))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut existing = fs::read_to_string(path).unwrap_or_default();
    existing
        .push_str(&serde_json::to_string(value).map_err(|e| format!("encode_jsonl_failed:{e}"))?);
    existing.push('\n');
    fs::write(path, existing).map_err(|e| format!("write_jsonl_failed:{e}"))
}

fn array_from<'a>(object: &'a mut Map<String, Value>, key: &str) -> &'a mut Vec<Value> {
    let value = object
        .entry(key.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !value.is_array() {
        *value = Value::Array(Vec::new());
    }
    value.as_array_mut().expect("array")
}

fn value_string(value: Option<&Value>, fallback: &str) -> String {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn value_f64(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback)
}

fn stable_proposal_id(proposal: &Value) -> String {
    let title = proposal
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("proposal");
    let kind = proposal
        .get("type")
        .or_else(|| proposal.get("kind"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("generic");
    let seed = json!({
        "title": title,
        "proposal_type": kind,
        "payload": proposal
    });
    deterministic_receipt_hash(&seed)[..16].to_string()
}

fn load_policy(root: &Path, argv: &[String]) -> Policy {
    let policy_path = resolve_path(
        root,
        parse_cli_flag(argv, "policy"),
        Path::new(DEFAULT_POLICY_PATH),
    );
    let raw = read_json(&policy_path).unwrap_or_else(|| json!({}));
    let auto = raw
        .get("auto_approval")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let degradation = auto
        .get("degradation_threshold")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let paths = raw
        .get("paths")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let state_path = resolve_path(
        root,
        parse_cli_flag(argv, "state-path").or_else(|| {
            paths
                .get("state_path")
                .and_then(Value::as_str)
                .map(str::to_string)
        }),
        Path::new(DEFAULT_STATE_PATH),
    );
    let latest_path = resolve_path(
        root,
        parse_cli_flag(argv, "latest-path").or_else(|| {
            paths
                .get("latest_path")
                .and_then(Value::as_str)
                .map(str::to_string)
        }),
        Path::new(DEFAULT_LATEST_PATH),
    );
    let receipts_path = resolve_path(
        root,
        parse_cli_flag(argv, "receipts-path").or_else(|| {
            paths
                .get("receipts_path")
                .and_then(Value::as_str)
                .map(str::to_string)
        }),
        Path::new(DEFAULT_RECEIPTS_PATH),
    );
    let regrets_path = resolve_path(
        root,
        parse_cli_flag(argv, "regrets-path").or_else(|| {
            paths
                .get("regrets_path")
                .and_then(Value::as_str)
                .map(str::to_string)
        }),
        Path::new(DEFAULT_REGRETS_PATH),
    );

    Policy {
        enabled: raw.get("enabled").and_then(Value::as_bool).unwrap_or(true)
            && auto.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        min_confidence: auto
            .get("min_confidence")
            .and_then(Value::as_f64)
            .unwrap_or(0.85),
        min_historical_success_rate: auto
            .get("min_historical_success_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.90),
        max_impact_score: auto
            .get("max_impact_score")
            .and_then(Value::as_f64)
            .unwrap_or(50.0),
        excluded_types: auto
            .get("excluded_types")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(|v| v.trim().to_ascii_lowercase())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        auto_rollback_on_degradation: auto
            .get("auto_rollback_on_degradation")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        max_drift_delta: degradation
            .get("max_drift_delta")
            .and_then(Value::as_f64)
            .unwrap_or(0.01),
        max_yield_drop: degradation
            .get("max_yield_drop")
            .and_then(Value::as_f64)
            .unwrap_or(0.05),
        rollback_window_minutes: auto
            .get("rollback_window_minutes")
            .and_then(Value::as_i64)
            .unwrap_or(30)
            .clamp(1, 10080),
        regret_issue_label: auto
            .get("regret_issue_label")
            .and_then(Value::as_str)
            .unwrap_or("auto_approval_regret")
            .to_string(),
        state_path,
        latest_path,
        receipts_path,
        regrets_path,
    }
}

fn load_state(state_path: &Path) -> Value {
    read_json(state_path).unwrap_or_else(|| {
        json!({
            "version": "1.0",
            "pending_commit": [],
            "committed": [],
            "rolled_back": []
        })
    })
}

fn store_state(policy: &Policy, state: &Value) -> Result<(), String> {
    write_json(&policy.state_path, state)
}

fn parse_proposal(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = parse_cli_flag(argv, "proposal-json") {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|e| format!("proposal_json_parse_failed:{e}"));
    }
    if let Some(file) = parse_cli_flag(argv, "proposal-file") {
        let raw = fs::read_to_string(file).map_err(|e| format!("proposal_file_read_failed:{e}"))?;
        return serde_json::from_str::<Value>(&raw)
            .map_err(|e| format!("proposal_file_parse_failed:{e}"));
    }
    Err("missing_proposal_payload".to_string())
}

fn proposal_summary(proposal: &Value) -> ProposalSummary {
    let id = proposal
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| stable_proposal_id(proposal));
    ProposalSummary {
        id,
        title: value_string(proposal.get("title"), "Untitled proposal"),
        proposal_type: value_string(
            proposal
                .get("proposal_type")
                .or_else(|| proposal.get("type"))
                .or_else(|| proposal.get("kind")),
            "generic",
        )
        .to_ascii_lowercase(),
        confidence: value_f64(proposal.get("confidence"), 0.0),
        historical_success_rate: value_f64(
            proposal
                .get("historical_success_rate")
                .or_else(|| proposal.get("historical_success")),
            0.0,
        ),
        impact_score: value_f64(proposal.get("impact_score"), 100.0),
        raw: proposal.clone(),
    }
}

fn evaluate_proposal(policy: &Policy, proposal: &ProposalSummary) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();
    if !policy.enabled {
        reasons.push("auto_approval_disabled".to_string());
    }
    if policy
        .excluded_types
        .iter()
        .any(|entry| entry == &proposal.proposal_type)
    {
        reasons.push(format!("excluded_type:{}", proposal.proposal_type));
    }
    if proposal.confidence < policy.min_confidence {
        reasons.push(format!(
            "confidence_below_floor:{:.3}<{:.3}",
            proposal.confidence, policy.min_confidence
        ));
    }
    if proposal.historical_success_rate < policy.min_historical_success_rate {
        reasons.push(format!(
            "historical_success_below_floor:{:.3}<{:.3}",
            proposal.historical_success_rate, policy.min_historical_success_rate
        ));
    }
    if proposal.impact_score > policy.max_impact_score {
        reasons.push(format!(
            "impact_score_above_cap:{:.3}>{:.3}",
            proposal.impact_score, policy.max_impact_score
        ));
    }
    (reasons.is_empty(), reasons)
}

fn remove_entry(rows: &mut Vec<Value>, proposal_id: &str) -> Option<Value> {
    let idx = rows.iter().position(|row| {
        row.get("proposal")
            .and_then(Value::as_object)
            .and_then(|proposal| proposal.get("id"))
            .and_then(Value::as_str)
            == Some(proposal_id)
    })?;
    Some(rows.remove(idx))
}

fn insert_pending(state: &mut Value, pending: Value) {
    let object = state.as_object_mut().expect("state object");
    let rows = array_from(object, "pending_commit");
    let proposal_id = pending
        .get("proposal")
        .and_then(Value::as_object)
        .and_then(|proposal| proposal.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if !proposal_id.is_empty() {
        rows.retain(|row| {
            row.get("proposal")
                .and_then(Value::as_object)
                .and_then(|proposal| proposal.get("id"))
                .and_then(Value::as_str)
                != Some(proposal_id)
        });
    }
    rows.push(pending);
}

fn base_receipt(kind: &str, command: &str, policy: &Policy) -> Value {
    json!({
        "ok": true,
        "type": kind,
        "authority": "core/layer2/ops",
        "command": command,
        "state_path": policy.state_path.to_string_lossy(),
        "latest_path": policy.latest_path.to_string_lossy(),
        "receipts_path": policy.receipts_path.to_string_lossy(),
        "regrets_path": policy.regrets_path.to_string_lossy(),
        "ts_epoch_ms": now_epoch_ms()
    })
