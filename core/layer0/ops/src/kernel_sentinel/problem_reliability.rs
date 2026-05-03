// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub(super) fn stream_problem_reliability(state_dir: &Path, issues: &[Value]) -> Value {
    let active_falsification = active_falsification_plan(issues);
    let historical_calibration = historical_calibration(state_dir, issues);
    let architecture_patterns = architecture_pattern_detection(issues);
    json!({
        "type": "kernel_sentinel_problem_finding_reliability",
        "schema_version": 1,
        "generated_at": crate::now_iso(),
        "purpose": "make Sentinel problem discovery falsifiable, historically calibrated, and architecture-aware before solution autonomy",
        "active_falsification": active_falsification,
        "historical_calibration": historical_calibration,
        "architecture_pattern_detection": architecture_patterns,
        "authority": {
            "human_review_required": true,
            "safe_to_file_issue": false,
            "safe_to_mutate_todo": false,
            "safe_to_auto_apply_patch": false
        }
    })
}

fn active_falsification_plan(issues: &[Value]) -> Value {
    let mut seen = BTreeSet::new();
    let mut probes = Vec::new();
    for row in issues {
        let pattern = super::report_output::stream_pattern_id(row);
        let fingerprint = text(row, "fingerprint");
        let key = format!(
            "{pattern}|{}",
            super::report_output::stream_owner_guess(row)
        );
        if !seen.insert(key) {
            continue;
        }
        probes.push(json!({
            "pattern": pattern,
            "finding_fingerprint": fingerprint,
            "owner_guess": super::report_output::stream_owner_guess(row),
            "probe": super::report_output::stream_falsification_probe(row),
            "suggested_command": suggested_probe_command(row),
            "execution_mode": "bounded_probe_request",
            "safe_to_auto_run": safe_to_auto_run(row),
            "expected_true_signal": expected_true_signal(row),
            "expected_false_signal": expected_false_signal(row),
            "promotion_rule": "do_not_promote_problem_hypothesis_until_probe_is_run_or_explicitly_deferred"
        }));
        if probes.len() >= 6 {
            break;
        }
    }
    let ready = probes
        .iter()
        .filter(|row| row["safe_to_auto_run"].as_bool().unwrap_or(false))
        .count();
    json!({
        "policy": "attach bounded falsification probes to each promoted structural pattern; run only safe deterministic probes automatically",
        "probe_count": probes.len(),
        "safe_auto_probe_count": ready,
        "probes": probes
    })
}

fn historical_calibration(state_dir: &Path, issues: &[Value]) -> Value {
    let score_path = state_dir.join("causal_pattern_scores_current.json");
    let score_rows = read_json_array(&score_path);
    let scores = score_rows
        .iter()
        .filter_map(|row| Some((text(row, "pattern"), score_for_row(row)?)))
        .filter(|(pattern, _)| !pattern.is_empty())
        .collect::<BTreeMap<_, _>>();
    let mut observed = pattern_counts(issues)
        .into_iter()
        .map(|(pattern, occurrences)| {
            let prior = scores.get(&pattern).copied();
            json!({
                "pattern": pattern,
                "current_occurrence_count": occurrences,
                "prior_pattern_score": prior,
                "calibration_state": if prior.is_some() { "history_available" } else { "needs_outcome_history" },
                "confidence_adjustment": confidence_adjustment(prior)
            })
        })
        .collect::<Vec<_>>();
    observed.sort_by(|a, b| {
        b["current_occurrence_count"]
            .as_u64()
            .unwrap_or(0)
            .cmp(&a["current_occurrence_count"].as_u64().unwrap_or(0))
    });
    json!({
        "policy": "confidence must be calibrated by confirmed/contradicted/fixed/stale outcomes over time",
        "score_path": score_path.display().to_string(),
        "history_available": !scores.is_empty(),
        "historical_pattern_count": scores.len(),
        "observed_pattern_count": observed.len(),
        "observed_patterns": observed
    })
}

