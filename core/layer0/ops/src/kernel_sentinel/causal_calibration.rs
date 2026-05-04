// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const CAUSAL_CALIBRATION_SCHEMA_VERSION: u32 = 1;
const DEFAULT_CALIBRATED_HYPOTHESIS_LIMIT: usize = 8;
const DEFAULT_PROMOTION_CONFIDENCE: u64 = 70;
const DEFAULT_PROMOTION_CAUSAL_POWER: u64 = 60;

#[derive(Default, Clone)]
struct OutcomeCounts {
    confirmed: u64,
    partially_confirmed: u64,
    contradicted: u64,
    unresolved: u64,
}

pub fn kernel_sentinel_causal_calibration_model() -> Value {
    json!({
        "type": "kernel_sentinel_causal_calibration_model",
        "schema_version": CAUSAL_CALIBRATION_SCHEMA_VERSION,
        "purpose": "calibrate Sentinel root-cause hypotheses against observed outcomes and fixes",
        "outcomes": ["confirmed", "partially_confirmed", "contradicted", "unresolved"],
        "artifacts": ["causal_hypothesis_ledger_current.jsonl", "causal_pattern_scores_current.json"],
        "authority": "draft_only_observability_feedback; no automatic TODO, GitHub issue, or patch mutation",
    })
}

pub(super) fn build_kernel_sentinel_causal_calibration(
    state_dir: &Path,
    synthesis: &Value,
    args: &[String],
) -> Value {
    let outcome_ledger_path = option_path(
        args,
        "--causal-outcome-ledger",
        state_dir.join("causal_hypothesis_outcomes.jsonl"),
    );
    let fix_results_path = option_path(
        args,
        "--causal-fix-results-path",
        state_dir.join("causal_fix_results.jsonl"),
    );
    let limit = option_usize(
        args,
        "--calibrated-hypothesis-limit",
        DEFAULT_CALIBRATED_HYPOTHESIS_LIMIT,
    );
    let outcome_rows = read_jsonl(&outcome_ledger_path);
    let fix_rows = read_jsonl(&fix_results_path);
    let outcome_counts = outcome_counts(&outcome_rows, &fix_rows);
    let cluster_sizes = cluster_sizes(synthesis);
    let mut calibrated = synthesis["top_hypotheses"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| calibrate_hypothesis(row, &outcome_counts, &cluster_sizes))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    calibrated.sort_by(|a, b| score_for_sort(b).cmp(&score_for_sort(a)));
    calibrated.truncate(limit);
    let current_ledger_entries = current_ledger_entries(&calibrated);
    let pattern_scores = pattern_scores_json(&outcome_counts);
    let clusters = root_cause_clusters(&calibrated);
    let promotion_ready_count = calibrated
        .iter()
        .filter(|row| row["calibrated_promotion_ready"].as_bool().unwrap_or(false))
        .count();
    let final_summary = final_report_summary(
        &outcome_ledger_path,
        &fix_results_path,
        &calibrated,
        &clusters,
        promotion_ready_count,
        &outcome_counts,
    );
    json!({
        "ok": true,
        "type": "kernel_sentinel_causal_calibration",
        "schema_version": CAUSAL_CALIBRATION_SCHEMA_VERSION,
        "model": kernel_sentinel_causal_calibration_model(),
        "outcome_ledger_path": outcome_ledger_path.display().to_string(),
        "fix_results_path": fix_results_path.display().to_string(),
        "outcome_record_count": outcome_rows.len(),
        "fix_result_record_count": fix_rows.len(),
        "calibrated_hypothesis_count": calibrated.len(),
        "promotion_ready_count": promotion_ready_count,
        "human_review_required": true,
        "safe_to_mutate_todo": false,
        "safe_to_file_github_issue": false,
        "safe_to_auto_apply_patch": false,
        "pattern_scores": pattern_scores,
        "root_cause_clusters": clusters,
        "current_ledger_entries": current_ledger_entries,
        "calibrated_top_hypotheses": calibrated,
        "final_report_summary": final_summary,
    })
}

