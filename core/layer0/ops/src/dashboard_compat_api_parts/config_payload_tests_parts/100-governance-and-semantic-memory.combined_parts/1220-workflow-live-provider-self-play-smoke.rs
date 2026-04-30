#[test]
fn workflow_live_provider_self_play_smoke_when_enabled() {
    if !truthy_test_env("INFRING_LIVE_WORKFLOW_SELF_PLAY_SMOKE") {
        return;
    }
    let _env_lock = GOVERNANCE_LIVE_WEB_ENV_MUTEX.lock().expect("lock");
    let model_ref = std::env::var("INFRING_LIVE_WORKFLOW_SELF_PLAY_MODEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "openai/gpt-5".to_string());
    if model_ref.starts_with("openai/")
        && std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .is_none()
    {
        return;
    }

    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let agent_id = workflow_self_play_agent(
        root.path(),
        &snapshot,
        "workflow-live-provider-self-play-agent",
    );
    let set_model_payload = serde_json::to_vec(&json!({"model": model_ref})).expect("model json");
    let set_model = handle(
        root.path(),
        "PUT",
        &format!("/api/agents/{agent_id}/model"),
        &set_model_payload,
        &snapshot,
    )
    .expect("set model");
    assert_eq!(set_model.status, 200);

    let response = workflow_self_play_message(
        root.path(),
        &snapshot,
        &agent_id,
        "As the agent, decide whether you need tools. If not, say hello in one sentence.",
    );
    assert_eq!(response.status, 200);
    assert!(
        !response
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .is_empty(),
        "{}",
        response.payload
    );
    assert_eq!(
        response
            .payload
            .pointer("/workflow_visibility/workflow_trace/final_authority")
            .and_then(Value::as_str),
        Some("llm_only")
    );
    assert_workflow_self_play_clean(&response.payload, "GHOST LIVE PROVIDER SHOULD NEVER APPEAR");
}
