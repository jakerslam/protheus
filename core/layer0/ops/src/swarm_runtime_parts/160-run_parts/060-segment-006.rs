                }
                "wait" => {
                    let filters = parse_result_filters(argv);
                    let min_count = parse_u64_flag(argv, "min-count", 1) as usize;
                    let timeout_ms =
                        (parse_f64_flag(argv, "timeout-sec", 30.0).max(0.1) * 1000.0) as u64;
                    match wait_for_results(&state_file, &state, &filters, min_count, timeout_ms) {
                        Ok(results) => Ok(json!({
                            "ok": true,
                            "type": "swarm_runtime_results_wait",
                            "min_count": min_count.max(1),
                            "timeout_ms": timeout_ms,
                            "result_count": results.len(),
                            "results": results,
                        })),
                        Err(err) => Err(err),
                    }
                }
                "show" => {
                    if let Some(result_id) =
                        parse_flag(argv, "result-id").filter(|value| !value.trim().is_empty())
                    {
                        if let Some(result) = state.result_registry.get(&result_id).cloned() {
                            Ok(json!({
                                "ok": true,
                                "type": "swarm_runtime_results_show",
                                "result": result,
                            }))
                        } else {
                            Err(format!("unknown_result:{result_id}"))
                        }
                    } else {
                        Err("result_id_required".to_string())
                    }
                }
                "consensus" => {
                    let filters = parse_result_filters(argv);
                    let field = parse_flag(argv, "field")
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "value".to_string());
                    let threshold = parse_f64_flag(argv, "threshold", 1.0).clamp(0.0, 1.0);
                    let results = query_results(&state, &filters);
                    let consensus = analyze_result_consensus(&results, &field, threshold);
                    append_event(
                        &mut state,
                        json!({
                            "type": "swarm_results_consensus",
                            "field": field,
                            "threshold": threshold,
                            "result_count": results.len(),
                            "status": consensus.get("status").cloned().unwrap_or(Value::Null),
                            "consensus_reached": consensus
                                .get("consensus_reached")
                                .cloned()
                                .unwrap_or(Value::Bool(false)),
                            "timestamp": now_iso(),
                        }),
                    );
                    Ok(json!({
                        "ok": true,
                        "type": "swarm_runtime_results_consensus",
                        "field": field,
                        "result_count": results.len(),
                        "consensus": consensus,
                    }))
                }
                "outliers" => {
                    let filters = parse_result_filters(argv);
                    let field = parse_flag(argv, "field")
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "value".to_string());
                    let results = query_results(&state, &filters);
                    let analysis = analyze_result_outliers(&results, &field);
                    append_event(
                        &mut state,
                        json!({
                            "type": "swarm_results_outliers",
                            "field": field,
                            "result_count": results.len(),
                            "status": analysis.get("status").cloned().unwrap_or(Value::Null),
                            "outlier_count": analysis.get("outlier_count").cloned().unwrap_or(json!(0)),
                            "timestamp": now_iso(),
                        }),
                    );
                    Ok(json!({
                        "ok": true,
                        "type": "swarm_runtime_results_outliers",
                        "field": field,
                        "result_count": results.len(),
                        "analysis": analysis,
                    }))
                }
                _ => Err(format!("unknown_results_subcommand:{sub}")),
            }
        }
        "metrics" => {
            let sub = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "queue".to_string());
            match sub.as_str() {
                "queue" => {
                    let snapshot = queue_metrics_snapshot(&state);
                    let format = parse_flag(argv, "format")
                        .unwrap_or_else(|| "json".to_string())
                        .to_ascii_lowercase();
                    let prometheus = if format == "prometheus" {
                        Some(queue_metrics_prometheus(&state, &snapshot))
                    } else {
                        None
                    };
                    Ok(json!({
                        "ok": true,
                        "type": "swarm_runtime_metrics_queue",
                        "format": format,
                        "snapshot": snapshot,
                        "prometheus": prometheus,
                    }))
                }
                _ => Err(format!("unknown_metrics_subcommand:{sub}")),
            }
        }
        "test" => {
            let suite = argv
                .get(1)
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "recursive".to_string());
            match suite.as_str() {
                "recursive" => run_test_recursive(&mut state, argv),
                "byzantine" => run_test_byzantine(&mut state, argv),
                "concurrency" => run_test_concurrency(&mut state, argv),
                "hierarchy" => run_test_hierarchy(&mut state, argv),
                "budget" => run_test_budget(&mut state, argv),
                "persistent" => run_test_persistent(&mut state, argv),
                "communication" => run_test_communication(&mut state, argv),
                "heterogeneous" => run_test_heterogeneous(&mut state, &state_file, argv),
                _ => Err(format!("unknown_test_suite:{suite}")),
            }
        }
        "thorn" => run_thorn_contract_in_state(&mut state, &argv[1..]).map(|mut payload| {
            payload["claim_evidence"] = json!([{
                "id": "V6-SEC-THORN-001",
                "claim": "thorn_cells_quarantine_compromised_sessions_with_restricted_capabilities_and_receipted_reroute_self_destruct_flow",
                "evidence": {
                    "command": argv.get(1).cloned().unwrap_or_else(|| "status".to_string()),
                    "state_path": state_file.display().to_string(),
                }
            }]);
            payload["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&payload));
            payload
        }),
        _ => Err(format!("unknown_command:{cmd}")),
    };

    state.updated_at = now_iso();
    let save_result = save_state(&state_file, &state);

    match result {
        Ok(payload) => {
            if let Err(err) = save_result {
                print_receipt(json!({
                    "ok": false,
                    "type": "swarm_runtime_error",
                    "command": cmd,
                    "error": err,
                    "state_path": state_file,
                }));
                return 2;
            }

