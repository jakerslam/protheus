// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
// Planner subdomain: decomposition + candidate generation for control-plane sequencing.
pub mod capability_registry;
pub mod plan_candidates;
pub mod preconditions;
pub mod scoring;

pub use plan_candidates::{
    build_plan_candidate, build_plan_candidates, propose_decomposition_candidate,
    propose_decomposition_candidate_with_template, propose_decomposition_candidates,
    propose_decomposition_candidates_with_template,
};
