mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn run_debate_reaches_runoff_consensus() {
        let dir = tempdir().expect("tmp");
        let root = dir.path();
        let policy_path = root.join("policy.json");
        let latest = root.join("local/state/latest.json");
        let history = root.join("local/state/history.jsonl");
        let receipts = root.join("local/state/receipts.jsonl");

        let policy = json!({
            "enabled": true,
            "rounds": { "max_rounds": 2, "min_agents": 3, "consensus_threshold": 0.7 },
            "debate_resolution": {
                "confidence_floor": 0.35,
                "disagreement_gap_threshold": 0.12,
                "runoff_enabled": true,
                "max_runoff_rounds": 1,
                "runoff_consensus_threshold": 0.57,
                "require_distinct_roles_for_quorum": true
            },
            "agent_roles": {
                "soldier_guard": { "weight": 1.1, "bias": "safety" },
                "creative_probe": { "weight": 1.0, "bias": "growth" },
                "orderly_executor": { "weight": 1.2, "bias": "delivery" }
            },
            "outputs": {
                "latest_path": latest,
                "history_path": history,
                "receipts_path": receipts
            }
        });
        write_json_atomic(&policy_path, &policy).expect("write policy");

        let input = json!({
            "objective_id": "mac_test",
            "objective": "Choose best value axis",
            "candidates": [
                { "candidate_id": "quality", "score": 0.72, "confidence": 0.72, "risk": "low" },
                { "candidate_id": "revenue", "score": 0.75, "confidence": 0.74, "risk": "high" },
                { "candidate_id": "learning", "score": 0.74, "confidence": 0.73, "risk": "medium" }
            ]
        });

        let out = run_multi_agent_debate(root, &input, Some(&policy_path), true, None);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("multi_agent_debate_orchestrator")
        );
        assert_eq!(
            out.get("debate_resolution")
                .and_then(Value::as_object)
                .and_then(|m| m.get("runoff_executed"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.get("debate_resolution")
                .and_then(Value::as_object)
                .and_then(|m| m.get("runoff_consensus"))
                .and_then(Value::as_bool),
            Some(true)
        );

        let status = debate_status(root, Some(&policy_path), None);
        assert_eq!(status.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            status.get("objective_id").and_then(Value::as_str),
            Some("mac_test")
        );
    }
}
