#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_nexus_message() {
        let parsed = parse_nexus_message("[AG1>COORD|swarm] ROUT Q=H BP=M").expect("parse");
        assert_eq!(parsed.from, "AG1");
        assert_eq!(parsed.to, "COORD");
        assert_eq!(parsed.module.as_deref(), Some("swarm"));
        assert_eq!(parsed.cmd, "ROUT");
        assert_eq!(parsed.kv.get("Q").map(String::as_str), Some("H"));
    }

    #[test]
    fn rejects_invalid_message_without_header() {
        let err = parse_nexus_message("AG1>COORD ROUT Q=H").expect_err("must fail");
        assert!(err.contains("missing_header_open"));
    }

    #[test]
    fn rejects_multiline_messages_for_strict_one_line_format() {
        let err = parse_nexus_message("[AG1>COORD|swarm] ROUT Q=H\nBP=M").expect_err("must fail");
        assert!(err.contains("multiline_not_allowed"));
    }

    #[test]
    fn enforces_module_limit() {
        let argv = vec!["--modules=memory,swarm,conduit,security".to_string()];
        let err = parse_modules(&argv).expect_err("must fail");
        assert!(err.contains("module_limit_exceeded"));
    }

    #[test]
    fn compress_round_trip_keeps_strict_format() {
        let modules = vec!["swarm".to_string()];
        let lexicon = active_lexicon(&modules).expect("lexicon");
        let reverse = reverse_lexicon(&lexicon);
        let (msg, fallback) = compress_text_to_message(
            "ag1",
            "coord",
            Some("swarm".to_string()),
            "ROUT",
            "backpressure queue_depth",
            &reverse,
        );
        assert!(!fallback);
        let line = format_nexus_message(&msg);
        let reparsed = parse_nexus_message(&line).expect("reparse");
        let decompressed = decompress_message(&reparsed, &lexicon);
        assert_eq!(
            decompressed
                .get("cmd")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "route"
        );
    }

    #[test]
    fn send_persists_receipt_and_burn_metrics() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let argv = vec![
            "send".to_string(),
            "--message=[AG1>COORD|swarm] ROUT Q=42 LAT=120".to_string(),
            "--modules=swarm".to_string(),
            "--raw-text=backpressure queue_depth".to_string(),
        ];
        let (payload, code) = send_command(root, &argv);
        assert_eq!(code, 0);
        let perf_after = payload
            .pointer("/perf_proof/after")
            .cloned()
            .unwrap_or(Value::Null);
        assert!(!perf_after.is_null(), "expected perf_proof.after");
        assert_eq!(
            perf_after.get("queue_depth").and_then(Value::as_f64),
            Some(42.0)
        );
        assert_eq!(
            perf_after.get("p95_latency_ms").and_then(Value::as_u64),
            Some(120)
        );
        let latest = read_json(&latest_path(root)).expect("latest");
        assert!(
            latest
                .get("total_nexus_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
        );
        assert!(latest.pointer("/perf/ops_per_sec").is_some());
    }

    #[test]
    fn status_includes_hot_path_allocator_metrics() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let seed = vec![
            "send".to_string(),
            "--message=[AG1>COORD|swarm] ROUT Q=H BP=M".to_string(),
            "--modules=swarm".to_string(),
            "--raw-text=backpressure queue_depth".to_string(),
        ];
        let (_seed_payload, seed_code) = send_command(root, &seed);
        assert_eq!(seed_code, 0);
        let argv = vec!["--limit=5".to_string(), "--modules=swarm".to_string()];
        let (payload, code) = status_command(root, &argv);
        assert_eq!(code, 0);
        let allocators = payload
            .get("hot_path_allocators")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let arena = allocators
            .get("arena")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let slab = allocators
            .get("slab")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert!(arena.contains_key("allocations"));
        assert!(slab.contains_key("checkouts"));
    }

    #[test]
    fn module_catalog_manifest_has_expected_core_modules() {
        let manifest = module_catalog_manifest();
        let rows = manifest.as_array().cloned().unwrap_or_default();
        assert!(rows
            .iter()
            .any(|row| row.get("name").and_then(Value::as_str) == Some("memory")));
        assert!(rows
            .iter()
            .any(|row| { row.get("name").and_then(Value::as_str) == Some("incident_response") }));
        assert!(rows
            .iter()
            .any(|row| { row.get("name").and_then(Value::as_str) == Some("physical_security") }));
    }

    #[test]
    fn validate_auto_loads_header_module_when_not_explicitly_provided() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let argv = vec![
            "--message=[AG1>COORD|swarm] ROUT Q=H BP=M".to_string(),
            "--task=coordinate multi-agent queue balancing".to_string(),
        ];
        let (payload, code) = validate_command(root, &argv);
        assert_eq!(code, 0);
        let modules = payload
            .get("modules_loaded")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(modules.iter().any(|value| value.as_str() == Some("swarm")));
    }

    #[test]
    fn resolve_modules_infers_from_task_and_role() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let argv = vec![
            "--task=investigate security incident and preserve forensic evidence".to_string(),
            "--role=blue_team".to_string(),
        ];
        let (payload, code) = resolve_modules_command(root, &argv);
        assert_eq!(code, 0);
        let modules = payload
            .get("modules_loaded")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let loaded = modules
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert!(loaded.iter().any(|value| value == "security"));
        assert!(
            loaded.iter().any(|value| value == "incident_response")
                || loaded.iter().any(|value| value == "forensics")
        );
        assert!(loaded.len() <= MAX_MODULES_PER_AGENT);
    }

    #[test]
    fn agent_prompt_can_auto_load_modules_from_task_context() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let argv = vec![
            "--agent=ag17".to_string(),
            "--task=optimize schedule and queue latency under load".to_string(),
            "--role=coordinator".to_string(),
        ];
        let (payload, code) = agent_prompt_command(root, &argv);
        assert_eq!(code, 0);
        let modules = payload
            .get("modules_loaded")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!modules.is_empty());
        assert!(
            modules
                .iter()
                .any(|value| value.as_str() == Some("scheduler"))
                || modules.iter().any(|value| value.as_str() == Some("swarm"))
        );
    }
}
