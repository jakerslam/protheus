use infring_orchestration_surface_v1::self_maintenance::analyzer::evidence_to_claim_bundle;
use infring_orchestration_surface_v1::self_maintenance::contracts::{
    ArchitectureAuditInput, CiReportInput, DependencyViolationInput, HealthMetricInput,
    MemoryPressureInput, ObservationInputs, OrphanedObjectInput, SupervisorMode,
    TaskFabricSignalInput,
};
use infring_orchestration_surface_v1::self_maintenance::executor::GovernedSelfMaintenanceSupervisor;
use infring_orchestration_surface_v1::self_maintenance::observer::collect_evidence_cards;
use infring_orchestration_surface_v1::self_maintenance::task_generator::claim_bundle_to_task_graph;
use infring_task_fabric_core_v1::LifecycleStatus;

fn base_inputs() -> ObservationInputs {
    ObservationInputs {
        architecture_audits: Vec::new(),
        dependency_violations: Vec::new(),
        task_fabric_signals: TaskFabricSignalInput {
            stale_tasks: Vec::new(),
            blocked_tasks: Vec::new(),
        },
        ci_reports: Vec::new(),
        health_metrics: Vec::new(),
        memory_pressure: Vec::new(),
        orphaned_objects: Vec::new(),
    }
}

#[test]
fn detection_produces_evidence() {
    let mut inputs = base_inputs();
    inputs.architecture_audits.push(ArchitectureAuditInput {
        audit_id: "audit-1".to_string(),
        summary: "forbidden import edge found".to_string(),
        severity: "high".to_string(),
        source_ref: "artifacts/arch_guard.json".to_string(),
    });
    inputs.ci_reports.push(CiReportInput {
        report_id: "ci-1".to_string(),
        status: "failed".to_string(),
        summary: "dependency boundary guard failed".to_string(),
        source_ref: ".github/workflows/ci.yml".to_string(),
    });
    let evidence = collect_evidence_cards(&inputs, 1000);
    assert!(evidence.len() >= 2);
}

#[test]
fn evidence_produces_claims() {
    let mut inputs = base_inputs();
    inputs.dependency_violations.push(DependencyViolationInput {
        violation_id: "dep-1".to_string(),
        summary: "client imports forbidden surface path".to_string(),
        source_ref: "client/runtime/systems/example.ts".to_string(),
    });
    let evidence = collect_evidence_cards(&inputs, 2_000);
    let bundle = evidence_to_claim_bundle("task-auto-1", &evidence);
    assert!(!bundle.claims.is_empty());
    assert_eq!(bundle.task_id, "task-auto-1");
}

#[test]
fn claims_produce_task_fabric_tasks() {
    let mut inputs = base_inputs();
    inputs.task_fabric_signals.stale_tasks = vec!["task-a".to_string(), "task-b".to_string()];
    let evidence = collect_evidence_cards(&inputs, 3_000);
    let bundle = evidence_to_claim_bundle("task-auto-2", &evidence);
    let generated = claim_bundle_to_task_graph(&bundle, "self_maintenance", 3_000);
    assert!(!generated.tasks.is_empty());
    assert!(generated
        .tasks
        .iter()
        .all(|task| task.lifecycle_status == LifecycleStatus::Pending));
}

#[test]
fn safe_apply_executes_allowed_fixes() {
    let mut inputs = base_inputs();
    inputs.dependency_violations.push(DependencyViolationInput {
        violation_id: "dep-2".to_string(),
        summary: "orchestration path mismatch".to_string(),
        source_ref: "surface/orchestration/src/lib.rs".to_string(),
    });

    let mut supervisor =
        GovernedSelfMaintenanceSupervisor::new(SupervisorMode::ApplySafe, "self_maintenance");
    let out = supervisor.run_cycle(inputs, 4_000).expect("run");
    assert!(!out.worker_outputs.is_empty());
    assert!(out.worker_outputs[0].produced_evidence_ids.len() >= 1);
}

#[test]
fn high_risk_actions_require_escalation() {
    let mut inputs = base_inputs();
    inputs.health_metrics.push(HealthMetricInput {
        metric_name: "latency_p95".to_string(),
        observed: 1800.0,
        threshold: 500.0,
        source_ref: "metrics/runtime.json".to_string(),
    });
    inputs.memory_pressure.push(MemoryPressureInput {
        scope: "core".to_string(),
        used_bytes: 900,
        limit_bytes: 1000,
    });
    let mut supervisor =
        GovernedSelfMaintenanceSupervisor::new(SupervisorMode::ApplySafe, "self_maintenance");
    let out = supervisor.run_cycle(inputs, 5_000).expect("run");
    assert!(out.worker_outputs.is_empty());
    assert!(!out.escalation_requests.is_empty());
}

#[test]
fn no_direct_core_mutation_bypass_exists() {
    let mut inputs = base_inputs();
    inputs.dependency_violations.push(DependencyViolationInput {
        violation_id: "dep-3".to_string(),
        summary: "docs path drift".to_string(),
        source_ref: "docs/workspace/SRS.md".to_string(),
    });
    let mut supervisor =
        GovernedSelfMaintenanceSupervisor::new(SupervisorMode::ApplySafe, "self_maintenance");
    let out = supervisor.run_cycle(inputs, 6_000).expect("run");
    let details = out
        .receipts
        .iter()
        .map(|row| row.detail.clone())
        .collect::<Vec<_>>();
    let idx_task = details
        .iter()
        .position(|v| v == "execute_path:task_fabric")
        .expect("task fabric path");
    let idx_tool = details
        .iter()
        .position(|v| v == "execute_path:tool_broker")
        .expect("tool broker path");
    let idx_evidence = details
        .iter()
        .position(|v| v == "execute_path:evidence_store")
        .expect("evidence path");
    let idx_verifier = details
        .iter()
        .position(|v| v == "execute_path:verifier")
        .expect("verifier path");
    let idx_memory = details
        .iter()
        .position(|v| v == "execute_path:memory")
        .expect("memory path");
    assert!(idx_task < idx_tool);
    assert!(idx_tool < idx_evidence);
    assert!(idx_evidence < idx_verifier);
    assert!(idx_verifier < idx_memory);
}

#[test]
fn ephemeral_state_is_used_and_cleaned() {
    let mut inputs = base_inputs();
    inputs.architecture_audits.push(ArchitectureAuditInput {
        audit_id: "audit-2".to_string(),
        summary: "normalization pass".to_string(),
        severity: "low".to_string(),
        source_ref: "artifacts/arch.json".to_string(),
    });
    inputs.orphaned_objects.push(OrphanedObjectInput {
        object_id: "obj-1".to_string(),
        summary: "orphaned cache key".to_string(),
        source_ref: "state/cache/index.json".to_string(),
    });
    let mut supervisor =
        GovernedSelfMaintenanceSupervisor::new(SupervisorMode::ObserveOnly, "self_maintenance");
    let _ = supervisor.run_cycle(inputs, 7_000).expect("run");
    assert!(supervisor.active_ephemeral_count() > 0);
    let cleaned = supervisor.sweep_ephemeral().expect("sweep");
    assert!(cleaned > 0);
}
