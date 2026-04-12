use infring_orchestration_surface_v1::contracts::{
    ClarificationReason, CoreContractCall, OrchestrationRequest, RequestClass,
};
use infring_orchestration_surface_v1::OrchestrationSurfaceRuntime;
use serde_json::json;

#[test]
fn orchestration_surface_cannot_bypass_tool_broker() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s1".to_string(),
            intent: "search web for release notes".to_string(),
            payload: json!({}),
        },
        1_000,
    );
    assert_eq!(
        package.core_contract_calls,
        vec![
            CoreContractCall::ToolCapabilityProbe,
            CoreContractCall::ToolBrokerRequest
        ]
    );
    assert_eq!(package.classification.request_class, RequestClass::ToolCall);
    assert!(package
        .fallback_actions
        .iter()
        .any(|row| row.kind == "inspect_tool_capabilities"));
}

#[test]
fn orchestration_surface_cannot_persist_private_durable_task_state() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let _ = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s2".to_string(),
            intent: "plan tasks".to_string(),
            payload: json!({}),
        },
        1_000,
    );
    assert_eq!(runtime.transient_entry_count(), 1);
    let swept = runtime.sweep_transient(31_500);
    assert_eq!(swept, 1);
    assert_eq!(runtime.transient_entry_count(), 0);
}

#[test]
fn orchestration_surface_cannot_canonize_truth() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s3".to_string(),
            intent: "update workflow".to_string(),
            payload: json!({"target":"task_fabric"}),
        },
        1_000,
    );
    assert!(package.requires_core_promotion);
    assert!(package
        .core_contract_calls
        .contains(&CoreContractCall::TaskFabricProposal));
}

#[test]
fn orchestration_transient_state_is_sweepable() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let _ = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s4".to_string(),
            intent: "read status".to_string(),
            payload: json!({}),
        },
        10_000,
    );
    assert_eq!(runtime.transient_entry_count(), 1);
    assert_eq!(runtime.sweep_transient(9_000), 0);
    assert_eq!(runtime.sweep_transient(40_001), 1);
}

#[test]
fn orchestration_transient_restart_requires_boot_sweep_before_resume() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let _ = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s5".to_string(),
            intent: "hold short-term context".to_string(),
            payload: json!({}),
        },
        10_000,
    );
    assert_eq!(runtime.transient_entry_count(), 1);
    assert_eq!(runtime.transient_ephemeral_count(), 1);

    runtime.begin_transient_restart();
    let blocked = runtime
        .resume_transient_after_restart()
        .expect_err("resume should block on stale transient payload");
    assert!(blocked.starts_with("transient_context_resume_blocked:"));

    let swept = runtime
        .sweep_transient_before_resume()
        .expect("boot sweep should succeed");
    assert_eq!(swept, 1);
    assert_eq!(runtime.transient_entry_count(), 0);
    assert_eq!(runtime.transient_ephemeral_count(), 0);
    runtime
        .resume_transient_after_restart()
        .expect("resume should succeed after boot sweep");
}

#[test]
fn orchestration_legacy_intent_path_still_produces_typed_tool_plan() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "legacy-s1".to_string(),
            intent: "  Search web for release notes  ".to_string(),
            payload: serde_json::Value::Null,
        },
        2_000,
    );
    assert_eq!(package.classification.request_class, RequestClass::ToolCall);
    assert!(!package.classification.needs_clarification);
    assert!(package
        .classification
        .reasons
        .iter()
        .any(|row| row == "legacy_intent_compatibility_shim"));
}

#[test]
fn ambiguous_legacy_intent_returns_machine_readable_clarification_reason() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s6".to_string(),
            intent: "maybe do something".to_string(),
            payload: json!({}),
        },
        2_500,
    );
    assert!(package.classification.needs_clarification);
    assert!(package
        .classification
        .clarification_reasons
        .contains(&ClarificationReason::AmbiguousOperation));
    assert!(package.summary.contains("clarification"));
}

#[test]
fn mutation_without_target_requires_typed_scope_clarification() {
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let package = runtime.orchestrate(
        OrchestrationRequest {
            session_id: "s7".to_string(),
            intent: "update workflow".to_string(),
            payload: json!({}),
        },
        3_000,
    );
    assert!(package.classification.needs_clarification);
    assert!(package
        .classification
        .clarification_reasons
        .contains(&ClarificationReason::MutationScopeRequired));
}
