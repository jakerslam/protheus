// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
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
    CoreExecutionObservation, DegradationReason, ExecutionCorrelation, ExecutionState,
    OrchestrationExecutionObservationUpdate, OrchestrationPlan, OrchestrationRequest,
    OrchestrationResultPackage, PlanCandidate, PlanScore, PlanStatus, PlanVariant,
    RecoveryDecision, RecoveryReason, RecoveryState,
};
use std::collections::BTreeMap;
use transient_context::{TransientContextStore, TransientSleepCleanupReport};

const DEFAULT_SLEEP_CYCLE_IDLE_GAP_MS: u64 = 8 * 60 * 60 * 1000;

#[derive(Debug)]
pub struct OrchestrationSurfaceRuntime {
    transient: TransientContextStore,
    execution_observations: BTreeMap<String, CoreExecutionObservation>,
    last_activity_ms: Option<u64>,
    sleep_cycle_idle_gap_ms: u64,
}

impl Default for OrchestrationSurfaceRuntime {
    fn default() -> Self {
        Self {
            transient: TransientContextStore::default(),
            execution_observations: BTreeMap::new(),
            last_activity_ms: None,
            sleep_cycle_idle_gap_ms: DEFAULT_SLEEP_CYCLE_IDLE_GAP_MS,
        }
    }
}

impl OrchestrationSurfaceRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_sleep_cycle_idle_gap_ms(mut self, idle_gap_ms: u64) -> Self {
        self.sleep_cycle_idle_gap_ms = idle_gap_ms;
        self
    }

    fn maybe_run_sleep_cycle_cleanup(&mut self, now_ms: u64) {
        let Some(last_activity_ms) = self.last_activity_ms else {
            return;
        };
        if now_ms <= last_activity_ms {
            return;
        }
        let idle_gap = now_ms.saturating_sub(last_activity_ms);
        if idle_gap < self.sleep_cycle_idle_gap_ms {
            return;
        }
        let cycle_id = format!("auto_idle_gap_{idle_gap}");
        let _ = self.transient.run_sleep_cycle_cleanup(cycle_id.as_str());
    }

    pub fn orchestrate(
        &mut self,
        request: OrchestrationRequest,
        now_ms: u64,
    ) -> OrchestrationResultPackage {
        self.maybe_run_sleep_cycle_cleanup(now_ms);
        let normalized = ingress::normalize_request(request);
        let mut typed_request = normalized.typed_request.clone();
        if let Some(update) = self
            .execution_observations
            .get(typed_request.session_id.as_str())
            .cloned()
        {
            typed_request.core_execution_observation = Some(update);
        }
        let classification = request_classification::classify_request(&normalized);
        if let Err(err) = self.transient.upsert(
            typed_request.session_id.as_str(),
            transient_summary(&typed_request),
            now_ms,
            30_000,
        ) {
            self.last_activity_ms = Some(now_ms);
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
                            typed_request.session_id
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
            clarification::clarification_prompt_for(&typed_request, &classification);
        let needs_clarification =
            classification.needs_clarification || clarification_prompt.is_some();
        let posture =
            posture::choose_posture(classification.request_class.clone(), needs_clarification);
        let mut plan_candidates =
            sequencing::build_plan_candidates(&typed_request, &classification);
        let selected_plan = plan_candidates
            .first()
            .cloned()
            .unwrap_or_else(|| sequencing::build_plan_candidate(&typed_request, &classification));
        let alternative_plans = if plan_candidates.len() > 1 {
            plan_candidates.drain(1..).collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let execution_state =
            progress::execution_state_for(&typed_request, &selected_plan, needs_clarification);

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
        let (plan, recovery_applied) = recovery::apply_recovery_policy(&typed_request, plan);
        let progress = progress::progress_message(&plan);
        let tool_fallback_context =
            sequencing::tool_fallback_context_from_payload(&typed_request.payload);
        let fallback_actions = sequencing::fallback_actions(
            &typed_request,
            plan.request_class.clone(),
            tool_fallback_context.as_ref(),
        );
        self.last_activity_ms = Some(now_ms);
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

    pub fn run_transient_sleep_cycle_cleanup(
        &mut self,
        sleep_cycle_id: &str,
    ) -> Result<TransientSleepCleanupReport, String> {
        self.transient.run_sleep_cycle_cleanup(sleep_cycle_id)
    }

    pub fn apply_execution_observation_update(
        &mut self,
        update: OrchestrationExecutionObservationUpdate,
    ) {
        let session_id = update.session_id.trim().to_string();
        if session_id.is_empty() {
            return;
        }
        self.execution_observations
            .insert(session_id, update.observation);
    }

    pub fn record_execution_observation(
        &mut self,
        session_id: impl Into<String>,
        observation: CoreExecutionObservation,
    ) {
        self.apply_execution_observation_update(OrchestrationExecutionObservationUpdate {
            session_id: session_id.into(),
            observation,
        });
    }

    pub fn clear_execution_observation(&mut self, session_id: &str) {
        self.execution_observations.remove(session_id);
    }
}

fn transient_summary(request: &crate::contracts::TypedOrchestrationRequest) -> String {
    format!(
        "kind={:?};operation={:?};resource={:?};legacy_intent={}",
        request.request_kind, request.operation_kind, request.resource_kind, request.legacy_intent
    )
    .to_lowercase()
}
