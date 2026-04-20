// Split from f100_reliability_certification.combined.rs into focused include parts for maintainability.
include!("f100_reliability_certification.combined_parts/010-prelude-and-shared.rs");
include!("f100_reliability_certification.combined_parts/020-lane-id-to-collect-incident-rate.rs");
include!("f100_reliability_certification.combined_parts/030-collect-change-fail-rate-to-evidence-status.rs");
include!("f100_reliability_certification.combined_parts/040-load-policy-to-append-jsonl.rs");
include!("f100_reliability_certification.combined_parts/050-evaluate.rs");
include!("f100_reliability_certification.combined_parts/060-cmd-to-run.rs");
include!("f100_reliability_certification.combined_parts/070-mod-tests.rs");
