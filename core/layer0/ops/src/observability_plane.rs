// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("observability_plane_parts/010-usage.rs");
include!("observability_plane_parts/020-run-monitor.rs");
include!("observability_plane_parts/030-run-incident.rs");
include!("observability_plane_parts/035-incident-external-dispatch.rs");
include!("observability_plane_parts/040-run-acp-provenance.rs");

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
