// AUTO-SPLIT: batch8 composes the web-facing parallel scrape and news decode lanes.
include!("research_batch8_parts/010-state-root.rs");
include!("research_batch8_parts/020-run-parallel-scrape-workers.rs");
include!("research_batch8_parts/030-run-decode-common.rs");
