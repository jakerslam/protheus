// Split from 030-run.rs into focused include parts for maintainability.
// NOTE: this module contains a contiguous function body, so we include a
// single combined expansion unit to keep Rust item parsing valid.
include!("030-run_parts/000-combined.rs");
