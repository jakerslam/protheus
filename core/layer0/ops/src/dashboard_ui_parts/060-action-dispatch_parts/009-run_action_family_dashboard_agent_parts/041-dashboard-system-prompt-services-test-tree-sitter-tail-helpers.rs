fn dashboard_prompt_services_test_git_helper_describe(payload: &Value) -> Value {
    let suite = clean_text(
        payload
            .get("suite")
            .and_then(Value::as_str)
            .unwrap_or("unit"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_test_git_helper_describe",
        "suite": suite
    })
}

fn dashboard_prompt_services_test_mode_describe(payload: &Value) -> Value {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("headless"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_test_mode_describe",
        "mode": mode
    })
}

fn dashboard_prompt_services_test_server_describe(payload: &Value) -> Value {
    let transport = clean_text(
        payload
            .get("transport")
            .and_then(Value::as_str)
            .unwrap_or("stdio"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_test_server_describe",
        "transport": transport
    })
}

fn dashboard_prompt_services_tree_sitter_index_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_index_describe",
        "exports": ["language_parser", "queries"]
    })
}

fn dashboard_prompt_services_tree_sitter_language_parser_describe(payload: &Value) -> Value {
    let language = clean_text(
        payload
            .get("language")
            .and_then(Value::as_str)
            .unwrap_or("typescript"),
        120,
    )
    .to_ascii_lowercase();
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_language_parser_describe",
        "language": language
    })
}

fn dashboard_prompt_services_tree_sitter_query_c_sharp_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("class_declaration"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_c_sharp_describe",
        "query": query
    })
}

fn dashboard_prompt_services_tree_sitter_query_c_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("function_definition"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_c_describe",
        "query": query
    })
}

fn dashboard_prompt_services_tree_sitter_query_cpp_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("namespace_definition"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_cpp_describe",
        "query": query
    })
}

fn dashboard_prompt_services_tree_sitter_query_go_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("function_declaration"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_go_describe",
        "query": query
    })
}

fn dashboard_prompt_services_tree_sitter_queries_index_describe() -> Value {
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_queries_index_describe",
        "exports": ["c_sharp", "c", "cpp", "go"]
    })
}

fn dashboard_prompt_services_test_tree_sitter_tail_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.services.test.gitHelper.describe" => {
            Some(dashboard_prompt_services_test_git_helper_describe(payload))
        }
        "dashboard.prompts.system.services.test.testMode.describe" => {
            Some(dashboard_prompt_services_test_mode_describe(payload))
        }
        "dashboard.prompts.system.services.test.testServer.describe" => {
            Some(dashboard_prompt_services_test_server_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.index.describe" => {
            Some(dashboard_prompt_services_tree_sitter_index_describe())
        }
        "dashboard.prompts.system.services.treeSitter.languageParser.describe" => {
            Some(dashboard_prompt_services_tree_sitter_language_parser_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.cSharp.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_c_sharp_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.c.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_c_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.cpp.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_cpp_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.go.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_go_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.index.describe" => {
            Some(dashboard_prompt_services_tree_sitter_queries_index_describe())
        }
        _ => dashboard_prompt_services_tree_sitter_queries_tail2_route_extension(root, normalized, payload),
    }
}

include!("042-dashboard-system-prompt-services-tree-sitter-queries-uri-tail-helpers.rs");
