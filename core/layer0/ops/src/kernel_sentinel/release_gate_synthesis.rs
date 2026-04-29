// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::Value;

pub fn release_gate_synthesis_status(architectural_incident_report: &Value) -> (u64, u64, u64, bool) {
    let multi_layer_incident_count = architectural_incident_report["multi_layer_incident_count"]
        .as_u64()
        .unwrap_or(0);
    let missing_architectural_synthesis_count = architectural_incident_report["synthesis_guard"]
        ["missing_architectural_synthesis_count"]
        .as_u64()
        .unwrap_or(0);
    let missing_remediation_classification_count = architectural_incident_report["synthesis_guard"]
        ["missing_remediation_classification_count"]
        .as_u64()
        .unwrap_or(0);
    let incident_synthesis_guard_pass = architectural_incident_report["synthesis_guard"]["pass"]
        .as_bool()
        .unwrap_or(multi_layer_incident_count == 0);
    (
        multi_layer_incident_count,
        missing_architectural_synthesis_count,
        missing_remediation_classification_count,
        incident_synthesis_guard_pass,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::governance::build_release_gate;
    use serde_json::json;

    #[test]
    fn release_gate_fails_when_multi_layer_incident_lacks_synthesis_or_remediation() {
        let issue = json!({"issue_drafts": []});
        let maintenance = json!({"suggestions": [], "automation_candidates": []});
        let governance = json!({"hard_fail_invariant_count": 0, "freshness_stale_count": 0});
        let evidence = json!({"normalized_record_count": 1, "data_starved": false, "observation_state": "healthy_observation"});
        let architectural_report = json!({
            "multi_layer_incident_count": 1,
            "synthesis_guard": {
                "pass": false,
                "missing_architectural_synthesis_count": 1,
                "missing_remediation_classification_count": 1
            }
        });
        let gate = build_release_gate(
            &[],
            &[],
            &architectural_report,
            &issue,
            &maintenance,
            &governance,
            &evidence,
        );
        assert_eq!(gate["pass"], false);
        assert_eq!(gate["incident_synthesis_guard_pass"], false);
        assert_eq!(gate["missing_architectural_synthesis_count"], 1);
        assert_eq!(gate["missing_remediation_classification_count"], 1);
    }
}
