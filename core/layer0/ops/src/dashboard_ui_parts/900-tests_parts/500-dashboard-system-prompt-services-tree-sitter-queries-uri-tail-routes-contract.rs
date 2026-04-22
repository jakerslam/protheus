#[test]
fn dashboard_system_prompt_services_tree_sitter_queries_uri_tail_routes_contract_wave_500() {
    let root = tempfile::tempdir().expect("tempdir");

    let java_query = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.java.describe",
        &json!({"query": "class_declaration"}),
    );
    assert!(java_query.ok);
    assert_eq!(
        java_query
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("class_declaration")
    );

    let javascript_query = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.javascript.describe",
        &json!({"query": "function_declaration"}),
    );
    assert!(javascript_query.ok);
    assert_eq!(
        javascript_query
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("function_declaration")
    );

    let kotlin_query = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.kotlin.describe",
        &json!({"query": "class_declaration"}),
    );
    assert!(kotlin_query.ok);
    assert_eq!(
        kotlin_query
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("class_declaration")
    );

    let php_query = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.php.describe",
        &json!({"query": "function_definition"}),
    );
    assert!(php_query.ok);
    assert_eq!(
        php_query
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("function_definition")
    );

    let python_query = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.python.describe",
        &json!({"query": "function_definition"}),
    );
    assert!(python_query.ok);
    assert_eq!(
        python_query
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("function_definition")
    );

    let ruby_query = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.ruby.describe",
        &json!({"query": "method"}),
    );
    assert!(ruby_query.ok);
    assert_eq!(
        ruby_query
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("method")
    );

    let rust_query = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.rust.describe",
        &json!({"query": "function_item"}),
    );
    assert!(rust_query.ok);
    assert_eq!(
        rust_query
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("function_item")
    );

    let swift_query = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.swift.describe",
        &json!({"query": "function_declaration"}),
    );
    assert!(swift_query.ok);
    assert_eq!(
        swift_query
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("function_declaration")
    );

    let typescript_query = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.typescript.describe",
        &json!({"query": "function_signature"}),
    );
    assert!(typescript_query.ok);
    assert_eq!(
        typescript_query
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("function_signature")
    );

    let shared_uri_handler = run_action(
        root.path(),
        "dashboard.prompts.system.services.uri.sharedUriHandler.describe",
        &json!({"uri": "file:///tmp/session.log"}),
    );
    assert!(shared_uri_handler.ok);
    assert_eq!(
        shared_uri_handler
            .payload
            .unwrap_or_else(|| json!({}))
            .get("uri")
            .and_then(Value::as_str),
        Some("file:///tmp/session.log")
    );
}
