fn state_object_len(state: &mut Value, key: &str) -> usize {
    as_object_mut(state, key).len()
}

fn state_array_len(state: &mut Value, key: &str) -> usize {
    as_array_mut(state, key).len()
}

fn status_payload(root: &Path, state_path: &Path, history_path: &Path, state: &mut Value) -> Value {
    json!({
        "ok": true,
        "state_path": rel(root, state_path),
        "history_path": rel(root, history_path),
        "pipelines": state_object_len(state, "pipelines"),
        "pipeline_runs": state_object_len(state, "pipeline_runs"),
        "agent_runs": state_object_len(state, "agent_runs"),
        "templates": state_object_len(state, "templates"),
        "template_renders": state_object_len(state, "template_renders"),
        "document_stores": state_object_len(state, "document_stores"),
        "retrieval_runs": state_object_len(state, "retrieval_runs"),
        "routes": state_object_len(state, "routes"),
        "evaluations": state_object_len(state, "evaluations"),
        "traces": state_array_len(state, "traces"),
        "connectors": state_object_len(state, "connectors"),
        "intakes": state_object_len(state, "intakes"),
        "last_receipt": state.get("last_receipt").cloned().unwrap_or(Value::Null),
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].as_str();
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("haystack_bridge_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let state_path = state_path(root, argv, input);
    let history_path = history_path(root, argv, input);
    let mut state = load_state(&state_path);

    let result = match command {
        "status" => Ok(status_payload(root, &state_path, &history_path, &mut state)),
        "register-pipeline" => register_pipeline(&mut state, input),
        "run-pipeline" => run_pipeline(root, argv, &mut state, input),
        "run-agent-toolset" => run_agent_toolset(root, argv, &mut state, input),
        "register-template" => register_template(&mut state, input),
        "render-template" => render_template(&mut state, input),
        "register-document-store" => register_document_store(root, &mut state, input),
        "retrieve-documents" => retrieve_documents(&mut state, input),
        "route-and-rank" => route_and_rank(&mut state, input),
        "record-multimodal-eval" => record_multimodal_eval(root, &mut state, input),
        "trace-run" => trace_run(root, &mut state, input),
        "import-connector" => import_connector(root, &mut state, input),
        "assimilate-intake" => assimilate_intake(root, &mut state, input),
        _ => Err(format!("unknown_haystack_bridge_command:{command}")),
    };

    match result {
        Ok(payload) => {
            let receipt = cli_receipt(
                &format!("haystack_bridge_{}", command.replace('-', "_")),
                payload,
            );
            state["last_receipt"] = receipt.clone();
            if let Err(err) = save_state(&state_path, &state)
                .and_then(|_| append_history(&history_path, &receipt))
            {
                print_json_line(&cli_error("haystack_bridge_error", &err));
                return 1;
            }
            print_json_line(&receipt);
            0
        }
        Err(err) => {
            print_json_line(&cli_error("haystack_bridge_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_render_replaces_variables() {
        let mut state = default_state();
        let payload = json!({"name": "support-template", "template": "Hello {{name}}"});
        let _ = register_template(&mut state, payload.as_object().unwrap()).expect("template");
        let template_id = state["templates"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .to_string();
        let render = render_template(
            &mut state,
            json!({"template_id": template_id, "variables": {"name": "Jay"}})
                .as_object()
                .unwrap(),
        )
        .expect("render");
        assert_eq!(render["render"]["output"].as_str(), Some("Hello Jay"));
    }

    #[test]
    fn template_render_supports_percent_tokens_and_rich_variable_values() {
        let mut state = default_state();
        let payload = json!({"name": "support-template", "template": "Hello %name%"});
        let _ = register_template(&mut state, payload.as_object().unwrap()).expect("template");
        let template_id = state["templates"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .to_string();
        let render = render_template(
            &mut state,
            json!({
                "template_id": template_id,
                "variables": {"name": {"value": "Jay", "description": "operator alias"}}
            })
            .as_object()
            .unwrap(),
        )
        .expect("render");
        assert_eq!(render["render"]["output"].as_str(), Some("Hello Jay"));
    }

    #[test]
    fn template_render_supports_text_and_content_variable_aliases_without_json_leak() {
        let mut state = default_state();
        let payload = json!({
            "name": "support-template",
            "template": "Hello %name% from {{city}}"
        });
        let _ = register_template(&mut state, payload.as_object().unwrap()).expect("template");
        let template_id = state["templates"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .to_string();
        let render = render_template(
            &mut state,
            json!({
                "template_id": template_id,
                "variables": {
                    "name": {"text": "Jay", "description": "operator alias"},
                    "city": {"content": "Denver", "meta": {"tz": "MST"}}
                }
            })
            .as_object()
            .unwrap(),
        )
        .expect("render");
        let output = render["render"]["output"].as_str().unwrap_or("");
        assert_eq!(output, "Hello Jay from Denver");
        assert!(!output.contains('{'));
    }

    #[test]
    fn template_render_supports_nested_rich_variable_aliases_without_json_leak() {
        let mut state = default_state();
        let payload = json!({
            "name": "support-template",
            "template": "Hello %name% from {{city}}"
        });
        let _ = register_template(&mut state, payload.as_object().unwrap()).expect("template");
        let template_id = state["templates"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .to_string();
        let render = render_template(
            &mut state,
            json!({
                "template_id": template_id,
                "variables": {
                    "name": {"value": {"text": "Jay"}},
                    "city": {"content": {"value": "Denver"}},
                    "ignored": {"value": {"meta": {"tz": "MST"}}}
                }
            })
            .as_object()
            .unwrap(),
        )
        .expect("render");
        let output = render["render"]["output"].as_str().unwrap_or("");
        assert_eq!(output, "Hello Jay from Denver");
        assert!(!output.contains('{'));
        assert!(!output.contains("meta"));
    }

    #[test]
    fn route_and_rank_is_deterministic() {
        let mut state = default_state();
        let out = route_and_rank(&mut state, json!({
            "name": "router",
            "query": "billing issue",
            "context": {"intent": "billing"},
            "routes": [
                {"id": "billing", "field": "intent", "equals": "billing", "reason": "billing path"},
                {"id": "general", "field": "intent", "equals": "general", "reason": "general path"}
            ],
            "candidates": [
                {"text": "billing policy doc", "metadata": {"kind": "policy"}},
                {"text": "general faq", "metadata": {"kind": "faq"}}
            ]
        }).as_object().unwrap()).expect("route");
        assert_eq!(
            out["route"]["selected_route"]["id"].as_str(),
            Some("billing")
        );
    }
}
