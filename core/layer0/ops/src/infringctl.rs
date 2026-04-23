include!("infringctl_parts/010-bool-env.rs");
include!("infringctl_parts/015-cli-domains.rs");
include!("infringctl_parts/020-evaluate-dispatch-security.rs");
include!("infringctl_parts/030-usage.rs");
include!("infringctl_parts/040-infringctl-tests.rs");

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
