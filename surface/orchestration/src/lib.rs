pub mod clarification;
pub mod contracts;
pub mod ingress;
pub mod posture;
pub mod progress;
pub mod recovery;
pub mod request_classification;
pub mod result_packaging;
pub mod self_maintenance;
pub mod sequencing;
pub mod transient_context;

use contracts::{OrchestrationPlan, OrchestrationRequest, OrchestrationResultPackage};
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
        if let Err(err) = self.transient.upsert(
            normalized.session_id.as_str(),
            normalized.intent.clone(),
            now_ms,
            30_000,
        ) {
            return OrchestrationResultPackage {
                summary: format!("orchestration_degraded:{err}"),
                progress_message:
                    "Transient orchestration context unavailable; halted before core contract planning"
                        .to_string(),
                recovery_applied: true,
                core_contract_calls: Vec::new(),
                requires_core_promotion: false,
            };
        }

        let request_class = request_classification::classify_request(&normalized);
        let clarification_prompt =
            clarification::clarification_prompt_for(&normalized, request_class.clone());
        let needs_clarification = clarification_prompt.is_some();
        let posture = posture::choose_posture(request_class.clone(), needs_clarification);
        let steps = sequencing::build_steps(&normalized, request_class.clone());

        let plan = OrchestrationPlan {
            request_class,
            posture,
            needs_clarification,
            clarification_prompt,
            steps,
        };
        let (plan, recovery_applied) = recovery::apply_recovery_policy(&normalized, plan);
        let progress = progress::progress_message(&plan);
        result_packaging::package_result(&plan, progress, recovery_applied)
    }

    pub fn sweep_transient(&mut self, now_ms: u64) -> usize {
        self.transient.sweep_expired(now_ms)
    }

    pub fn transient_entry_count(&self) -> usize {
        self.transient.len()
    }
}
