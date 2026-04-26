include!("inversion_parts/010-normalize-impact-input.rs");
include!("inversion_parts/020-normalize-band-map-output.rs");
include!("inversion_parts/030-rel-path-input.rs");
include!("inversion_parts/040-normalize-library-row-input.rs");
include!("inversion_parts/050-compute-normalize-impact.rs");
include!("inversion_parts/060-normalize-token-runtime.rs");
include!("inversion_parts/070-compute-parse-candidate-list-from-llm-payload.rs");
include!("inversion_parts/080-compute-default-tier-scope.rs");
include!("inversion_parts/090-compute-coerce-tier-event-map.rs");
include!("inversion_parts/100-compute-default-first-principle-lock-state.rs");
include!("inversion_parts/110-compute-resolve-parity-confidence.rs");
include!("inversion_parts/120-compute-build-output-interfaces.rs");
include!("inversion_parts/130-compute-read-json.rs");
include!("inversion_parts/140-compute-normalize-harness-suite.rs");
include!("inversion_parts/150-compute-load-active-sessions.rs");
include!("inversion_parts/160-compute-trim-library.rs");
include!("inversion_parts/170-compute-parse-lane-decision.rs");
include!("inversion_parts/180-compute-evaluate-impossibility-trigger.rs");
include!("inversion_parts/190-compute-extract-failure-cluster-principle.rs");
include!("inversion_parts/200-compute-conclave-high-risk-flags.rs");
include!("inversion_parts/210-run-inversion-json.rs");
include!("inversion_parts/220-normalize-impact-matches-expected-set.rs");

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
