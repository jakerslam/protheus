// Split from v8_kernel.combined.rs into focused include parts for maintainability.
include!("v8_kernel.combined_parts/010-prelude-and-shared.rs");
include!("v8_kernel.combined_parts/020-default-receipt-history-max-bytes-to-append-binary-queue.rs");
include!("v8_kernel.combined_parts/030-enforce-jsonl-tail-limit-to-date-or-today.rs");
include!("v8_kernel.combined_parts/040-parse-i64-str-to-deterministic-merkle-root.rs");
include!("v8_kernel.combined_parts/050-merkle-proof-to-parse-csv-or-file-unique.rs");
include!("v8_kernel.combined_parts/060-mod-tests.rs");
