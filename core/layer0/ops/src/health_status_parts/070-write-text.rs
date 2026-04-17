
#[cfg(test)]
mod tests {
    use super::*;

    fn write_text(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdirs");
        }
        std::fs::write(path, body).expect("write");
    }

    fn seed_source_of_truth_fixture(root: &Path) {
        write_text(
            root,
            RUST_SOURCE_OF_TRUTH_POLICY_REL,
            r#"{
  "version": "1.0",
  "rust_entrypoint_gate": {
    "path": "core/layer0/ops/src/main.rs",
    "required_tokens": ["\"spine\" =>"]
  },
  "conduit_strict_gate": {
    "path": "client/runtime/systems/ops/protheusd.ts",
    "required_tokens": ["PROTHEUS_CONDUIT_STRICT"]
  },
  "conduit_budget_gate": {
    "path": "core/layer2/conduit/src/lib.rs",
    "required_tokens": ["MAX_CONDUIT_MESSAGE_TYPES: usize = 10"]
  },
  "status_dashboard_gate": {
    "path": "client/runtime/systems/ops/protheus_status_dashboard.ts",
    "required_tokens": ["status", "--dashboard"]
  },
  "js_wrapper_contract": {
    "required_wrapper_paths": ["client/runtime/systems/ops/protheusd.js"]
  },
  "rust_shim_contract": {
    "entries": [
      {
        "path": "client/runtime/systems/ops/state_kernel.js",
        "required_tokens": ["spawnSync('cargo'"]
      }
    ]
  },
  "ts_surface_allowlist_prefixes": [
    "client/runtime/systems/ops/"
  ]
}"#,
        );

        write_text(
            root,
            "core/layer0/ops/src/main.rs",
            "match x { \"spine\" => {} }",
        );
        write_text(
            root,
            "client/runtime/systems/ops/protheusd.ts",
            "const PROTHEUS_CONDUIT_STRICT = true;",
        );
        write_text(
            root,
            "core/layer2/conduit/src/lib.rs",
            "pub const MAX_CONDUIT_MESSAGE_TYPES: usize = 10;",
        );
        write_text(
            root,
            "client/runtime/systems/ops/protheus_status_dashboard.ts",
            "run status --dashboard",
        );
        write_text(
            root,
            "client/runtime/systems/ops/protheusd.js",
            "#!/usr/bin/env node\n'use strict';\nrequire('../../client/runtime/lib/ts_bootstrap').bootstrap(__filename, module);\n",
        );
        write_text(
            root,
            "client/runtime/systems/ops/state_kernel.js",
            "spawnSync('cargo', ['run']);",
        );
    }

    #[test]
    fn defaults_to_status_and_emits_deterministic_hash() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_source_of_truth_fixture(root.path());
        write_text(
            root.path(),
            CRON_JOBS_REL,
            r#"{"jobs":[{"id":"j1","name":"job","enabled":true,"sessionTarget":"isolated","delivery":{"mode":"announce","channel":"last"}}]}"#,
        );

        let payload = status_receipt(root.path(), "status", &[], false);
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        let hash = payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .expect("hash")
            .to_string();
        let mut unhashed = payload.clone();
        unhashed
            .as_object_mut()
            .expect("obj")
            .remove("receipt_hash");
        assert_eq!(receipt_hash(&unhashed), hash);
    }

    #[test]
    fn unknown_command_fails_closed() {
        let payload = cli_error_receipt(&["nope".to_string()], "unknown_command", 2);
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(payload.get("exit_code").and_then(Value::as_i64), Some(2));
    }

    #[test]
    fn accepts_legacy_date_first_arg() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_source_of_truth_fixture(root.path());
        write_text(
            root.path(),
            CRON_JOBS_REL,
            r#"{"jobs":[{"id":"j1","name":"job","enabled":true,"sessionTarget":"isolated","delivery":{"mode":"announce","channel":"last"}}]}"#,
        );

        let exit = run(
            root.path(),
            &["2026-03-05".to_string(), "--window=daily".to_string()],
        );
        assert_eq!(exit, 0);
    }

    #[test]
    fn cron_delivery_none_is_rejected() {
        let root = tempfile::tempdir().expect("tempdir");
        write_text(
            root.path(),
            CRON_JOBS_REL,
            r#"{"jobs":[{"id":"j1","name":"job","enabled":true,"sessionTarget":"isolated","delivery":{"mode":"none","channel":"last"}}]}"#,
        );

        let audit = audit_cron_delivery(root.path());
        assert_eq!(audit.get("ok").and_then(Value::as_bool), Some(false));
        let issues = audit
            .get("issues")
            .and_then(Value::as_array)
            .expect("issues");
        assert!(issues.iter().any(|row| {
            row.get("reason")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("delivery_mode_none_forbidden")
        }));
    }

    #[test]
    fn cron_missing_delivery_is_rejected_for_enabled_jobs() {
        let root = tempfile::tempdir().expect("tempdir");
        write_text(
            root.path(),
            CRON_JOBS_REL,
            r#"{"jobs":[{"id":"j1","name":"job","enabled":true,"sessionTarget":"main"}]}"#,
        );

        let audit = audit_cron_delivery(root.path());
        assert_eq!(audit.get("ok").and_then(Value::as_bool), Some(false));
        let issues = audit
            .get("issues")
            .and_then(Value::as_array)
            .expect("issues");
        assert!(issues.iter().any(|row| {
            row.get("reason")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("missing_delivery_for_enabled_job")
        }));
    }

    #[test]
    fn percentile_helpers_cover_p99_path() {
        let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(percentile_95(&values), Some(50.0));
        assert_eq!(percentile_99(&values), Some(50.0));
        assert_eq!(percentile(&[], 0.50), None);
    }

    #[test]
    fn escalation_dashboard_metric_tracks_open_rate() {
        let root = tempfile::tempdir().expect("tempdir");
        write_text(
            root.path(),
            "client/runtime/local/state/security/autonomy_human_escalations.jsonl",
            r#"{"type":"autonomy_human_escalation","escalation_id":"e1","status":"open"}
{"type":"autonomy_human_escalation","escalation_id":"e2","status":"resolved"}
"#,
        );
        let metric = collect_human_escalation_dashboard_metric(root.path());
        let payload = metric
            .get("human_escalation_open_rate")
            .expect("metric payload");
        assert_eq!(payload.get("open_count").and_then(Value::as_u64), Some(1));
        assert_eq!(
            payload.get("resolved_count").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(payload.get("status").and_then(Value::as_str), Some("warn"));
    }

    #[test]
    fn token_burn_cost_metric_summarizes_budget_events() {
        let root = tempfile::tempdir().expect("tempdir");
        write_text(
            root.path(),
            "client/runtime/local/state/autonomy/budget_events.jsonl",
            r#"{"type":"system_budget_record","date":"2026-03-06","module":"sensory_focus","tokens_est":120}
{"type":"system_budget_record","date":"2026-03-06","module":"sensory_focus","tokens_est":80}
{"type":"system_budget_record","date":"2026-03-06","module":"reflex","tokens_est":50}
{"type":"system_budget_decision","decision":"deny","module":"sensory_focus"}
"#,
        );
        let metric = collect_token_burn_cost_dashboard_metric(root.path());
        let payload = metric
            .get("token_burn_cost_attribution")
            .expect("metric payload");
        assert_eq!(
            payload.get("latest_day_tokens").and_then(Value::as_i64),
            Some(250)
        );
        assert_eq!(
            payload.get("deny_decisions").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(payload.get("status").and_then(Value::as_str), Some("pass"));
    }

    #[test]
    fn spine_dashboard_metric_marks_stale_history() {
        let root = tempfile::tempdir().expect("tempdir");
        write_text(
            root.path(),
            "client/runtime/local/state/spine/runs/2024-01-01.jsonl",
            r#"{"type":"spine_run_complete","elapsed_ms":120,"ts":"2024-01-01T00:00:00Z"}"#,
        );

        let metric = collect_spine_dashboard_metrics(root.path());
        let success = metric
            .get("spine_success_rate")
            .expect("spine success metric");
        let p95 = metric
            .get("receipt_latency_p95_ms")
            .expect("spine latency metric");
        assert_eq!(success.get("status").and_then(Value::as_str), Some("stale"));
        assert_eq!(success.get("stale").and_then(Value::as_bool), Some(true));
        assert_eq!(p95.get("status").and_then(Value::as_str), Some("stale"));
        assert_eq!(p95.get("stale").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn spine_dashboard_metric_uses_fresh_rows() {
        let root = tempfile::tempdir().expect("tempdir");
        let fresh_ts = Utc::now().to_rfc3339();
        write_text(
            root.path(),
            "client/runtime/local/state/spine/runs/2099-01-01.jsonl",
            &format!(
                "{{\"type\":\"spine_run_complete\",\"elapsed_ms\":42,\"ts\":\"{}\"}}",
                fresh_ts
            ),
        );

        let metric = collect_spine_dashboard_metrics(root.path());
        let success = metric
            .get("spine_success_rate")
            .expect("spine success metric");
        let p95 = metric
            .get("receipt_latency_p95_ms")
            .expect("spine latency metric");
        assert_eq!(success.get("status").and_then(Value::as_str), Some("pass"));
        assert_eq!(success.get("stale").and_then(Value::as_bool), Some(false));
        assert_eq!(p95.get("status").and_then(Value::as_str), Some("pass"));
        assert_eq!(p95.get("stale").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn moltbook_credentials_metric_warns_when_jobs_enabled_without_credentials() {
        let root = tempfile::tempdir().expect("tempdir");
        write_text(
            root.path(),
            CRON_JOBS_REL,
            r#"{"jobs":[{"id":"molt","name":"Moltbook Times","enabled":true,"payload":{"kind":"systemEvent","text":"MOLTCHECK heartbeat"}}]}"#,
        );
        write_text(
            root.path(),
            "client/runtime/config/secret_broker_policy.json",
            r#"{
              "version": "1.0",
              "secrets": {
                "moltbook_api_key": {
                  "providers": [
                    { "type": "env", "env": "INTENTIONALLY_UNSET_MOLTBOOK_TEST_TOKEN" }
                  ]
                }
              }
            }"#,
        );

        let metric = collect_moltbook_credentials_dashboard_metric(root.path());
        let payload = metric
            .get("moltbook_credentials_surface")
            .expect("metric payload");
        assert_eq!(payload.get("status").and_then(Value::as_str), Some("warn"));
        assert_eq!(
            payload
                .get("suppression_recommended")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn moltbook_credentials_metric_passes_with_json_secret_provider() {
        let root = tempfile::tempdir().expect("tempdir");
        write_text(
            root.path(),
            CRON_JOBS_REL,
            r#"{"jobs":[{"id":"molt","name":"Moltbook Times","enabled":true,"payload":{"kind":"systemEvent","text":"MOLTCHECK heartbeat"}}]}"#,
        );
        write_text(
            root.path(),
            "client/runtime/config/secret_broker_policy.json",
            r#"{
              "version": "1.0",
              "secrets": {
                "moltbook_api_key": {
                  "providers": [
                    { "type": "json_file", "paths": ["secrets/moltbook.credentials.json"], "field": "api_key" }
                  ]
                }
              }
            }"#,
        );
        write_text(
            root.path(),
            "secrets/moltbook.credentials.json",
            r#"{"api_key":"moltbook_sk_test_123"}"#,
        );

        let metric = collect_moltbook_credentials_dashboard_metric(root.path());
        let payload = metric
            .get("moltbook_credentials_surface")
            .expect("metric payload");
        assert_eq!(payload.get("status").and_then(Value::as_str), Some("pass"));
        assert_eq!(
            payload
                .get("credentials_available")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn dopamine_metric_warns_when_ambient_data_is_stale() {
        let root = tempfile::tempdir().expect("tempdir");
        write_text(
            root.path(),
            "client/runtime/local/state/dopamine/ambient/latest.json",
            r#"{
              "ts": "2024-01-01T00:00:00Z",
              "severity": "info",
              "threshold_breached": false,
              "sds": 4.2
            }"#,
        );

        let metric = collect_dopamine_ambient_dashboard_metric(root.path());
        let payload = metric.get("dopamine_ambient").expect("metric payload");
        assert_eq!(payload.get("status").and_then(Value::as_str), Some("warn"));
        assert_eq!(
            payload.get("freshness_status").and_then(Value::as_str),
            Some("stale")
        );
    }

    #[test]
    fn external_eyes_metric_warns_when_cross_signals_absent() {
        let root = tempfile::tempdir().expect("tempdir");
        write_text(
            root.path(),
            "client/runtime/local/state/attention/latest.json",
            &format!(
                r#"{{"ts":"{}","queued_total":10}}"#,
                Utc::now().to_rfc3339()
            ),
        );
        let mut queue_rows = String::new();
        for idx in 0..EXTERNAL_EYES_CROSS_SIGNAL_MIN_EVENTS {
            queue_rows.push_str(&format!(
                "{{\"ts\":\"{}\",\"source\":\"external_eyes\",\"source_type\":\"external_item\",\"summary\":\"item {idx}\"}}\n",
                Utc::now().to_rfc3339()
            ));
        }
        write_text(
            root.path(),
            "client/runtime/local/state/attention/queue.jsonl",
            &queue_rows,
        );

        let metric = collect_external_eyes_dashboard_metric(root.path());
        let payload = metric
            .get("external_eyes_cross_signal_surface")
            .expect("metric payload");
        assert_eq!(payload.get("status").and_then(Value::as_str), Some("warn"));
        assert_eq!(
            payload.get("cross_signal_absent").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn cron_audit_exposes_runtime_web_tooling_contract_fields() {
        let root = tempfile::tempdir().expect("tempdir");
        write_text(
            root.path(),
            CRON_JOBS_REL,
            r#"{
              "jobs":[
                {
                  "id":"w1",
                  "name":"web search pulse",
                  "enabled":true,
                  "sessionTarget":"isolated",
                  "command":"web-conduit search --query status",
                  "delivery":{"mode":"announce","channel":"last"}
                }
              ]
            }"#,
        );

        let audit = audit_cron_delivery(root.path());
        assert_eq!(
            audit.get("web_tooling_jobs").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(audit.get("web_tooling_auth_present").is_some(), true);
        assert_eq!(
            audit.get("web_tooling_auth_sources")
                .and_then(Value::as_array)
                .is_some(),
            true
        );
    }

    #[test]
    fn dashboard_status_includes_web_tooling_readiness_metric() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_source_of_truth_fixture(root.path());
        write_text(
            root.path(),
            CRON_JOBS_REL,
            r#"{
              "jobs":[
                {
                  "id":"w2",
                  "name":"web fetch watcher",
                  "enabled":true,
                  "sessionTarget":"isolated",
                  "payload":{"text":"web fetch pulse"},
                  "delivery":{"mode":"announce","channel":"last"}
                }
              ]
            }"#,
        );

        let payload = status_receipt(root.path(), "dashboard", &["dashboard".to_string()], true);
        assert_eq!(
            payload
                .pointer("/dashboard_metrics/web_tooling_auth_readiness/source")
                .and_then(Value::as_str),
            Some("cron_delivery_integrity")
        );
        assert_eq!(
            payload
                .pointer("/dashboard_metrics/web_tooling_auth_readiness/status")
                .and_then(Value::as_str)
                .is_some(),
            true
        );
    }
}

