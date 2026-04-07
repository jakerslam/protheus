use crate::heap_interface::{NexusRouteContext, UnifiedMemoryHeap, UnifiedMemoryHeapConfig};
use crate::policy::{
    DefaultVerityMemoryPolicy, MemoryPolicyGate, MemoryPolicyRequest, PolicyAction,
};
use crate::schemas::{
    CapabilityAction, CapabilityToken, Classification, MemoryObject, MemoryScope,
    OwnerExportRedactionPolicy, OwnerScopeSettings, TrustState,
};
use serde_json::json;

fn route() -> NexusRouteContext {
    NexusRouteContext {
        issuer: "memory_heap_tests".to_string(),
        source: "client_ingress".to_string(),
        target: "memory_heap".to_string(),
        schema_id: "memory.heap.write".to_string(),
        lease_id: "lease_test".to_string(),
        template_version_id: Some("v1".to_string()),
        ttl_ms: Some(30_000),
    }
}

fn token(
    principal_id: &str,
    scopes: Vec<MemoryScope>,
    actions: Vec<CapabilityAction>,
) -> CapabilityToken {
    CapabilityToken {
        token_id: format!("cap_{principal_id}"),
        principal_id: principal_id.to_string(),
        scopes,
        allowed_actions: actions,
        expires_at_ms: u64::MAX,
        verity_class: "standard".to_string(),
        receipt_id: "cap_receipt".to_string(),
    }
}

fn object(object_id: &str, scope: MemoryScope, payload: serde_json::Value) -> MemoryObject {
    MemoryObject {
        object_id: object_id.to_string(),
        scope,
        classification: Classification::Internal,
        namespace: "memory.tests".to_string(),
        key: "item".to_string(),
        payload,
        metadata: json!({}),
        created_at_ms: 1,
        updated_at_ms: 1,
    }
}

#[test]
fn scope_isolation_blocks_cross_agent_read() {
    let policy = DefaultVerityMemoryPolicy;
    let mut heap = UnifiedMemoryHeap::new(policy);
    let route = route();
    let agent_a = token(
        "agent:alpha",
        vec![MemoryScope::Agent("alpha".to_string())],
        vec![CapabilityAction::Read, CapabilityAction::Write],
    );
    let agent_b = token(
        "agent:beta",
        vec![MemoryScope::Agent("beta".to_string())],
        vec![CapabilityAction::Read],
    );
    heap.write_memory_object(
        &route,
        "agent:alpha",
        &agent_a,
        object(
            "obj_agent",
            MemoryScope::Agent("alpha".to_string()),
            json!({"secret":"alpha_only"}),
        ),
        TrustState::Proposed,
        vec!["lineage:a".to_string()],
    )
    .expect("write");

    let err = heap
        .read_head_version("agent:beta", &agent_b, "obj_agent")
        .expect_err("should deny");
    assert!(err.contains("memory_read_denied"));
}

#[test]
fn cross_scope_promotion_creates_new_version_with_lineage() {
    let policy = DefaultVerityMemoryPolicy;
    let mut heap = UnifiedMemoryHeap::new(policy);
    let route = route();

    let agent_token = token(
        "agent:alpha",
        vec![MemoryScope::Agent("alpha".to_string())],
        vec![
            CapabilityAction::Read,
            CapabilityAction::Write,
            CapabilityAction::Promote,
            CapabilityAction::Canonicalize,
        ],
    );
    let core_token = token(
        "core:memory",
        vec![MemoryScope::Agent("alpha".to_string()), MemoryScope::Core],
        vec![
            CapabilityAction::Read,
            CapabilityAction::Write,
            CapabilityAction::Promote,
            CapabilityAction::Canonicalize,
        ],
    );

    let proposed = heap
        .write_memory_object(
            &route,
            "agent:alpha",
            &agent_token,
            object(
                "obj_promote",
                MemoryScope::Agent("alpha".to_string()),
                json!({"fact":"draft"}),
            ),
            TrustState::Proposed,
            vec!["lineage:root".to_string()],
        )
        .expect("proposed write");

    let corroborated = heap
        .promote_version(
            &route,
            "agent:alpha",
            &agent_token,
            "obj_promote",
            proposed.version_id.as_str(),
            MemoryScope::Agent("alpha".to_string()),
            TrustState::Corroborated,
            vec!["lineage:corroborated".to_string()],
        )
        .expect("corroborated");
    let validated = heap
        .promote_version(
            &route,
            "agent:alpha",
            &agent_token,
            "obj_promote",
            corroborated.version_id.as_str(),
            MemoryScope::Agent("alpha".to_string()),
            TrustState::Validated,
            vec!["lineage:validated".to_string()],
        )
        .expect("validated");
    let canonical_core = heap
        .promote_version(
            &route,
            "core:memory",
            &core_token,
            "obj_promote",
            validated.version_id.as_str(),
            MemoryScope::Core,
            TrustState::Canonical,
            vec!["lineage:canonical".to_string()],
        )
        .expect("canonical");

    assert_eq!(canonical_core.scope, MemoryScope::Core);
    assert_ne!(canonical_core.object_id, "obj_promote");
    assert!(canonical_core
        .lineage_refs
        .iter()
        .any(|row| row == &validated.version_id));
}

