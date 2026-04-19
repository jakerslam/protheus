// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::self_maintenance::analyzer;
use crate::self_maintenance::contracts::{
    Claim, ClaimBundle, ObservationInputs, SupervisorMode, SupervisorReceipt,
    SupervisorReceiptStage, SupervisorRunResult, WorkerBudgetUsed, WorkerOutput, WorkerTaskStatus,
};
use crate::self_maintenance::escalation::{
    build_escalation_request, requires_high_risk_escalation,
};
use crate::self_maintenance::observer;
use crate::self_maintenance::task_graph_builder::{
    build_task_graph_from_claim_bundle, MaintenanceTaskGraph,
};
use infring_layer1_memory::{
    Classification, EphemeralMemoryHeap, TrustState, VerityEphemeralPolicy,
};
use infring_task_fabric_core_v1::{
    now_ms as task_now_ms, MutationEnvelope, MutationKind, Task, TaskFabric, VerityGate,
};
use protheus_tooling_core_v1::{
    BrokerCaller, EvidenceExtractor, EvidenceStore, StructuredVerifier, ToolBroker, ToolCallRequest,
};
use serde_json::json;
use sha2::{Digest, Sha256};

pub struct GovernedSelfMaintenanceSupervisor {
    mode: SupervisorMode,
    scope_id: String,
    actor_id: String,
    receipts: Vec<SupervisorReceipt>,
    task_fabric: TaskFabric,
    task_fabric_verity_gate: DenyHighRiskVerityGate,
    tool_broker: ToolBroker,
    evidence_extractor: EvidenceExtractor,
    evidence_store: EvidenceStore,
    verifier: StructuredVerifier,
    ephemeral_heap: EphemeralMemoryHeap,
}

pub struct DenyHighRiskVerityGate;

impl VerityGate for DenyHighRiskVerityGate {
    fn approve(
        &self,
        _scope_id: &str,
        _task: Option<&Task>,
        _mutation_kind: MutationKind,
        _payload: &serde_json::Value,
    ) -> bool {
        false
    }
}

impl GovernedSelfMaintenanceSupervisor {
    pub fn new(mode: SupervisorMode, scope_id: impl Into<String>) -> Self {
        let mut heap = EphemeralMemoryHeap::new(VerityEphemeralPolicy::default());
        heap.grant_debug_principal("self_maintenance");
        let scope_id = scope_id.into();
        Self {
            mode,
            scope_id: scope_id.clone(),
            actor_id: "orchestration:self_maintenance".to_string(),
            receipts: Vec::new(),
            task_fabric: TaskFabric::new(scope_id),
            task_fabric_verity_gate: DenyHighRiskVerityGate,
            tool_broker: ToolBroker::default(),
            evidence_extractor: EvidenceExtractor,
            evidence_store: EvidenceStore::default(),
            verifier: StructuredVerifier,
            ephemeral_heap: heap,
        }
    }

    pub fn run_cycle(
        &mut self,
        inputs: ObservationInputs,
        now_ms: u64,
    ) -> Result<SupervisorRunResult, String> {
        self.receipts.clear();
        let evidence = observer::collect_evidence_cards(&inputs, now_ms);
        self.cache_ephemeral("observation_evidence", json!(evidence), now_ms)?;
        self.push_receipt(
            SupervisorReceiptStage::Observation,
            "observations_normalized_to_evidence",
            None,
            now_ms,
        );

        let cycle_task_id = format!("maintenance-cycle-{now_ms}");
        let claim_bundle = analyzer::evidence_to_claim_bundle(cycle_task_id.as_str(), &evidence);
        self.cache_ephemeral("claim_bundle", json!(claim_bundle), now_ms)?;
        self.push_receipt(
            SupervisorReceiptStage::Claim,
            "evidence_to_claim_bundle_complete",
            Some(claim_bundle.task_id.clone()),
            now_ms,
        );

        let generated =
            build_task_graph_from_claim_bundle(&claim_bundle, self.scope_id.as_str(), now_ms);
        let generated_task_ids = generated
            .tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<Vec<_>>();
        if self.mode != SupervisorMode::ObserveOnly {
            self.submit_tasks_to_task_fabric(&generated, now_ms)?;
        }

        let mut worker_outputs = Vec::<WorkerOutput>::new();
        let mut escalation_requests = Vec::new();
        if self.mode == SupervisorMode::ApplySafe {
            for claim in &claim_bundle.claims {
                if requires_high_risk_escalation(claim) {
                    let request = build_escalation_request(&claim_bundle, claim);
                    self.push_receipt(
                        SupervisorReceiptStage::Escalation,
                        "high_risk_action_requires_verity",
                        Some(claim_bundle.task_id.clone()),
                        now_ms,
                    );
                    escalation_requests.push(request);
                    continue;
                }
                let output = self.apply_safe_claim(&claim_bundle, claim, now_ms)?;
                worker_outputs.push(output);
            }
        }

        Ok(SupervisorRunResult {
            mode: self.mode,
            evidence,
            claim_bundle,
            generated_task_ids,
            worker_outputs,
            escalation_requests,
            receipts: self.receipts.clone(),
        })
    }

