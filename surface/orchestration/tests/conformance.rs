use infring_orchestration_surface_v1::contracts::{CoreContractCall, OrchestrationRequest};
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
