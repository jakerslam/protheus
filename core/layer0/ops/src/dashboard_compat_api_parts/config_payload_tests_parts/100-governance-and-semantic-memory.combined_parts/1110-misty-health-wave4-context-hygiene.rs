// SRS: V12-MISTY-HEALTH-WAVE4-001

#[test]
fn misty_wave4_current_turn_dominance_flags_stale_context() {
    let stale = "Project overview: legacy invoice dashboard. Data source: MySQL exports. Tools used: PHP artisan commands. Key features: invoice aging, billing sync, and queue status. Future work: migrate controllers and add cache invalidation.";
    let direct = "Yes, the current tool workflow is now menu-driven and the latest turn remains in control.";

    assert!(
        response_current_turn_dominance_violation(
            "Is the current tool workflow giving you control now?",
            stale,
            &[],
        ),
        "stale project context should not dominate the latest workflow question"
    );
    assert!(
        !response_current_turn_dominance_violation(
            "Is the current tool workflow giving you control now?",
            direct,
            &[],
        ),
        "direct answer should satisfy current-turn dominance"
    );
}

#[test]
fn misty_wave4_simple_direct_turn_suppresses_passive_memory_context() {
    assert!(simple_direct_chat_suppresses_passive_context("hey", false));
    assert!(simple_direct_chat_suppresses_passive_context(
        "what about now? any better?",
        false
    ));
    assert!(!simple_direct_chat_suppresses_passive_context(
        "what did we decide earlier about the workflow?",
        false
    ));
    assert!(!simple_direct_chat_suppresses_passive_context(
        "search the workspace for workflow files",
        true
    ));
    assert!(simple_direct_chat_suppresses_passive_context(
        "Dry run only: tell me which file tool you would use, but do not run tools yet.",
        true
    ));
}

#[test]
fn misty_wave4_simple_direct_turn_uses_slim_active_context_window() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave4-slim-context-agent","role":"assistant"}"#,
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
    let long_prior_context = "legacy billing dashboard context, PHP controllers, invoice queues, migrations, web receipts, and stale tool notes. ".repeat(80);
    let mut history = Vec::<Value>::new();
    for index in 0..36 {
        history.push(json!({
            "role": "user",
            "text": format!("prior user turn {index}: {long_prior_context}"),
            "ts": format!("2026-04-26T00:{index:02}:00Z")
        }));
        history.push(json!({
            "role": "assistant",
            "text": format!("prior assistant turn {index}: {long_prior_context}"),
            "ts": format!("2026-04-26T00:{index:02}:30Z")
        }));
    }
    save_session_state(
        root.path(),
        &agent_id,
        &json!({
            "agent_id": agent_id,
            "active_session_id": "default",
            "sessions": [{
                "session_id": "default",
                "label": "Session",
                "created_at": "2026-04-26T00:00:00Z",
                "updated_at": "2026-04-26T00:36:00Z",
                "messages": history
            }]
        }),
    );
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [{"response": "Hey! How can I help you today?"}, {"response": "Hey! How can I help you today?"}],
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
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some("Hey! How can I help you today?")
    );
    assert_eq!(
        response
            .payload
            .pointer("/context_pool/active_target_tokens")
            .and_then(Value::as_i64),
        Some(768)
    );
    assert_eq!(
        response
            .payload
            .pointer("/context_pool/min_recent_messages")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert!(
        response
            .payload
            .pointer("/context_pool/active_messages")
            .and_then(Value::as_u64)
            .unwrap_or(999)
            <= 8,
        "simple direct turns should not hydrate the whole long session"
    );
}

#[test]
fn misty_wave4_simple_direct_turn_routes_to_fast_chat_model() {
    let _fast_model = ScopedEnvVar::set("INFRING_SIMPLE_CHAT_FAST_MODEL", "openai/gpt-5-mini");
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    crate::dashboard_provider_runtime::save_provider_key(root.path(), "openai", "sk-test-openai");
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave4-fast-chat-agent","role":"assistant"}"#,
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
    let set_model_payload =
        serde_json::to_vec(&json!({"model": "openai/gpt-5"})).expect("serialize model");
    let set_model = handle(
        root.path(),
        "PUT",
        &format!("/api/agents/{agent_id}/model"),
        &set_model_payload,
        &snapshot,
    )
    .expect("set model");
    assert_eq!(set_model.status, 200);
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [{"response": "Hey! How can I help you today?"}],
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
    assert_eq!(
        response.payload.get("model").and_then(Value::as_str),
        Some("gpt-5-mini")
    );
    assert_eq!(
        response
            .payload
            .pointer("/auto_route/reason")
            .and_then(Value::as_str),
        Some("simple_direct_chat_fast_model")
    );
    let script = read_json(&governance_test_chat_script_path(root.path())).expect("script");
    assert_eq!(
        script
            .pointer("/calls/0/model")
            .and_then(Value::as_str),
        Some("gpt-5-mini")
    );
}