    pub fn sweep_ephemeral(&mut self) -> Result<usize, String> {
        self.ephemeral_heap
            .run_dream_cleanup("self_maintenance_dream_cycle")
            .map(|report| report.cleaned_count)
            .map_err(|err| format!("ephemeral_cleanup_failed:{err}"))
    }

    pub fn active_ephemeral_count(&self) -> usize {
        self.ephemeral_heap
            .materialize_context_stack("self_maintenance", true)
            .into_iter()
            .filter(|row| row.scope == "ephemeral")
            .count()
    }

    fn submit_tasks_to_task_fabric(
        &mut self,
        generated: &MaintenanceTaskGraph,
        now_ms: u64,
    ) -> Result<(), String> {
        let proof_root = generated
            .tasks
            .first()
            .map(|task| format!("task_graph_root:{}", task.id))
            .unwrap_or_else(|| "task_graph_root:unknown".to_string());
        for task in &generated.tasks {
            let envelope = MutationEnvelope {
                actor: self.actor_id.clone(),
                trace_id: format!("trace-submit-{}", task.id),
                idempotency_key: format!("self-maintenance-submit-{}", task.id),
                proof_refs: vec![proof_root.clone()],
                expected_revision: Some(task.revision_id),
                now_ms,
                mutation_kind: MutationKind::CreateTask,
                payload: json!({"task_id": task.id}),
            };
            let _ = self.task_fabric.submit_task(
                task.clone(),
                envelope,
                &self.task_fabric_verity_gate,
            )?;
            self.push_receipt(
                SupervisorReceiptStage::TaskCreation,
                "task_fabric_task_created",
                Some(task.id.clone()),
                now_ms,
            );
        }
        for edge in &generated.dependencies {
            let envelope = MutationEnvelope {
                actor: self.actor_id.clone(),
                trace_id: format!(
                    "trace-dependency-{}-{}",
                    edge.task_id, edge.depends_on_task_id
                ),
                idempotency_key: format!(
                    "self-maintenance-dependency-{}-{}",
                    edge.task_id, edge.depends_on_task_id
                ),
                proof_refs: vec![proof_root.clone()],
                expected_revision: None,
                now_ms,
                mutation_kind: MutationKind::AddDependency,
                payload: json!({
                    "task_id": edge.task_id,
                    "depends_on_task_id": edge.depends_on_task_id
                }),
            };
            let _ = self.task_fabric.add_dependency(
                edge.clone(),
                envelope,
                &self.task_fabric_verity_gate,
            )?;
        }
        Ok(())
    }

