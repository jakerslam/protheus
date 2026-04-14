use super::*;
use crate::heap_interface::{NexusRouteContext, UnifiedMemoryHeap};
use crate::policy::DefaultVerityMemoryPolicy;
use crate::schemas::{CapabilityAction, CapabilityToken, Classification, MemoryObject};

fn route() -> NexusRouteContext {
    NexusRouteContext {
        issuer: "memory_recall_tests".to_string(),
        source: "test".to_string(),
        target: "memory_heap".to_string(),
        schema_id: "memory.recall".to_string(),
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

fn obj(id: &str, kind: MemoryKind, payload: Value, metadata: Value) -> MemoryObject {
    MemoryObject {
        object_id: id.to_string(),
        scope: MemoryScope::Core,
        kind,
        classification: Classification::Internal,
        namespace: "memory.tests".to_string(),
        key: id.to_string(),
        payload,
        metadata,
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

#[test]
fn hybrid_recall_supports_multi_hop_and_feedback_and_invalidation() {
    let mut heap = UnifiedMemoryHeap::new(DefaultVerityMemoryPolicy);
    let route = route();
    let cap = token();
    let writes = vec![
        obj(
            "alice_project",
            MemoryKind::Episodic,
            json!({"person":"Alice","project":"Atlas","summary":"Alice leads Project Atlas"}),
            json!({"entity_refs":["person:alice","project:atlas"]}),
        ),
        obj(
            "atlas_system",
            MemoryKind::Episodic,
            json!({"project":"Atlas","system":"PostgreSQL","summary":"Project Atlas uses PostgreSQL"}),
            json!({"entity_refs":["project:atlas","system:postgresql"]}),
        ),
        obj(
            "postgres_outage",
            MemoryKind::Episodic,
            json!({"system":"PostgreSQL","incident":"Tuesday outage","summary":"The PostgreSQL cluster had a Tuesday outage"}),
            json!({"entity_refs":["system:postgresql","incident:tuesday_outage"]}),
        ),
    ];
    for object in writes {
        heap.write_memory_object(
            &route,
            "core:memory",
            &cap,
            object,
            TrustState::Validated,
            vec![],
        )
        .expect("write");
    }
    let hits = heap
        .hybrid_recall(
            "core:memory",
            &cap,
            MemoryRecallQuery {
                query: "Was Alice's project affected by Tuesday's outage?".to_string(),
                requested_scopes: vec![MemoryScope::Core],
                top_k: 3,
                allowed_kinds: vec![],
                session_entity_hints: vec![],
            },
        )
        .expect("recall");
    assert!(!hits.is_empty());
    assert!(hits.iter().any(|row| row.explanation.graph_score > 0));

    let reinforced = heap
        .record_retrieval_feedback(
            &route,
            "core:memory",
            &cap,
            &hits[0].version_id,
            MemoryRecallFeedbackSignal {
                useful: true,
                cited_in_response: true,
                corrected_user: false,
                explicit_pin: true,
            },
            vec![],
        )
        .expect("feedback");
    assert!(reinforced.salience.score > hits[0].explanation.salience_score as u32);

    let invalidated = heap
        .invalidate_version(
            &route,
            "core:memory",
            &cap,
            &hits[0].version_id,
            Some(reinforced.version_id.clone()),
            MemoryInvalidationReason::Corrected,
            json!({"reason":"newer better formulation"}),
            vec![],
        )
        .expect("invalidate");
    assert_eq!(invalidated.target_version_id, hits[0].version_id);
}
