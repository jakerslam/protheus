// AUTO-SPLIT: this file is composed from smaller parts to enforce <=1000 line policy.
include!("orchestration_parts/010-print-json-line.rs");
include!("orchestration_parts/020-validate-finding.rs");
include!("orchestration_parts/030-detect-scope-overlaps.rs");
include!("orchestration_parts/040-load-task-group.rs");
include!("orchestration_parts/050-maybe-checkpoint.rs");
include!("orchestration_parts/060-retrieve-partial-results.rs");
include!("orchestration_parts/070-run-coordinator.rs");
include!("orchestration_parts/080-invoke.rs");
