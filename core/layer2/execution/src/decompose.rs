include!("decompose_parts/010-default.rs");
include!("decompose_parts/020-default-max-depth.rs");
include!("decompose_parts/030-compose-micro-tasks.rs");
include!("decompose_parts/040-build-queue-rows.rs");
include!("decompose_parts/050-evaluate-route.rs");
include!("decompose_parts/060-evaluate-heroic-gate.rs");
include!("decompose_parts/070-decompose-generates-micro-tasks.rs");
include!("decompose_parts/080-placeholder.rs");

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
