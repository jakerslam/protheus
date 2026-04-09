// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/tools/task_fabric (authoritative task graph primitive).

pub mod concurrency;
pub mod policy;
pub mod query_api;
pub mod status_machine;
pub mod stomach_integration;
pub mod task_graph;

pub use concurrency::{validate_expected_revision, ConcurrencyState, MutationEnvelope, TaskEvent};
pub use policy::{
    enforce_mutation, AllowAllVerityGate, MutationKind, MutationRisk, PolicyDecision, VerityGate,
};
pub use stomach_integration::{
    build_stomach_template, phase_task_id, root_task_id, StomachPhase, StomachTemplateBundle,
    STOMACH_TEMPLATE,
};
pub use task_graph::{
    Blocker, BlockerKind, DependencyEdge, LifecycleStatus, ReadinessStatus, RelatedLink, Task,
    TaskGraph, TaskId,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FabricReceipt {
    pub receipt_id: String,
    pub trace_id: String,
    pub event_id: String,
    pub scope_id: String,
    pub task_id: Option<String>,
    pub timestamp_ms: u64,
    pub mutation_kind: MutationKind,
    pub dna_lineage: Vec<String>,
    pub policy_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NexusConduitRequest {
    pub operation: String,
    pub args: Value,
}

#[derive(Debug, Clone)]
pub struct TaskFabric {
    pub graph: TaskGraph,
    pub events: Vec<TaskEvent>,
    pub receipts: Vec<FabricReceipt>,
    pub concurrency: ConcurrencyState,
    pub stale_after_ms: u64,
}

impl TaskFabric {
    pub fn new(scope_id: impl Into<String>) -> Self {
        Self {
            graph: TaskGraph::new(scope_id),
            events: Vec::new(),
            receipts: Vec::new(),
            concurrency: ConcurrencyState::default(),
            stale_after_ms: 30 * 60 * 1000,
        }
    }

    pub fn submit_task(
        &mut self,
        mut task: Task,
        envelope: MutationEnvelope,
        verity: &dyn VerityGate,
    ) -> Result<TaskEvent, String> {
        if let Some(event) = self.concurrency.idempotent_event(&envelope.idempotency_key) {
            return Ok(event);
        }
        validate_expected_revision(envelope.expected_revision, task.revision_id)?;
        let policy = enforce_mutation(
            &self.graph.scope_id,
            None,
            MutationKind::CreateTask,
            &envelope.payload,
            verity,
        )?;
        task.scope_id = self.graph.scope_id.clone();
        task.updated_at = envelope.now_ms;
        self.graph.insert_task(task.clone())?;
        let event = self.append_event(
            Some(task.id),
            None,
            Some(task.revision_id),
            envelope,
            policy,
        )?;
        Ok(event)
    }

    pub fn add_dependency(
        &mut self,
        edge: DependencyEdge,
        envelope: MutationEnvelope,
        verity: &dyn VerityGate,
    ) -> Result<TaskEvent, String> {
        if let Some(event) = self.concurrency.idempotent_event(&envelope.idempotency_key) {
            return Ok(event);
        }
        let task = self
            .graph
            .task(&edge.task_id)
            .ok_or_else(|| "task_not_found".to_string())?;
        validate_expected_revision(envelope.expected_revision, task.revision_id)?;
        let payload =
            json!({"task_id": edge.task_id, "depends_on_task_id": edge.depends_on_task_id});
        let policy = enforce_mutation(
            &self.graph.scope_id,
            Some(task),
            MutationKind::AddDependency,
            &payload,
            verity,
        )?;
        self.graph.add_dependency(edge.clone())?;
        let next_rev = self.bump_task_revision(&edge.task_id, envelope.now_ms)?;
        self.append_event(
            Some(edge.task_id),
            Some(next_rev.saturating_sub(1)),
            Some(next_rev),
            envelope,
            policy,
        )
    }

    pub fn transition_status(
        &mut self,
        task_id: &str,
        next_status: LifecycleStatus,
        envelope: MutationEnvelope,
        verity: &dyn VerityGate,
    ) -> Result<TaskEvent, String> {
        if let Some(event) = self.concurrency.idempotent_event(&envelope.idempotency_key) {
            return Ok(event);
        }
        let current = self
            .graph
            .task(task_id)
            .ok_or_else(|| "task_not_found".to_string())?;
        validate_expected_revision(envelope.expected_revision, current.revision_id)?;
        let payload = json!({
            "task_id": task_id,
            "next_status": format!("{next_status:?}").to_ascii_lowercase()
        });
        let policy = enforce_mutation(
            &self.graph.scope_id,
            Some(current),
            MutationKind::UpdateStatus,
            &payload,
            verity,
        )?;
        let previous = current.revision_id;
        let next_revision = {
            let task = self
                .graph
                .task_mut(task_id)
                .ok_or_else(|| "task_not_found".to_string())?;
            status_machine::apply_transition(task, next_status, envelope.now_ms)?;
            task.revision_id
        };
        self.append_event(
            Some(task_id.to_string()),
            Some(previous),
            Some(next_revision),
            envelope,
            policy,
        )
    }

    pub fn claim_lease(
        &mut self,
        task_id: &str,
        assignee: &str,
        lease_ms: u64,
        envelope: MutationEnvelope,
        verity: &dyn VerityGate,
    ) -> Result<TaskEvent, String> {
        if let Some(event) = self.concurrency.idempotent_event(&envelope.idempotency_key) {
            return Ok(event);
        }
        let current = self
            .graph
            .task(task_id)
            .ok_or_else(|| "task_not_found".to_string())?;
        validate_expected_revision(envelope.expected_revision, current.revision_id)?;
        let payload = json!({
            "task_id": task_id,
            "assignee": assignee,
            "lease_ms": lease_ms
        });
        let policy = enforce_mutation(
            &self.graph.scope_id,
            Some(current),
            MutationKind::ClaimLease,
            &payload,
            verity,
        )?;
        let previous = current.revision_id;
        let next_revision = {
            let task = self
                .graph
                .task_mut(task_id)
                .ok_or_else(|| "task_not_found".to_string())?;
            task.assignee = Some(assignee.to_string());
            task.last_heartbeat_at = Some(envelope.now_ms);
            task.lease_expires_at = Some(envelope.now_ms.saturating_add(lease_ms));
            task.updated_at = envelope.now_ms;
            task.revision_id = task.revision_id.saturating_add(1);
            if task.lifecycle_status == LifecycleStatus::Pending {
                task.lifecycle_status = LifecycleStatus::InProgress;
                task.started_at.get_or_insert(envelope.now_ms);
            }
            task.revision_id
        };
        self.append_event(
            Some(task_id.to_string()),
            Some(previous),
            Some(next_revision),
            envelope,
            policy,
        )
    }

    pub fn heartbeat(
        &mut self,
        task_id: &str,
        extend_lease_ms: u64,
        envelope: MutationEnvelope,
        verity: &dyn VerityGate,
    ) -> Result<TaskEvent, String> {
        if let Some(event) = self.concurrency.idempotent_event(&envelope.idempotency_key) {
            return Ok(event);
        }
        let current = self
            .graph
            .task(task_id)
            .ok_or_else(|| "task_not_found".to_string())?;
        validate_expected_revision(envelope.expected_revision, current.revision_id)?;
        let policy = enforce_mutation(
            &self.graph.scope_id,
            Some(current),
            MutationKind::Heartbeat,
            &envelope.payload,
            verity,
        )?;
        let previous = current.revision_id;
        let next_revision = {
            let task = self
                .graph
                .task_mut(task_id)
                .ok_or_else(|| "task_not_found".to_string())?;
            task.last_heartbeat_at = Some(envelope.now_ms);
            if let Some(ts) = task.lease_expires_at {
                task.lease_expires_at =
                    Some(ts.max(envelope.now_ms).saturating_add(extend_lease_ms));
            }
            task.updated_at = envelope.now_ms;
            task.revision_id = task.revision_id.saturating_add(1);
            task.revision_id
        };
        self.append_event(
            Some(task_id.to_string()),
            Some(previous),
            Some(next_revision),
            envelope,
            policy,
        )
    }

    pub fn handoff(
        &mut self,
        task_id: &str,
        next_assignee: &str,
        envelope: MutationEnvelope,
        verity: &dyn VerityGate,
    ) -> Result<TaskEvent, String> {
        if let Some(event) = self.concurrency.idempotent_event(&envelope.idempotency_key) {
            return Ok(event);
        }
        let current = self
            .graph
            .task(task_id)
            .ok_or_else(|| "task_not_found".to_string())?;
        validate_expected_revision(envelope.expected_revision, current.revision_id)?;
        let payload = json!({"task_id": task_id, "next_assignee": next_assignee});
        let policy = enforce_mutation(
            &self.graph.scope_id,
            Some(current),
            MutationKind::Handoff,
            &payload,
            verity,
        )?;
        let previous = current.revision_id;
        let next_revision = {
            let task = self
                .graph
                .task_mut(task_id)
                .ok_or_else(|| "task_not_found".to_string())?;
            task.assignee = Some(next_assignee.to_string());
            task.lease_expires_at = None;
            task.last_heartbeat_at = Some(envelope.now_ms);
            task.updated_at = envelope.now_ms;
            task.revision_id = task.revision_id.saturating_add(1);
            task.revision_id
        };
        self.append_event(
            Some(task_id.to_string()),
            Some(previous),
            Some(next_revision),
            envelope,
            policy,
        )
    }

    pub fn integrate_stomach_template(
        &mut self,
        item_key: &str,
        owner: Option<String>,
        assignee: Option<String>,
        envelope: MutationEnvelope,
        verity: &dyn VerityGate,
    ) -> Result<Vec<TaskEvent>, String> {
        if self.graph.task(&root_task_id(item_key)).is_some() {
            return Ok(Vec::new());
        }
        let bundle = build_stomach_template(
            &self.graph.scope_id,
            item_key,
            owner,
            assignee,
            envelope.now_ms,
        );
        let mut out = Vec::<TaskEvent>::new();
        out.push(self.submit_task(
            bundle.root.clone(),
            MutationEnvelope {
                idempotency_key: format!("{}::root", envelope.idempotency_key),
                mutation_kind: MutationKind::CreateTask,
                ..envelope.clone()
            },
            verity,
        )?);
        for phase in bundle.phases {
            out.push(self.submit_task(
                phase.clone(),
                MutationEnvelope {
                    idempotency_key: format!("{}::task::{}", envelope.idempotency_key, phase.id),
                    mutation_kind: MutationKind::CreateTask,
                    ..envelope.clone()
                },
                verity,
            )?);
            self.graph.set_parent(&phase.id, &root_task_id(item_key))?;
        }
        for edge in bundle.dependencies {
            out.push(self.add_dependency(
                edge.clone(),
                MutationEnvelope {
                    idempotency_key: format!(
                        "{}::dep::{}::{}",
                        envelope.idempotency_key, edge.task_id, edge.depends_on_task_id
                    ),
                    mutation_kind: MutationKind::AddDependency,
                    ..envelope.clone()
                },
                verity,
            )?);
        }
        Ok(out)
    }

    pub fn query_via_hierarchical_nexus(
        &self,
        request: NexusConduitRequest,
        now_ms: u64,
    ) -> Result<Value, String> {
        query_api::dispatch_named_query(
            &self.graph,
            &request.operation,
            &request.args,
            now_ms,
            self.stale_after_ms,
        )
    }

    fn bump_task_revision(&mut self, task_id: &str, now_ms: u64) -> Result<u64, String> {
        let task = self
            .graph
            .task_mut(task_id)
            .ok_or_else(|| "task_not_found".to_string())?;
        task.updated_at = now_ms;
        task.revision_id = task.revision_id.saturating_add(1);
        Ok(task.revision_id)
    }

    fn append_event(
        &mut self,
        task_id: Option<String>,
        previous_revision: Option<u64>,
        next_revision: Option<u64>,
        envelope: MutationEnvelope,
        policy: PolicyDecision,
    ) -> Result<TaskEvent, String> {
        let sequence = self.concurrency.allocate_event_sequence();
        let event_id = deterministic_hash(&json!({
            "scope_id": self.graph.scope_id,
            "sequence": sequence,
            "task_id": task_id,
            "trace_id": envelope.trace_id,
            "mutation_kind": format!("{:?}", envelope.mutation_kind).to_ascii_lowercase(),
            "timestamp_ms": envelope.now_ms
        }));
        let receipt_id = deterministic_hash(&json!({
            "kind": "task_fabric_receipt_v1",
            "event_id": event_id,
            "trace_id": envelope.trace_id,
            "scope_id": self.graph.scope_id
        }));
        let mut dna_lineage = vec![format!("scope:{}", self.graph.scope_id)];
        if let Some(task) = task_id.as_ref() {
            dna_lineage.push(format!("task:{task}"));
        }
        dna_lineage.push(format!(
            "mutation:{}",
            format!("{:?}", envelope.mutation_kind).to_ascii_lowercase()
        ));
        let event = TaskEvent {
            event_id: event_id.clone(),
            event_sequence: sequence,
            task_id: task_id.clone(),
            scope_id: self.graph.scope_id.clone(),
            mutation_kind: envelope.mutation_kind,
            actor: envelope.actor.clone(),
            trace_id: envelope.trace_id.clone(),
            idempotency_key: envelope.idempotency_key.clone(),
            previous_revision,
            next_revision,
            timestamp_ms: envelope.now_ms,
            policy: policy.clone(),
            dna_lineage: dna_lineage.clone(),
            receipt_id: receipt_id.clone(),
            payload: envelope.payload.clone(),
        };
        let receipt = FabricReceipt {
            receipt_id,
            trace_id: envelope.trace_id,
            event_id,
            scope_id: self.graph.scope_id.clone(),
            task_id,
            timestamp_ms: envelope.now_ms,
            mutation_kind: envelope.mutation_kind,
            dna_lineage,
            policy_reason: policy.reason_code,
        };
        self.events.push(event.clone());
        self.receipts.push(receipt);
        self.concurrency.record_event(event.clone());
        Ok(event)
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn deterministic_hash(value: &Value) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    let mut hasher = Sha256::new();
    hasher.update(&payload);
    format!("{:x}", hasher.finalize())
}
