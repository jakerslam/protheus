// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("symbiosis_coherence_kernel_parts/010-usage.rs");
include!("symbiosis_coherence_kernel_parts/020-compute-identity-component.rs");
include!("symbiosis_coherence_kernel_parts/030-run.rs");

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
