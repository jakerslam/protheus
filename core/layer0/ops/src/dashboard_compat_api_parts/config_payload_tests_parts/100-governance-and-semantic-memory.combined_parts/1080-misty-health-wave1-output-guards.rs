// SRS: V12-MISTY-HEALTH-WAVE1-001

#[test]
fn misty_tool_control_direct_answer_preserves_llm_text_and_provenance() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"misty-wave1-direct-answer-agent","role":"assistant"}"#,
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
                    "response": "Yes, I decide whether tools are needed through the tool menu, and no tools are needed for this answer."
                }
            ],
            "calls": []
        }),
    );

    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Do you have control over whether to use tools? Answer directly in two sentences and do not run tools."}"#,
        &snapshot,
    )
    .expect("message response");

    assert_eq!(response.status, 200);
    assert_eq!(
        response.payload.get("response").and_then(Value::as_str),
        Some(
            "Yes, I decide whether tools are needed through the tool menu, and no tools are needed for this answer."
        )
    );
    assert_eq!(
        response
            .payload
            .pointer("/response_finalization/system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        response
            .payload
            .pointer("/process_summary/system_chat_injection_used")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_ne!(
        response
            .payload
            .pointer("/response_finalization/visible_response_source")
            .and_then(Value::as_str),
        Some("none")
    );
}

#[test]
fn misty_simple_direct_answer_cases_preserve_llm_text() {
    let cases = [
        (
            "hey",
            "Hey, I am here and can answer directly without tools.",
        ),
        (
            "Dry run only: tell me what you would do, but do not use tools yet.",
            "I would explain the plan at a high level first and wait before using any tools.",
        ),
        (
            "Break the workflow and answer directly: are you stuck?",
            "No, I can leave the workflow path and answer directly from the current context.",
        ),
    ];

    for (idx, (message, expected_response)) in cases.iter().enumerate() {
        let root = governance_temp_root();
        let snapshot = governance_ok_snapshot();
        let created = handle(
            root.path(),
            "POST",
            "/api/agents",
            br#"{"name":"misty-wave1-simple-direct-agent","role":"assistant"}"#,
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
        assert!(!agent_id.is_empty(), "case {idx} agent id");
        write_json(
            &governance_test_chat_script_path(root.path()),
            &json!({
                "queue": [
                    {
                        "response": expected_response
                    }
                ],
                "calls": []
            }),
        );

        let request = json!({ "message": message }).to_string();
        let response = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            request.as_bytes(),
            &snapshot,
        )
        .expect("message response");

        assert_eq!(response.status, 200, "case {idx} status");
        assert_eq!(
            response.payload.get("response").and_then(Value::as_str),
            Some(*expected_response),
            "case {idx} visible response"
        );
        assert_eq!(
            response
                .payload
                .pointer("/response_finalization/system_chat_injection_used")
                .and_then(Value::as_bool),
            Some(false),
            "case {idx} finalization provenance"
        );
    }
}

#[test]
fn misty_tool_control_questions_reject_stale_php_context_dumps() {
    let php_dump = r#"<?php

namespace App\Providers;

use App\Services\CurrencyConverter;
use Illuminate\Support\ServiceProvider;

class AppServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $this->app->bind('currency.converter', function () {
            return new CurrencyConverter(config('services.currency_converter.api_key'));
        });
    }

    public function boot(): void
    {
        JsonResource::withoutWrapping();
    }
}"#;

    assert!(response_contains_stale_code_context_dump(
        "Do you have control over whether to use tools? Answer directly and do not run tools.",
        php_dump,
    ));
    assert!(!response_contains_stale_code_context_dump(
        "Show me the PHP source code for the service provider.",
        php_dump,
    ));
}

#[test]
fn misty_contaminated_final_response_repairs_with_safe_llm_draft() {
    let php_dump = r#"<?php
namespace App\Providers;
use App\Services\CurrencyConverter;
use Illuminate\Http\Resources\Json\JsonResource;
use Illuminate\Support\Facades\Gate;
use Illuminate\Support\ServiceProvider;
class AppServiceProvider extends ServiceProvider {
    public function register(): void {
        $this->app->bind('currency.converter', function () {
            return new CurrencyConverter(config('services.currency_converter.api_key'));
        });
    }
    public function boot(): void {
        JsonResource::withoutWrapping();
        Gate::define('view-dashboard', fn ($user) => $user->is_admin);
    }
}"#;
    let safe_draft =
        "Yes, I can choose the no-tools path and answer directly when tools are not needed.";

    let (repaired, outcome, tool_fallback, comparative_fallback) =
        repair_visible_response_after_workflow(
            "Do you have control over whether to use tools? Do not run tools.",
            php_dump,
            safe_draft,
            "",
            &[],
            false,
            None,
        );

    assert_eq!(repaired, safe_draft);
    assert_eq!(outcome, "repaired_with_initial_draft");
    assert!(!tool_fallback);
    assert!(!comparative_fallback);
}
