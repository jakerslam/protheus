include!("canyon_plane_extensions_parts/010-footprint-path.rs");
include!("canyon_plane_extensions_parts/020-lazy-substrate-command.rs");
include!("canyon_plane_extensions_parts/030-release-pipeline-command.rs");
include!("canyon_plane_extensions_parts/040-package-release-command.rs");

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
