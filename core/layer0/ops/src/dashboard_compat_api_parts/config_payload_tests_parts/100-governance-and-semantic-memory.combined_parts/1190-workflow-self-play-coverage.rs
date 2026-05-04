// SRS: V13-WORKFLOW-GATE-003

fn workflow_self_play_agent(root: &Path, snapshot: &Value, name: &str) -> String {
    crate::dashboard_provider_runtime::save_provider_key(root, "openai", "sk-test-openai");
    let created = handle(
        root,
        "POST",
        "/api/agents",
        format!(r#"{{"name":"{name}","role":"operator"}}"#).as_bytes(),
        snapshot,
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
    let set_model_payload =
        serde_json::to_vec(&json!({"model": "openai/gpt-5"})).expect("serialize model");
    let set_model = handle(
        root,
        "PUT",
        &format!("/api/agents/{agent_id}/model"),
        &set_model_payload,
        snapshot,
    )
    .expect("set model");
    assert_eq!(set_model.status, 200);
    agent_id
}

fn workflow_self_play_message(
    root: &Path,
    snapshot: &Value,
    agent_id: &str,
    message: &str,
) -> CompatApiResponse {
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
    let agent_id =
        workflow_self_play_agent(root.path(), &snapshot, "workflow-self-play-no-tools-agent");
    assert!(!agent_id.is_empty());

    let ghost = "GHOST NO-TOOLS SECOND PASS SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {"response": "Respond directly"},
                {"response": "Hey, hello from the direct no-tool self-play path."},
                {"response": ghost}
            ],
            "calls": []
        }),
    );

    let response = workflow_self_play_message(root.path(), &snapshot, &agent_id, "hey");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("Hey, hello from the direct no-tool self-play path.")
    );
    assert!(
        response.payload.get("pending_tool_request").is_none(),
        "{}",
        response.payload
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
            .pointer("/workflow_visibility/workflow_trace/gate_id")
            .and_then(Value::as_str),
        Some("gate_1_work_category_menu")
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/workflow_trace/input_kind")
            .and_then(Value::as_str),
        Some("multiple_choice")
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/workflow_trace/selected_option")
            .and_then(Value::as_str),
        Some("Respond directly")
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/workflow_trace/final_authority")
            .and_then(Value::as_str),
        Some("llm_only")
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
                    "response": "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"source\":\"web\",\"query\":\"agent framework current comparison\",\"aperture\":\"medium\"}."
                },
                {
                    "response": "Compared with current agent frameworks, Infring should be judged on orchestration depth, tool reliability, recovery behavior, and synthesis quality."
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
        Some("web_search"),
        "{}",
        choose_tool.payload
    );
    assert_eq!(
        choose_tool.payload.get("response").and_then(Value::as_str),
        Some("")
    );
    assert_eq!(
        choose_tool
            .payload
            .pointer("/workflow_visibility/workflow_trace/tool_name")
            .and_then(Value::as_str),
        Some("web_search")
    );
    assert_eq!(
        choose_tool
            .payload
            .pointer("/workflow_visibility/workflow_trace/confirmation_state")
            .and_then(Value::as_str),
        Some("pending_confirmation")
    );
    assert_workflow_self_play_clean(&choose_tool.payload, ghost);
    let confirmed = workflow_self_play_message(root.path(), &snapshot, &agent_id, "yes");
    assert!(
        confirmed
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("Infring should be judged"),
        "{}",
        confirmed.payload
    );
    assert_eq!(
        confirmed
            .payload
            .pointer("/tools/0/name")
            .and_then(Value::as_str),
        Some("web_search")
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
    let agent_id =
        workflow_self_play_agent(root.path(), &snapshot, "workflow-self-play-failure-agent");
    assert!(!agent_id.is_empty());

    let ghost = "GHOST FAILURE THIRD PASS SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Category: Workspace/files. Tool family: Workspace/files. Tool: read_file. Request payload: {\"path\":\"missing/self_play.txt\"}."
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

    let choose_tool = workflow_self_play_message(
        root.path(),
        &snapshot,
        &agent_id,
        "Read the missing file if needed.",
    );
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
    let agent_id =
        workflow_self_play_agent(root.path(), &snapshot, "workflow-self-play-cancel-agent");
    assert!(!agent_id.is_empty());

    let ghost = "GHOST CANCEL THIRD PASS SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Category: Workspace/files. Tool family: Workspace/files. Tool: read_file. Request payload: {\"path\":\"notes/cancel.txt\"}."
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

    let choose_tool =
        workflow_self_play_message(root.path(), &snapshot, &agent_id, "Prepare to read a file.");
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
    assert!(
        cancelled.payload.get("pending_tool_request").is_none(),
        "{}",
        cancelled.payload
    );
    let tool_script =
        read_json(&governance_test_tool_script_path(root.path())).expect("tool script");
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
    let agent_id =
        workflow_self_play_agent(root.path(), &snapshot, "workflow-self-play-loopback-agent");
    assert!(!agent_id.is_empty());

    let ghost = "GHOST LOOPBACK FIFTH PASS SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "Category: Workspace/files. Tool family: Workspace/files. Tool: read_file. Request payload: {\"path\":\"notes/loop.txt\"}."
                },
                {
                    "response": "The file result is in hand. I can run another tool if useful."
                },
                {
                    "response": "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"source\":\"web\",\"query\":\"loopback workflow validation\",\"aperture\":\"small\"}."
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

    let choose_file = workflow_self_play_message(
        root.path(),
        &snapshot,
        &agent_id,
        "Use the local file first.",
    );
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

    let choose_web =
        workflow_self_play_message(root.path(), &snapshot, &agent_id, "Run another tool.");
    assert_eq!(
        choose_web
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("web_search"),
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
        Some("web_search")
    );
    assert_workflow_self_play_clean(&web_done.payload, ghost);
}

#[test]
fn workflow_scripted_agent_self_play_flags_prompt_analysis_leak_without_runtime_rewrite() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let agent_id =
        workflow_self_play_agent(root.path(), &snapshot, "workflow-self-play-leak-guard-agent");
    assert!(!agent_id.is_empty());

    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [{
                "response": "Respond directly"
            }, {
                "response": "We are in the runtime context of 2026-05-02T06:14:40Z. The user asks for a reply in exactly five words. We must reply in one short sentence."
            }],
            "calls": []
        }),
    );

    let response = workflow_self_play_message(
        root.path(),
        &snapshot,
        &agent_id,
        "hey, reply in five words, no tools",
    );
    assert_eq!(response.status, 200);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(
        response_text,
        "We are in the runtime context of 2026-05-02T06:14:40Z. The user asks for a reply in exactly five words. We must reply in one short sentence.",
        "{}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/diagnostic_reject_reason")
            .and_then(Value::as_str),
        Some("workflow_prompt_analysis_leak"),
        "{}",
        response.payload
    );
    assert_workflow_self_play_clean(
        &response.payload,
        "GHOST PROMPT ANALYSIS SHOULD NEVER APPEAR",
    );
}

