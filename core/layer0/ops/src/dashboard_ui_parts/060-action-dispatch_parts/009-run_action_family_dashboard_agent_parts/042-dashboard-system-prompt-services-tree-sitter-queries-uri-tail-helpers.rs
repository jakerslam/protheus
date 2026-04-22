fn dashboard_prompt_services_tree_sitter_query_java_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("class_declaration"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_java_describe",
        "query": query
    })
}

fn dashboard_prompt_services_tree_sitter_query_javascript_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("function_declaration"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_javascript_describe",
        "query": query
    })
}

fn dashboard_prompt_services_tree_sitter_query_kotlin_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("class_declaration"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_kotlin_describe",
        "query": query
    })
}

fn dashboard_prompt_services_tree_sitter_query_php_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("function_definition"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_php_describe",
        "query": query
    })
}

fn dashboard_prompt_services_tree_sitter_query_python_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("function_definition"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_python_describe",
        "query": query
    })
}

fn dashboard_prompt_services_tree_sitter_query_ruby_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("method"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_ruby_describe",
        "query": query
    })
}

fn dashboard_prompt_services_tree_sitter_query_rust_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("function_item"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_rust_describe",
        "query": query
    })
}

fn dashboard_prompt_services_tree_sitter_query_swift_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("function_declaration"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_swift_describe",
        "query": query
    })
}

fn dashboard_prompt_services_tree_sitter_query_typescript_describe(payload: &Value) -> Value {
    let query = clean_text(
        payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("function_signature"),
        160,
    );
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_tree_sitter_query_typescript_describe",
        "query": query
    })
}

fn dashboard_prompt_services_uri_shared_uri_handler_describe(payload: &Value) -> Value {
    let uri = clean_text(payload.get("uri").and_then(Value::as_str).unwrap_or(""), 1400);
    json!({
        "ok": true,
        "type": "dashboard_prompts_system_services_uri_shared_uri_handler_describe",
        "uri": uri
    })
}

fn dashboard_prompt_services_tree_sitter_queries_tail2_route_extension(
    root: &Path,
    normalized: &str,
    payload: &Value,
) -> Option<Value> {
    match normalized {
        "dashboard.prompts.system.services.treeSitter.queries.java.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_java_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.javascript.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_javascript_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.kotlin.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_kotlin_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.php.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_php_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.python.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_python_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.ruby.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_ruby_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.rust.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_rust_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.swift.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_swift_describe(payload))
        }
        "dashboard.prompts.system.services.treeSitter.queries.typescript.describe" => {
            Some(dashboard_prompt_services_tree_sitter_query_typescript_describe(payload))
        }
        "dashboard.prompts.system.services.uri.sharedUriHandler.describe" => {
            Some(dashboard_prompt_services_uri_shared_uri_handler_describe(payload))
        }
        _ => dashboard_prompt_shared_settings_tail_route_extension(root, normalized, payload),
    }
}

include!("043-dashboard-system-prompt-shared-settings-messages-tail-helpers.rs");
