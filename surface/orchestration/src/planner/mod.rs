// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
pub mod capability_registry;
pub mod planner;
pub mod preconditions;
pub mod scoring;

pub use planner::{build_plan_candidate, build_plan_candidates};
