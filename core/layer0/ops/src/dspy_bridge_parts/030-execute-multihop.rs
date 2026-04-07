fn execute_multihop(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(payload.get("name").and_then(Value::as_str), "dspy-multihop");
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let swarm_path = swarm_state_path(root, argv, payload);
    let integration_ids = parse_string_list(payload.get("integration_ids"));
    if !integration_ids.iter().all(|id| {
        state
            .get("integrations")
            .and_then(Value::as_object)
            .map(|rows| rows.contains_key(id))
            .unwrap_or(false)
    }) {
        return Err("dspy_multihop_integration_missing".to_string());
    }
    let mut hops = payload
        .get("hops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if hops.is_empty() {
        let program_id = clean_token(payload.get("program_id").and_then(Value::as_str), "");
        if let Some(program) = state
            .get("compiled_programs")
            .and_then(Value::as_object)
            .and_then(|rows| rows.get(&program_id))
            .and_then(Value::as_object)
        {
            hops = program
                .get("modules")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|row| {
                    let obj = row.as_object().cloned().unwrap_or_default();
                    json!({
                        "label": obj.get("label").cloned().unwrap_or_else(|| json!("hop")),
                        "signature_id": obj.get("signature_id").cloned().unwrap_or(Value::Null),
                        "query": obj.get("prompt_template").cloned().unwrap_or_else(|| json!("compile-derived-query")),
                    })
                })
                .collect();
        }
    }
    if hops.is_empty() {
        return Err("dspy_multihop_hops_required".to_string());
    }
    let degraded = matches!(profile.as_str(), "pure" | "tiny-max") && hops.len() > 2;
    let selected_hops = if degraded { hops[..2].to_vec() } else { hops };
    let coordinator_task = format!("dspy:multihop:{name}:coordinator");
    let coordinator_id = ensure_session_for_task(
        root,
        &swarm_path,
        &coordinator_task,
        &clean_token(
            payload.get("coordinator_label").and_then(Value::as_str),
            "dspy-coordinator",
        ),
        Some("coordinator"),
        None,
        parse_u64_value(payload.get("budget"), 960, 96, 12288),
    )?;
    let mut rows = Vec::new();
    for (idx, hop) in selected_hops.iter().enumerate() {
        let obj = hop
            .as_object()
            .ok_or_else(|| "dspy_multihop_hop_object_required".to_string())?;
        let label = clean_token(
            obj.get("label").and_then(Value::as_str),
            &format!("hop-{}", idx + 1),
        );
        let task = format!("dspy:multihop:{name}:{label}");
        let child_id = ensure_session_for_task(
            root,
            &swarm_path,
            &task,
            &label,
            Some("reasoner"),
            Some(&coordinator_id),
            parse_u64_value(obj.get("budget"), 224, 32, 4096),
        )?;
        let handoff_exit = crate::swarm_runtime::run(
            root,
            &[
                "sessions".to_string(),
                "handoff".to_string(),
                format!("--session-id={coordinator_id}"),
                format!("--target-session-id={child_id}"),
                format!(
                    "--reason={}",
                    clean_text(obj.get("reason").and_then(Value::as_str), 120)
                        .if_empty_then(&format!("dspy_hop_{label}"))
                ),
                format!(
                    "--importance={:.2}",
                    parse_f64_value(obj.get("importance"), 0.76, 0.0, 1.0)
                ),
                format!("--state-path={}", swarm_path.display()),
            ],
        );
        if handoff_exit != 0 {
            return Err(format!("dspy_multihop_handoff_failed:{label}"));
        }
        let context = json!({
            "query": clean_text(obj.get("query").and_then(Value::as_str), 200),
            "signature_id": clean_token(obj.get("signature_id").and_then(Value::as_str), ""),
            "integration_ids": integration_ids,
            "tool_tags": parse_string_list(obj.get("tool_tags")),
        });
        let context_exit = crate::swarm_runtime::run(
            root,
            &[
                "sessions".to_string(),
                "context-put".to_string(),
                format!("--session-id={child_id}"),
                format!("--context-json={}", encode_json_arg(&context)?),
                "--merge=1".to_string(),
                format!("--state-path={}", swarm_path.display()),
            ],
        );
        if context_exit != 0 {
            return Err(format!("dspy_multihop_context_put_failed:{label}"));
        }
        rows.push(json!({
            "label": label,
            "session_id": child_id,
            "signature_id": clean_token(obj.get("signature_id").and_then(Value::as_str), ""),
            "budget": parse_u64_value(obj.get("budget"), 224, 32, 4096),
        }));
    }
    let record = json!({
        "multihop_id": stable_id("dspmulti", &json!({"name": name, "profile": profile, "hops": rows})),
        "name": name,
        "profile": profile,
        "coordinator_session_id": coordinator_id,
        "integration_ids": integration_ids,
        "hop_count": rows.len(),
        "executed_hops": rows,
        "degraded": degraded,
        "reason_code": if degraded { "multihop_profile_limited" } else { "multihop_ok" },
        "executed_at": now_iso(),
    });
    let multihop_id = record
        .get("multihop_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "multihop_runs").insert(multihop_id.clone(), record.clone());
    emit_native_trace(
        root,
        &multihop_id,
        "dspy_multihop",
        &format!(
            "name={} hops={}",
            name,
            record["hop_count"].as_u64().unwrap_or(0)
        ),
    )?;
    Ok(json!({
        "ok": true,
        "multihop": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-017.5", dspy_claim("V6-WORKFLOW-017.5")),
    }))
}

