// Split from action_receipts_kernel.combined.rs into focused include parts for maintainability.
include!("action_receipts_kernel.combined_parts/010-prelude-and-shared.rs");
include!("action_receipts_kernel.combined_parts/020-hmacsha256-to-with-receipt-contract-value.rs");
include!("action_receipts_kernel.combined_parts/030-with-receipt-integrity-value-to-discover-lineage-paths.rs");
include!("action_receipts_kernel.combined_parts/040-read-jsonl-rows-to-lower-compact-type.rs");
include!("action_receipts_kernel.combined_parts/050-replay-task-lineage-value-to-query-task-lineage.rs");
include!("action_receipts_kernel.combined_parts/060-command-to-run.rs");
include!("action_receipts_kernel.combined_parts/070-mod-tests.rs");
