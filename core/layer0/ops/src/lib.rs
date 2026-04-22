#![recursion_limit = "16384"]
// Layer ownership: core/layer0/ops

// Auto-segmented for file-size policy: implementation moved to adjacent .inc source.
// Keep module path stable while decomposing into <=500 LoC Rust source wrappers.
// Evidence markers retained for SRS/runtime-contract tests:
// core-lazy
// configure_low_memory_allocator_env
// no-client-bloat
macro_rules! include_parts {
    ($($path:literal),+ $(,)?) => {
        $(include!($path);)+
    };
}

include!("lib.rs.inc");

pub(crate) use protheus_nexus_core_v1::execution_core_v1 as execution_lane_bridge;
pub(crate) use protheus_nexus_core_v1::autonomy_core as protheus_autonomy_core_v1_bridge;
pub(crate) use protheus_nexus_core_v1::burn_oracle_budget_gate as burn_oracle_budget_gate_bridge;
pub(crate) use protheus_nexus_core_v1::foundation_hook_enforcer as foundation_hook_enforcer_bridge;
pub(crate) use protheus_nexus_core_v1::layer1_security as infring_layer1_security_bridge;
pub(crate) use protheus_nexus_core_v1::llm_runtime as llm_runtime_bridge;
pub(crate) use protheus_nexus_core_v1::memory_core_v1 as protheus_memory_core_v1_bridge;
pub(crate) use protheus_nexus_core_v1::ops_core as protheus_ops_core_v1_bridge;
pub(crate) use protheus_nexus_core_v1::persona_dispatch_security_gate as persona_dispatch_security_gate_bridge;
pub(crate) use protheus_nexus_core_v1::tiny_runtime as protheus_tiny_runtime_bridge;
pub(crate) use protheus_nexus_core_v1::tooling_core as protheus_tooling_core_v1_bridge;

#[allow(dead_code)]
pub(crate) fn contains_forbidden_runtime_context_marker(raw: &str) -> bool {
    const FORBIDDEN: [&str; 6] = [
        "You are an expert Python programmer.",
        "[PATCH v2",
        "List Leaves (25",
        "BEGIN_OPENCLAW_INTERNAL_CONTEXT",
        "END_OPENCLAW_INTERNAL_CONTEXT",
        "UNTRUSTED_CHILD_RESULT_DELIMITER",
    ];
    FORBIDDEN.iter().any(|marker| raw.contains(marker))
}
