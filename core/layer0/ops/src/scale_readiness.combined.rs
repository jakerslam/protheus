// Split from scale_readiness.combined.rs into focused include parts for maintainability.
include!("scale_readiness.combined_parts/010-prelude-and-shared.rs");
include!("scale_readiness.combined_parts/020-scale-ids-to-default-policy.rs");
include!("scale_readiness.combined_parts/030-load-policy-to-json-script.rs");
include!("scale_readiness.combined_parts/040-synth-load-summary.rs");
include!("scale_readiness.combined_parts/050-lane-scale.rs");
include!("scale_readiness.combined_parts/060-write-lane-receipt-to-usage.rs");
include!("scale_readiness.combined_parts/070-run-to-mod-tests.rs");
