// Split from stomach_kernel.rs into focused include parts for maintainability.
include!("stomach_kernel_parts/010-prelude-and-shared.rs");
include!("stomach_kernel_parts/020-usage-to-priority-score.rs");
include!("stomach_kernel_parts/030-scored-candidate-rows-to-nexus-force-block-pair-enabled.rs");
include!("stomach_kernel_parts/040-authorize-stomach-command-with-nexus-inner-to-parse-transform.rs");
include!("stomach_kernel_parts/050-cycle-to-purge-cycle.rs");
include!("stomach_kernel_parts/060-retention-cycle-to-run.rs");
include!("stomach_kernel_parts/070-mod-tests.rs");
