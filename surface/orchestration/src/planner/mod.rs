// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
pub mod capability_registry;
pub mod plan_candidates;
pub mod preconditions;
pub mod scoring;

pub use plan_candidates::{build_plan_candidate, build_plan_candidates};
