pub mod clarification;
pub mod contracts;
pub mod ingress;
pub mod planner;
pub mod posture;
pub mod progress;
pub mod recovery;
pub mod request_classification;
pub mod result_packaging;
pub mod self_maintenance;
pub mod sequencing;
pub mod transient_context;

use contracts::{
    DegradationReason, ExecutionCorrelation, ExecutionState, OrchestrationPlan,
    OrchestrationRequest, OrchestrationResultPackage, PlanCandidate, PlanScore, PlanStatus,
    PlanVariant, RecoveryDecision, RecoveryReason, RecoveryState,
};
use transient_context::TransientContextStore;

#[derive(Debug, Default)]
pub struct OrchestrationSurfaceRuntime {
    transient: TransientContextStore,
}

impl OrchestrationSurfaceRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn orchestrate(
        &mut self,
        request: OrchestrationRequest,
        now_ms: u64,
    ) -> OrchestrationResultPackage {
        let normalized = ingress::normalize_request(request);
        let classification = request_classification::classify_request(&normalized);
        if let Err(err) = self.transient.upsert(
            normalized.typed_request.session_id.as_str(),
            transient_summary(&normalized.typed_request),
            now_ms,
            30_000,
        ) {
            return OrchestrationResultPackage {
                summary: format!("orchestration_degraded:{err}"),
                progress_message:
                    "Transient orchestration context unavailable; halted before core contract planning"
                        .to_string(),
                execution_state: ExecutionState {
                    plan_status: PlanStatus::Blocked,
                    steps: Vec::new(),
                    recovery: Some(RecoveryState {
                        decision: RecoveryDecision::Halt,
                        reason: Some(RecoveryReason::TransportFailure),
                        retryable: true,
                        note: "transient context unavailable".to_string(),
                    }),
                    degradation: None,
                    correlation: ExecutionCorrelation {
                        orchestration_trace_id: format!(
                            "orch_{}_transient",
                            normalized.typed_request.session_id
                        ),
                        expected_core_contract_ids: Vec::new(),
                        observed_core_receipt_ids: Vec::new(),
                        observed_core_outcome_refs: Vec::new(),
                    },
                },
                recovery_applied: true,
                fallback_actions: Vec::new(),
                core_contract_calls: Vec::new(),
                requires_core_promotion: false,
                classification,
                selected_plan: PlanCandidate {
                    plan_id: "plan_transient_context_failed".to_string(),
                    variant: PlanVariant::ClarificationFirst,
                    steps: Vec::new(),
                    confidence: 0.0,
                    score: PlanScore {
                        overall: 0.0,
                        authority_cost: 0.0,
                        transport_dependency: 0.0,
                        mutation_risk: 0.0,
                        fallback_quality: 0.0,
                        target_specificity: 0.0,
                    },
                    requires_clarification: true,
                    blocked_on: Vec::new(),
                    degradation: vec![DegradationReason::TransportFailure],
                    capabilities: Vec::new(),
                    capability_probes: Vec::new(),
                    reasons: vec!["transient_context_unavailable".to_string()],
                },
                alternative_plans: Vec::new(),
            };
        }

        let clarification_prompt =
            clarification::clarification_prompt_for(&normalized.typed_request, &classification);
        let needs_clarification =
            classification.needs_clarification || clarification_prompt.is_some();
        let posture =
            posture::choose_posture(classification.request_class.clone(), needs_clarification);
        let mut plan_candidates =
            sequencing::build_plan_candidates(&normalized.typed_request, &classification);
        let selected_plan = plan_candidates.first().cloned().unwrap_or_else(|| {
            sequencing::build_plan_candidate(&normalized.typed_request, &classification)
        });
        let alternative_plans = if plan_candidates.len() > 1 {
            plan_candidates.drain(1..).collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let execution_state =
            progress::execution_state_for(&normalized.typed_request, &selected_plan, needs_clarification);

        let plan = OrchestrationPlan {
            request_class: classification.request_class.clone(),
            classification,
            posture,
            needs_clarification,
            clarification_prompt,
            selected_plan,
            alternative_plans,
            execution_state,
        };
        let (plan, recovery_applied) =
            recovery::apply_recovery_policy(&normalized.typed_request, plan);
        let progress = progress::progress_message(&plan);
        let tool_fallback_context =
            sequencing::tool_fallback_context_from_payload(&normalized.typed_request.payload);
        let fallback_actions = sequencing::fallback_actions(
            &normalized.typed_request,
            plan.request_class.clone(),
            tool_fallback_context.as_ref(),
        );
        result_packaging::package_result(&plan, progress, recovery_applied, fallback_actions)
    }

    pub fn sweep_transient(&mut self, now_ms: u64) -> usize {
        self.transient.sweep_expired(now_ms)
    }

    pub fn transient_entry_count(&self) -> usize {
        self.transient.len()
    }

    pub fn transient_ephemeral_count(&self) -> usize {
        self.transient.active_ephemeral_count()
    }

    pub fn begin_transient_restart(&mut self) {
        self.transient.begin_restart();
    }

    pub fn sweep_transient_before_resume(&mut self) -> Result<usize, String> {
        self.transient.sweep_stale_before_resume()
    }

    pub fn resume_transient_after_restart(&mut self) -> Result<(), String> {
        self.transient.resume_after_restart()
    }
}

fn transient_summary(request: &crate::contracts::TypedOrchestrationRequest) -> String {
    format!(
        "kind={:?};operation={:?};resource={:?};legacy_intent={}",
        request.request_kind, request.operation_kind, request.resource_kind, request.legacy_intent
    )
    .to_lowercase()
}
