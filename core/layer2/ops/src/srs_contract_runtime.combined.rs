// Split from srs_contract_runtime.combined.rs into focused include parts for maintainability.
include!("srs_contract_runtime.combined_parts/010-prelude-and-shared.rs");
include!("srs_contract_runtime.combined_parts/020-contract-root-to-runtime-lane-to-domain.rs");
include!("srs_contract_runtime.combined_parts/030-parse-runtime-lane-argv-to-execute-contract.rs");
include!("srs_contract_runtime.combined_parts/040-execute-contract-with-options-to-usage.rs");
include!("srs_contract_runtime.combined_parts/050-run.rs");
include!("srs_contract_runtime.combined_parts/060-mod-tests.rs");
