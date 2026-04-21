
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

#[test]
fn canonical_record_and_replay_rows_are_receipt_replayable() {
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
            CapabilityAction::Canonicalize,
        ],
    );

    let v1 = heap
        .write_memory_object(
            &route,
            "agent:alpha",
            &cap,
            object(
                "obj_replay",
                MemoryScope::Agent("alpha".to_string()),
                json!({"value":"v1"}),
            ),
            TrustState::Proposed,
            vec!["lineage:replay:v1".to_string()],
        )
        .expect("write v1");

    let _v2 = heap
        .promote_version(
            &route,
            "agent:alpha",
            &cap,
            "obj_replay",
            v1.version_id.as_str(),
            MemoryScope::Agent("alpha".to_string()),
            TrustState::Corroborated,
            vec!["lineage:replay:v2".to_string()],
        )
        .expect("promote v2");

    let canonical = heap
        .canonical_head_record("agent:alpha", &cap, "obj_replay")
        .expect("canonical record")
        .expect("record present");
    assert_eq!(canonical.object_id, "obj_replay");
    assert_eq!(canonical.scope, MemoryScope::Agent("alpha".to_string()));
    assert_eq!(canonical.capability_action, CapabilityAction::Read);
    assert_eq!(canonical.capability_token_id, cap.token_id);

    let replay = heap.replay_mutation_rows();
    assert!(replay.len() >= 2);
    assert!(replay
        .iter()
        .all(|row| row.receipt_id.starts_with("receipt_")));
    assert!(replay
        .iter()
        .all(|row| row.object_id == "obj_replay" || row.object_id.starts_with("obj_")));
}

#[test]
fn retention_purge_rules_are_append_only_and_reconstruct_context_deterministically() {
    let policy = DefaultVerityMemoryPolicy;
    let mut heap = UnifiedMemoryHeap::new(policy);
    let route = route();
    let cap = token(
        "core:memory",
        vec![MemoryScope::Core],
        vec![
            CapabilityAction::Read,
            CapabilityAction::Write,
            CapabilityAction::Promote,
            CapabilityAction::MaterializeContext,
            CapabilityAction::Canonicalize,
        ],
    );

    heap.write_memory_object(
        &route,
        "core:memory",
        &cap,
        object("obj_retention", MemoryScope::Core, json!({"n":1})),
        TrustState::Proposed,
        vec!["lineage:retention:1".to_string()],
    )
    .expect("v1");
    heap.write_memory_object(
        &route,
        "core:memory",
        &cap,
        object("obj_retention", MemoryScope::Core, json!({"n":2})),
        TrustState::Proposed,
        vec!["lineage:retention:2".to_string()],
    )
    .expect("v2");
    heap.write_memory_object(
        &route,
        "core:memory",
        &cap,
        object("obj_retention", MemoryScope::Core, json!({"n":3})),
        TrustState::Proposed,
        vec!["lineage:retention:3".to_string()],
    )
    .expect("v3");
    heap.write_memory_object(
        &route,
        "core:memory",
        &cap,
        object("obj_retention", MemoryScope::Core, json!({"n":4})),
        TrustState::Proposed,
        vec!["lineage:retention:4".to_string()],
    )
    .expect("v4");

    let report = heap
        .apply_retention_policy_and_purge(
            &route,
            "core:memory",
            &cap,
            MemoryRetentionPolicy {
                max_versions_per_object: 1,
                retain_window_ms: None,
                protect_trust_states: vec![TrustState::Canonical],
            },
            PurgeRelationType::PurgedByRetention,
            "retention_cap",
            vec!["lineage:purge".to_string()],
        )
        .expect("retention purge");
    assert!(report.purged_versions >= 2);
    assert_eq!(heap.purge_records().len(), report.purged_versions);

    let view_a = heap
        .reconstruct_context_view(
            &route,
            "core:memory",
            &cap,
            vec![MemoryScope::Core],
            None,
            None,
            vec!["lineage:reconstruct:a".to_string()],
        )
        .expect("reconstruct a");
    let view_b = heap
        .reconstruct_context_view(
            &route,
            "core:memory",
            &cap,
            vec![MemoryScope::Core],
            None,
            None,
            vec!["lineage:reconstruct:b".to_string()],
        )
        .expect("reconstruct b");
    assert_eq!(view_a.entries, view_b.entries);
}

#[test]
fn anti_poisoning_blocks_quarantined_versions_from_context_views() {
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
            CapabilityAction::MaterializeContext,
            CapabilityAction::Canonicalize,
        ],
    );

    let proposed = heap
        .write_memory_object(
            &route,
            "agent:alpha",
            &cap,
            object(
                "obj_poison",
                MemoryScope::Agent("alpha".to_string()),
                json!({"signal":"candidate"}),
            ),
            TrustState::Proposed,
            vec!["lineage:poison:proposed".to_string()],
        )
        .expect("proposed");
    let _quarantined = heap
        .promote_version(
            &route,
            "agent:alpha",
            &cap,
            "obj_poison",
            proposed.version_id.as_str(),
            MemoryScope::Agent("alpha".to_string()),
            TrustState::Quarantined,
            vec!["lineage:poison:quarantined".to_string()],
        )
        .expect("quarantined");

    let view = heap
        .materialize_context_stack(
            &route,
            "agent:alpha",
            &cap,
            vec![MemoryScope::Agent("alpha".to_string())],
            vec!["lineage:poison:view".to_string()],
        )
        .expect("materialize");
    assert!(
        view.entries.iter().all(|row| row.object_id != "obj_poison"),
        "quarantined head must be excluded from context materialization"
    );
}
