// Split from queue_sqlite_kernel.combined.rs into focused include parts for maintainability.
include!("queue_sqlite_kernel.combined_parts/010-prelude-and-shared.rs");
include!("queue_sqlite_kernel.combined_parts/020-sqlitecfg-to-cfg-to-value.rs");
include!("queue_sqlite_kernel.combined_parts/030-ensure-parent-to-read-jsonl-rows.rs");
include!("queue_sqlite_kernel.combined_parts/040-migrate-history-to-upsert-item.rs");
include!("queue_sqlite_kernel.combined_parts/050-append-event-to-backpressure-policy.rs");
include!("queue_sqlite_kernel.combined_parts/060-run.rs");
include!("queue_sqlite_kernel.combined_parts/070-mod-tests.rs");
