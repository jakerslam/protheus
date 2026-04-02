pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].as_str();
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("langchain_bridge_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let state_path = state_path(root, argv, input);
    let history_path = history_path(root, argv, input);
    let mut state = load_state(&state_path);

    let result = match command {
        "status" => Ok(json!({
            "ok": true,
            "state_path": rel(root, &state_path),
            "history_path": rel(root, &history_path),
            "chains": as_object_mut(&mut state, "chains").len(),
            "chain_runs": as_object_mut(&mut state, "chain_runs").len(),
            "middleware_hooks": as_object_mut(&mut state, "middleware_hooks").len(),
            "agent_runs": as_object_mut(&mut state, "agent_runs").len(),
            "memory_bridges": as_object_mut(&mut state, "memory_bridges").len(),
            "memory_queries": as_object_mut(&mut state, "memory_queries").len(),
            "integrations": as_object_mut(&mut state, "integrations").len(),
            "prompt_routes": as_object_mut(&mut state, "prompt_routes").len(),
            "structured_outputs": as_object_mut(&mut state, "structured_outputs").len(),
            "traces": as_array_mut(&mut state, "traces").len(),
            "checkpoints": as_object_mut(&mut state, "checkpoints").len(),
            "intakes": as_object_mut(&mut state, "intakes").len(),
            "last_receipt": state.get("last_receipt").cloned().unwrap_or(Value::Null),
        })),
        "register-chain" => register_chain(&mut state, input),
        "execute-chain" => execute_chain(root, argv, &mut state, input),
        "register-middleware" => register_middleware(&mut state, input),
        "run-deep-agent" => run_deep_agent(root, argv, &mut state, input),
        "register-memory-bridge" => register_memory_bridge(root, &mut state, input),
        "recall-memory" => recall_memory(&mut state, input),
        "import-integration" => import_integration(root, &mut state, input),
        "route-prompt" => route_prompt(&mut state, input),
        "parse-structured-output" => parse_structured_output(&mut state, input),
        "record-trace" => record_trace(root, &mut state, input),
        "checkpoint-run" => checkpoint_run(root, argv, &mut state, input),
        "assimilate-intake" => assimilate_intake(root, &mut state, input),
        _ => Err(format!("unknown_langchain_bridge_command:{command}")),
    };

    match result {
        Ok(payload) => {
            let receipt = cli_receipt(
                &format!("langchain_bridge_{}", command.replace('-', "_")),
                payload,
            );
            state["last_receipt"] = receipt.clone();
            if let Err(err) = save_state(&state_path, &state)
                .and_then(|_| append_history(&history_path, &receipt))
            {
                print_json_line(&cli_error("langchain_bridge_error", &err));
                return 1;
            }
            print_json_line(&receipt);
            0
        }
        Err(err) => {
            print_json_line(&cli_error("langchain_bridge_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_prompt_renders_variables_and_respects_fallback() {
        let mut state = default_state();
        let route = route_prompt(
            &mut state,
            json!({
                "name": "support-template",
                "provider": "anthropic",
                "fallback_provider": "openai-compatible",
                "supported_providers": ["anthropic", "openai-compatible"],
                "profile": "pure",
                "template": "Hello {{name}}",
                "variables": {"name": "Jay"}
            })
            .as_object()
            .unwrap(),
        )
        .expect("route");
        assert_eq!(
            route["route"]["rendered_prompt"].as_str(),
            Some("Hello Jay")
        );
        assert_eq!(
            route["route"]["selected_provider"].as_str(),
            Some("openai-compatible")
        );
    }

    #[test]
    fn recall_memory_is_deterministic() {
        let mut state = default_state();
        let _ = register_memory_bridge(
            Path::new("."),
            &mut state,
            json!({
                "name": "memory",
                "documents": [
                    {"text": "billing policy doc", "metadata": {"kind": "policy"}},
                    {"text": "general faq", "metadata": {"kind": "faq"}}
                ]
            })
            .as_object()
            .unwrap(),
        )
        .expect("memory");
        let memory_id = state["memory_bridges"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .to_string();
        let out = recall_memory(
            &mut state,
            json!({"memory_id": memory_id, "query": "billing issue", "mode": "hybrid"})
                .as_object()
                .unwrap(),
        )
        .expect("recall");
        assert_eq!(
            out["recall"]["results"][0]["text"].as_str(),
            Some("billing policy doc")
        );
    }

    #[test]
    fn parse_structured_output_accepts_valid_payload() {
        let mut state = default_state();
        let out = parse_structured_output(
            &mut state,
            json!({
                "name": "incident-json",
                "schema": {
                    "required_fields": ["answer", "confidence"],
                    "field_types": {"answer": "string", "confidence": "number"}
                },
                "output_json": {"answer": "ok", "confidence": 0.91}
            })
            .as_object()
            .unwrap(),
        )
        .expect("parse");
        assert_eq!(
            out["structured_output"]["validated_output"]["answer"].as_str(),
            Some("ok")
        );
    }

    #[test]
    fn parse_structured_output_rejects_mismatched_type() {
        let mut state = default_state();
        let err = parse_structured_output(
            &mut state,
            json!({
                "schema": {
                    "required_fields": ["answer", "confidence"],
                    "field_types": {"answer": "string", "confidence": "number"}
                },
                "output_json": {"answer": "ok", "confidence": "high"}
            })
            .as_object()
            .unwrap(),
        )
        .expect_err("expected fail-closed validation error");
        assert!(err.contains("langchain_structured_output_validation_failed"));
    }
}
