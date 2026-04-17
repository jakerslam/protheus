// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("seed_protocol_parts/010-state-root.rs");
include!("seed_protocol_parts/020-command-deploy.rs");
include!("seed_protocol_parts/030-command-select.rs");
include!("seed_protocol_parts/040-env-guard.rs");

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
