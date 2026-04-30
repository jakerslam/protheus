// SRS: V13-WORKFLOW-GATE-003

fn workflow_self_play_agent(root: &Path, snapshot: &Value, name: &str) -> String {
    let created = handle(
        root,
        "POST",
        "/api/agents",
        format!(r#"{{"name":"{name}","role":"operator"}}"#).as_bytes(),
        snapshot,
    )
    .expect("agent create");
    clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    )
}

fn workflow_self_play_message(root: &Path, snapshot: &Value, agent_id: &str, message: &str) -> CompatApiResponse {
    handle(
        root,
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        serde_json::to_vec(&json!({"message": message}))
            .expect("message json")
            .as_slice(),
        snapshot,
    )
    .expect("message response")
}

fn assert_workflow_self_play_clean(payload: &Value, ghost: &str) {
    assert_eq!(
        payload
            .get("system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false),
        "{payload}"
    );
    assert_eq!(
        payload
            .pointer("/response_finalization/workflow_system_fallback_used")
            .and_then(Value::as_bool),
        Some(false),
        "{payload}"
    );
    assert_payload_has_no_ghost_interference(payload, ghost);
}

#[test]
fn workflow_scripted_agent_self_play_can_exit_no_tools_directly() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let agent_id = workflow_self_play_agent(root.path(), &snapshot, "workflow-self-play-no-tools-agent");
    assert!(!agent_id.is_empty());

    let ghost = "GHOST NO-TOOLS SECOND PASS SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {"response": "No. Hello from the direct no-tool self-play path."},
                {"response": ghost}
            ],
            "calls": []
        }),
    );

    let response = workflow_self_play_message(root.path(), &snapshot, &agent_id, "hey");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("Hello from the direct no-tool self-play path.")
    );
    assert!(response.payload.get("pending_tool_request").is_none(), "{}", response.payload);
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("skipped_not_required")
    );
    assert_workflow_self_play_clean(&response.payload, ghost);
}

#[test]
fn workflow_scripted_agent_self_play_can_choose_web_confirm_and_synthesize() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let agent_id = workflow_self_play_agent(root.path(), &snapshot, "workflow-self-play-web-agent");
    assert!(!agent_id.is_empty());

    let ghost = "GHOST WEB THIRD PASS SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Yes. Tool family: Web Search. Tool: Web search. Request payload: {\"source\":\"web\",\"query\":\"agent framework current comparison\",\"aperture\":\"medium\"}."
                },
                {
                    "response": "The web result is usable: the scripted search evidence names current agent framework comparison points."
                },
                {"response": ghost}
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [{
                "tool": "batch_query",
                "payload": {
                    "ok": true,
                    "status": "ok",
                    "summary": "Scripted web evidence names current agent framework comparison points."
                }
            }],
            "calls": []
        }),
    );

    let choose_tool = workflow_self_play_message(
        root.path(),
        &snapshot,
        &agent_id,
        "Use the workflow as yourself and search if needed.",
    );
    assert_eq!(
        choose_tool
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("batch_query"),
        "{}",
        choose_tool.payload
    );
    assert_eq!(choose_tool.payload.get("response").and_then(Value::as_str), Some(""));
    assert_workflow_self_play_clean(&choose_tool.payload, ghost);

    let confirmed = workflow_self_play_message(root.path(), &snapshot, &agent_id, "yes");
    assert!(
        confirmed
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("web result is usable"),
        "{}",
        confirmed.payload
    );
    assert_eq!(
        confirmed
            .payload
            .pointer("/tools/0/name")
            .and_then(Value::as_str),
        Some("batch_query")
    );
    assert_eq!(
        confirmed
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
    assert_workflow_self_play_clean(&confirmed.payload, ghost);
}

#[test]
fn workflow_scripted_agent_self_play_surfaces_failed_tool_for_llm_recovery() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let agent_id = workflow_self_play_agent(root.path(), &snapshot, "workflow-self-play-failure-agent");
    assert!(!agent_id.is_empty());

    let ghost = "GHOST FAILURE THIRD PASS SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Yes. Tool family: File / Workspace. Tool: Read file. Request payload: {\"path\":\"missing/self_play.txt\"}."
                },
                {
                    "response": "The file tool failed cleanly, so I need a corrected path before I can synthesize that file."
                },
                {"response": ghost}
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [{
                "tool": "file_read",
                "payload": {
                    "ok": false,
                    "status": "error",
                    "error": "file_not_found",
                    "summary": "missing/self_play.txt was not found."
                }
            }],
            "calls": []
        }),
    );

    let choose_tool = workflow_self_play_message(root.path(), &snapshot, &agent_id, "Read the missing file if needed.");
    assert_eq!(
        choose_tool
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("file_read"),
        "{}",
        choose_tool.payload
    );

    let confirmed = workflow_self_play_message(root.path(), &snapshot, &agent_id, "yes");
    let response_text = confirmed
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(response_text.contains("failed cleanly"), "{response_text}");
    assert_eq!(
        confirmed
            .payload
            .pointer("/tools/0/status")
            .and_then(Value::as_str),
        Some("error")
    );
    assert_eq!(
        confirmed
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );
    assert_workflow_self_play_clean(&confirmed.payload, ghost);
}