trait EmptyStringFallback {
    fn if_empty_then<'a>(&'a self, fallback: &'a str) -> &'a str;
}

impl EmptyStringFallback for String {
    fn if_empty_then<'a>(&'a self, fallback: &'a str) -> &'a str {
        if self.is_empty() {
            fallback
        } else {
            self.as_str()
        }
    }
}

fn record_benchmark(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let record = json!({
        "benchmark_id": stable_id("dspbench", &json!({"program_id": payload.get("program_id"), "metrics": payload.get("metrics")})),
        "program_id": clean_token(payload.get("program_id").and_then(Value::as_str), ""),
        "benchmark_name": clean_token(payload.get("benchmark_name").and_then(Value::as_str), "dspy-benchmark"),
        "profile": clean_token(payload.get("profile").and_then(Value::as_str), "rich"),
        "score": parse_f64_value(payload.get("score"), 0.0, 0.0, 1.0),
        "metrics": payload.get("metrics").cloned().unwrap_or_else(|| json!({})),
        "recorded_at": now_iso(),
    });
    let benchmark_id = record
        .get("benchmark_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "benchmarks").insert(benchmark_id.clone(), record.clone());
    emit_native_trace(
        root,
        &benchmark_id,
        "dspy_benchmark",
        &format!(
            "program_id={} score={:.2}",
            record["program_id"].as_str().unwrap_or(""),
            record["score"].as_f64().unwrap_or(0.0)
        ),
    )?;
    Ok(json!({
        "ok": true,
        "benchmark": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-017.6", dspy_claim("V6-WORKFLOW-017.6")),
    }))
}

fn record_optimization_trace(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let record = json!({
        "trace_id": stable_id("dsptrace", &json!({"program_id": payload.get("program_id"), "optimization_id": payload.get("optimization_id"), "seed": payload.get("seed")})),
        "program_id": clean_token(payload.get("program_id").and_then(Value::as_str), ""),
        "optimization_id": clean_token(payload.get("optimization_id").and_then(Value::as_str), ""),
        "profile": clean_token(payload.get("profile").and_then(Value::as_str), "rich"),
        "seed": parse_u64_value(payload.get("seed"), 7, 0, u64::MAX),
        "reproducible": parse_bool_value(payload.get("reproducible"), true),
        "message": clean_text(payload.get("message").and_then(Value::as_str), 160),
        "recorded_at": now_iso(),
    });
    let trace_id = record
        .get("trace_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "optimization_traces").insert(trace_id.clone(), record.clone());
    emit_native_trace(
        root,
        &trace_id,
        "dspy_trace",
        &format!(
            "program_id={} reproducible={}",
            record["program_id"].as_str().unwrap_or(""),
            record["reproducible"].as_bool().unwrap_or(false)
        ),
    )?;
    Ok(json!({
        "ok": true,
        "optimization_trace": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-017.8", dspy_claim("V6-WORKFLOW-017.8")),
    }))
}

fn assimilate_intake(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let shell_path = normalize_shell_path(
        root,
        payload
            .get("shell_path")
            .and_then(Value::as_str)
            .unwrap_or("client/runtime/systems/workflow/dspy_bridge.ts"),
    )?;
    let record = json!({
        "intake_id": stable_id("dspintake", &json!({"shell_path": shell_path, "target": payload.get("target")})),
        "shell_name": clean_token(payload.get("shell_name").and_then(Value::as_str), "dspy-shell"),
        "shell_path": shell_path,
        "target": clean_token(payload.get("target").and_then(Value::as_str), "local"),
        "artifact_path": clean_text(payload.get("artifact_path").and_then(Value::as_str), 240),
        "deletable": true,
        "authority_delegate": "core://dspy-bridge",
        "deployed_at": now_iso(),
    });
    let intake_id = record
        .get("intake_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "intakes").insert(intake_id, record.clone());
    Ok(json!({
        "ok": true,
        "intake": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-017.7", dspy_claim("V6-WORKFLOW-017.7")),
    }))
}

fn status(root: &Path, state: &Value, state_path: &Path, history_path: &Path) -> Value {
    json!({
        "ok": true,
        "state_path": rel(root, state_path),
        "history_path": rel(root, history_path),
        "signatures": state.get("signatures").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "compiled_programs": state.get("compiled_programs").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "optimization_runs": state.get("optimization_runs").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "assertion_runs": state.get("assertion_runs").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "integrations": state.get("integrations").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "multihop_runs": state.get("multihop_runs").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "benchmarks": state.get("benchmarks").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "optimization_traces": state.get("optimization_traces").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "intakes": state.get("intakes").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "last_receipt": state.get("last_receipt").cloned().unwrap_or(Value::Null),
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|row| row.trim().to_ascii_lowercase()) else {
        usage();
        return 0;
    };
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("dspy_bridge_error", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let state_path = state_path(root, argv, payload);
    let history_path = history_path(root, argv, payload);
    let mut state = load_state(&state_path);

    let result = match command.as_str() {
        "status" => Ok(status(root, &state, &state_path, &history_path)),
        "register-signature" => register_signature(&mut state, payload),
        "compile-program" => compile_program(&mut state, payload),
        "optimize-program" => optimize_program(root, &mut state, payload),
        "assert-program" => assert_program(&mut state, payload),
        "import-integration" => import_integration(root, &mut state, payload),
        "execute-multihop" => execute_multihop(root, argv, &mut state, payload),
        "record-benchmark" => record_benchmark(root, &mut state, payload),
        "record-optimization-trace" => record_optimization_trace(root, &mut state, payload),
        "assimilate-intake" => assimilate_intake(root, &mut state, payload),
        other => Err(format!("dspy_bridge_unknown_command:{other}")),
    };

    match result {
        Ok(payload_out) => {
            let receipt = cli_receipt(
                match command.as_str() {
                    "status" => "dspy_bridge_status",
                    "register-signature" => "dspy_bridge_register_signature",
                    "compile-program" => "dspy_bridge_compile_program",
                    "optimize-program" => "dspy_bridge_optimize_program",
                    "assert-program" => "dspy_bridge_assert_program",
                    "import-integration" => "dspy_bridge_import_integration",
                    "execute-multihop" => "dspy_bridge_execute_multihop",
                    "record-benchmark" => "dspy_bridge_record_benchmark",
                    "record-optimization-trace" => "dspy_bridge_record_optimization_trace",
                    "assimilate-intake" => "dspy_bridge_assimilate_intake",
                    _ => "dspy_bridge_command",
                },
                payload_out,
            );
            state["last_receipt"] = receipt.clone();
            if command != "status" {
                if let Err(err) = save_state(&state_path, &state)
                    .and_then(|_| append_history(&history_path, &receipt))
                {
                    print_json_line(&cli_error("dspy_bridge_error", &err));
                    return 1;
                }
            }
            print_json_line(&receipt);
            if receipt.get("ok").and_then(Value::as_bool).unwrap_or(true) {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_json_line(&cli_error("dspy_bridge_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optimize_program_degrades_in_pure_profile() {
        let mut state = default_state();
        let _ = register_signature(
            &mut state,
            &Map::from_iter(vec![
                ("name".to_string(), json!("qa_signature")),
                ("input_fields".to_string(), json!(["question"])),
                ("output_fields".to_string(), json!(["answer"])),
            ]),
        )
        .expect("signature");
        let signature_id = state
            .get("signatures")
            .and_then(Value::as_object)
            .and_then(|rows| rows.values().next())
            .and_then(|row| row.get("signature_id"))
            .and_then(Value::as_str)
            .unwrap()
            .to_string();
        let _ = compile_program(
            &mut state,
            &Map::from_iter(vec![
                ("name".to_string(), json!("qa_program")),
                (
                    "modules".to_string(),
                    json!([{"label": "answer", "signature_id": signature_id}]),
                ),
            ]),
        )
        .expect("program");
        let program_id = state
            .get("compiled_programs")
            .and_then(Value::as_object)
            .and_then(|rows| rows.values().next())
            .and_then(|row| row.get("program_id"))
            .and_then(Value::as_str)
            .unwrap()
            .to_string();
        let tmp = tempfile::tempdir().expect("tempdir");
        let response = optimize_program(
            tmp.path(),
            &mut state,
            &Map::from_iter(vec![
                ("program_id".to_string(), json!(program_id)),
                ("profile".to_string(), json!("pure")),
                ("max_trials".to_string(), json!(8)),
            ]),
        )
        .expect("optimize");
        assert_eq!(response["optimization"]["degraded"], json!(true));
        assert_eq!(
            response["claim_evidence"][0]["id"],
            json!("V6-WORKFLOW-017.3")
        );
    }
}

