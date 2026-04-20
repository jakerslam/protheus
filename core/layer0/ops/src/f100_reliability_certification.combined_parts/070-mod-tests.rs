
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    fn write_json(path: &Path, value: &Value) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(
            path,
            format!("{}\n", serde_json::to_string_pretty(value).unwrap()),
        )
        .expect("write json");
    }

    fn write_jsonl(path: &Path, rows: &[Value]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        let mut buf = String::new();
        for row in rows {
            buf.push_str(&serde_json::to_string(row).unwrap());
            buf.push('\n');
        }
        fs::write(path, buf).expect("write jsonl");
    }

    fn write_policy(root: &Path, strict_default: bool) {
        let policy = json!({
            "strict_default": strict_default,
            "active_tier": "seed",
            "window_days": 30,
            "missing_metric_fail_closed": false,
            "tiers": {
                "seed": {
                    "min_uptime": 0.90,
                    "max_receipt_p95_ms": 200.0,
                    "max_receipt_p99_ms": 300.0,
                    "max_incident_rate": 0.35,
                    "max_change_fail_rate": 0.50,
                    "max_error_budget_burn_ratio": 0.45
                }
            },
            "sources": {
                "execution_reliability_path": "local/state/ops/execution_reliability_slo.json",
                "error_budget_latest_path": "local/state/ops/error_budget_release_gate/latest.json",
                "error_budget_history_path": "local/state/ops/error_budget_release_gate/history.jsonl",
                "spine_runs_dir": "local/state/spine/runs",
                "incident_log_path": "local/state/security/autonomy_human_escalations.jsonl",
                "drill_evidence_paths": [
                    "local/state/ops/dr_gameday_gate_receipts.jsonl"
                ],
                "rollback_evidence_paths": [
                    "local/state/ops/error_budget_release_gate/freeze_state.json"
                ],
                "min_drill_evidence_count": 1,
                "min_rollback_evidence_count": 1
            },
            "outputs": {
                "latest_path": "local/state/ops/f100_reliability_certification/latest.json",
                "history_path": "local/state/ops/f100_reliability_certification/history.jsonl"
            }
        });
        write_json(
            &root.join("client/runtime/config/f100_reliability_certification_policy.json"),
            &policy,
        );
    }

    fn write_common_fixtures(root: &Path, burn_ratio: f64) {
        write_json(
            &root.join("local/state/ops/execution_reliability_slo.json"),
            &json!({
                "measured": {
                    "execution_success_rate": 0.97
                }
            }),
        );
        write_json(
            &root.join("local/state/ops/error_budget_release_gate/latest.json"),
            &json!({
                "ok": burn_ratio <= 0.45,
                "gate": {
                    "burn_ratio": burn_ratio,
                    "promotion_blocked": burn_ratio > 0.45
                }
            }),
        );
        write_jsonl(
            &root.join("local/state/ops/error_budget_release_gate/history.jsonl"),
            &[
                json!({"ts": "2026-03-01T10:00:00Z", "ok": true, "gate": {"promotion_blocked": false}}),
                json!({"ts": "2026-03-02T10:00:00Z", "ok": true, "gate": {"promotion_blocked": false}}),
            ],
        );
        write_jsonl(
            &root.join("local/state/security/autonomy_human_escalations.jsonl"),
            &[
                json!({"type":"autonomy_human_escalation", "ts":"2026-03-02T12:00:00Z", "status":"resolved"}),
            ],
        );
        write_jsonl(
            &root.join("local/state/spine/runs/2026-03-02.jsonl"),
            &[
                json!({"type":"spine_run_complete", "elapsed_ms": 85.0}),
                json!({"type":"spine_run_complete", "elapsed_ms": 95.0}),
                json!({"type":"spine_observability_trace", "trace_duration_ms": 100.0}),
            ],
        );
        write_jsonl(
            &root.join("local/state/ops/dr_gameday_gate_receipts.jsonl"),
            &[json!({"ok": true, "type": "drill"})],
        );
        write_json(
            &root.join("local/state/ops/error_budget_release_gate/freeze_state.json"),
            &json!({"frozen": false}),
        );
    }

    #[test]
    fn strict_run_blocks_when_error_budget_exceeds_threshold() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        write_policy(root, true);
        write_common_fixtures(root, 0.91);

        let policy = load_policy(root, None);
        let (_payload, code) = run_cmd(&policy, true).expect("run cmd");
        assert_eq!(code, 1);

        let latest =
            read_json(&root.join("local/state/ops/f100_reliability_certification/latest.json"))
                .expect("latest should exist");
        assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(false));
        assert!(latest
            .get("blocking_checks")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|v| v.as_str() == Some("error_budget_burn_ratio")))
            .unwrap_or(false));
    }

    #[test]
    fn strict_run_passes_under_seed_thresholds_with_evidence() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        write_policy(root, true);
        write_common_fixtures(root, 0.20);

        let policy = load_policy(root, None);
        let (_payload, code) = run_cmd(&policy, true).expect("run cmd");
        assert_eq!(code, 0);

        let latest =
            read_json(&root.join("local/state/ops/f100_reliability_certification/latest.json"))
                .expect("latest should exist");
        assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            latest
                .get("release_gate")
                .and_then(|v| v.get("promotion_blocked"))
                .and_then(Value::as_bool),
            Some(false)
        );
    }
}

