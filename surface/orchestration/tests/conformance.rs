// Layer ownership: tests (regression proof for orchestration surface contracts).
use infring_orchestration_surface_v1::contracts::{
    Capability, CapabilityProbeSnapshot, ClarificationReason, CoreContractCall, CoreProbeEnvelope,
    Mutability, OperationKind, OrchestrationRequest, PolicyScope, Precondition, RequestClass,
    RequestKind, RequestSurface, ResourceKind, TargetDescriptor, TypedOrchestrationRequest,
    WorkflowStage,
};
use infring_orchestration_surface_v1::OrchestrationSurfaceRuntime;
use serde_json::json;

#[path = "conformance/adapter_contracts_probe_enforcement.rs"]
mod adapter_contracts_probe_enforcement;
#[path = "conformance/adapter_contracts_surface.rs"]
mod adapter_contracts_surface;
#[path = "conformance/lifecycle_feedback.rs"]
mod lifecycle_feedback;
#[path = "conformance/planning_execution.rs"]
mod planning_execution;
#[path = "conformance/probe_matrix.rs"]
mod probe_matrix;
#[path = "conformance/quality_planner_runtime.rs"]
mod quality_planner_runtime;
#[path = "conformance/quality_surface.rs"]
mod quality_surface;
#[path = "conformance/sovereignty_transient.rs"]
mod sovereignty_transient;
