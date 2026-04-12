use crate::contracts::{
    ClarificationReason, ExecutionPosture, OrchestrationPlan, TypedOrchestrationRequest,
};

pub fn apply_recovery_policy(
    _request: &TypedOrchestrationRequest,
    mut plan: OrchestrationPlan,
) -> (OrchestrationPlan, bool) {
    if plan.steps.is_empty() {
        plan.posture = ExecutionPosture::Ask;
        plan.needs_clarification = true;
        plan.clarification_prompt = Some("no executable plan steps were generated".to_string());
        if !plan
            .classification
            .clarification_reasons
            .contains(&ClarificationReason::PlannerGap)
        {
            plan.classification
                .clarification_reasons
                .push(ClarificationReason::PlannerGap);
        }
        plan.classification.needs_clarification = true;
        plan.classification.confidence = plan.classification.confidence.min(0.40);
        plan.classification
            .reasons
            .push("planner:no_executable_steps".to_string());
        return (plan, true);
    }
    (plan, false)
}