pub(super) fn write_causal_calibration_artifacts(
    state_dir: &Path,
    report: &Value,
) -> Result<(), String> {
    let entries_path = state_dir.join("causal_hypothesis_ledger_current.jsonl");
    let score_path = state_dir.join("causal_pattern_scores_current.json");
    if let Some(parent) = entries_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let entries = report["causal_calibration"]["current_ledger_entries"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let body = entries
        .iter()
        .filter_map(|row| serde_json::to_string(row).ok())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        &entries_path,
        if body.is_empty() {
            body
        } else {
            format!("{body}\n")
        },
    )
    .map_err(|err| err.to_string())?;
    super::write_json(&score_path, &report["causal_calibration"]["pattern_scores"])
}

fn calibrate_hypothesis(
    row: &Value,
    counts_by_pattern: &BTreeMap<String, OutcomeCounts>,
    cluster_sizes: &BTreeMap<String, usize>,
) -> Value {
    let pattern = text(row, "pattern");
    let counts = counts_by_pattern.get(&pattern).cloned().unwrap_or_default();
    let delta = confidence_delta(&counts);
    let base_confidence = row["confidence_percent"].as_u64().unwrap_or(0);
    let calibrated_confidence = add_delta(base_confidence, delta, 0, 99);
    let cluster_key = cluster_key(row);
    let cluster_size = cluster_sizes.get(&cluster_key).copied().unwrap_or(1);
    let gate = promotion_gate(row, calibrated_confidence, cluster_size, &counts);
    let mut output = row.clone();
    output["calibration"] = json!({
        "outcome_counts": counts_json(&counts),
        "confidence_delta": delta,
        "calibrated_confidence_percent": calibrated_confidence,
        "pattern_trust_score": pattern_trust_score(&counts),
        "cluster_key": cluster_key,
        "cluster_size": cluster_size,
    });
    output["calibrated_promotion_ready"] = json!(gate["ok"].as_bool().unwrap_or(false));
    output["promotion_gate"] = gate;
    output
}

fn promotion_gate(
    row: &Value,
    calibrated_confidence: u64,
    cluster_size: usize,
    counts: &OutcomeCounts,
) -> Value {
    let mut missing = Vec::new();
    if row["support_evidence"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0)
        == 0
    {
        missing.push("support_evidence");
    }
    if row["counter_evidence"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0)
        == 0
    {
        missing.push("counter_evidence");
    }
    if text_at(row, &["falsification_probe", "probe"]).is_empty() {
        missing.push("falsification_probe");
    }
    if owner_guess(row).is_empty() {
        missing.push("owner_guess");
    }
    if text_at(row, &["causal_ladder", "likely_root_cause"]).is_empty() {
        missing.push("root_cause_hypothesis");
    }
    if !concrete_next_action(&text(row, "next_action")) {
        missing.push("concrete_next_action");
    }
    if calibrated_confidence < DEFAULT_PROMOTION_CONFIDENCE {
        missing.push("calibrated_confidence");
    }
    if row["causal_power_score"].as_u64().unwrap_or(0) < DEFAULT_PROMOTION_CAUSAL_POWER {
        missing.push("causal_power");
    }
    let recurrent_or_confirmed =
        cluster_size > 1 || counts.confirmed + counts.partially_confirmed > 0;
    if !recurrent_or_confirmed {
        missing.push("recurrence_or_confirmed_outcome");
    }
    json!({
        "ok": missing.is_empty(),
        "missing_requirements": missing,
        "owner_guess": owner_guess(row),
        "human_review_required": true,
        "safe_to_mutate_todo": false,
        "safe_to_file_github_issue": false,
        "safe_to_auto_apply_patch": false,
        "policy": "promote_only_evidenced_recurrent_or_confirmed_falsifiable_hypotheses",
    })
}

fn current_ledger_entries(rows: &[Value]) -> Vec<Value> {
    rows.iter()
        .map(|row| {
            json!({
                "type": "kernel_sentinel_causal_hypothesis_ledger_entry",
                "schema_version": CAUSAL_CALIBRATION_SCHEMA_VERSION,
                "generated_at": crate::now_iso(),
                "hypothesis_id": text(row, "id"),
                "finding_fingerprint": text(row, "finding_fingerprint"),
                "pattern": text(row, "pattern"),
                "outcome_status": "unresolved",
                "calibrated_confidence_percent": row["calibration"]["calibrated_confidence_percent"].clone(),
                "promotion_ready": row["calibrated_promotion_ready"].clone(),
                "falsification_probe": row["falsification_probe"]["probe"].clone(),
                "next_action": row["next_action"].clone(),
            })
        })
        .collect()
}