fn architecture_pattern_detection(issues: &[Value]) -> Value {
    let counts = pattern_counts(issues);
    let owners = owner_set(issues);
    let total_occurrences = counts.values().sum::<u64>();
    let mut patterns = Vec::new();

    if has(&counts, "receipt_integrity_gap") {
        patterns.push(architecture_pattern(
            "authority_truth_drift",
            "Kernel evidence and receipt truth are diverging; treat this as a source-of-truth risk, not a local symptom.",
            "kernel",
            "receipt_integrity_gap",
            86,
        ));
    }
    if has(&counts, "response_finalization_gap") && has(&counts, "eval_feedback_recurrence_gap") {
        patterns.push(architecture_pattern(
            "eval_runtime_overlap",
            "Response symptoms are recurring in both runtime finalization evidence and eval feedback; split Eval quality judgment from Sentinel runtime failure before promotion.",
            "observability",
            "response_finalization_gap + eval_feedback_recurrence_gap",
            82,
        ));
    }
    if total_occurrences >= 100 && owners.len() >= 2 {
        patterns.push(architecture_pattern(
            "fragmented_observability_feedback_absorption",
            "Many findings span more than one owner, suggesting the feedback loop is observing problems faster than the system absorbs them.",
            "observability",
            "cross_owner_high_recurrence",
            78,
        ));
    }
    if counts.len() >= 3 && total_occurrences >= 50 {
        patterns.push(architecture_pattern(
            "local_ticketing_should_pause",
            "Multiple recurring problem families are active at once; require structural diagnosis before creating more local symptom tickets.",
            "governance",
            "multi_pattern_recurrence",
            76,
        ));
    }

    json!({
        "policy": "Sentinel must escalate from local symptoms to structural diagnosis when recurring evidence crosses owners or architecture boundaries.",
        "pattern_count": patterns.len(),
        "detected_patterns": patterns,
        "owner_count": owners.len(),
        "total_occurrence_count": total_occurrences,
        "local_ticketing_gate": if patterns.iter().any(|row| text(row, "pattern_id") == "local_ticketing_should_pause") { "pause_for_structural_diagnosis" } else { "local_ticketing_allowed" }
    })
}

fn architecture_pattern(
    id: &str,
    hypothesis: &str,
    owner: &str,
    evidence_pattern: &str,
    confidence: u64,
) -> Value {
    json!({
        "pattern_id": id,
        "hypothesis": hypothesis,
        "owner_guess": owner,
        "evidence_pattern": evidence_pattern,
        "confidence_percent": confidence,
        "promotion_state": "review_only_structural_diagnosis",
        "required_next_step": "confirm with falsification probe and historical calibration before issue/TODO promotion"
    })
}

fn suggested_probe_command(row: &Value) -> String {
    match super::report_output::stream_pattern_id(row).as_str() {
        "receipt_integrity_gap" => "cargo test --manifest-path core/layer0/ops/Cargo.toml kernel_sentinel -- --nocapture",
        "response_finalization_gap" => "cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_self_play_empty_reply_keeps_trace_diagnostic_from_beginning_to_end -- --nocapture",
        "eval_feedback_recurrence_gap" => "cargo test --manifest-path core/layer0/ops/Cargo.toml kernel_sentinel::self_study_feedback_tests -- --nocapture",
        _ => "infring-ops kernel-sentinel report --strict=0 --watch-refresh=1",
    }
    .to_string()
}

fn safe_to_auto_run(row: &Value) -> bool {
    matches!(
        super::report_output::stream_pattern_id(row).as_str(),
        "receipt_integrity_gap" | "response_finalization_gap"
    ) && !super::report_output::stream_falsification_probe(row).is_empty()
}

fn expected_true_signal(row: &Value) -> String {
    match super::report_output::stream_pattern_id(row).as_str() {
        "receipt_integrity_gap" => {
            "probe reproduces receipt/action drift or missing authoritative receipt linkage"
        }
        "response_finalization_gap" => {
            "probe reproduces execution evidence without assistant-visible finalization"
        }
        "eval_feedback_recurrence_gap" => "eval recurrence remains after dedupe and owner split",
        _ => "probe reproduces the finding fingerprint",
    }
    .to_string()
}

