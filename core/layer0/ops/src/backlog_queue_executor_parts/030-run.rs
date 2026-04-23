
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  infring-ops backlog-queue-executor run [--all=1] [--ids=A,B] [--max=N] [--dry-run=1] [--with-tests=1]");
        println!("  infring-ops backlog-queue-executor status");
        return 0;
    }

    let latest = latest_path(root);
    let history = history_path(root);

    if command == "status" {
        let mut out = json!({
            "ok": true,
            "type": "backlog_queue_executor_status",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "latest": lane_utils::read_json(&latest)
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        print_receipt(&out);
        return 0;
    }

    let dry_run = lane_utils::parse_bool(
        parsed
            .flags
            .get("dry-run")
            .or_else(|| parsed.flags.get("dry_run"))
            .map(String::as_str),
        false,
    );
    let with_tests = lane_utils::parse_bool(
        parsed
            .flags
            .get("with-tests")
            .or_else(|| parsed.flags.get("with_tests"))
            .map(String::as_str),
        false,
    );
    let max =
        lane_utils::parse_i64_clamped(parsed.flags.get("max").map(String::as_str), 50, 1, 2000);
    let ids = clean(parsed.flags.get("ids").cloned().unwrap_or_default(), 4000);
    let all = lane_utils::parse_bool(parsed.flags.get("all").map(String::as_str), false);
    let allow_dynamic_legacy = lane_utils::parse_opt_bool(
        parsed
            .flags
            .get("allow-dynamic-legacy")
            .or_else(|| parsed.flags.get("allow_dynamic_legacy"))
            .map(String::as_str),
    )
    .unwrap_or(false);

    let srs_rows = parse_srs_rows(&root.join("docs/workspace/SRS.md"));
    let actionable_status = ["queued", "in_progress"];
    let mut candidates: Vec<String> = srs_rows
        .into_iter()
        .filter(|(_, status)| actionable_status.contains(&status.as_str()))
        .map(|(id, _)| id)
        .collect();

    let requested_ids = parse_ids_csv(&ids);
    if !requested_ids.is_empty() {
        let requested: std::collections::HashSet<String> = requested_ids.into_iter().collect();
        candidates.retain(|id| requested.contains(id));
    }

    if !all && (candidates.len() as i64) > max {
        candidates.truncate(max as usize);
    }
    let mut dedup = std::collections::HashSet::new();
    candidates.retain(|id| dedup.insert(id.clone()));

    let scripts = load_npm_scripts(root);
    let lane_registry = load_lane_registry(root);
    let mut executed = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let mut blocked_dynamic_stub = 0usize;
    let mut rows = Vec::new();

    for id in candidates.iter() {
        let lane_script = INDEXED_LANE_SCRIPT.to_string();
        let test_script = INDEXED_TEST_LANE_SCRIPT.to_string();
        let lane_cmd = scripts
            .get(&lane_script)
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let lane_exists = !lane_cmd.is_empty() && lane_registry_command(&lane_registry, "run", id).is_some();
        let mut test_exists =
            scripts.get(&test_script).and_then(|v| v.as_str()).is_some()
                && lane_registry_command(&lane_registry, "test", id).is_some();

        let use_core_contract_lane = runtime_contract_profile_for(id).is_some();
        let use_dynamic_lane = !lane_exists && !use_core_contract_lane;
        if use_dynamic_lane
            && !root
                .join("client")
                .join("runtime")
                .join("systems")
                .join("compat")
                .join("legacy_alias_adapter.ts")
                .exists()
        {
            skipped += 1;
            rows.push(json!({
                "id": id,
                "lane_script": lane_script,
                "status": "skipped",
                "reason": "lane_script_missing"
            }));
            continue;
        }
        if use_dynamic_lane && !allow_dynamic_legacy {
            failed += 1;
            blocked_dynamic_stub += 1;
            rows.push(json!({
                "id": id,
                "lane_script": format!("dynamic:legacy_alias_adapter:{id}"),
                "status": "failed",
                "reason": "dynamic_stub_route_disallowed",
                "lane_route": "dynamic_legacy_adapter",
                "unblock": "add concrete lane script + tests or core contract runtime with non-scaffold behavior"
            }));
            continue;
        }

        if !use_dynamic_lane && !use_core_contract_lane {
            let mut lane_seen = std::collections::HashSet::new();
            if let Some(missing_entry) = detect_missing_entrypoint_for_script(
                root,
                &scripts,
                &lane_script,
                0,
                &mut lane_seen,
            ) {
                skipped += 1;
                rows.push(json!({
                    "id": id,
                    "lane_script": lane_script,
                    "status": "skipped",
                    "reason": "lane_entrypoint_missing",
                    "missing_entrypoint": missing_entry
                }));
                continue;
            }
        }
        let mut test_skip_reason = Value::Null;
        if test_exists && !use_dynamic_lane && !use_core_contract_lane {
            let mut test_seen = std::collections::HashSet::new();
            if let Some(missing_test_entry) = detect_missing_entrypoint_for_script(
                root,
                &scripts,
                &test_script,
                0,
                &mut test_seen,
            ) {
                test_exists = false;
                test_skip_reason = json!({
                    "reason": "test_entrypoint_missing",
                    "missing_entrypoint": missing_test_entry
                });
            }
        }

        if dry_run {
            skipped += 1;
            let (lane_script_row, test_script_row, route) = row_scripts_and_route(
                id,
                &lane_script,
                &test_script,
                test_exists,
                use_core_contract_lane,
                use_dynamic_lane,
            );
            rows.push(json!({
                "id": id,
                "lane_script": lane_script_row,
                "test_script": test_script_row,
                "status": "planned",
                "lane_route": route,
                "test_skip_reason": test_skip_reason
            }));
            continue;
        }

        let lane_result = if use_core_contract_lane {
            run_core_runtime_system_lane(root, id)
        } else if use_dynamic_lane {
            run_dynamic_legacy_lane(root, id)
        } else {
            run_npm_script(root, &lane_script, &[format!("--id={id}")])
        };
        let mut test_result = Value::Null;
        let lane_ok = lane_result
            .get("ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mut test_ok = true;
        if with_tests && test_exists && lane_ok && !use_dynamic_lane && !use_core_contract_lane {
            test_result = run_npm_script(root, &test_script, &[format!("--id={id}")]);
            test_ok = test_result
                .get("ok")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        }

        if lane_ok && test_ok {
            executed += 1;
            let (lane_script_row, test_script_row, route) = row_scripts_and_route(
                id,
                &lane_script,
                &test_script,
                test_exists,
                use_core_contract_lane,
                use_dynamic_lane,
            );
            rows.push(json!({
                "id": id,
                "lane_script": lane_script_row,
                "test_script": test_script_row,
                "status": "executed",
                "lane_route": route,
                "test_skip_reason": test_skip_reason,
                "lane_result": lane_result,
                "test_result": test_result
            }));
        } else {
            failed += 1;
            let (lane_script_row, test_script_row, route) = row_scripts_and_route(
                id,
                &lane_script,
                &test_script,
                test_exists,
                use_core_contract_lane,
                use_dynamic_lane,
            );
            rows.push(json!({
                "id": id,
                "lane_script": lane_script_row,
                "test_script": test_script_row,
                "status": "failed",
                "lane_route": route,
                "test_skip_reason": test_skip_reason,
                "lane_result": lane_result,
                "test_result": test_result
            }));
        }
    }

    let mut out = json!({
        "ok": failed == 0,
        "type": "backlog_queue_executor",
        "lane": "core/layer0/ops",
        "ts": now_iso(),
        "command": command,
        "dry_run": dry_run,
        "all": all,
        "max": max,
        "with_tests": with_tests,
        "allow_dynamic_legacy": allow_dynamic_legacy,
        "ids": ids,
        "counts": {
            "scanned": candidates.len(),
            "executed": executed,
            "skipped": skipped,
            "failed": failed,
            "blocked_dynamic_stub": blocked_dynamic_stub
        },
        "rows": rows
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    let _ = lane_utils::write_json(&latest, &out);
    let _ = lane_utils::append_jsonl(&history, &out);
    print_receipt(&out);
    if failed == 0 {
        0
    } else {
        1
    }
}
