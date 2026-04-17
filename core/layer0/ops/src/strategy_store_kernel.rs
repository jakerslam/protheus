// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("strategy_store_kernel_parts/010-usage.rs");
include!("strategy_store_kernel_parts/020-emit-strategy-pointer.rs");
include!("strategy_store_kernel_parts/030-normalize-queue-item.rs");
include!("strategy_store_kernel_parts/040-set-state.rs");
include!("strategy_store_kernel_parts/050-evaluate-gc-candidates-value.rs");

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
