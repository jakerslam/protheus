use crate::contracts::{ExecutionPosture, OrchestrationPlan, OrchestrationRequest};

pub fn apply_recovery_policy(
    _request: &OrchestrationRequest,
    mut plan: OrchestrationPlan,
) -> (OrchestrationPlan, bool) {
    if plan.steps.is_empty() {
        plan.posture = ExecutionPosture::Ask;
        plan.needs_clarification = true;
        plan.clarification_prompt = Some("no executable plan steps were generated".to_string());
        return (plan, true);
    }
    (plan, false)
}
