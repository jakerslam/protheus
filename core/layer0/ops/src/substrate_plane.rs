// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("substrate_plane_parts/010-usage.rs");
include!("substrate_plane_parts/020-run-csi-module.rs");
include!("substrate_plane_parts/030-run-csi-policy.rs");
include!("substrate_plane_parts/040-run-bio-feedback.rs");
include!("substrate_plane_parts/050-run-bioethics-policy.rs");

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
