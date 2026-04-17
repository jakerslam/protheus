// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("contract_check_parts/010-run.rs");
include!("contract_check_parts/020-check-rust-source-of-truth-contract.rs");
include!("contract_check_parts/030-is-ts-bootstrap-wrapper.rs");

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
