use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct KernelSentinelInvariant {
    pub id: &'static str,
    pub title: &'static str,
    pub law: &'static str,
    pub authority: &'static str,
    pub enforcement_scope: &'static str,
    pub failure_level: &'static str,
    pub root_frame: &'static str,
    pub remediation_level: &'static str,
    pub required_evidence: &'static [&'static str],
}

pub const KERNEL_SENTINEL_INVARIANTS: &[KernelSentinelInvariant] = &[
    KernelSentinelInvariant {
        id: "kernel_truth_is_authoritative",
        title: "Kernel truth is authoritative",
        law: "Canonical runtime truth, admission, policy, receipts, and state transitions must be decided by Kernel authority, not Shell projection or artifact pass/fail summaries.",
        authority: "kernel",
        enforcement_scope: "runtime_truth",
        failure_level: "L3_policy_truth_failure",
        root_frame: "policy_truth_contradiction",
        remediation_level: "policy_realignment",
        required_evidence: &[
            "kernel_policy_receipt",
            "runtime_state_observation",
            "authority_boundary_check",
        ],
    },
    KernelSentinelInvariant {
        id: "receipts_bind_state_transitions",
        title: "Receipts bind state transitions",
        law: "Every authoritative state transition must be reconstructable from a receipt or lifecycle observation with source, decision, and outcome context.",
        authority: "kernel",
        enforcement_scope: "receipt_integrity",
        failure_level: "L3_policy_truth_failure",
        root_frame: "policy_truth_contradiction",
        remediation_level: "policy_realignment",
        required_evidence: &[
            "transition_receipt",
            "lifecycle_observation",
            "state_before_after",
        ],
    },
    KernelSentinelInvariant {
        id: "capability_checks_precede_execution",
        title: "Capability checks precede execution",
        law: "Kernel execution paths must prove capability, lease, and boundary posture before performing privileged work.",
        authority: "kernel",
        enforcement_scope: "capability_enforcement",
        failure_level: "L3_policy_truth_failure",
        root_frame: "policy_truth_contradiction",
        remediation_level: "policy_realignment",
        required_evidence: &[
            "capability_check",
            "lease_context",
            "boundary_posture",
        ],
    },
    KernelSentinelInvariant {
        id: "gateway_success_requires_durable_listener",
        title: "Gateway success requires durable listener",
        law: "Gateway start, restart, and recovery success must only be reported when restart output, listener probes, watchdog state, and lifecycle receipts agree that a durable listener is active.",
        authority: "kernel",
        enforcement_scope: "gateway_lifecycle",
        failure_level: "L2_boundary_contract_breach",
        root_frame: "cross_boundary_contract",
        remediation_level: "boundary_repair",
        required_evidence: &[
            "restart_output",
            "listener_probe",
            "watchdog_state",
            "lifecycle_receipt",
        ],
    },
    KernelSentinelInvariant {
        id: "shell_connectivity_uses_authoritative_runtime_state",
        title: "Shell connectivity uses authoritative runtime state",
        law: "Shell connectivity indicators must project backend-provided runtime state and freshness metadata, and must not infer online/offline truth from presentation-local guesses.",
        authority: "kernel",
        enforcement_scope: "shell_runtime_projection",
        failure_level: "L2_boundary_contract_breach",
        root_frame: "cross_boundary_contract",
        remediation_level: "boundary_repair",
        required_evidence: &[
            "shell_taskbar_state",
            "gateway_status",
            "kernel_lifecycle_truth",
            "freshness_metadata",
        ],
    },
    KernelSentinelInvariant {
        id: "api_routing_preserves_semantic_request_information",
        title: "API routing preserves semantic request information",
        law: "API compatibility and Rust routing layers must preserve semantic request information, including path intent, query strings, target identifiers, and routing metadata, across boundary translation.",
        authority: "kernel",
        enforcement_scope: "api_boundary_routing",
        failure_level: "L2_boundary_contract_breach",
        root_frame: "cross_boundary_contract",
        remediation_level: "boundary_repair",
        required_evidence: &[
            "incoming_request_path",
            "query_string",
            "rust_compatibility_route",
            "semantic_request_payload",
            "routing_metadata",
        ],
    },
    KernelSentinelInvariant {
        id: "boot_critical_routes_are_bounded_lightweight_and_nonblocking",
        title: "Boot-critical routes are bounded, lightweight, and nonblocking",
        law: "Boot-critical routes must remain bounded, lightweight, and nonblocking; startup success cannot depend on slow optional surfaces, unbounded waits, or presentation-only readiness.",
        authority: "kernel",
        enforcement_scope: "boot_surface",
        failure_level: "L1_component_regression",
        root_frame: "component_runtime_regression",
        remediation_level: "component_repair",
        required_evidence: &[
            "boot_route_inventory",
            "startup_latency_budget",
            "nonblocking_readiness_probe",
            "optional_surface_deferral",
            "boot_lifecycle_receipt",
        ],
    },
    KernelSentinelInvariant {
        id: "watchdog_owns_process_uniqueness_and_stale_host_cleanup",
        title: "Watchdog owns process uniqueness and stale host cleanup",
        law: "Runtime watchdog authority must own process uniqueness, stale host detection, and cleanup decisions so duplicate hosts cannot remain active as presentation or session churn.",
        authority: "kernel",
        enforcement_scope: "watchdog_process_lifecycle",
        failure_level: "L2_boundary_contract_breach",
        root_frame: "cross_boundary_contract",
        remediation_level: "boundary_repair",
        required_evidence: &[
            "watchdog_state",
            "process_inventory",
            "duplicate_host_probe",
            "stale_host_cleanup_receipt",
            "lifecycle_receipt",
        ],
    },
    KernelSentinelInvariant {
        id: "sentinel_feedback_preserves_root_frame",
        title: "Sentinel feedback preserves root frame",
        law: "Sentinel findings, feedback, and issue candidates must retain failure level, root frame, and remediation level so remediation does not collapse into symptom patching.",
        authority: "kernel_sentinel",
        enforcement_scope: "self_understanding",
        failure_level: "L5_self_model_failure",
        root_frame: "system_self_model",
        remediation_level: "self_model_repair",
        required_evidence: &[
            "failure_level",
            "root_frame",
            "remediation_level",
        ],
    },
];

