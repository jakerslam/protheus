// Split from 030-op-dispatch.rs into focused include parts for maintainability.
// NOTE: this module contains a contiguous function body, so we include a
// single combined expansion unit to keep Rust item parsing valid.
include!("030-op-dispatch_parts/000-combined.rs");
