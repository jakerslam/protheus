
fn cmd_run(root: &Path, cli: &CliArgs, policy: &Policy, paths: &RuntimePaths) -> Value {
    let run_start = Instant::now();
    let strict = to_bool(
        cli.flags.get("strict").map(String::as_str),
        policy.execution.strict,
    );
    let sleep_only = to_bool(cli.flags.get("sleep-only").map(String::as_str), false);
    let force = to_bool(cli.flags.get("force").map(String::as_str), false);
    let scope = cli
        .flags
        .get("scope")
        .map(String::as_str)
        .filter(|s| ["critical", "changed", "all"].contains(s))
        .unwrap_or(policy.execution.default_scope.as_str())
        .to_string();
    let max_tests = clamp_i64(
        cli.flags.get("max-tests").map(String::as_str),
        1,
        500,
        policy.execution.max_tests_per_run as i64,
    ) as usize;
    let run_timeout_ms = clamp_i64(
        cli.flags.get("run-timeout-ms").map(String::as_str),
        1_000,
        2 * 60 * 60 * 1_000,
        policy.execution.run_timeout_ms,
    );

    let run_deadline = Instant::now() + Duration::from_millis(run_timeout_ms as u64);
    let mut phase_ms = json!({
        "sync_ms": 0,
        "select_ms": 0,
        "execute_ms": 0,
        "total_ms": 0
    });

    let sync_started = Instant::now();
    let sync_out = sync_state(root, paths, policy);
    phase_ms["sync_ms"] = json!(sync_started.elapsed().as_millis());

    let mut status = load_status(paths);
    let external_health = summarize_external_health(paths, policy);
    let sleep_gate = in_sleep_window(policy);
    let resources = runtime_resource_within(policy);
    let spine_hot = is_spine_hot(paths, policy.runtime_guard.spine_hot_window_sec);

    let mut skip_reasons = Vec::<String>::new();
    if sleep_only && !sleep_gate {
        skip_reasons.push("outside_sleep_window".to_string());
    }
    if !resources.get("ok").and_then(Value::as_bool).unwrap_or(true) {
        skip_reasons.push("resource_guard".to_string());
    }
    if spine_hot
        .get("hot")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        skip_reasons.push("spine_hot".to_string());
    }

    if !skip_reasons.is_empty() && !force {
        let now = now_iso();
        phase_ms["total_ms"] = json!(run_start.elapsed().as_millis());
        let mut out = json!({
            "ok": true,
            "type": "autotest_run",
            "ts": now,
            "scope": scope,
            "strict": strict,
            "skipped": true,
            "skip_reasons": skip_reasons,
            "synced": sync_out,
            "external_health": external_health,
            "sleep_window_ok": sleep_gate,
            "resource_guard": resources,
            "spine_hot": spine_hot,
            "run_timeout_ms": run_timeout_ms,
            "phase_ms": phase_ms,
            "claim_evidence": [
                {
                    "id": "execution_gate",
                    "claim": "autotest_run_was_safely_skipped",
                    "evidence": {
                        "skip_reasons": skip_reasons
                    }
                }
            ],
            "persona_lenses": {
                "operator": {
                    "mode": "defensive",
                    "reason": "runtime_guard"
                }
            }
        });
        out["receipt_hash"] = Value::String(receipt_hash(&out));
        let _ = write_json_atomic(&paths.latest_path, &out);
        let _ = append_jsonl(&paths.runs_dir.join(format!("{}.jsonl", &now[..10])), &out);
        return out;
    }

    let select_started = Instant::now();
    let test_ids = test_set_for_scope(&status, &scope);
    let prioritized = prioritize_tests(&status, &test_ids);
    let selected = prioritized
        .iter()
        .take(max_tests)
        .map(|row| row.test.clone())
        .collect::<Vec<_>>();
    let selection_preview = prioritized
        .iter()
        .take(24)
        .map(|row| {
            json!({
                "id": row.id,
                "score": row.score,
                "priority": row.priority
            })
        })
        .collect::<Vec<_>>();
    let test_to_modules = reverse_module_mapping(&status);
    phase_ms["select_ms"] = json!(select_started.elapsed().as_millis());

    let execute_started = Instant::now();
    let mut results = Vec::<Value>::new();
    let mut guard_blocked = 0usize;
    let mut flaky_count = 0usize;
    let mut quarantined_count = 0usize;
    let mut executed_status = HashMap::<String, String>::new();