fn final_report_summary(
    ledger_path: &Path,
    fix_path: &Path,
    rows: &[Value],
    clusters: &[Value],
    promotion_ready_count: usize,
    counts: &BTreeMap<String, OutcomeCounts>,
) -> Value {
    let top = rows
        .iter()
        .take(3)
        .map(compact_hypothesis)
        .collect::<Vec<_>>();
    json!({
        "type": "kernel_sentinel_causal_calibration_summary",
        "ledger_path": ledger_path.display().to_string(),
        "fix_results_path": fix_path.display().to_string(),
        "calibrated_hypothesis_count": rows.len(),
        "promotion_ready_count": promotion_ready_count,
        "root_cause_cluster_count": clusters.len(),
        "outcome_summary": total_counts_json(counts),
        "top_calibrated_hypotheses": top,
        "human_review_required": true,
    })
}

fn compact_hypothesis(row: &Value) -> Value {
    json!({
        "hypothesis_id": text(row, "id"),
        "pattern": text(row, "pattern"),
        "calibrated_confidence_percent": row["calibration"]["calibrated_confidence_percent"].clone(),
        "pattern_trust_score": row["calibration"]["pattern_trust_score"].clone(),
        "promotion_ready": row["calibrated_promotion_ready"].clone(),
        "missing_requirements": row["promotion_gate"]["missing_requirements"].clone(),
        "falsification_probe": row["falsification_probe"]["probe"].clone(),
    })
}

fn root_cause_clusters(rows: &[Value]) -> Vec<Value> {
    let mut order = Vec::new();
    let mut clusters: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for row in rows {
        let key = cluster_key(row);
        if !clusters.contains_key(&key) {
            order.push(key.clone());
        }
        clusters.entry(key).or_default().push(row);
    }
    order
        .iter()
        .filter_map(|key| clusters.get(key).map(|rows| cluster_json(key, rows)))
        .collect()
}

fn cluster_json(key: &str, rows: &[&Value]) -> Value {
    let exemplar = rows.first().copied().unwrap_or(&Value::Null);
    let ids = rows
        .iter()
        .map(|row| text(row, "id"))
        .take(6)
        .collect::<Vec<_>>();
    json!({
        "cluster_key": key,
        "pattern": text(exemplar, "pattern"),
        "violated_invariant": text_at(exemplar, &["causal_ladder", "violated_invariant"]),
        "occurrence_count": rows.len(),
        "sample_hypothesis_ids": ids,
        "dedupe_policy": "multiple symptoms under one pattern/invariant collapse to one structural hypothesis",
    })
}

fn outcome_counts(outcome_rows: &[Value], fix_rows: &[Value]) -> BTreeMap<String, OutcomeCounts> {
    let mut counts = BTreeMap::new();
    for row in outcome_rows.iter().chain(fix_rows.iter()) {
        let patterns = patterns_for_row(row);
        let outcome = outcome_for_row(row);
        for pattern in patterns {
            let entry = counts.entry(pattern).or_insert_with(OutcomeCounts::default);
            match outcome.as_str() {
                "confirmed" => entry.confirmed += 1,
                "partially_confirmed" => entry.partially_confirmed += 1,
                "contradicted" | "wrong" => entry.contradicted += 1,
                _ => entry.unresolved += 1,
            }
        }
    }
    counts
}

fn patterns_for_row(row: &Value) -> BTreeSet<String> {
    let mut patterns = BTreeSet::new();
    for key in [
        "pattern",
        "validated_pattern",
        "partially_validated_pattern",
        "contradicted_pattern",
    ] {
        let value = text(row, key);
        if !value.is_empty() {
            patterns.insert(value);
        }
    }
    patterns
}

fn outcome_for_row(row: &Value) -> String {
    let raw = text(row, "outcome");
    if !raw.is_empty() {
        return raw;
    }
    if !text(row, "validated_hypothesis_id").is_empty()
        || !text(row, "validated_pattern").is_empty()
    {
        "confirmed".to_string()
    } else if !text(row, "partially_validated_hypothesis_id").is_empty()
        || !text(row, "partially_validated_pattern").is_empty()
    {
        "partially_confirmed".to_string()
    } else if !text(row, "contradicted_hypothesis_id").is_empty()
        || !text(row, "contradicted_pattern").is_empty()
    {
        "contradicted".to_string()
    } else {
        "unresolved".to_string()
    }
}

