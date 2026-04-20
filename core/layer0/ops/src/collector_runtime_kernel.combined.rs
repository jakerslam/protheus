// Split from collector_runtime_kernel.combined.rs into focused include parts for maintainability.
include!("collector_runtime_kernel.combined_parts/010-prelude-and-shared.rs");
include!("collector_runtime_kernel.combined_parts/020-default-rate-state-rel-to-default-u64-from-env.rs");
include!("collector_runtime_kernel.combined_parts/030-resolve-rate-state-path-to-row-u64.rs");
include!("collector_runtime_kernel.combined_parts/040-set-row-u64-to-mark-failure.rs");
include!("collector_runtime_kernel.combined_parts/050-prepare-run-to-sample-title.rs");
include!("collector_runtime_kernel.combined_parts/060-finalize-run-to-dispatch.rs");
include!("collector_runtime_kernel.combined_parts/070-run-to-mod-collector-runtime-kernel-tests.rs");
