pub fn run(root: &Path, argv: &[String]) -> i32 {
    let policy = load_policy(root);
    if argv.is_empty() {
        usage();
        let receipt = cli_error_receipt(&policy, "unknown", "missing_command", 2);
        println!(
            "{}",
            serde_json::to_string(&receipt).unwrap_or_else(|_| "{\"ok\":false}".to_string())
        );
        return 2;
    }

    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();

    if command == "status" {
        let latest = read_json(&policy.latest_path).unwrap_or_else(|| json!({}));
        let status_source = if latest.get("type").and_then(Value::as_str) == Some("memory_ambient")
        {
            "cached_latest"
        } else {
            "cold_status"
        };
        let mut claim_evidence = Vec::new();
        if latest
            .get("memory_command")
            .and_then(Value::as_str)
            .map(is_nano_memory_command)
            .unwrap_or(false)
        {
            claim_evidence.push(json!({
                "id": "V6-COCKPIT-026.5",
                "claim": "nano_mode_observability_is_surfaceable_through_status_dashboard_receipts",
                "evidence": {
                    "status_source": status_source,
                    "last_memory_command": latest
                        .get("memory_command")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                }
            }));
        }
        if latest
            .get("memory_command")
            .and_then(Value::as_str)
            .map(|cmd| matches!(cmd, "memory-taxonomy" | "stable-memory-taxonomy"))
            .unwrap_or(false)
        {
            claim_evidence.push(json!({
                "id": "V6-MEMORY-011.5",
                "claim": "taxonomy_health_metrics_are_surfaceable_through_status_dashboard_receipts",
                "evidence": {
                    "status_source": status_source,
                    "last_memory_command": latest
                        .get("memory_command")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    "row_count": latest
                        .get("memory_payload")
                        .and_then(|v| v.get("row_count"))
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                }
            }));
        }
        let mut receipt = json!({
            "ok": true,
            "type": "memory_ambient_status",
            "ts": now_iso(),
            "status_source": status_source,
            "ambient_mode_active": policy.enabled,
            "rust_authoritative": policy.rust_authoritative,
            "policy": policy_snapshot(&policy),
            "last": latest,
            "claim_evidence": claim_evidence
        });
        receipt["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&receipt));
        println!(
            "{}",
            serde_json::to_string(&receipt).unwrap_or_else(|_| "{\"ok\":false}".to_string())
        );
        return 0;
    }

    if command != "run" {
        usage();
        let receipt = cli_error_receipt(&policy, &command, "unknown_command", 2);
        println!(
            "{}",
            serde_json::to_string(&receipt).unwrap_or_else(|_| "{\"ok\":false}".to_string())
        );
        return 2;
    }

    let invocation =
        match extract_memory_invocation(&argv.iter().skip(1).cloned().collect::<Vec<_>>()) {
            Ok(v) => v,
            Err(reason) => {
                let receipt = cli_error_receipt(&policy, &command, &reason, 2);
                println!(
                    "{}",
                    serde_json::to_string(&receipt)
                        .unwrap_or_else(|_| "{\"ok\":false}".to_string())
                );
                return 2;
            }
        };

    let (memory_command, memory_args, run_context) = invocation;
    let run_flags = parse_cli_flags(&argv.iter().skip(1).cloned().collect::<Vec<_>>());
    let strict = parse_bool_value(run_flags.get("strict").map(String::as_str), false)
        || parse_bool_value(parse_arg_value(&memory_args, "strict").as_deref(), false);
    let bypass_requested = parse_bool_value(run_flags.get("bypass").map(String::as_str), false)
        || parse_bool_value(run_flags.get("client-bypass").map(String::as_str), false)
        || parse_bool_value(parse_arg_value(&memory_args, "bypass").as_deref(), false)
        || parse_bool_value(
            parse_arg_value(&memory_args, "client-bypass").as_deref(),
            false,
        );
    if strict
        && bypass_requested
        && (is_nano_memory_command(&memory_command) || is_batch22_memory_command(&memory_command))
    {
        let gate_claim_ids = if is_nano_memory_command(&memory_command) {
            vec!["V6-COCKPIT-026.4"]
        } else {
            memory_batch22_command_claim_ids(&memory_command).to_vec()
        };
        let gate_claim_evidence = gate_claim_ids
            .iter()
            .map(|claim_id| {
                json!({
                    "id": claim_id,
                    "claim": "memory_commands_fail_closed_when_conduit_bypass_is_requested",
                    "evidence": {
                        "memory_command": memory_command.clone(),
                        "bypass_requested": bypass_requested
                    }
                })
            })
            .collect::<Vec<Value>>();
        let mut receipt = json!({
            "ok": false,
            "type": "memory_ambient_conduit_gate",
            "ts": now_iso(),
            "command": command.clone(),
            "memory_command": memory_command.clone(),
            "strict": strict,
            "errors": ["conduit_bypass_rejected"],
            "claim_evidence": gate_claim_evidence,
            "policy": policy_snapshot(&policy)
        });
        receipt["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&receipt));
        write_json(&policy.latest_path, &receipt);
        append_jsonl(&policy.receipts_path, &receipt);
        update_mech_suit_status(
            &policy,
            json!({
                "ambient": policy.enabled,
                "rust_authoritative": policy.rust_authoritative,
                "push_attention_queue": policy.push_attention_queue,
                "quiet_non_critical": policy.quiet_non_critical,
                "last_result": "memory_ambient_conduit_gate",
                "last_command": command,
                "last_memory_command": memory_command,
                "last_ok": false,
                "last_severity": "critical",
                "last_attention_decision": "conduit_bypass_rejected"
            }),
        );
        println!(
            "{}",
            serde_json::to_string(&receipt).unwrap_or_else(|_| "{\"ok\":false}".to_string())
        );
        return 1;
    }

    if memory_command == "cryonics-tier" {
        let receipt = cryonics_compat_receipt(&policy, &command, &run_context, &memory_args);
        write_json(&policy.latest_path, &receipt);
        append_jsonl(&policy.receipts_path, &receipt);
        update_mech_suit_status(
            &policy,
            json!({
                "ambient": policy.enabled,
                "rust_authoritative": policy.rust_authoritative,
                "push_attention_queue": policy.push_attention_queue,
                "quiet_non_critical": policy.quiet_non_critical,
                "last_result": "memory_ambient_compat",
                "last_command": command,
                "last_memory_command": "cryonics-tier",
                "last_ok": true,
                "last_severity": "info",
                "last_attention_decision": "compatibility_no_enqueue"
            }),
        );
        println!(
            "{}",
            serde_json::to_string(&receipt).unwrap_or_else(|_| "{\"ok\":false}".to_string())
        );
        return 0;
    }

    if !is_allowed_memory_command(&memory_command) {
        let receipt = cli_error_receipt(
            &policy,
            &command,
            &format!("memory_command_not_allowed:{memory_command}"),
            1,
        );
        println!(
            "{}",
            serde_json::to_string(&receipt).unwrap_or_else(|_| "{\"ok\":false}".to_string())
        );
        return 1;
    }

    let (mut memory_payload, stdout, stderr, exit_code, command_info) =
        match run_memory_command(root, &memory_command, &memory_args) {
            Ok(value) => value,
            Err(reason) => {
                let receipt = cli_error_receipt(&policy, &command, &reason, 1);
                println!(
                    "{}",
                    serde_json::to_string(&receipt)
                        .unwrap_or_else(|_| "{\"ok\":false}".to_string())
                );
                return 1;
            }
        };
    ensure_memory_contract_digests(&memory_command, &memory_args, &mut memory_payload);

    let memory_ok = memory_payload
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let op_ok = exit_code == 0 && memory_ok;
    let severity = classify_severity(&memory_command, op_ok, &memory_payload);
    let surfaced = policy.enabled
        && policy.push_attention_queue
        && should_surface(&policy, &severity)
        && (!policy.quiet_non_critical || severity == "critical");

    let summary_line = if op_ok {
        format!("memory op ok ({memory_command})")
    } else {
        format!("memory op failed ({memory_command})")
    };
    let token_telemetry = build_token_telemetry(&memory_command, &memory_args, &memory_payload);
    let mut claim_evidence =
        cockpit_claim_evidence(&memory_command, &memory_args, &token_telemetry);
    claim_evidence.extend(memory_batch22_claim_evidence(
        &memory_command,
        &memory_args,
        &memory_payload,
    ));

    let attention_queue = if surfaced {
        match enqueue_attention(
            root,
            &memory_command,
            &severity,
            op_ok,
            &run_context,
            &summary_line,
        ) {
            Ok(value) => value,
            Err(reason) => {
                let receipt = cli_error_receipt(&policy, &command, &reason, 1);
                println!(
                    "{}",
                    serde_json::to_string(&receipt)
                        .unwrap_or_else(|_| "{\"ok\":false}".to_string())
                );
                return 1;
            }
        }
    } else {
        json!({
            "ok": true,
            "queued": false,
            "decision": if !policy.enabled { "ambient_disabled" } else if !policy.push_attention_queue { "attention_queue_disabled" } else if !should_surface(&policy, &severity) { "below_threshold" } else if policy.quiet_non_critical && severity != "critical" { "quiet_non_critical" } else { "not_enqueued" },
            "routed_via": "rust_attention_queue"
        })
    };

    let mut receipt = json!({
        "ok": op_ok,
        "type": "memory_ambient",
        "ts": now_iso(),
        "command": command,
        "run_context": run_context,
        "ambient_mode_active": policy.enabled,
        "rust_authoritative": policy.rust_authoritative,
        "memory_command": memory_command,
        "memory_args_count": memory_args.len(),
        "memory_args_hash": crate::deterministic_receipt_hash(&json!(memory_args)),
        "severity": severity,
        "surfaced": surfaced,
        "attention_queue": attention_queue,
        "memory_payload": memory_payload,
        "memory_command_info": command_info,
        "stdout": if op_ok && policy.quiet_non_critical { "".to_string() } else { clean_text(Some(&stdout), 2_000) },
        "stderr": clean_text(Some(&stderr), 2_000),
        "exit_code": exit_code,
        "token_telemetry": token_telemetry,
        "claim_evidence": claim_evidence,
        "policy": policy_snapshot(&policy)
    });
    receipt["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&receipt));

    append_jsonl(
        &token_telemetry_path(root),
        &json!({
            "ts": now_iso(),
            "lane": "memory_recall",
            "retrieval_mode": receipt
                .get("token_telemetry")
                .and_then(|v| v.get("retrieval_mode"))
                .and_then(Value::as_str)
                .unwrap_or("index_only"),
            "reason_codes": receipt
                .get("token_telemetry")
                .and_then(|v| v.get("reason_codes"))
                .cloned()
                .unwrap_or_else(|| json!([])),
            "tokens": receipt
                .get("token_telemetry")
                .and_then(|v| v.get("tokens"))
                .cloned()
                .unwrap_or_else(|| json!({})),
            "threshold_tokens": receipt
                .get("token_telemetry")
                .and_then(|v| v.get("threshold_tokens"))
                .and_then(Value::as_i64)
                .unwrap_or(200),
            "command": receipt
                .get("token_telemetry")
                .and_then(|v| v.get("command"))
                .and_then(Value::as_str)
                .unwrap_or("query")
        }),
    );

    write_json(&policy.latest_path, &receipt);
    append_jsonl(&policy.receipts_path, &receipt);

    update_mech_suit_status(
        &policy,
        json!({
            "ambient": policy.enabled,
            "rust_authoritative": policy.rust_authoritative,
            "push_attention_queue": policy.push_attention_queue,
            "quiet_non_critical": policy.quiet_non_critical,
            "last_result": "memory_ambient",
            "last_command": command,
            "last_memory_command": receipt.get("memory_command").and_then(Value::as_str).unwrap_or("unknown"),
            "last_ok": op_ok,
            "last_severity": severity,
            "last_attention_decision": receipt
                .get("attention_queue")
                .and_then(|v| v.get("decision"))
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        }),
    );

    println!(
        "{}",
        serde_json::to_string(&receipt).unwrap_or_else(|_| "{\"ok\":false}".to_string())
    );

    if op_ok {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_policy_respects_level_filter() {
        let policy = MemoryAmbientPolicy {
            enabled: true,
            rust_authoritative: true,
            push_attention_queue: true,
            quiet_non_critical: false,
            surface_levels: vec!["warn".to_string(), "critical".to_string()],
            latest_path: PathBuf::from("/tmp/latest.json"),
            receipts_path: PathBuf::from("/tmp/receipts.jsonl"),
            status_path: PathBuf::from("/tmp/status.json"),
            history_path: PathBuf::from("/tmp/history.jsonl"),
            policy_path: PathBuf::from("/tmp/policy.json"),
        };
        assert!(!should_surface(&policy, "info"));
        assert!(should_surface(&policy, "warn"));
        assert!(should_surface(&policy, "critical"));
    }
}

