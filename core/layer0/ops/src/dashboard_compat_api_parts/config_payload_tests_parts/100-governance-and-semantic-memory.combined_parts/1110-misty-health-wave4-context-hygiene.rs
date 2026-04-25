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