#[test]
fn workflow_scripted_agent_self_play_exact_web_gate_creates_pending_tool() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let agent_id = workflow_self_play_agent(
        root.path(),
        &snapshot,
        "workflow-self-play-exact-web-gate-agent",
    );
    assert!(!agent_id.is_empty());

    let ghost = "GHOST NATURAL WEB CHOICE SHOULD NEVER APPEAR";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {"response": "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"source\":\"web\",\"query\":\"one current OpenHands source\",\"aperture\":\"medium\"}."},
                {"response": "Category: Web research. Tool family: Web research. Tool: web_search. Request payload: {\"source\":\"web\",\"query\":\"one current OpenHands source\",\"aperture\":\"medium\"}."},
                {"response": ghost}
            ],
            "calls": []
        }),
    );

    let response = workflow_self_play_message(
        root.path(),
        &snapshot,
        &agent_id,
        "Use web search to find one current sentence about OpenHands, then answer in one sentence.",
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/tool_name")
            .and_then(Value::as_str),
        Some("web_search"),
        "{}",
        response.payload
    );
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("")
    );
    assert_eq!(
        response
            .payload
            .pointer("/pending_tool_request/input/query")
            .and_then(Value::as_str),
        Some("one current OpenHands source"),
        "{}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/workflow_trace/confirmation_state")
            .and_then(Value::as_str),
        Some("pending_confirmation")
    );
    assert_workflow_self_play_clean(&response.payload, ghost);
}
