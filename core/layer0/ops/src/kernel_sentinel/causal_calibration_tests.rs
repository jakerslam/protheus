use super::*;
use serde_json::json;
use std::fs;

fn hypothesis(id: &str, pattern: &str, invariant: &str, confidence: u64) -> Value {
    json!({
        "id": id,
        "finding_fingerprint": id.replace("causal_hypothesis:", ""),
        "pattern": pattern,
        "support_evidence": ["evidence://sentinel/fresh"],
        "counter_evidence": ["no direct contradiction observed yet; keep falsification probe mandatory"],
        "confidence_percent": confidence,
        "causal_power_score": 80,
        "causal_ladder": {
            "violated_invariant": invariant,
            "likely_root_cause": "authority shape survived a surface rewrite",
        },
        "falsification_probe": {
            "probe": "run boundary guard and inspect runtime trace",
            "expected_if_true": "authority-shaped fields remain",
            "expected_if_false": "runtime trace has no authority-shaped fields",
        },
        "next_action": "run the falsification probe before promotion",
    })
}

#[test]
fn calibration_tracks_hypotheses_and_tunes_pattern_scores_from_fix_results() {
    let state_dir = std::env::temp_dir().join(format!(
        "ksent-causal-calibration-{}",
        crate::deterministic_receipt_hash(&json!({"test": "causal-calibration"}))
    ));
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("causal_fix_results.jsonl"),
        serde_json::to_string(&json!({
            "type": "kernel_sentinel_fix_result",
            "validated_pattern": "authority_shape_residue",
        }))
        .unwrap(),
    )
    .unwrap();
    let synthesis = json!({
        "top_hypotheses": [
            hypothesis("causal_hypothesis:authority:a", "authority_shape_residue", "authority_removed_at_behavior_and_shape_level", 66),
            hypothesis("causal_hypothesis:authority:b", "authority_shape_residue", "authority_removed_at_behavior_and_shape_level", 72),
        ]
    });
    let report = build_kernel_sentinel_causal_calibration(&state_dir, &synthesis, &[]);
    assert_eq!(report["calibrated_hypothesis_count"], 2);
    assert_eq!(report["root_cause_clusters"][0]["occurrence_count"], 2);
    assert_eq!(report["pattern_scores"][0]["score"], 58);
    assert_eq!(report["promotion_ready_count"], 2);
    assert_eq!(
        report["current_ledger_entries"][0]["outcome_status"],
        "unresolved"
    );
    assert_eq!(report["safe_to_auto_apply_patch"], false);
    assert_eq!(
        report["final_report_summary"]["top_calibrated_hypotheses"][0]["promotion_ready"],
        true
    );
}

#[test]
fn promotion_gate_requires_a_falsification_probe() {
    let state_dir = std::env::temp_dir().join(format!(
        "ksent-causal-calibration-probe-{}",
        crate::deterministic_receipt_hash(&json!({"test": "causal-calibration-probe"}))
    ));
    fs::create_dir_all(&state_dir).unwrap();
    let mut row = hypothesis(
        "causal_hypothesis:authority:probe",
        "authority_shape_residue",
        "authority_removed_at_behavior_and_shape_level",
        90,
    );
    row["falsification_probe"] = json!({});
    let synthesis = json!({"top_hypotheses": [row]});
    let report = build_kernel_sentinel_causal_calibration(&state_dir, &synthesis, &[]);
    let gate = &report["calibrated_top_hypotheses"][0]["promotion_gate"];
    assert_eq!(gate["ok"], false);
    assert!(gate["missing_requirements"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row.as_str() == Some("falsification_probe")));
    assert_eq!(report["promotion_ready_count"], 0);
}
