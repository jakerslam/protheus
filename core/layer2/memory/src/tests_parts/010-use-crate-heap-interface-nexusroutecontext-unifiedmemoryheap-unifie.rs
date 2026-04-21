use crate::heap_interface::{NexusRouteContext, UnifiedMemoryHeap, UnifiedMemoryHeapConfig};
use crate::policy::{
    DefaultVerityMemoryPolicy, MemoryPolicyGate, MemoryPolicyRequest, PolicyAction,
};
use crate::schemas::{
    CapabilityAction, CapabilityToken, Classification, MemoryKind, MemoryObject,
    MemoryRetentionPolicy, MemoryScope, OwnerExportRedactionPolicy, OwnerScopeSettings,
    PurgeRelationType, TrustState,
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

fn agent_token(agent_id: &str, actions: Vec<CapabilityAction>) -> CapabilityToken {
    token(
        &format!("agent:{agent_id}"),
        vec![MemoryScope::Agent(agent_id.to_string())],
        actions,
    )
}

fn core_token(agent_id: &str, actions: Vec<CapabilityAction>) -> CapabilityToken {
    token(
        "core:memory",
        vec![MemoryScope::Agent(agent_id.to_string()), MemoryScope::Core],
        actions,
    )
}

fn object(object_id: &str, scope: MemoryScope, payload: serde_json::Value) -> MemoryObject {
    MemoryObject {
        object_id: object_id.to_string(),
        scope,
        kind: MemoryKind::Episodic,
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
    let agent_a = agent_token(
        "alpha",
        vec![CapabilityAction::Read, CapabilityAction::Write],
    );
    let agent_b = agent_token("beta", vec![CapabilityAction::Read]);
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

    let agent_token = agent_token(
        "alpha",
        vec![
            CapabilityAction::Read,
            CapabilityAction::Write,
            CapabilityAction::Promote,
            CapabilityAction::Canonicalize,
        ],
    );
    let core_token = core_token(
        "alpha",
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
