#[test]
fn workflow_web_tooling_context_soak_32_turns_reports_zero_terminal_failures() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-web-context-soak-agent","role":"researcher"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());

    let mut taxonomy = json!({
        "turns": 32,
        "empty_final": 0,
        "deferred_final": 0,
        "placeholder_final": 0,
        "off_topic_final": 0,
        "meta_status_tool_leak": 0,
        "web_missing_tool_attempt": 0
    });

    for turn in 0..32usize {
        let mode = turn % 4;
        let message = if mode == 0 {
            "that was just a test".to_string()
        } else if mode == 3 {
            "did you do the web request?".to_string()
        } else {
            format!("search the web for current top ai agent frameworks turn {turn}")
        };
        let (chat_queue, tool_queue) = if mode == 0 {
            (
                vec![
                    json!({"response": "Acknowledged. This is a test-only turn with no web call."}),
                    json!({"response": "Acknowledged. This is a test-only turn with no web call."}),
                ],
                Vec::<Value>::new(),
            )
        } else if mode == 3 {
            (
                vec![
                    json!({"response": "Status: the prior web run completed; no new query execution in this status-check turn."}),
                    json!({"response": "Status: the prior web run completed; no new query execution in this status-check turn."}),
                ],
                Vec::<Value>::new(),
            )
        } else {
            let query = format!("top ai agent frameworks turn {turn}");
            let second = if turn % 8 == 1 {
                "I'll get you an update on that web search."
            } else {
                "I can retry with a narrower query if you'd like."
            };
            let third = if turn % 8 == 1 {
                "Live retrieval was low-signal in this pass, but the run completed with a recorded failure classification."
            } else {
                "Key findings: LangGraph and OpenAI Agents SDK remained visible in this pass."
            };
            let payload = if turn % 8 == 1 {
                json!({
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "no_results",
                        "summary": "Web retrieval ran, but low-signal snippets prevented synthesis in this pass."
                    }
                })
            } else {
                json!({
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "Key findings: LangGraph and OpenAI Agents SDK remained visible in this pass."
                    }
                })
            };
            (
                vec![
                    json!({"response": format!("<function=batch_query>{{\"source\":\"web\",\"query\":\"{}\",\"aperture\":\"medium\"}}</function>", query)}),
                    json!({"response": second}),
                    json!({"response": third}),
                ],
                vec![payload],
            )
        };

        write_json(
            &governance_test_chat_script_path(root.path()),
            &json!({
                "queue": chat_queue,
                "calls": []
            }),
        );
        write_json(
            &governance_test_tool_script_path(root.path()),
            &json!({
                "queue": tool_queue,
                "calls": []
            }),
        );

