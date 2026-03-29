fn build_claim_evidence(
    constitution_hash: &Option<String>,
    constitution_ok: bool,
    evidence_plan: &Value,
    evidence_ok: i64,
) -> Value {
    json!([
        {
            "id": "constitution_integrity",
            "claim": "agent_constitution_integrity_verified",
            "evidence": {
                "constitution_hash": constitution_hash.clone(),
                "integrity_ok": constitution_ok
            }
        },
        {
            "id": "evidence_loop",
            "claim": "autonomy_evidence_loop_respected_budget_plan",
            "evidence": {
                "plan": evidence_plan,
                "evidence_ok": evidence_ok
            }
        }
    ])
}

fn build_persona_lenses(cli: &CliArgs, constitution_ok: bool, evidence_plan: &Value) -> Value {
    json!({
        "guardian": {
            "clearance": std::env::var("CLEARANCE").ok().unwrap_or_else(|| "3".to_string()),
            "constitution_integrity_ok": constitution_ok
        },
        "strategist": {
            "mode": cli.mode,
            "evidence_runs": evidence_plan.get("evidence_runs").and_then(Value::as_i64).unwrap_or(0)
        }
    })
}

struct TerminalReceiptContext<'a> {
    run_id: &'a str,
    cli: &'a CliArgs,
    policy: &'a MechSuitPolicy,
    constitution_hash: &'a Option<String>,
    constitution_ok: bool,
    evidence_plan: &'a Value,
    evidence_ok: i64,
    started_ms: i64,
}

fn build_resource_snapshot(started_ms: i64) -> Value {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let elapsed_ms = (now_ms - started_ms).max(0);
    json!({
        "pid": std::process::id(),
        "uptime_sec": (elapsed_ms as f64) / 1000.0
    })
}

fn emit_terminal_receipt(
    ledger: &mut LedgerWriter,
    context: &TerminalReceiptContext<'_>,
    ok: bool,
    failure_reason: Option<&str>,
) -> i32 {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let elapsed_ms = (now_ms - context.started_ms).max(0);
    let terminal_step = ledger
        .last_type()
        .unwrap_or("spine_run_started")
        .to_string();
    let mut receipt = json!({
        "ok": ok,
        "type": if ok { "spine_run_complete" } else { "spine_run_failed" },
        "ts": now_iso(),
        "run_id": context.run_id,
        "mode": context.cli.mode,
        "date": context.cli.date,
        "elapsed_ms": elapsed_ms,
        "terminal_step": terminal_step,
        "resource_snapshot": build_resource_snapshot(context.started_ms),
        "claim_evidence": build_claim_evidence(
            context.constitution_hash,
            context.constitution_ok,
            context.evidence_plan,
            context.evidence_ok
        ),
        "persona_lenses": build_persona_lenses(
            context.cli,
            context.constitution_ok,
            context.evidence_plan
        ),
        "constitution_hash": context.constitution_hash.clone(),
        "constitution_integrity_ok": context.constitution_ok,
        "evidence_plan": context.evidence_plan,
        "evidence_ok": context.evidence_ok
    });

    if let Some(reason) = failure_reason {
        receipt["failure_reason"] = Value::String(reason.to_string());
    }

    receipt["receipt_hash"] = Value::String(receipt_hash(&receipt));
    ledger.append(receipt.clone());

    if !ok {
        enqueue_spine_attention(
            &ledger.root,
            "spine_run_failed",
            "critical",
            failure_reason.unwrap_or("spine_run_failed"),
        );
    }

    if !ok || !(context.policy.enabled && context.policy.quiet_non_critical) {
        println!(
            "{}",
            serde_json::to_string(&receipt)
                .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
        );
    }

    update_mech_suit_status(
        &ledger.root,
        context.policy,
        "spine",
        json!({
            "ambient": context.policy.enabled,
            "heartbeat_hours": context.policy.heartbeat_hours,
            "manual_triggers_allowed": context.policy.manual_triggers_allowed,
            "quiet_non_critical": context.policy.quiet_non_critical,
            "silent_subprocess_output": context.policy.silent_subprocess_output,
            "attention_emission_owner": "eyes",
            "attention_escalation_authority": "runtime_policy",
            "last_result": if ok { "run_complete" } else { "run_failed" },
            "last_mode": context.cli.mode,
            "last_date": context.cli.date,
            "last_terminal_step": terminal_step,
            "last_failure_reason": failure_reason.map(|s| s.to_string())
        }),
    );

    if ok {
        0
    } else {
        1
    }
}

