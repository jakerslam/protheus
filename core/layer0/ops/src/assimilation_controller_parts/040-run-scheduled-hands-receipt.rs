
fn run_scheduled_hands_receipt(root: &Path, argv: &[String], strict: bool) -> Value {
    let op = parse_flag(argv, "op")
        .or_else(|| first_non_flag(argv, 1))
        .unwrap_or_else(|| "status".to_string())
        .to_ascii_lowercase();
    let contract = read_json(&root.join(SCHEDULED_HANDS_CONTRACT_PATH)).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "kind": "scheduled_hands_contract",
            "schedule": "*/15 * * * *",
            "max_iterations_per_run": 5,
            "usd_per_iteration": 0.25,
            "token_per_iteration": 0.5,
            "cross_reference_sources": ["memory", "research", "crm"],
            "requires_bedrock_proxy": true
        })
    });
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("scheduled_hands_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "scheduled_hands_contract"
    {
        errors.push("scheduled_hands_contract_kind_invalid".to_string());
    }
    let max_iterations = contract
        .get("max_iterations_per_run")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .max(1);
    let usd_per_iteration = contract
        .get("usd_per_iteration")
        .and_then(Value::as_f64)
        .unwrap_or(0.25)
        .max(0.0);
    let token_per_iteration = contract
        .get("token_per_iteration")
        .and_then(Value::as_f64)
        .unwrap_or(0.5)
        .max(0.0);
    let cross_reference_sources = contract
        .get("cross_reference_sources")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    if cross_reference_sources.is_empty() {
        errors.push("scheduled_hands_cross_reference_sources_required".to_string());
    }
    let schedule = contract
        .get("schedule")
        .and_then(Value::as_str)
        .unwrap_or("*/15 * * * *")
        .to_string();
    let requires_bedrock_proxy = contract
        .get("requires_bedrock_proxy")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let state_path = scheduled_hands_state_path(root);
    let history_path = scheduled_hands_history_path(root);
    let earnings_path = scheduled_hands_earnings_path(root);
    let mut state = read_json(&state_path).unwrap_or_else(|| {
        json!({
            "enabled": false,
            "schedule": schedule,
            "max_iterations_per_run": max_iterations,
            "run_count": 0,
            "last_run_hash": Value::Null,
            "cross_refs_total": 0,
            "earnings_total_usd": 0.0,
            "earnings_total_token": 0.0,
            "updated_at": Value::Null
        })
    });
    let enabled = state
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let bedrock_enabled = read_json(
        &crate::core_state_root(root)
            .join("ops")
            .join("enterprise_hardening")
            .join("bedrock_proxy")
            .join("profile.json"),
    )
    .and_then(|row| row.get("ok").and_then(Value::as_bool))
    .unwrap_or(false);

    let mut run_payload = Value::Null;
    if op == "enable" {
        if requires_bedrock_proxy && strict && !bedrock_enabled {
            errors.push("scheduled_hands_requires_bedrock_proxy".to_string());
        } else {
            state["enabled"] = Value::Bool(true);
            state["schedule"] = Value::String(schedule.clone());
            state["max_iterations_per_run"] = Value::Number(max_iterations.into());
            state["updated_at"] = Value::String(now_iso());
        }
    } else if op == "disable" {
        state["enabled"] = Value::Bool(false);
        state["updated_at"] = Value::String(now_iso());
    } else if op == "run" {
        if strict && !enabled {
            errors.push("scheduled_hands_not_enabled".to_string());
        }
        if requires_bedrock_proxy && strict && !bedrock_enabled {
            errors.push("scheduled_hands_requires_bedrock_proxy".to_string());
        }
        let requested_iterations = parse_u64_flag(parse_flag(argv, "iterations"), max_iterations);
        let iterations = requested_iterations.min(max_iterations).max(1);
        if strict && requested_iterations > max_iterations {
            errors.push("scheduled_hands_iteration_cap_exceeded".to_string());
        }
        let task = parse_flag(argv, "task").unwrap_or_else(|| "scheduled-hand-cycle".to_string());
        let cross_refs = parse_flag(argv, "cross-refs")
            .unwrap_or_else(|| "memory,research".to_string())
            .split(',')
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        let cross_refs_valid = cross_refs
            .iter()
            .all(|row| cross_reference_sources.iter().any(|allowed| allowed == row));
        if strict && !cross_refs_valid {
            errors.push("scheduled_hands_cross_reference_source_invalid".to_string());
        }

        if errors.is_empty() {
            let prev_hash = state
                .get("last_run_hash")
                .and_then(Value::as_str)
                .unwrap_or("GENESIS")
                .to_string();
            let mut step_receipts = Vec::<Value>::new();
            let mut step_prev = prev_hash.clone();
            for idx in 0..iterations {
                let seq = idx + 1;
                let step = json!({
                    "seq": seq,
                    "ts": now_iso(),
                    "task": task,
                    "cross_refs": cross_refs,
                    "previous_hash": step_prev
                });
                let step_hash = receipt_hash(&step);
                step_prev = step_hash.clone();
                step_receipts.push(json!({
                    "seq": seq,
                    "previous_hash": step.get("previous_hash").cloned().unwrap_or(Value::Null),
                    "step_hash": step_hash
                }));
            }
            let cross_ref_count = cross_refs.len() as u64 * iterations;
            let earnings_usd = usd_per_iteration * (iterations as f64);
            let earnings_token = token_per_iteration * (iterations as f64);
            let trace_id = format!(
                "trace_{}",
                &receipt_hash(&json!({"task": task, "iterations": iterations, "ts": now_iso()}))
                    [..16]
            );
            run_payload = json!({
                "task": task,
                "iterations": iterations,
                "cross_refs": cross_refs,
                "causality": {
                    "trace_id": trace_id,
                    "previous_run_hash": prev_hash,
                    "run_hash": step_prev,
                    "step_receipts": step_receipts
                },
                "earnings": {
                    "usd": earnings_usd,
                    "token": earnings_token
                }
            });

            let run_count = state
                .get("run_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .saturating_add(1);
            state["run_count"] = Value::Number(run_count.into());
            state["last_run_hash"] = run_payload
                .pointer("/causality/run_hash")
                .cloned()
                .unwrap_or(Value::Null);
            state["cross_refs_total"] = Value::Number(
                state
                    .get("cross_refs_total")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    .saturating_add(cross_ref_count)
                    .into(),
            );
            state["earnings_total_usd"] = Value::from(
                state
                    .get("earnings_total_usd")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
                    + earnings_usd,
            );
            state["earnings_total_token"] = Value::from(
                state
                    .get("earnings_total_token")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
                    + earnings_token,
            );
            state["updated_at"] = Value::String(now_iso());
            let _ = append_jsonl(
                &history_path,
                &json!({
                    "type": "scheduled_hands_run",
                    "ts": now_iso(),
                    "trace_id": run_payload.pointer("/causality/trace_id").cloned().unwrap_or(Value::Null),
                    "run_hash": run_payload.pointer("/causality/run_hash").cloned().unwrap_or(Value::Null),
                    "task": run_payload.get("task").cloned().unwrap_or(Value::Null),
                    "iterations": run_payload.get("iterations").cloned().unwrap_or(Value::Null),
                    "cross_refs": run_payload.get("cross_refs").cloned().unwrap_or(Value::Null)
                }),
            );
            let _ = append_jsonl(
                &earnings_path,
                &json!({
                    "type": "scheduled_hands_earnings",
                    "ts": now_iso(),
                    "trace_id": run_payload.pointer("/causality/trace_id").cloned().unwrap_or(Value::Null),
                    "usd": run_payload.pointer("/earnings/usd").cloned().unwrap_or(Value::Null),
                    "token": run_payload.pointer("/earnings/token").cloned().unwrap_or(Value::Null)
                }),
            );
        }
    } else if !matches!(op.as_str(), "status" | "dashboard") {
        errors.push(format!("unknown_scheduled_hands_op:{op}"));
    }

    if matches!(op.as_str(), "enable" | "disable") && errors.is_empty() {
        if let Some(parent) = state_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(
            &state_path,
            serde_json::to_string_pretty(&state).unwrap_or_else(|_| "{}".to_string()) + "\n",
        );
    } else if op == "run" && errors.is_empty() {
        if let Some(parent) = state_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(
            &state_path,
            serde_json::to_string_pretty(&state).unwrap_or_else(|_| "{}".to_string()) + "\n",
        );
    }

    let history_rows = fs::read_to_string(&history_path)
        .ok()
        .map(|body| body.lines().count())
        .unwrap_or(0usize);
    let mut claim_evidence = vec![json!({
        "id": "V7-ASSIMILATE-001.5.2",
        "claim": "scheduled_hands_runtime_executes_policy_bounded_iteration_cycles_via_conduit",
        "evidence": {"op": op, "max_iterations_per_run": max_iterations}
    })];
    if matches!(op.as_str(), "run" | "dashboard") {
        claim_evidence.push(json!({
            "id": "V7-ASSIMILATE-001.5.3",
            "claim": "scheduled_hands_runs_emit_causality_linked_step_receipts_and_earnings_metadata",
            "evidence": {
                "run_hash": run_payload.pointer("/causality/run_hash").cloned().unwrap_or(Value::Null),
                "trace_id": run_payload.pointer("/causality/trace_id").cloned().unwrap_or(Value::Null)
            }
        }));
    }
    if matches!(op.as_str(), "enable" | "status" | "dashboard") {
        claim_evidence.push(json!({
            "id": "V7-ASSIMILATE-001.5.4",
            "claim": "scheduled_hands_has_one_command_activation_and_live_operations_dashboard_metrics",
            "evidence": {
                "enabled": state.get("enabled").cloned().unwrap_or(Value::Bool(false)),
                "run_count": state.get("run_count").cloned().unwrap_or(Value::from(0)),
                "history_rows": history_rows
            }
        }));
    }

    let ok = errors.is_empty();
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "assimilation_controller_scheduled_hands",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "contract_path": SCHEDULED_HANDS_CONTRACT_PATH,
        "state_path": state_path.display().to_string(),
        "history_path": history_path.display().to_string(),
        "earnings_path": earnings_path.display().to_string(),
        "state": state,
        "run": run_payload,
        "dashboard": {
            "history_rows": history_rows,
            "cross_refs_total": read_json(&state_path)
                .and_then(|v| v.get("cross_refs_total").cloned())
                .unwrap_or(Value::from(0)),
            "earnings_total_usd": read_json(&state_path)
                .and_then(|v| v.get("earnings_total_usd").cloned())
                .unwrap_or(Value::from(0.0)),
            "earnings_total_token": read_json(&state_path)
                .and_then(|v| v.get("earnings_total_token").cloned())
                .unwrap_or(Value::from(0.0))
        },
        "requires_bedrock_proxy": requires_bedrock_proxy,
        "bedrock_proxy_enabled": bedrock_enabled,
        "errors": errors,
        "claim_evidence": claim_evidence
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    persist_receipt(root, &out);
    out
}

fn cli_error_receipt(argv: &[String], err: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "assimilation_controller_cli_error",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": code
    });
    out["receipt_hash"] = Value::String(receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let strict = parse_bool_flag(parse_flag(argv, "strict"), false);
    if command_claim_ids(&cmd).len() > 0 {
        let conduit = conduit_enforcement(argv, &cmd, strict);
        if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            let mut out = json!({
                "ok": false,
                "type": "assimilation_controller_conduit_gate",
                "lane": LANE_ID,
                "ts": now_iso(),
                "command": cmd,
                "strict": strict,
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            });
            out["receipt_hash"] = Value::String(receipt_hash(&out));
            persist_receipt(root, &out);
            print_json_line(&out);
            return 1;
        }
    }

    match cmd.as_str() {
        "status" | "run" | "assess" | "record-use" | "rollback" => {
            let out = native_receipt(root, &cmd, argv);
            persist_receipt(root, &out);
            print_json_line(&out);
            0
        }
        "skills-enable" => {
            print_json_line(&skills_enable_receipt(root, argv));
            0
        }
        "skill-create" => {
            print_json_line(&skill_create_receipt(root, argv));
            0
        }
        "skills-dashboard" => {
            print_json_line(&skills_dashboard_receipt(root));
            0
        }
        "skills-spawn-subagents" => {
            print_json_line(&skills_spawn_subagents_receipt(root, argv));
            0
        }
        "skills-computer-use" => {
            print_json_line(&skills_computer_use_receipt(root, argv));
            0
        }
        "variant-profiles" => {
            let out = run_variant_profiles_receipt(root, strict);
            let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&out);
            if ok {
                0
            } else {
                1
            }
        }
        "mpu-compartments" => {
            let out = run_mpu_compartments_receipt(root, strict);
            let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&out);
            if ok {
                0
            } else {
                1
            }
        }
        "capability-ledger" => {
            let out = run_capability_ledger_receipt(root, argv, strict);
            let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&out);
            if ok {
                0
            } else {
                1
            }
        }
        "wasm-dual-meter" => {
            let out = run_wasm_dual_meter_receipt(root, argv, strict);
            let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&out);
            if ok {
                0
            } else {
                1
            }
        }
        "hands-runtime" => {
            let out = run_hands_runtime_receipt(root, argv, strict);
            let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&out);
            if ok {
                0
            } else {
                1
            }
        }
        "scheduled-hands" => {
            let out = run_scheduled_hands_receipt(root, argv, strict);
            let ok = out.get("ok").and_then(Value::as_bool).unwrap_or(false);
            print_json_line(&out);
            if ok {
                0
            } else {
                1
            }
        }
        _ => {
            usage();
            print_json_line(&cli_error_receipt(argv, "unknown_command", 2));
            2
        }
    }
}
