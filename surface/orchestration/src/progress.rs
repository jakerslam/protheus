use crate::contracts::OrchestrationPlan;

pub fn progress_message(plan: &OrchestrationPlan) -> String {
    let posture = format!("{:?}", plan.posture).to_lowercase();
    format!(
        "orchestration posture={} steps={} clarification={} confidence={:.2}",
        posture,
        plan.steps.len(),
        plan.needs_clarification,
        plan.classification.confidence
    )
}
