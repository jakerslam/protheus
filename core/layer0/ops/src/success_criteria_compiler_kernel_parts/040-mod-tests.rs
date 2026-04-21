
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_rows_detects_token_and_duration_metrics() {
        let payload = json!([
            { "metric": "latency", "target": "under 5 s", "horizon": "next run" },
            { "metric": "token usage", "target": "at most 1.2k tokens" }
        ]);
        let rows = compile_success_criteria_rows(Some(&payload), "success_criteria");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].get("metric").and_then(Value::as_str),
            Some("duration_ms")
        );
        assert_eq!(
            rows[1].get("target").and_then(Value::as_str),
            Some("tokens <=1200")
        );
    }

    #[test]
    fn compile_proposal_remaps_outreach_metrics_for_non_outreach_capability() {
        let payload = json!({
            "proposal": {
                "success_criteria": [
                    { "metric": "reply_or_interview_count", "target": ">=1 interview signal" },
                    { "metric": "reply_or_interview_count", "target": ">=1 interview signal" }
                ]
            },
            "opts": {
                "capability_key": "proposal:maintenance_patch",
                "allow_fallback": false
            }
        });
        let rows = compile_proposal_success_criteria(payload_obj(&payload));
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].get("metric").and_then(Value::as_str),
            Some("artifact_count")
        );
    }
}