fn run_guard(root: &Path, files: &[&str]) -> StepResult {
    let mut missing = Vec::new();
    for file in files {
        if !root.join(file).is_file() {
            missing.push((*file).to_string());
        }
    }
    let ok = missing.is_empty();
    let payload = json!({
        "ok": ok,
        "type": "spine_guard_payload",
        "checked_count": files.len(),
        "checked_files": files,
        "missing_files": missing
    });
    StepResult {
        ok,
        code: if ok { 0 } else { 1 },
        payload: Some(payload.clone()),
        stdout: serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
        stderr: String::new(),
    }
}

fn step_ops_domain(
    root: &Path,
    name: &str,
    domain: &str,
    args: Vec<String>,
    run_context: Option<&str>,
    ledger: &mut LedgerWriter,
    mode: &str,
    date: &str,
) -> Result<StepResult, String> {
    let res = run_ops_domain_json(root, domain, &args, run_context);
    ledger.append(json!({
        "type": "spine_step",
        "mode": mode,
        "date": date,
        "step": name,
        "domain": domain,
        "ok": res.ok,
        "code": res.code,
        "payload": res.payload,
        "reason": if res.ok { Value::Null } else { Value::String(clean_reason(&res.stderr, &res.stdout)) }
    }));

    if res.ok {
        Ok(res)
    } else {
        Err(format!("step_failed:{name}:{}", res.code))
    }
}

fn append_self_documentation_closeout(
    root: &Path,
    ledger: &mut LedgerWriter,
    mode: &str,
    date: &str,
) {
    if mode != "daily" {
        return;
    }

    let args = vec![
        "client/runtime/systems/ops/run_protheus_ops.js".to_string(),
        "autonomy-controller".to_string(),
        "self-documentation-closeout".to_string(),
        format!("--date={date}"),
        "--approve=1".to_string(),
    ];
    let res = run_node_json(root, &args);
    ledger.append(json!({
        "type": "spine_step",
        "mode": mode,
        "date": date,
        "step": "self_documentation_closeout",
        "ok": res.ok,
        "code": res.code,
        "non_blocking": true,
        "payload": res.payload,
        "reason": if res.ok { Value::Null } else { Value::String(clean_reason(&res.stderr, &res.stdout)) }
    }));
}

fn emit_terminal_with_closeout(
    root: &Path,
    ledger: &mut LedgerWriter,
    context: &TerminalReceiptContext<'_>,
    ok: bool,
    failure_reason: Option<&str>,
) -> i32 {
    append_self_documentation_closeout(root, ledger, &context.cli.mode, &context.cli.date);
    if context.cli.mode == "daily" {
        let (_code, payload) = execute_sleep_cleanup(root, true, false, "spine_daily");
        ledger.append(json!({
            "type": "spine_step_non_blocking",
            "mode": context.cli.mode,
            "date": context.cli.date,
            "step": "sleep_cleanup_cycle",
            "ok": payload.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "non_blocking": true,
            "payload": payload
        }));
    }
    emit_terminal_receipt(ledger, context, ok, failure_reason)
}

fn clean_reason(stderr: &str, stdout: &str) -> String {
    let merged = format!("{} {}", stderr, stdout)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if merged.len() <= 180 {
        merged
    } else {
        merged[..180].to_string()
    }
}

