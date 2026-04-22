#[test]
fn dashboard_system_prompt_services_test_tree_sitter_tail_routes_contract_wave_490() {
    let root = tempfile::tempdir().expect("tempdir");

    let test_git_helper = run_action(
        root.path(),
        "dashboard.prompts.system.services.test.gitHelper.describe",
        &json!({"suite": "integration"}),
    );
    assert!(test_git_helper.ok);
    assert_eq!(
        test_git_helper
            .payload
            .unwrap_or_else(|| json!({}))
            .get("suite")
            .and_then(Value::as_str),
        Some("integration")
    );

    let test_mode = run_action(
        root.path(),
        "dashboard.prompts.system.services.test.testMode.describe",
        &json!({"mode": "headless"}),
    );
    assert!(test_mode.ok);
    assert_eq!(
        test_mode
            .payload
            .unwrap_or_else(|| json!({}))
            .get("mode")
            .and_then(Value::as_str),
        Some("headless")
    );

    let test_server = run_action(
        root.path(),
        "dashboard.prompts.system.services.test.testServer.describe",
        &json!({"transport": "stdio"}),
    );
    assert!(test_server.ok);
    assert_eq!(
        test_server
            .payload
            .unwrap_or_else(|| json!({}))
            .get("transport")
            .and_then(Value::as_str),
        Some("stdio")
    );

    let tree_sitter_index = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.index.describe",
        &json!({}),
    );
    assert!(tree_sitter_index.ok);
    assert_eq!(
        tree_sitter_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_services_tree_sitter_index_describe")
    );

    let language_parser = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.languageParser.describe",
        &json!({"language": "typescript"}),
    );
    assert!(language_parser.ok);
    assert_eq!(
        language_parser
            .payload
            .unwrap_or_else(|| json!({}))
            .get("language")
            .and_then(Value::as_str),
        Some("typescript")
    );

    let query_c_sharp = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.cSharp.describe",
        &json!({"query": "class_declaration"}),
    );
    assert!(query_c_sharp.ok);
    assert_eq!(
        query_c_sharp
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("class_declaration")
    );

    let query_c = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.c.describe",
        &json!({"query": "function_definition"}),
    );
    assert!(query_c.ok);
    assert_eq!(
        query_c
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("function_definition")
    );

    let query_cpp = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.cpp.describe",
        &json!({"query": "namespace_definition"}),
    );
    assert!(query_cpp.ok);
    assert_eq!(
        query_cpp
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("namespace_definition")
    );

    let query_go = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.go.describe",
        &json!({"query": "function_declaration"}),
    );
    assert!(query_go.ok);
    assert_eq!(
        query_go
            .payload
            .unwrap_or_else(|| json!({}))
            .get("query")
            .and_then(Value::as_str),
        Some("function_declaration")
    );

    let queries_index = run_action(
        root.path(),
        "dashboard.prompts.system.services.treeSitter.queries.index.describe",
        &json!({}),
    );
    assert!(queries_index.ok);
    assert_eq!(
        queries_index
            .payload
            .unwrap_or_else(|| json!({}))
            .get("type")
            .and_then(Value::as_str),
        Some("dashboard_prompts_system_services_tree_sitter_queries_index_describe")
    );
}
