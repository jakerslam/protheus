fn execute_native(root: &Path, cli: &CliArgs) -> i32 {
    if std::env::var("CLEARANCE")
        .ok()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        std::env::set_var("CLEARANCE", "3");
    }
    let policy = load_mech_suit_policy(root);
    if cli.command == "status" {
        return emit_status(root, cli, &policy);
    }

    let run_context = std::env::var("SPINE_RUN_CONTEXT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "manual".to_string());
    if policy.enabled && !policy.manual_triggers_allowed && run_context != "heartbeat" {
        let receipt = ambient_gate_blocked_receipt(cli, &policy, &run_context);
        enqueue_spine_attention(
            root,
            "spine_ambient_gate",
            "critical",
            "manual_trigger_blocked_mech_suit_mode",
        );
        update_mech_suit_status(
            root,
            &policy,
            "spine",
            json!({
                "ambient": policy.enabled,
                "heartbeat_hours": policy.heartbeat_hours,
                "manual_triggers_allowed": policy.manual_triggers_allowed,
                "quiet_non_critical": policy.quiet_non_critical,
                "silent_subprocess_output": policy.silent_subprocess_output,
                "attention_emission_owner": "eyes",
                "attention_escalation_authority": "runtime_policy",
                "last_result": "manual_trigger_blocked",
                "last_mode": cli.mode,
                "last_date": cli.date,
                "last_run_context": run_context
            }),
        );
        print_json_line(&receipt);
        return 2;
    }

    let run_started_ms = chrono::Utc::now().timestamp_millis();
    let run_id = format!(
        "spine_{}_{}",
        to_base36(run_started_ms as u64),
        std::process::id()
    );

    let mut ledger = LedgerWriter::new(root, &cli.date, &run_id);
    let invoked = vec![
        "core/layer0/ops/src/spine.rs",
        "core/layer0/ops/src/security_plane.rs",
        "core/layer0/ops/src/autonomy_controller.rs",
        "core/layer0/ops/src/sensory_eyes_intake.rs",
        "client/runtime/systems/ops/run_protheus_ops.js",
    ];

    let (constitution_ok, constitution_hash, expected_hash) = constitution_hash(root);
    let mut evidence_ok = 0i64;
    let mut evidence_plan = default_evidence_plan();

    ledger.append(json!({
        "type": "spine_run_started",
        "mode": cli.mode,
        "date": cli.date,
        "max_eyes": cli.max_eyes,
        "files_touched": invoked,
        "constitution_hash": constitution_hash,
        "expected_constitution_hash": expected_hash,
        "constitution_integrity_ok": constitution_ok
    }));

    if !constitution_ok {
        return emit_terminal_with_closeout(
            root,
            &mut ledger,
            &TerminalReceiptContext {
                run_id: &run_id,
                cli,
                policy: &policy,
                constitution_hash: &constitution_hash,
                constitution_ok,
                evidence_plan: &evidence_plan,
                evidence_ok,
                started_ms: run_started_ms,
            },
            false,
            Some("constitution_integrity_failed"),
        );
    }

    let guard_res = run_guard(root, &invoked);
    ledger.append(json!({
        "type": "spine_guard",
        "mode": cli.mode,
        "date": cli.date,
        "ok": guard_res.ok,
        "code": guard_res.code,
        "reason": if guard_res.ok { Value::Null } else { Value::String(clean_reason(&guard_res.stderr, &guard_res.stdout)) }
    }));
    if !guard_res.ok {
        return emit_terminal_with_closeout(
            root,
            &mut ledger,
            &TerminalReceiptContext {
                run_id: &run_id,
                cli,
                policy: &policy,
                constitution_hash: &constitution_hash,
                constitution_ok,
                evidence_plan: &evidence_plan,
                evidence_ok,
                started_ms: run_started_ms,
            },
            false,
            Some("guard_failed"),
        );
    }

    if let Err(reason) = step_ops_domain(
        root,
        "sensory_eyes_intake_run",
        "sensory-eyes-intake",
        vec!["run".to_string()],
        Some(&run_context),
        &mut ledger,
        &cli.mode,
        &cli.date,
    ) {
        return emit_terminal_with_closeout(
            root,
            &mut ledger,
            &TerminalReceiptContext {
                run_id: &run_id,
                cli,
                policy: &policy,
                constitution_hash: &constitution_hash,
                constitution_ok,
                evidence_plan: &evidence_plan,
                evidence_ok,
                started_ms: run_started_ms,
            },
            false,
            Some(&reason),
        );
    }

    if let Err(reason) = step_ops_domain(
        root,
        "autonomy_status",
        "autonomy-controller",
        vec!["status".to_string(), format!("--date={}", cli.date)],
        Some(&run_context),
        &mut ledger,
        &cli.mode,
        &cli.date,
    ) {
        return emit_terminal_with_closeout(
            root,
            &mut ledger,
            &TerminalReceiptContext {
                run_id: &run_id,
                cli,
                policy: &policy,
                constitution_hash: &constitution_hash,
                constitution_ok,
                evidence_plan: &evidence_plan,
                evidence_ok,
                started_ms: run_started_ms,
            },
            false,
            Some(&reason),
        );
    }

    if cli.mode == "daily" {
        let configured = std::env::var("AUTONOMY_EVIDENCE_RUNS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok());
        let budget_pressure = std::env::var("SPINE_BUDGET_PRESSURE").ok();
        let projected_pressure = std::env::var("SPINE_PROJECTED_BUDGET_PRESSURE").ok();
        let plan = compute_evidence_run_plan(
            configured,
            budget_pressure.as_deref(),
            projected_pressure.as_deref(),
        );

        let runs = plan
            .get("evidence_runs")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0);

        let type_cap = std::env::var("SPINE_AUTONOMY_EVIDENCE_MAX_PER_TYPE")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(1)
            .clamp(0, 6);
        let mut per_type = HashMap::<String, i64>::new();

        for idx in 0..runs {
            let res = run_ops_domain_json(
                root,
                "autonomy-controller",
                &[
                    "run".to_string(),
                    format!("--date={}", cli.date),
                    "--strict=0".to_string(),
                ],
                Some(&run_context),
            );
            let proposal_type = "autonomy_run".to_string();
            let current = per_type.get(&proposal_type).copied().unwrap_or(0);
            let over_cap = type_cap > 0 && current >= type_cap;
            if over_cap {
                ledger.append(json!({
                    "type": "spine_autonomy_evidence_skipped_type_cap",
                    "mode": cli.mode,
                    "date": cli.date,
                    "attempt": idx + 1,
                    "proposal_type": proposal_type,
                    "type_cap": type_cap
                }));
                continue;
            }

            if !proposal_type.is_empty() {
                per_type.insert(proposal_type.clone(), current + 1);
            }

            if res.ok {
                evidence_ok += 1;
            }
            ledger.append(json!({
                "type": "spine_autonomy_evidence",
                "mode": cli.mode,
                "date": cli.date,
                "attempt": idx + 1,
                "ok": res.ok,
                "proposal_type": Value::String(proposal_type),
                "preview_receipt_id": res.payload.as_ref().and_then(|p| p.get("preview_receipt_id")).cloned().unwrap_or(Value::Null),
                "reason": if res.ok { Value::Null } else { Value::String(clean_reason(&res.stderr, &res.stdout)) }
            }));
        }

        evidence_plan = plan;
    }

    if cli.mode == "daily" {
        if let Err(reason) = step_ops_domain(
            root,
            "dopamine_closeout",
            "dopamine-ambient",
            vec![
                "closeout".to_string(),
                format!("--date={}", cli.date),
                "--run-context=spine".to_string(),
            ],
            Some(&run_context),
            &mut ledger,
            &cli.mode,
            &cli.date,
        ) {
            ledger.append(json!({
                "type": "spine_step_non_blocking",
                "mode": cli.mode,
                "date": cli.date,
                "step": "dopamine_closeout",
                "ok": false,
                "non_blocking": true,
                "reason": reason
            }));
        }
    }

    emit_terminal_with_closeout(
        root,
        &mut ledger,
        &TerminalReceiptContext {
            run_id: &run_id,
            cli,
            policy: &policy,
            constitution_hash: &constitution_hash,
            constitution_ok,
            evidence_plan: &evidence_plan,
            evidence_ok,
            started_ms: run_started_ms,
        },
        true,
        None,
    )
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if let Some(first) = argv.first() {
        let command = first.trim().to_ascii_lowercase();
        if command == "sleep-cleanup" || command == "sleep_cleanup" {
            return run_sleep_cleanup_command(root, &argv[1..]);
        }
        if command == "cleanup" || command == "cleanup-purge" || command == "purge" {
            let mut routed = Vec::<String>::with_capacity(argv.len() + 1);
            routed.push("purge".to_string());
            routed.extend(argv.iter().skip(1).cloned());
            return run_sleep_cleanup_command(root, &routed);
        }
        if command == "background-hands-scheduler" || command == "background_hands_scheduler" {
            let (code, payload) = run_background_hands_scheduler(root, &argv[1..]);
            print_json_line(&payload);
            return code;
        }
        if command == "rsi-idle-hands-scheduler" || command == "rsi_idle_hands_scheduler" {
            let (code, payload) = run_rsi_idle_hands_scheduler(root, &argv[1..]);
            print_json_line(&payload);
            return code;
        }
        if command == "evidence-run-plan" || command == "evidence_run_plan" {
            let (code, payload) = run_evidence_run_plan(&argv[1..]);
            print_json_line(&payload);
            return code;
        }
    }

    let Some(cli) = parse_cli(argv) else {
        usage();
        print_json_line(&cli_error_receipt(argv, "invalid_args", 2));
        return 2;
    };

    execute_native(root, &cli)
}

