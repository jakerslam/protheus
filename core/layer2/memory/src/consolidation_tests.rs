use super::*;
use crate::heap_interface::{NexusRouteContext, UnifiedMemoryHeap};
use crate::policy::DefaultVerityMemoryPolicy;

fn route() -> NexusRouteContext {
    NexusRouteContext {
        issuer: "memory_consolidation_tests".to_string(),
        source: "test".to_string(),
        target: "memory_heap".to_string(),
        schema_id: "memory.consolidation".to_string(),
        lease_id: "lease".to_string(),
        template_version_id: Some("v1".to_string()),
        ttl_ms: Some(1000),
    }
}

fn token() -> CapabilityToken {
    CapabilityToken {
        token_id: "cap".to_string(),
        principal_id: "core:memory".to_string(),
        scopes: vec![MemoryScope::Core],
        allowed_actions: vec![
            CapabilityAction::Read,
            CapabilityAction::Write,
            CapabilityAction::Promote,
            CapabilityAction::Canonicalize,
            CapabilityAction::MaterializeContext,
        ],
        expires_at_ms: u64::MAX,
        verity_class: "standard".to_string(),
        receipt_id: "r".to_string(),
    }
}

#[test]
fn consolidation_derives_semantic_and_procedural_memory() {
    let mut heap = UnifiedMemoryHeap::new(DefaultVerityMemoryPolicy);
    let route = route();
    let cap = token();
    for idx in 0..2 {
        heap.write_memory_object(
            &route,
            "core:memory",
            &cap,
            MemoryObject {
                object_id: format!("pref-{idx}"),
                scope: MemoryScope::Core,
                kind: MemoryKind::Episodic,
                classification: Classification::Internal,
                namespace: "memory.tests".to_string(),
                key: format!("pref-{idx}"),
                payload: json!({
                    "person": "Alice",
                    "preference_key": "summary_style",
                    "preference_value": "executive"
                }),
                metadata: json!({"entity_refs":["person:alice"]}),
                created_at_ms: 0,
                updated_at_ms: 0,
            },
            TrustState::Validated,
            vec!["lineage:pref".to_string()],
        )
        .expect("write pref");
    }
    for (idx, step) in ["check purchase date", "issue refund"].iter().enumerate() {
        heap.write_memory_object(
            &route,
            "core:memory",
            &cap,
            MemoryObject {
                object_id: format!("proc-{idx}"),
                scope: MemoryScope::Core,
                kind: MemoryKind::Episodic,
                classification: Classification::Internal,
                namespace: "memory.tests".to_string(),
                key: format!("proc-{idx}"),
                payload: json!({
                    "procedure_name": "refund_request",
                    "procedure_step": step,
                    "step_index": idx as u64
                }),
                metadata: json!({}),
                created_at_ms: 0,
                updated_at_ms: 0,
            },
            TrustState::Validated,
            vec!["lineage:proc".to_string()],
        )
        .expect("write proc");
    }
    let report = heap
        .run_consolidation_cycle(&route, "core:memory", &cap, vec![MemoryScope::Core], vec![])
        .expect("consolidate");
    assert!(report.derived_semantic >= 1);
    assert!(report.derived_procedural >= 1);
}
