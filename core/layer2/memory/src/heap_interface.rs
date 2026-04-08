use crate::context_materializer::{materialize_context, ContextMaterialization};
use crate::graph_subsystem::{GraphEdge, GraphNode, GraphSubsystem, TaskFabricLease};
use crate::policy::{MemoryPolicyDecision, MemoryPolicyGate, MemoryPolicyRequest, PolicyAction};
use crate::promotion::{is_valid_trust_transition, rollback_head_from_version};
use crate::record_store::RecordStore;
use crate::schemas::{
    CanonicalMemoryRecord, CapabilityAction, CapabilityToken, MemoryMutationReplayRow,
    MemoryObject, MemoryPurgeRecord, MemoryReceipt, MemoryScope, MemoryVersion, OwnerScopeSettings,
    TrustState,
};
use crate::version_ledger::VersionLedger;
use crate::{deterministic_hash, now_ms, BlobStore, VectorIndex};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[path = "heap_workflows.rs"]
mod workflows;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NexusRouteContext {
    pub issuer: String,
    pub source: String,
    pub target: String,
    pub schema_id: String,
    pub lease_id: String,
    pub template_version_id: Option<String>,
    pub ttl_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnifiedMemoryHeapConfig {
    pub owner_settings: OwnerScopeSettings,
}

impl Default for UnifiedMemoryHeapConfig {
    fn default() -> Self {
        Self {
            owner_settings: OwnerScopeSettings::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnifiedMemoryHeap<P: MemoryPolicyGate + Clone> {
    policy: P,
    config: UnifiedMemoryHeapConfig,
    record_store: RecordStore,
    version_ledger: VersionLedger,
    graph_subsystem: GraphSubsystem,
    vector_index: VectorIndex,
    blob_store: BlobStore,
    receipts: Vec<MemoryReceipt>,
}

impl<P: MemoryPolicyGate + Clone> UnifiedMemoryHeap<P> {
    pub fn new(policy: P) -> Self {
        Self::with_config(policy, UnifiedMemoryHeapConfig::default())
    }

    pub fn with_config(policy: P, config: UnifiedMemoryHeapConfig) -> Self {
        Self {
            policy,
            config,
            record_store: RecordStore::default(),
            version_ledger: VersionLedger::default(),
            graph_subsystem: GraphSubsystem::default(),
            vector_index: VectorIndex::default(),
            blob_store: BlobStore::default(),
            receipts: Vec::new(),
        }
    }

    pub fn receipts(&self) -> &[MemoryReceipt] {
        self.receipts.as_slice()
    }

    pub fn owner_settings(&self) -> &OwnerScopeSettings {
        &self.config.owner_settings
    }

    pub fn record_store(&self) -> &RecordStore {
        &self.record_store
    }

    pub fn graph_subsystem(&self) -> &GraphSubsystem {
        &self.graph_subsystem
    }

    pub fn vector_index(&self) -> &VectorIndex {
        &self.vector_index
    }

    pub fn blob_store(&self) -> &BlobStore {
        &self.blob_store
    }

    pub fn replay_mutation_rows(&self) -> Vec<MemoryMutationReplayRow> {
        self.version_ledger.replay_rows()
    }

    pub fn purge_records(&self) -> &[MemoryPurgeRecord] {
        self.version_ledger.purge_records()
    }

    pub fn canonical_head_record(
        &self,
        principal_id: &str,
        capability: &CapabilityToken,
        object_id: &str,
    ) -> Result<Option<CanonicalMemoryRecord>, String> {
        let Some(object) = self.record_store.get_object(object_id).cloned() else {
            return Ok(None);
        };
        let read_decision = self.policy.evaluate(&MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action: PolicyAction::Read,
            source_scope: object.scope.clone(),
            target_scope: None,
            trust_state: None,
            capability: Some(capability.clone()),
            owner_settings: self.config.owner_settings.clone(),
        });
        if !read_decision.allow {
            return Err(format!("canonical_record_denied:{}", read_decision.reason));
        }
        let Some(head_version_id) = self.record_store.head_version_id(object_id) else {
            return Ok(None);
        };
        if self.version_ledger.is_purged(head_version_id.as_str()) {
            return Ok(None);
        }
        let Some(head) = self.version_ledger.get(head_version_id.as_str()).cloned() else {
            return Ok(None);
        };
        Ok(Some(CanonicalMemoryRecord {
            record_id: format!("record:{}:{}", head.object_id, head.version_id),
            object_id: head.object_id,
            version_id: head.version_id,
            scope: head.scope,
            classification: object.classification,
            trust_state: head.trust_state,
            capability_action: CapabilityAction::Read,
            capability_token_id: capability.token_id.clone(),
            payload: head.payload,
            metadata: object.metadata,
            timestamp_ms: head.timestamp_ms,
        }))
    }

    pub fn write_memory_object(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        mut object: MemoryObject,
        trust_state: TrustState,
        lineage_refs: Vec<String>,
    ) -> Result<MemoryVersion, String> {
        self.ensure_routed(route)?;
        if object.object_id.trim().is_empty() {
            object.object_id = format!(
                "object_{}",
                &deterministic_hash(&(
                    object.scope.label(),
                    object.namespace.clone(),
                    object.key.clone(),
                    now_ms()
                ))[..24]
            );
        }
        let write_decision = self.evaluate_policy(MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action: PolicyAction::Write,
            source_scope: object.scope.clone(),
            target_scope: None,
            trust_state: Some(trust_state.clone()),
            capability: Some(capability.clone()),
            owner_settings: self.config.owner_settings.clone(),
        });
        if !write_decision.allow {
            return Err(format!("memory_write_denied:{}", write_decision.reason));
        }

        let ts = now_ms();
        let existing_head = self.record_store.head_version_id(&object.object_id);
        let receipt = self.push_receipt(
            route,
            "memory_write",
            write_decision,
            lineage_refs.clone(),
            json!({
                "object_id": object.object_id,
                "scope": object.scope.label(),
            }),
        );
        object.updated_at_ms = ts;
        if object.created_at_ms == 0 {
            object.created_at_ms = ts;
        }
        self.record_store.upsert_object(object.clone());

        let payload_hash = deterministic_hash(&(object.payload.clone(), object.scope.label()));
        let version = MemoryVersion {
            version_id: format!(
                "version_{}",
                &deterministic_hash(&(
                    object.object_id.clone(),
                    existing_head.clone(),
                    payload_hash.clone(),
                    ts
                ))[..24]
            ),
            object_id: object.object_id.clone(),
            scope: object.scope,
            parent_version_id: existing_head,
            lineage_refs,
            receipt_id: receipt.receipt_id.clone(),
            trust_state,
            payload: object.payload,
            payload_hash,
            timestamp_ms: ts,
            proposed_by: principal_id.to_string(),
        };
        self.version_ledger.append(version.clone())?;
        self.record_store
            .register_version(&version.object_id, &version.version_id);
        self.record_store
            .set_head_version(&version.object_id, &version.version_id);
        Ok(version)
    }

    pub fn read_head_version(
        &self,
        principal_id: &str,
        capability: &CapabilityToken,
        object_id: &str,
    ) -> Result<Option<MemoryVersion>, String> {
        let Some(object) = self.record_store.get_object(object_id) else {
            return Ok(None);
        };
        let read_decision = self.policy.evaluate(&MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action: PolicyAction::Read,
            source_scope: object.scope.clone(),
            target_scope: None,
            trust_state: None,
            capability: Some(capability.clone()),
            owner_settings: self.config.owner_settings.clone(),
        });
        if !read_decision.allow {
            return Err(format!("memory_read_denied:{}", read_decision.reason));
        }
        let Some(head_version_id) = self.record_store.head_version_id(object_id) else {
            return Ok(None);
        };
        if !self.version_ledger.is_purged(head_version_id.as_str()) {
            return Ok(self.version_ledger.get(head_version_id.as_str()).cloned());
        }
        let fallback = self
            .version_ledger
            .active_versions_for_object(object_id)
            .into_iter()
            .max_by(|a, b| {
                a.timestamp_ms
                    .cmp(&b.timestamp_ms)
                    .then_with(|| a.version_id.cmp(&b.version_id))
            });
        Ok(fallback)
    }

