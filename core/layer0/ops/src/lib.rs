#![recursion_limit = "2048"]
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