pub fn kernel_sentinel_invariant_registry() -> &'static [KernelSentinelInvariant] {
    KERNEL_SENTINEL_INVARIANTS
}

pub fn kernel_sentinel_invariant_by_id(id: &str) -> Option<&'static KernelSentinelInvariant> {
    KERNEL_SENTINEL_INVARIANTS
        .iter()
        .find(|invariant| invariant.id == id)
}

pub fn kernel_sentinel_invariant_registry_report() -> Value {
    json!({
        "schema": "kernel_sentinel_invariant_registry.v1",
        "authority": "kernel",
        "artifact_pass_fail_is_sufficient": false,
        "invariant_count": KERNEL_SENTINEL_INVARIANTS.len(),
        "invariants": KERNEL_SENTINEL_INVARIANTS,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariant_registry_is_kernel_owned_and_explicit() {
        let report = kernel_sentinel_invariant_registry_report();

        assert_eq!(report["authority"], "kernel");
        assert_eq!(report["artifact_pass_fail_is_sufficient"], false);
        assert!(report["invariant_count"].as_u64().unwrap() >= 9);
        assert!(kernel_sentinel_invariant_by_id("kernel_truth_is_authoritative").is_some());
        assert!(kernel_sentinel_invariant_by_id("receipts_bind_state_transitions").is_some());
        assert!(kernel_sentinel_invariant_by_id("capability_checks_precede_execution").is_some());
        assert!(
            kernel_sentinel_invariant_by_id("gateway_success_requires_durable_listener").is_some()
        );
        assert!(
            kernel_sentinel_invariant_by_id(
                "shell_connectivity_uses_authoritative_runtime_state"
            )
            .is_some()
        );
        assert!(
            kernel_sentinel_invariant_by_id(
                "api_routing_preserves_semantic_request_information"
            )
            .is_some()
        );
        assert!(
            kernel_sentinel_invariant_by_id(
                "boot_critical_routes_are_bounded_lightweight_and_nonblocking"
            )
            .is_some()
        );
        assert!(
            kernel_sentinel_invariant_by_id(
                "watchdog_owns_process_uniqueness_and_stale_host_cleanup"
            )
            .is_some()
        );
        assert!(
            kernel_sentinel_invariant_by_id("sentinel_feedback_preserves_root_frame").is_some()
        );
    }

    #[test]
    fn invariants_carry_failure_and_evidence_contracts() {
        for invariant in kernel_sentinel_invariant_registry() {
            assert!(!invariant.failure_level.is_empty());
            assert!(!invariant.root_frame.is_empty());
            assert!(!invariant.remediation_level.is_empty());
            assert!(!invariant.required_evidence.is_empty());
            assert_ne!(invariant.enforcement_scope, "artifact_pass_fail");
        }
    }

    #[test]
    fn gateway_success_invariant_requires_durable_listener_evidence() {
        let invariant =
            kernel_sentinel_invariant_by_id("gateway_success_requires_durable_listener").unwrap();

        assert_eq!(invariant.failure_level, "L2_boundary_contract_breach");
        assert_eq!(invariant.root_frame, "cross_boundary_contract");
        assert_eq!(invariant.remediation_level, "boundary_repair");
        assert!(invariant.required_evidence.contains(&"restart_output"));
        assert!(invariant.required_evidence.contains(&"listener_probe"));
        assert!(invariant.required_evidence.contains(&"watchdog_state"));
        assert!(invariant.required_evidence.contains(&"lifecycle_receipt"));
    }

    #[test]
    fn shell_connectivity_invariant_requires_authoritative_runtime_evidence() {
        let invariant =
            kernel_sentinel_invariant_by_id("shell_connectivity_uses_authoritative_runtime_state")
                .unwrap();

        assert_eq!(invariant.failure_level, "L2_boundary_contract_breach");
        assert_eq!(invariant.root_frame, "cross_boundary_contract");
        assert_eq!(invariant.remediation_level, "boundary_repair");
        assert_eq!(invariant.authority, "kernel");
        assert!(invariant.required_evidence.contains(&"shell_taskbar_state"));
        assert!(invariant.required_evidence.contains(&"gateway_status"));
        assert!(invariant
            .required_evidence
            .contains(&"kernel_lifecycle_truth"));
        assert!(invariant.required_evidence.contains(&"freshness_metadata"));
    }

    #[test]
    fn api_routing_invariant_requires_semantic_request_preservation_evidence() {
        let invariant =
            kernel_sentinel_invariant_by_id("api_routing_preserves_semantic_request_information")
                .unwrap();

        assert_eq!(invariant.failure_level, "L2_boundary_contract_breach");
        assert_eq!(invariant.root_frame, "cross_boundary_contract");
        assert_eq!(invariant.remediation_level, "boundary_repair");
        assert_eq!(invariant.authority, "kernel");
        assert!(invariant.required_evidence.contains(&"incoming_request_path"));
        assert!(invariant.required_evidence.contains(&"query_string"));
        assert!(invariant
            .required_evidence
            .contains(&"rust_compatibility_route"));
        assert!(invariant
            .required_evidence
            .contains(&"semantic_request_payload"));
        assert!(invariant.required_evidence.contains(&"routing_metadata"));
    }

    #[test]
    fn boot_surface_invariant_requires_bounded_nonblocking_evidence() {
        let invariant = kernel_sentinel_invariant_by_id(
            "boot_critical_routes_are_bounded_lightweight_and_nonblocking",
        )
        .unwrap();

        assert_eq!(invariant.failure_level, "L1_component_regression");
        assert_eq!(invariant.root_frame, "component_runtime_regression");
        assert_eq!(invariant.remediation_level, "component_repair");
        assert_eq!(invariant.authority, "kernel");
        assert!(invariant.required_evidence.contains(&"boot_route_inventory"));
        assert!(invariant.required_evidence.contains(&"startup_latency_budget"));
        assert!(invariant
            .required_evidence
            .contains(&"nonblocking_readiness_probe"));
        assert!(invariant
            .required_evidence
            .contains(&"optional_surface_deferral"));
        assert!(invariant.required_evidence.contains(&"boot_lifecycle_receipt"));
    }

    #[test]
    fn watchdog_process_invariant_requires_uniqueness_and_cleanup_evidence() {
        let invariant = kernel_sentinel_invariant_by_id(
            "watchdog_owns_process_uniqueness_and_stale_host_cleanup",
        )
        .unwrap();

        assert_eq!(invariant.failure_level, "L2_boundary_contract_breach");
        assert_eq!(invariant.root_frame, "cross_boundary_contract");
        assert_eq!(invariant.remediation_level, "boundary_repair");
        assert_eq!(invariant.authority, "kernel");
        assert_eq!(invariant.enforcement_scope, "watchdog_process_lifecycle");
        assert!(invariant.required_evidence.contains(&"watchdog_state"));
        assert!(invariant.required_evidence.contains(&"process_inventory"));
        assert!(invariant.required_evidence.contains(&"duplicate_host_probe"));
        assert!(invariant
            .required_evidence
            .contains(&"stale_host_cleanup_receipt"));
        assert!(invariant.required_evidence.contains(&"lifecycle_receipt"));
    }
}