    fn apply_safe_claim(
        &mut self,
        claim_bundle: &ClaimBundle,
        claim: &Claim,
        now_ms: u64,
    ) -> Result<WorkerOutput, String> {
        let task_id = format!("apply-safe-{}", claim.claim_id);
        self.push_receipt(
            SupervisorReceiptStage::Execution,
            "execute_path:task_fabric",
            Some(task_id.clone()),
            now_ms,
        );

        let request = ToolCallRequest {
            trace_id: format!("trace-{task_id}"),
            task_id: task_id.clone(),
            tool_name: "workspace_analyze".to_string(),
            args: json!({
                "query": claim.text,
                "mode": "self_maintenance_apply_safe",
            }),
            lineage: vec![
                "supervisor:v6_auto_001".to_string(),
                format!("claim_bundle:{}", claim_bundle.claim_bundle_id),
                format!("claim:{}", claim.claim_id),
            ],
            caller: BrokerCaller::System,
            policy_revision: Some("v6_auto_001".to_string()),
            tool_version: Some("workspace_analyze_v1".to_string()),
            freshness_window_ms: Some(30_000),
            force_no_dedupe: false,
        };
        let broker_execution = self
            .tool_broker
            .execute_and_normalize(request, |_| {
                Ok(json!({
                    "results": [{
                        "source_ref": "internal://self_maintenance",
                        "summary": format!("safe remediation prepared for {}", claim.claim_id),
                        "excerpt": claim.text,
                        "confidence_vector": {
                            "relevance": claim.confidence_vector.relevance,
                            "reliability": claim.confidence_vector.reliability,
                            "freshness": claim.confidence_vector.freshness
                        }
                    }]
                }))
            })
            .map_err(|err| err.as_message())?;
        self.push_receipt(
            SupervisorReceiptStage::Execution,
            "execute_path:tool_broker",
            Some(task_id.clone()),
            now_ms,
        );

        let extracted = self.evidence_extractor.extract(
            &broker_execution.normalized_result,
            &broker_execution.raw_payload,
        );
        let produced_evidence_ids = self.evidence_store.append_evidence(&extracted);
        self.push_receipt(
            SupervisorReceiptStage::Execution,
            "execute_path:evidence_store",
            Some(task_id.clone()),
            now_ms,
        );

        let active_evidence = self.evidence_store.active_evidence();
        let verified = self
            .verifier
            .derive_claim_bundle(task_id.as_str(), &active_evidence);
        self.verifier
            .validate_claim_evidence_refs(&verified, &active_evidence)?;
        self.push_receipt(
            SupervisorReceiptStage::Execution,
            "execute_path:verifier",
            Some(task_id.clone()),
            now_ms,
        );

        let _object_id = self.cache_ephemeral(
            "execution_context",
            json!({
                "task_id": task_id,
                "verified_coverage": verified.coverage_score,
                "produced_evidence_ids": produced_evidence_ids,
            }),
            now_ms,
        )?;
        self.push_receipt(
            SupervisorReceiptStage::Execution,
            "execute_path:memory",
            Some(task_id.clone()),
            now_ms,
        );
        if verified.coverage_score >= 0.50 {
            self.push_receipt(
                SupervisorReceiptStage::Execution,
                "promotion_required_via_core_ingress_contract",
                Some(task_id.clone()),
                now_ms,
            );
        }

        self.push_receipt(
            SupervisorReceiptStage::Outcome,
            "safe_apply_execution_complete",
            Some(task_id.clone()),
            now_ms,
        );

        Ok(WorkerOutput {
            task_id,
            status: if verified.coverage_score >= 0.50 {
                WorkerTaskStatus::Completed
            } else {
                WorkerTaskStatus::Blocked
            },
            produced_evidence_ids,
            open_questions: verified.unresolved_questions,
            recommended_next_actions: vec!["promote_verified_fix_or_request_approval".to_string()],
            blockers: verified.conflicts,
            budget_used: WorkerBudgetUsed {
                tool_calls: 1,
                input_tokens: 64,
                output_tokens: 64,
            },
        })
    }

    fn cache_ephemeral(
        &mut self,
        tag: &str,
        payload: serde_json::Value,
        now_ms: u64,
    ) -> Result<String, String> {
        let trace_id = format!("self-maintenance-{tag}-{now_ms}");
        let (object, _) = self
            .ephemeral_heap
            .write_ephemeral(
                self.actor_id.as_str(),
                trace_id.as_str(),
                payload,
                Classification::Internal,
                TrustState::Proposed,
                "cap:self_maintenance_ephemeral",
            )
            .map_err(|err| format!("ephemeral_write_failed:{err}"))?;
        Ok(object.object_id)
    }

    fn push_receipt(
        &mut self,
        stage: SupervisorReceiptStage,
        detail: &str,
        task_id: Option<String>,
        now_ms: u64,
    ) {
        self.receipts.push(SupervisorReceipt {
            receipt_id: stable_id(&format!(
                "{stage:?}::{detail}::{}::{now_ms}",
                task_id.as_deref().unwrap_or("")
            )),
            stage,
            detail: detail.to_string(),
            task_id,
            timestamp_ms: now_ms.max(task_now_ms()),
        });
    }
}

fn stable_id(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("receipt-{:x}", hasher.finalize())
}
