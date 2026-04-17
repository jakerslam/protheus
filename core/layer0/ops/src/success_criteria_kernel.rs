// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("success_criteria_kernel_parts/010-print-json-line.rs");
include!("success_criteria_kernel_parts/020-classify-metric.rs");
include!("success_criteria_kernel_parts/030-evaluate-row.rs");
include!("success_criteria_kernel_parts/040-evaluate-success-criteria-value.rs");

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