    pub fn promote_version(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        object_id: &str,
        version_id: &str,
        target_scope: MemoryScope,
        target_trust_state: TrustState,
        lineage_refs: Vec<String>,
    ) -> Result<MemoryVersion, String> {
        self.ensure_routed(route)?;
        let source_version = self
            .version_ledger
            .get(version_id)
            .cloned()
            .ok_or_else(|| "source_version_not_found".to_string())?;
        if source_version.object_id != object_id {
            return Err("source_version_object_mismatch".to_string());
        }
        if !is_valid_trust_transition(
            source_version.trust_state.clone(),
            target_trust_state.clone(),
        ) {
            return Err("invalid_trust_state_transition".to_string());
        }

        let promote_decision = self.evaluate_policy(MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action: PolicyAction::Promote,
            source_scope: source_version.scope.clone(),
            target_scope: Some(target_scope.clone()),
            trust_state: Some(source_version.trust_state.clone()),
            capability: Some(capability.clone()),
            owner_settings: self.config.owner_settings.clone(),
        });
        if !promote_decision.allow {
            return Err(format!(
                "memory_promotion_denied:{}",
                promote_decision.reason
            ));
        }

        if target_trust_state == TrustState::Canonical {
            let canonical_decision = self.evaluate_policy(MemoryPolicyRequest {
                principal_id: principal_id.to_string(),
                action: PolicyAction::Canonicalize,
                source_scope: target_scope.clone(),
                target_scope: None,
                trust_state: Some(TrustState::Validated),
                capability: Some(capability.clone()),
                owner_settings: self.config.owner_settings.clone(),
            });
            if !canonical_decision.allow {
                return Err(format!(
                    "memory_canonicalize_denied:{}",
                    canonical_decision.reason
                ));
            }
        }

