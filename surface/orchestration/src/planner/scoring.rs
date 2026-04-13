use crate::contracts::{
    DegradationReason, Mutability, Precondition, RequestClassification, TypedOrchestrationRequest,
};

pub fn score_candidate(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
    step_count: usize,
    blocked_on: &[Precondition],
    degradation: Option<&DegradationReason>,
    requires_clarification: bool,
) -> f32 {
    let mut score = classification.confidence;

    if !request.target_refs.is_empty() {
        score += 0.05;
    }
    if !request.tool_hints.is_empty() {
        score += 0.05;
    }
    if step_count > 2 {
        score -= 0.05;
    }
    if matches!(request.mutability, Mutability::Mutation) {
        score -= 0.05;
    }
    score -= 0.12 * blocked_on.len() as f32;
    if degradation.is_some() {
        score -= 0.10;
    }
    if requires_clarification {
        score -= 0.20;
    }

    score.clamp(0.0, 0.99)
}