fn pattern_scores_json(counts: &BTreeMap<String, OutcomeCounts>) -> Vec<Value> {
    counts
        .iter()
        .map(|(pattern, counts)| {
            json!({
                "pattern": pattern,
                "score": pattern_trust_score(counts),
                "confidence_delta": confidence_delta(counts),
                "outcome_counts": counts_json(counts),
            })
        })
        .collect()
}

fn confidence_delta(counts: &OutcomeCounts) -> i64 {
    ((counts.confirmed as i64) * 8 + (counts.partially_confirmed as i64) * 4
        - (counts.contradicted as i64) * 10
        - (counts.unresolved.min(4) as i64))
        .clamp(-20, 20)
}

fn pattern_trust_score(counts: &OutcomeCounts) -> u64 {
    add_delta(50, confidence_delta(counts), 0, 100)
}

fn counts_json(counts: &OutcomeCounts) -> Value {
    json!({
        "confirmed": counts.confirmed,
        "partially_confirmed": counts.partially_confirmed,
        "contradicted": counts.contradicted,
        "unresolved": counts.unresolved,
    })
}

fn total_counts_json(counts: &BTreeMap<String, OutcomeCounts>) -> Value {
    let mut total = OutcomeCounts::default();
    for counts in counts.values() {
        total.confirmed += counts.confirmed;
        total.partially_confirmed += counts.partially_confirmed;
        total.contradicted += counts.contradicted;
        total.unresolved += counts.unresolved;
    }
    counts_json(&total)
}

fn cluster_sizes(synthesis: &Value) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    if let Some(rows) = synthesis["top_hypotheses"].as_array() {
        for row in rows {
            *counts.entry(cluster_key(row)).or_insert(0) += 1;
        }
    }
    counts
}

fn cluster_key(row: &Value) -> String {
    format!(
        "{}|{}",
        normalized(&text(row, "pattern")),
        normalized(&text_at(row, &["causal_ladder", "violated_invariant"]))
    )
}

fn owner_guess(row: &Value) -> String {
    match text(row, "pattern").as_str() {
        "gateway_lifecycle_truth_contradiction" | "installed_runtime_identity_invalid" => {
            "gateways".to_string()
        }
        "observability_noise_release" | "boundedness_budget_regression" => {
            "observability".to_string()
        }
        "authority_shape_residue" | "receipt_integrity_gap" => "kernel".to_string(),
        "projection_surface_became_runtime_owner" => "gateway_shell_boundary".to_string(),
        "response_finalization_gap" => "orchestration_control_plane".to_string(),
        _ => "kernel_sentinel".to_string(),
    }
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .map(|body| {
            body.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn option_path(args: &[String], name: &str, fallback: PathBuf) -> PathBuf {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(PathBuf::from))
        .unwrap_or(fallback)
}

fn option_usize(args: &[String], name: &str, fallback: usize) -> usize {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| {
            arg.strip_prefix(&prefix)
                .and_then(|raw| raw.parse::<usize>().ok())
        })
        .unwrap_or(fallback)
}

fn score_for_sort(row: &Value) -> u64 {
    row["calibration"]["calibrated_confidence_percent"]
        .as_u64()
        .unwrap_or(0)
        * 1000
        + row["causal_power_score"].as_u64().unwrap_or(0)
}

fn add_delta(value: u64, delta: i64, min: u64, max: u64) -> u64 {
    (value as i64 + delta).clamp(min as i64, max as i64) as u64
}

fn concrete_next_action(action: &str) -> bool {
    action.trim().chars().count() >= 16 && action.split_whitespace().count() >= 4
}

fn text(row: &Value, key: &str) -> String {
    row[key].as_str().unwrap_or("").to_string()
}

fn text_at(row: &Value, keys: &[&str]) -> String {
    let mut current = row;
    for key in keys {
        current = &current[*key];
    }
    current.as_str().unwrap_or("").to_string()
}

fn normalized(raw: &str) -> String {
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
#[path = "causal_calibration_tests.rs"]
mod causal_calibration_tests;