        let source_object = self
            .record_store
            .get_object(object_id)
            .cloned()
            .ok_or_else(|| "source_object_not_found".to_string())?;
        let mut target_object = source_object.clone();
        target_object.scope = target_scope;
        target_object.payload = source_version.payload.clone();
        target_object.updated_at_ms = now_ms();
        if target_object.scope != source_object.scope {
            target_object.object_id = format!(
                "object_{}",
                &deterministic_hash(&(
                    source_object.object_id.clone(),
                    source_version.version_id.clone(),
                    target_object.scope.label(),
                    now_ms()
                ))[..24]
            );
        }
        self.record_store.upsert_object(target_object.clone());

        let target_head = self.record_store.head_version_id(&target_object.object_id);
        let receipt = self.push_receipt(
            route,
            "memory_promotion",
            promote_decision,
            lineage_refs.clone(),
            json!({
                "source_object_id": object_id,
                "source_version_id": source_version.version_id,
                "target_object_id": target_object.object_id,
                "target_scope": target_object.scope.label(),
            }),
        );
        let payload_hash =
            deterministic_hash(&(target_object.payload.clone(), target_object.scope.label()));
        let mut lineage = lineage_refs;
        lineage.push(version_id.to_string());
        let promoted = MemoryVersion {
            version_id: format!(
                "version_{}",
                &deterministic_hash(&(
                    target_object.object_id.clone(),
                    target_head.clone(),
                    receipt.receipt_id.clone(),
                    payload_hash.clone(),
                    now_ms()
                ))[..24]
            ),
            object_id: target_object.object_id.clone(),
            scope: target_object.scope.clone(),
            parent_version_id: target_head,
            lineage_refs: lineage,
            receipt_id: receipt.receipt_id,
            trust_state: target_trust_state,
            payload: target_object.payload.clone(),
            payload_hash,
            timestamp_ms: now_ms(),
            proposed_by: principal_id.to_string(),
        };
        self.version_ledger.append(promoted.clone())?;
        self.record_store
            .register_version(&promoted.object_id, &promoted.version_id);
        self.record_store
            .set_head_version(&promoted.object_id, &promoted.version_id);
        Ok(promoted)
    }

