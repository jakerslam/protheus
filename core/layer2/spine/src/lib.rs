// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/spine (authoritative spine runtime control).

pub mod authority;
pub mod spine;

pub use authority::{
    compute_evidence_run_plan, run_background_hands_scheduler, run_evidence_run_plan,
    run_rsi_idle_hands_scheduler,
};
pub use spine::spine_contract_receipt;

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