#[test]
fn context_materialization_respects_capability_and_redaction() {
    let policy = DefaultVerityMemoryPolicy;
    let mut heap = UnifiedMemoryHeap::with_config(
        policy,
        UnifiedMemoryHeapConfig {
            owner_settings: OwnerScopeSettings {
                consent_mode: crate::schemas::OwnerConsentMode::ExplicitApproval,
                export_redaction_policy: OwnerExportRedactionPolicy::SummarizeOnly,
            },
        },
    );
    let route = route();

    let owner_token = token(
        "owner",
        vec![MemoryScope::Owner],
        vec![
            CapabilityAction::Read,
            CapabilityAction::Write,
            CapabilityAction::MaterializeContext,
            CapabilityAction::ExportOwnerRaw,
        ],
    );
    heap.write_memory_object(
        &route,
        "owner",
        &owner_token,
        object(
            "obj_owner",
            MemoryScope::Owner,
            json!({"journal":"private owner detail"}),
        ),
        TrustState::Validated,
        vec!["lineage:owner".to_string()],
    )
    .expect("owner write");

    let materialized = heap
        .materialize_context_stack(
            &route,
            "owner",
            &owner_token,
            vec![MemoryScope::Owner],
            vec!["lineage:ctx".to_string()],
        )
        .expect("materialize");
    assert_eq!(materialized.entries.len(), 1);
    assert!(materialized.entries[0].redacted);
    assert!(materialized.entries[0]
        .payload
        .get("summary_only")
        .is_some());

    let outsider = token(
        "agent:outsider",
        vec![MemoryScope::Agent("outsider".to_string())],
        vec![CapabilityAction::Read, CapabilityAction::MaterializeContext],
    );
    let outsider_view = heap
        .materialize_context_stack(
            &route,
            "agent:outsider",
            &outsider,
            vec![MemoryScope::Owner],
            vec!["lineage:ctx2".to_string()],
        )
        .expect("materialize outsider");
    assert!(outsider_view.entries.is_empty());
}

#[test]
fn append_only_and_rollback_create_new_head_version() {
    let policy = DefaultVerityMemoryPolicy;
    let mut heap = UnifiedMemoryHeap::new(policy);
    let route = route();
    let cap = token(
        "agent:alpha",
        vec![MemoryScope::Agent("alpha".to_string())],
        vec![
            CapabilityAction::Read,
            CapabilityAction::Write,
            CapabilityAction::Promote,
        ],
    );
    let v1 = heap
        .write_memory_object(
            &route,
            "agent:alpha",
            &cap,
            object(
                "obj_roll",
                MemoryScope::Agent("alpha".to_string()),
                json!({"value":"v1"}),
            ),
            TrustState::Proposed,
            vec!["lineage:v1".to_string()],
        )
        .expect("v1");
    let v2 = heap
        .write_memory_object(
            &route,
            "agent:alpha",
            &cap,
            object(
                "obj_roll",
                MemoryScope::Agent("alpha".to_string()),
                json!({"value":"v2"}),
            ),
            TrustState::Proposed,
            vec!["lineage:v2".to_string()],
        )
        .expect("v2");
    let rollback = heap
        .rollback_head(
            &route,
            "agent:alpha",
            &cap,
            "obj_roll",
            v1.version_id.as_str(),
            vec!["lineage:rollback".to_string()],
        )
        .expect("rollback");
    assert_ne!(rollback.version_id, v1.version_id);
    assert_ne!(rollback.version_id, v2.version_id);
    assert_eq!(rollback.parent_version_id, Some(v2.version_id.clone()));
    assert_eq!(
        rollback.payload.get("value").and_then(|row| row.as_str()),
        Some("v1")
    );
    assert_eq!(
        heap.record_store().version_ids_for_object("obj_roll").len(),
        3
    );
}