    pub fn rollback_head(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        object_id: &str,
        target_version_id: &str,
        lineage_refs: Vec<String>,
    ) -> Result<MemoryVersion, String> {
        self.ensure_routed(route)?;
        let object = self
            .record_store
            .get_object(object_id)
            .cloned()
            .ok_or_else(|| "rollback_object_not_found".to_string())?;
        let write_decision = self.evaluate_policy(MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action: PolicyAction::Write,
            source_scope: object.scope.clone(),
            target_scope: None,
            trust_state: None,
            capability: Some(capability.clone()),
            owner_settings: self.config.owner_settings.clone(),
        });
        if !write_decision.allow {
            return Err(format!("rollback_denied:{}", write_decision.reason));
        }
        let source_version = self
            .version_ledger
            .get(target_version_id)
            .cloned()
            .ok_or_else(|| "rollback_target_version_not_found".to_string())?;
        if source_version.object_id != object_id {
            return Err("rollback_target_object_mismatch".to_string());
        }
        let receipt = self.push_receipt(
            route,
            "memory_rollback",
            write_decision,
            lineage_refs,
            json!({
                "object_id": object_id,
                "target_version_id": target_version_id,
            }),
        );
        let rollback = rollback_head_from_version(
            object_id,
            &source_version,
            self.record_store.head_version_id(object_id),
            receipt.receipt_id.as_str(),
            principal_id,
        );
        self.version_ledger.append(rollback.clone())?;
        self.record_store
            .register_version(&rollback.object_id, &rollback.version_id);
        self.record_store
            .set_head_version(&rollback.object_id, &rollback.version_id);
        Ok(rollback)
    }

    pub fn materialize_context_stack(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        requested_scopes: Vec<MemoryScope>,
        lineage_refs: Vec<String>,
    ) -> Result<ContextMaterialization, String> {
        self.ensure_routed(route)?;
        let mut visible = Vec::<MemoryVersion>::new();
        for object in self.record_store.all_objects() {
            if !requested_scopes.is_empty()
                && !requested_scopes.iter().any(|row| row == &object.scope)
            {
                continue;
            }
            let decision = self.policy.evaluate(&MemoryPolicyRequest {
                principal_id: principal_id.to_string(),
                action: PolicyAction::Read,
                source_scope: object.scope.clone(),
                target_scope: None,
                trust_state: None,
                capability: Some(capability.clone()),
                owner_settings: self.config.owner_settings.clone(),
            });
            if !decision.allow {
                continue;
            }
            let Some(head_version_id) = self.record_store.head_version_id(&object.object_id) else {
                continue;
            };
            if self.version_ledger.is_purged(head_version_id.as_str()) {
                continue;
            }
            let Some(version) = self.version_ledger.get(head_version_id.as_str()).cloned() else {
                continue;
            };
            if version.trust_state.is_poisoned() {
                continue;
            }
            visible.push(version);
        }
        let materialized = materialize_context(
            principal_id,
            requested_scopes.as_slice(),
            self.config.owner_settings.export_redaction_policy.clone(),
            visible.as_slice(),
        );
        let decision = MemoryPolicyDecision {
            allow: true,
            decision_id: format!(
                "policy_{}",
                &deterministic_hash(&(principal_id.to_string(), "materialize_context"))[..24]
            ),
            reason: "policy_allow".to_string(),
        };
        self.push_receipt(
            route,
            "context_materialization",
            decision,
            lineage_refs,
            json!({
                "principal_id": principal_id,
                "entry_count": materialized.entries.len(),
            }),
        );
        Ok(materialized)
    }

    pub fn export_owner_memory(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        lineage_refs: Vec<String>,
    ) -> Result<Vec<Value>, String> {
        self.ensure_routed(route)?;
        let owner_versions = self
            .record_store
            .all_objects()
            .into_iter()
            .filter(|row| row.scope == MemoryScope::Owner)
            .filter_map(|object| {
                let version_id = self
                    .record_store
                    .head_version_id(object.object_id.as_str())?;
                if self.version_ledger.is_purged(version_id.as_str()) {
                    return None;
                }
                let version = self.version_ledger.get(version_id.as_str()).cloned()?;
                if version.trust_state.is_poisoned() {
                    return None;
                }
                Some(version)
            })
            .collect::<Vec<_>>();
        if owner_versions.is_empty() {
            return Ok(Vec::new());
        }
        let export_decision = self.evaluate_policy(MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action: PolicyAction::ExportOwner,
            source_scope: MemoryScope::Owner,
            target_scope: None,
            trust_state: None,
            capability: Some(capability.clone()),
            owner_settings: self.config.owner_settings.clone(),
        });
        if !export_decision.allow {
            return Err(format!("owner_export_denied:{}", export_decision.reason));
        }
        let materialized = materialize_context(
            principal_id,
            &[MemoryScope::Owner],
            self.config.owner_settings.export_redaction_policy.clone(),
            owner_versions.as_slice(),
        );
        self.push_receipt(
            route,
            "owner_export",
            export_decision,
            lineage_refs,
            json!({
                "entry_count": materialized.entries.len(),
                "redaction_policy": format!("{:?}", self.config.owner_settings.export_redaction_policy),
            }),
        );
        Ok(materialized
            .entries
            .into_iter()
            .map(|row| row.payload)
            .collect::<Vec<_>>())
    }

    pub fn create_task_node(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        node_id: &str,
        payload: Value,
    ) -> Result<GraphNode, String> {
        self.ensure_routed(route)?;
        self.require_task_fabric_permission(principal_id, capability)?;
        let node = self.graph_subsystem.create_task_node(node_id, payload);
        self.push_receipt(
            route,
            "task_fabric_create_node",
            MemoryPolicyDecision {
                allow: true,
                decision_id: format!(
                    "policy_{}",
                    &deterministic_hash(&(
                        principal_id.to_string(),
                        node_id.to_string(),
                        "task_create"
                    ))[..24]
                ),
                reason: "policy_allow".to_string(),
            },
            vec![node.node_id.clone()],
            json!({ "node_id": node.node_id }),
        );
        Ok(node)
    }

    pub fn issue_task_lease(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        node_id: &str,
        ttl_ms: u64,
    ) -> Result<TaskFabricLease, String> {
        self.ensure_routed(route)?;
        self.require_task_fabric_permission(principal_id, capability)?;
        let lease = self
            .graph_subsystem
            .issue_lease(node_id, principal_id, ttl_ms)?;
        self.push_receipt(
            route,
            "task_fabric_issue_lease",
            MemoryPolicyDecision {
                allow: true,
                decision_id: format!(
                    "policy_{}",
                    &deterministic_hash(&(
                        principal_id.to_string(),
                        lease.lease_id.clone(),
                        "task_lease"
                    ))[..24]
                ),
                reason: "policy_allow".to_string(),
            },
            vec![lease.lease_id.clone()],
            json!({ "node_id": node_id, "lease_id": lease.lease_id }),
        );
        Ok(lease)
    }

    pub fn mutate_task_node(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        node_id: &str,
        lease_id: &str,
        expected_cas: u64,
        payload: Value,
    ) -> Result<GraphNode, String> {
        self.ensure_routed(route)?;
        self.require_task_fabric_permission(principal_id, capability)?;
        let node = self.graph_subsystem.mutate_task_node(
            node_id,
            lease_id,
            principal_id,
            expected_cas,
            payload,
        )?;
        self.push_receipt(
            route,
            "task_fabric_mutate_node",
            MemoryPolicyDecision {
                allow: true,
                decision_id: format!(
                    "policy_{}",
                    &deterministic_hash(&(
                        principal_id.to_string(),
                        node_id.to_string(),
                        expected_cas
                    ))[..24]
                ),
                reason: "policy_allow".to_string(),
            },
            vec![node.node_id.clone()],
            json!({ "node_id": node_id, "lease_id": lease_id, "cas": node.cas_version }),
        );
        Ok(node)
    }

    pub fn add_task_edge(
        &mut self,
        route: &NexusRouteContext,
        principal_id: &str,
        capability: &CapabilityToken,
        source_node_id: &str,
        target_node_id: &str,
        lease_id: &str,
        expected_source_cas: u64,
        edge_type: &str,
    ) -> Result<GraphEdge, String> {
        self.ensure_routed(route)?;
        self.require_task_fabric_permission(principal_id, capability)?;
        let edge = self.graph_subsystem.add_edge(
            source_node_id,
            target_node_id,
            lease_id,
            principal_id,
            expected_source_cas,
            edge_type,
        )?;
        self.push_receipt(
            route,
            "task_fabric_add_edge",
            MemoryPolicyDecision {
                allow: true,
                decision_id: format!(
                    "policy_{}",
                    &deterministic_hash(&(principal_id.to_string(), edge.edge_id.clone()))[..24]
                ),
                reason: "policy_allow".to_string(),
            },
            vec![edge.edge_id.clone()],
            json!({
                "source_node_id": source_node_id,
                "target_node_id": target_node_id,
                "lease_id": lease_id,
                "edge_type": edge_type,
            }),
        );
        Ok(edge)
    }

    fn require_task_fabric_permission(
        &self,
        principal_id: &str,
        capability: &CapabilityToken,
    ) -> Result<(), String> {
        let decision = self.policy.evaluate(&MemoryPolicyRequest {
            principal_id: principal_id.to_string(),
            action: PolicyAction::TaskFabricMutate,
            source_scope: MemoryScope::Core,
            target_scope: None,
            trust_state: None,
            capability: Some(capability.clone()),
            owner_settings: self.config.owner_settings.clone(),
        });
        if decision.allow {
            Ok(())
        } else {
            Err(format!("task_fabric_mutation_denied:{}", decision.reason))
        }
    }

    fn evaluate_policy(&self, request: MemoryPolicyRequest) -> MemoryPolicyDecision {
        self.policy.evaluate(&request)
    }

    fn ensure_routed(&self, route: &NexusRouteContext) -> Result<(), String> {
        if route.source.trim().is_empty() || route.target.trim().is_empty() {
            return Err("nexus_route_invalid_missing_source_target".to_string());
        }
        if route.schema_id.trim().is_empty() {
            return Err("nexus_route_invalid_missing_schema".to_string());
        }
        if route.lease_id.trim().is_empty() {
            return Err("nexus_route_invalid_missing_lease".to_string());
        }
        Ok(())
    }

    fn push_receipt(
        &mut self,
        route: &NexusRouteContext,
        event_type: &str,
        decision: MemoryPolicyDecision,
        lineage_refs: Vec<String>,
        details: Value,
    ) -> MemoryReceipt {
        let receipt = MemoryReceipt {
            receipt_id: format!(
                "receipt_{}",
                &deterministic_hash(&(
                    route.source.clone(),
                    route.target.clone(),
                    event_type.to_string(),
                    decision.decision_id.clone(),
                    now_ms()
                ))[..24]
            ),
            event_type: event_type.to_string(),
            issuer: route.issuer.clone(),
            source: route.source.clone(),
            target: route.target.clone(),
            schema_id: route.schema_id.clone(),
            template_version_id: route.template_version_id.clone(),
            ttl_ms: route.ttl_ms,
            policy_decision_ref: decision.decision_id,
            revocation_cause: None,
            timestamp_ms: now_ms(),
            lineage_refs,
            details,
        };
        self.receipts.push(receipt.clone());
        receipt
    }
}