#[test]
fn misty_wave4_manual_toolbox_turn_routes_to_fast_chat_model() {
    let _fast_model = ScopedEnvVar::set("INFRING_SIMPLE_CHAT_FAST_MODEL", "openai/gpt-5-mini");
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    crate::dashboard_provider_runtime::save_provider_key(root.path(), "openai", "sk-test-openai");
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave4-fast-toolbox-agent","role":"assistant"}"#,
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
    let set_model_payload =
        serde_json::to_vec(&json!({"model": "openai/gpt-5"})).expect("serialize model");
    let set_model = handle(
        root.path(),
        "PUT",
        &format!("/api/agents/{agent_id}/model"),
        &set_model_payload,
        &snapshot,
    )
    .expect("set model");
    assert_eq!(set_model.status, 200);
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [{"response": "I would choose web search for that comparison."}],
            "calls": []
        }),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Use web search to compare Infring to other agent frameworks."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("model").and_then(Value::as_str),
        Some("gpt-5-mini")
    );
    assert_eq!(
        response
            .payload
            .pointer("/auto_route/reason")
            .and_then(Value::as_str),
        Some("manual_toolbox_fast_model")
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/model")
            .and_then(Value::as_str),
        Some("gpt-5-mini")
    );
}

#[test]
fn misty_wave4_simple_direct_fast_chat_skips_reasoning_models() {
    assert!(!simple_direct_chat_model_allows_visible_chat(
        "ollama/smallthinker:latest"
    ));
    assert!(!simple_direct_chat_model_allows_visible_chat(
        "ollama/kimi-k2.6:thinking"
    ));
    assert!(simple_direct_chat_model_allows_visible_chat(
        "ollama/qwen2.5:3b"
    ));
}

#[test]
fn misty_wave4_live_eval_flags_visible_internal_deliberation() {
    assert!(visible_response_looks_like_internal_deliberation(
        "I'm trying to craft a response that adheres to the given constraints. \
         To do this, I need to ensure my answer is concise. Given the original question is hey, \
         my approach should be to provide a straightforward acknowledgment."
    ));
    assert!(!visible_response_looks_like_internal_deliberation(
        "Hey! How can I help you today?"
    ));
}

#[test]
fn misty_wave4_finalization_unwraps_visible_response_json_wrapper() {
    let (finalized, outcome, _) = finalize_user_facing_response_with_outcome(
        "\"response\": \"The tool run hit issues: batch query did not produce enough source coverage.\" }"
            .to_string(),
        None,
    );

    assert_eq!(
        finalized,
        "The tool run hit issues: batch query did not produce enough source coverage."
    );
    assert_eq!(outcome, "normalized_raw_payload_json");
    assert!(!finalized.contains("\"response\""));
}

#[test]
fn misty_wave4_finalization_unwraps_visible_text_json_wrapper() {
    let (finalized, outcome, _) = finalize_user_facing_response_with_outcome(
        "\"text\": \"I'm sorry, but the search did not find enough relevant information.\" }"
            .to_string(),
        None,
    );

    assert_eq!(
        finalized,
        "I'm sorry, but the search did not find enough relevant information."
    );
    assert_eq!(outcome, "normalized_raw_payload_json");
    assert!(!finalized.contains("\"text\""));
}

#[test]
fn misty_wave4_live_eval_flags_visible_response_json_wrapper() {
    assert!(visible_response_looks_like_json_response_wrapper(
        "\"response\": \"The tool run hit issues: batch query did not produce enough source coverage.\" }"
    ));
    assert!(visible_response_looks_like_json_response_wrapper(
        "\"text\": \"I'm sorry, but the search did not find enough relevant information.\" }"
    ));
    assert!(!visible_response_looks_like_json_response_wrapper(
        "The tool run hit issues: batch query did not produce enough source coverage."
    ));
}

#[test]
fn misty_wave4_stale_final_synthesis_repairs_to_current_turn_draft() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave4-current-turn-agent","role":"assistant"}"#,
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
    let safe_draft = "Yes, the current tool workflow is menu-driven now, and this answer is focused on your latest question.";
    let stale_final = "Project overview: legacy billing dashboard. Data source: MySQL invoices and CSV exports. Tools used: PHP artisan commands and Laravel service providers. Key features: billing queues, invoice aging, and admin dashboards. Future work: migrate controllers and add cache invalidation.";
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [{"response": safe_draft}, {"response": stale_final}],
            "calls": []
        }),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Is the current tool workflow giving you control now? Answer directly."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("response").and_then(Value::as_str), Some(safe_draft));
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/current_turn_dominance/dominant")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response
            .payload
            .pointer("/agent_health_snapshot/status")
            .and_then(Value::as_str),
        Some("healthy")
    );
}

#[test]
fn misty_wave4_unrequested_code_without_tool_evidence_is_withheld_and_snapshotted() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave4-contamination-agent","role":"assistant"}"#,
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
    let php_dump = r#"```php
<?php
namespace App\Providers;
use Illuminate\Support\ServiceProvider;
class AppServiceProvider extends ServiceProvider {
    public function register(): void {
        $this->app->bind('legacy.billing', fn () => new BillingService());
    }
    public function boot(): void {
        JsonResource::withoutWrapping();
    }
}
```"#;
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({"queue": [{"response": php_dump}], "calls": []}),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"what is going on with the workflow right now?"}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(response.payload.get("response").and_then(Value::as_str), Some(""));
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/contamination_guard/detected")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        response
            .payload
            .pointer("/agent_health_snapshot/status")
            .and_then(Value::as_str),
        Some("degraded")
    );
    assert!(agent_control_plane_health_snapshot_path(root.path(), &agent_id).exists());
}