#[test]
fn workflow_scripted_agent_self_play_can_cancel_pending_tool_without_execution() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let agent_id = workflow_self_play_agent(root.path(), &snapshot, "workflow-self-play-cancel-agent");
    assert!(!agent_id.is_empty());

    let ghost = "GHOST CANCEL THIRD PASS SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Yes. Tool family: File / Workspace. Tool: Read file. Request payload: {\"path\":\"notes/cancel.txt\"}."
                },
                {"response": "Cancelled. I will not run that file tool."},
                {"response": ghost}
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({"queue": [], "calls": []}),
    );

    let choose_tool = workflow_self_play_message(root.path(), &snapshot, &agent_id, "Prepare to read a file.");
    assert_eq!(
        choose_tool
            .payload
            .pointer("/pending_tool_request/status")
            .and_then(Value::as_str),
        Some("pending_confirmation")
    );

    let cancelled = workflow_self_play_message(root.path(), &snapshot, &agent_id, "cancel");
    assert_eq!(
        cancelled.payload.get("response").and_then(Value::as_str),
        Some("Cancelled. I will not run that file tool."),
        "{}",
        cancelled.payload
    );
    assert!(cancelled.payload.get("pending_tool_request").is_none(), "{}", cancelled.payload);
    let tool_script = read_json(&governance_test_tool_script_path(root.path())).expect("tool script");
    assert_eq!(
        tool_script
            .get("calls")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0),
        "{tool_script}"
    );
    assert_workflow_self_play_clean(&cancelled.payload, ghost);
}

#[test]
fn workflow_scripted_agent_self_play_can_loop_back_for_another_tool() {
    let root = governance_temp_root();
    init_git_repo(root.path());
    std::fs::create_dir_all(root.path().join("notes")).expect("mkdir");
    std::fs::write(root.path().join("notes/loop.txt"), "LOOPBACK_FILE_OK").expect("fixture");

    let snapshot = governance_ok_snapshot();
    let agent_id = workflow_self_play_agent(root.path(), &snapshot, "workflow-self-play-loopback-agent");
    assert!(!agent_id.is_empty());

    let ghost = "GHOST LOOPBACK FIFTH PASS SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Yes. Tool family: File / Workspace. Tool: Read file. Request payload: {\"path\":\"notes/loop.txt\"}."
                },
                {
                    "response": "The file result is in hand. I can run another tool if useful."
                },
                {
                    "response": "Yes. Tool family: Web Search. Tool: Web search. Request payload: {\"source\":\"web\",\"query\":\"loopback workflow validation\",\"aperture\":\"small\"}."
                },
                {
                    "response": "Loopback complete: I used the file result first and then the scripted web result."
                },
                {"response": ghost}
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [{
                "tool": "batch_query",
                "payload": {
                    "ok": true,
                    "status": "ok",
                    "summary": "Scripted web loopback evidence."
                }
            }],
            "calls": []
        }),
    );

    let choose_file = workflow_self_play_message(root.path(), &snapshot, &agent_id, "Start with the local file.");
    assert_eq!(
        choose_file
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("file_read")
    );
    let file_done = workflow_self_play_message(root.path(), &snapshot, &agent_id, "yes");
    assert!(
        file_done
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("file result is in hand"),
        "{}",
        file_done.payload
    );

    let choose_web = workflow_self_play_message(root.path(), &snapshot, &agent_id, "Run another tool.");
    assert_eq!(
        choose_web
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("batch_query"),
        "{}",
        choose_web.payload
    );
    let web_done = workflow_self_play_message(root.path(), &snapshot, &agent_id, "yes");
    assert!(
        web_done
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("Loopback complete"),
        "{}",
        web_done.payload
    );
    assert_eq!(
        web_done
            .payload
            .pointer("/tools/0/name")
            .and_then(Value::as_str),
        Some("batch_query")
    );
    assert_workflow_self_play_clean(&web_done.payload, ghost);
}