#[test]
fn task_fabric_operations_enforce_lease_and_cas() {
    let policy = DefaultVerityMemoryPolicy;
    let mut heap = UnifiedMemoryHeap::new(policy);
    let route = route();
    let task_cap = token(
        "core:planner",
        vec![MemoryScope::Core],
        vec![CapabilityAction::TaskFabricMutate],
    );
    heap.create_task_node(
        &route,
        "core:planner",
        &task_cap,
        "task_a",
        json!({"status":"queued"}),
    )
    .expect("task_a");
    heap.create_task_node(
        &route,
        "core:planner",
        &task_cap,
        "task_b",
        json!({"status":"queued"}),
    )
    .expect("task_b");
    let lease = heap
        .issue_task_lease(&route, "core:planner", &task_cap, "task_a", 60_000)
        .expect("lease");
    let updated = heap
        .mutate_task_node(
            &route,
            "core:planner",
            &task_cap,
            "task_a",
            lease.lease_id.as_str(),
            0,
            json!({"status":"running"}),
        )
        .expect("mutate");
    assert_eq!(updated.cas_version, 1);

    let cas_err = heap
        .mutate_task_node(
            &route,
            "core:planner",
            &task_cap,
            "task_a",
            lease.lease_id.as_str(),
            0,
            json!({"status":"stale"}),
        )
        .expect_err("should fail stale cas");
    assert!(cas_err.contains("task_cas_mismatch"));

    let lease_err = heap
        .add_task_edge(
            &route,
            "core:planner",
            &task_cap,
            "task_a",
            "task_b",
            "lease_missing",
            1,
            "blocks",
        )
        .expect_err("missing lease");
    assert!(lease_err.contains("task_lease_not_found"));

    let edge = heap
        .add_task_edge(
            &route,
            "core:planner",
            &task_cap,
            "task_a",
            "task_b",
            lease.lease_id.as_str(),
            1,
            "blocks",
        )
        .expect("edge");
    assert_eq!(edge.edge_type, "blocks");
}

#[test]
fn owner_export_policy_is_enforced() {
    let route = route();
    let owner_cap = token(
        "owner",
        vec![MemoryScope::Owner],
        vec![
            CapabilityAction::Read,
            CapabilityAction::Write,
            CapabilityAction::ExportOwnerRaw,
        ],
    );

    let mut deny_heap = UnifiedMemoryHeap::with_config(
        DefaultVerityMemoryPolicy,
        UnifiedMemoryHeapConfig {
            owner_settings: OwnerScopeSettings {
                consent_mode: crate::schemas::OwnerConsentMode::ExplicitApproval,
                export_redaction_policy: OwnerExportRedactionPolicy::AllowRedacted,
            },
        },
    );
    deny_heap
        .write_memory_object(
            &route,
            "owner",
            &owner_cap,
            object(
                "obj_owner_export",
                MemoryScope::Owner,
                json!({"raw":"sensitive"}),
            ),
            TrustState::Validated,
            vec!["lineage:owner_export".to_string()],
        )
        .expect("owner write");
    let err = deny_heap
        .export_owner_memory(&route, "owner", &owner_cap, vec!["lineage:exp".to_string()])
        .expect_err("should deny");
    assert!(err.contains("owner_export_denied"));

    let mut allow_heap = UnifiedMemoryHeap::with_config(
        DefaultVerityMemoryPolicy,
        UnifiedMemoryHeapConfig {
            owner_settings: OwnerScopeSettings {
                consent_mode: crate::schemas::OwnerConsentMode::ExplicitApproval,
                export_redaction_policy: OwnerExportRedactionPolicy::AllowFull,
            },
        },
    );
    allow_heap
        .write_memory_object(
            &route,
            "owner",
            &owner_cap,
            object(
                "obj_owner_export_2",
                MemoryScope::Owner,
                json!({"raw":"sensitive"}),
            ),
            TrustState::Validated,
            vec!["lineage:owner_export2".to_string()],
        )
        .expect("owner write");
    let exported = allow_heap
        .export_owner_memory(
            &route,
            "owner",
            &owner_cap,
            vec!["lineage:exp2".to_string()],
        )
        .expect("allow export");
    assert_eq!(exported.len(), 1);
    assert_eq!(
        exported[0].get("raw").and_then(|row| row.as_str()),
        Some("sensitive")
    );
}

#[test]
fn verity_policy_decisions_are_explainable() {
    let policy = DefaultVerityMemoryPolicy;
    let denied = policy.evaluate(&MemoryPolicyRequest {
        principal_id: "agent:alpha".to_string(),
        action: PolicyAction::Read,
        source_scope: MemoryScope::Agent("alpha".to_string()),
        target_scope: None,
        trust_state: None,
        capability: None,
        owner_settings: OwnerScopeSettings::default(),
    });
    assert!(!denied.allow);
    assert!(!denied.reason.is_empty());
    assert!(denied.decision_id.starts_with("policy_"));

    let canon_cap = token(
        "core:memory",
        vec![MemoryScope::Core],
        vec![CapabilityAction::Canonicalize],
    );
    let allowed = policy.evaluate(&MemoryPolicyRequest {
        principal_id: "core:memory".to_string(),
        action: PolicyAction::Canonicalize,
        source_scope: MemoryScope::Core,
        target_scope: None,
        trust_state: Some(TrustState::Validated),
        capability: Some(canon_cap),
        owner_settings: OwnerScopeSettings::default(),
    });
    assert!(allowed.allow);
    assert_eq!(allowed.reason, "policy_allow");
}
