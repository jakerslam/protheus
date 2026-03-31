#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn simulation_identity_projection_filters_unknown_objective() {
        let dir = tempdir().expect("tmp");
        let root = dir.path();
        let runs_dir = dir.path().join("runs");
        let proposals_dir = dir.path().join("proposals");
        let day = "2026-02-25";

        std::fs::create_dir_all(&runs_dir).expect("runs dir");
        std::fs::create_dir_all(&proposals_dir).expect("proposals dir");

        append_jsonl(
            &runs_dir.join(format!("{day}.jsonl")),
            &json!({
                "ts": "2026-02-25T01:00:00.000Z",
                "type": "autonomy_run",
                "result": "executed",
                "outcome": "shipped",
                "objective_id": "T1_build_sovereign_capital_v1"
            }),
        )
        .expect("append row1");
        append_jsonl(
            &runs_dir.join(format!("{day}.jsonl")),
            &json!({
                "ts": "2026-02-25T01:05:00.000Z",
                "type": "autonomy_run",
                "result": "executed",
                "outcome": "no_change",
                "objective_id": "UNKNOWN_OBJECTIVE_SHOULD_BLOCK"
            }),
        )
        .expect("append row2");
        write_json_atomic(&proposals_dir.join(format!("{day}.json")), &json!([]))
            .expect("proposal");

        std::env::set_var("AUTONOMY_SIM_RUNS_DIR", &runs_dir);
        std::env::set_var("AUTONOMY_SIM_PROPOSALS_DIR", &proposals_dir);
        std::env::set_var("AUTONOMY_SIM_LINEAGE_REQUIRED", "0");
        std::env::set_var("AUTONOMY_SIM_IDENTITY_PROJECTION_ENABLED", "1");
        std::env::set_var("AUTONOMY_SIM_IDENTITY_BLOCK_UNKNOWN_OBJECTIVE", "1");

        let out = run_autonomy_simulation(root, Some(day), 1, false);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("identity_projection")
                .and_then(Value::as_object)
                .and_then(|m| m.get("enabled"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.get("effective_counters")
                .and_then(Value::as_object)
                .and_then(|m| m.get("attempts"))
                .and_then(Value::as_i64),
            Some(1)
        );

        std::env::remove_var("AUTONOMY_SIM_RUNS_DIR");
        std::env::remove_var("AUTONOMY_SIM_PROPOSALS_DIR");
        std::env::remove_var("AUTONOMY_SIM_LINEAGE_REQUIRED");
        std::env::remove_var("AUTONOMY_SIM_IDENTITY_PROJECTION_ENABLED");
        std::env::remove_var("AUTONOMY_SIM_IDENTITY_BLOCK_UNKNOWN_OBJECTIVE");
    }
}