fn expected_false_signal(row: &Value) -> String {
    match super::report_output::stream_pattern_id(row).as_str() {
        "receipt_integrity_gap" => {
            "receipt and action evidence converge and the drift fingerprint stops emitting"
        }
        "response_finalization_gap" => {
            "assistant-visible finalization is emitted for the affected scenario family"
        }
        "eval_feedback_recurrence_gap" => {
            "recurring evaluator signature drops below promotion threshold"
        }
        _ => "finding fingerprint disappears or is contradicted by fresh evidence",
    }
    .to_string()
}

fn pattern_counts(issues: &[Value]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for row in issues {
        let pattern = super::report_output::stream_pattern_id(row);
        *counts.entry(pattern).or_insert(0) += row["occurrence_count"].as_u64().unwrap_or(1);
    }
    counts
}

fn owner_set(issues: &[Value]) -> BTreeSet<String> {
    issues
        .iter()
        .map(super::report_output::stream_owner_guess)
        .filter(|owner| !owner.is_empty())
        .collect()
}

fn has(counts: &BTreeMap<String, u64>, pattern: &str) -> bool {
    counts.get(pattern).copied().unwrap_or(0) > 0
}

fn read_json_array(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
}

fn score_for_row(row: &Value) -> Option<u64> {
    row["score"]
        .as_u64()
        .or_else(|| row["trust_score"].as_u64())
        .or_else(|| row["pattern_trust_score"].as_u64())
}

fn confidence_adjustment(prior: Option<u64>) -> &'static str {
    match prior {
        Some(80..) => "raise_if_current_evidence_is_fresh",
        Some(50..=79) => "hold_steady_until_probe_confirms",
        Some(1..=49) => "downgrade_until_new_confirmation",
        Some(0) => "unknown_prior",
        None => "no_history_do_not_overtrust",
    }
}

fn text(row: &Value, key: &str) -> String {
    row[key].as_str().unwrap_or("").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn issue(fingerprint: &str, severity: &str, occurrences: u64) -> Value {
        json!({
            "title": "Synthetic user harness failure in casual_greeting_direct_answer/hello_001: empty_assistant_response",
            "severity": severity,
            "category": "runtime_correctness",
            "fingerprint": fingerprint,
            "occurrence_count": occurrences,
            "evidence": ["fixture://evidence"],
            "summary": "empty_assistant_response"
        })
    }

    #[test]
    fn response_and_eval_overlap_becomes_structural_pattern() {
        let issues = vec![
            issue("synthetic_user_chat_harness:failures", "high", 18),
            issue("eval_agent_feedback:agent.attention", "high", 67),
        ];
        let reliability = stream_problem_reliability(&temp_dir(), &issues);
        let detected = reliability["architecture_pattern_detection"]["detected_patterns"]
            .as_array()
            .unwrap();
        assert!(detected
            .iter()
            .any(|row| text(row, "pattern_id") == "eval_runtime_overlap"));
        assert_eq!(
            reliability["active_falsification"]["safe_auto_probe_count"].as_u64(),
            Some(1)
        );
    }

    #[test]
    fn calibration_reads_existing_pattern_scores() {
        let dir = temp_dir();
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("causal_pattern_scores_current.json"),
            r#"[{"pattern":"response_finalization_gap","score":88}]"#,
        )
        .unwrap();
        let reliability = stream_problem_reliability(
            &dir,
            &[issue("synthetic_user_chat_harness:failures", "high", 3)],
        );
        assert_eq!(
            reliability["historical_calibration"]["history_available"].as_bool(),
            Some(true)
        );
        assert_eq!(
            reliability["historical_calibration"]["observed_patterns"][0]["prior_pattern_score"]
                .as_u64(),
            Some(88)
        );
    }

    fn temp_dir() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("sentinel-problem-reliability-{stamp}"))
    }
}
