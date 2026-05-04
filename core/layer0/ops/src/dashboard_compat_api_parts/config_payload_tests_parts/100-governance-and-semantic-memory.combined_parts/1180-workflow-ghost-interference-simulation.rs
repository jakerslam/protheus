// SRS: V13-WORKFLOW-GATE-003

fn assert_payload_has_no_ghost_interference(payload: &Value, sentinel: &str) {
    let rendered = payload.to_string();
    let banned = [
        ["visible_response_", "recovery_model"].concat(),
        ["forced_live_web_", "invariant"].concat(),
        ["draft_retry_", "signal"].concat(),
        ["final_response_guard_", "recovered_by_llm"].concat(),
        ["empty_final_response_", "menu_recovery"].concat(),
        ["response_", "menu_recovery_model"].concat(),
        sentinel.to_owned(),
    ];
    for phrase in banned {
        assert!(
            !rendered.contains(&phrase),
            "ghost workflow phrase leaked: {phrase}\npayload: {rendered}"
        );
    }
}

#[test]
fn workflow_scripted_agent_simulation_has_no_ghost_second_pass_for_direct_chat() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-ghost-direct-chat-agent","role":"assistant"}"#,
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

    let ghost = "GHOST SECOND PASS SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {"response": "Respond directly"},
                {"response": "No tools needed. I can answer directly from here."},
                {"response": ghost}
            ],
            "calls": []
        }),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"hey"}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("No tools needed. I can answer directly from here.")
    );
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_payload_has_no_ghost_interference(&response.payload, ghost);

    let script = read_json(&governance_test_chat_script_path(root.path())).expect("script");
    let calls = script.get("calls").and_then(Value::as_array).unwrap();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(
        script
            .pointer("/queue/0/response")
            .and_then(Value::as_str),
        Some(ghost)
    );
}

#[test]
fn workflow_scripted_agent_simulation_has_no_ghost_retry_for_tool_gate_choice() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-ghost-tool-gate-agent","role":"researcher"}"#,
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

    let ghost = "GHOST TOOL RECOVERY SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "\"workflow_gate\": 3}"
                },
                {
                    "response": "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"source\":\"web\",\"query\":\"latest agent frameworks\",\"aperture\":\"medium\"}."
                },
                {"response": ghost}
            ],
            "calls": []
        }),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Use your workflow gate yourself for a current agent-framework comparison. If you need tools, choose the tool and payload only."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(response.payload.get("response").and_then(Value::as_str), Some(""));
    assert_eq!(
        response
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/status")
            .and_then(Value::as_str),
        Some("pending_confirmation")
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("web_search")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/pending_tool_request/status")
            .and_then(Value::as_str),
        Some("pending_confirmation")
    );
    assert_eq!(
        response
            .payload
            .pointer("/live_eval_monitor/issue_count")
            .and_then(Value::as_u64),
        Some(0),
        "{}",
        response.payload
    );
    assert_payload_has_no_ghost_interference(&response.payload, ghost);

    let script = read_json(&governance_test_chat_script_path(root.path())).expect("script");
    let calls = script.get("calls").and_then(Value::as_array).unwrap();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(
        script
            .pointer("/queue/0/response")
            .and_then(Value::as_str),
        Some(ghost)
    );
}

#[test]
fn workflow_prompt_markup_draft_still_enters_private_tool_gate() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-markup-draft-gate-agent","role":"researcher","model":"kimi-k2.6:cloud","provider":"ollama"}"#,
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

    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "provider": "ollama",
                    "runtime_model": "kimi-k2.6:cloud",
                    "response": "I'll research this now. <tool>web_search</tool><query>agentic AI frameworks April 2026 comparison</query>"
                },
                {
                    "provider": "ollama",
                    "runtime_model": "kimi-k2.6:cloud",
                    "response": "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"source\":\"web\",\"query\":\"agentic AI frameworks April 2026 comparison\",\"aperture\":\"medium\"}."
                }
            ],
            "calls": []
        }),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Compare Infring to top agentic frameworks in April 2026. If you need current information, use web research through the workflow."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(response.payload.get("response").and_then(Value::as_str), Some(""));
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/status")
            .and_then(Value::as_str),
        Some("pending_confirmation")
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("web_search")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("skipped_pending_tool_confirmation")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );

    let script = read_json(&governance_test_chat_script_path(root.path())).expect("script");
    let calls = script.get("calls").and_then(Value::as_array).unwrap();
    assert_eq!(calls.len(), 2, "{calls:?}");
}

#[test]
fn workflow_scripted_agent_self_play_can_choose_confirm_and_synthesize_tool_result() {
    let root = governance_temp_root();
    init_git_repo(root.path());
    std::fs::create_dir_all(root.path().join("notes")).expect("mkdir");
    std::fs::write(
        root.path().join("notes/self_play.txt"),
        "SELF_PLAY_OK: the agent selected file_read, waited for confirmation, and synthesized the result.",
    )
    .expect("write fixture");

    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"workflow-self-play-agent","role":"operator"}"#,
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

    let ghost = "GHOST SELF-PLAY THIRD PASS SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Category: Workspace/files. Tool family: Workspace/files. Tool: read_file. Request payload: {\"path\":\"notes/self_play.txt\"}."
                },
                {
                    "response": "SELF_PLAY_OK confirmed. I read notes/self_play.txt and can answer from the recorded file result."
                },
                {"response": ghost}
            ],
            "calls": []
        }),
    );

    let choose_tool = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Play the workflow as yourself. If you need a tool, choose the exact tool and payload; otherwise answer directly."}"#,
        &snapshot,
    )
    .expect("tool choice response");
    assert_eq!(choose_tool.status, 200);
    assert_eq!(choose_tool.payload.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(choose_tool.payload.get("response").and_then(Value::as_str), Some(""));
    assert_eq!(
        choose_tool
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("file_read")
    );
    assert_eq!(
        choose_tool
            .payload
            .pointer("/pending_tool_request/status")
            .and_then(Value::as_str),
        Some("pending_confirmation")
    );
    assert_eq!(
        choose_tool
            .payload
            .pointer("/live_eval_monitor/issue_count")
            .and_then(Value::as_u64),
        Some(0),
        "{}",
        choose_tool.payload
    );
    assert_payload_has_no_ghost_interference(&choose_tool.payload, ghost);

    let confirm_tool = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"yes"}"#,
        &snapshot,
    )
    .expect("confirmed tool response");
    assert_eq!(confirm_tool.status, 200);
    assert_eq!(
        confirm_tool.payload.get("ok").and_then(Value::as_bool),
        Some(true),
        "{}",
        confirm_tool.payload
    );
    let final_response = confirm_tool
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        final_response.contains("SELF_PLAY_OK confirmed"),
        "{final_response}"
    );
    assert_eq!(
        confirm_tool
            .payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        confirm_tool
            .payload
            .pointer("/response_finalization/pending_confirmation_replayed")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        confirm_tool
            .payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        confirm_tool
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
    assert!(
        confirm_tool
            .payload
            .get("pending_tool_request")
            .is_none(),
        "{}",
        confirm_tool.payload
    );
    assert_payload_has_no_ghost_interference(&confirm_tool.payload, ghost);

    let script = read_json(&governance_test_chat_script_path(root.path())).expect("script");
    let calls = script.get("calls").and_then(Value::as_array).unwrap();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(
        script
            .pointer("/queue/0/response")
            .and_then(Value::as_str),
        Some(ghost)
    );
}
