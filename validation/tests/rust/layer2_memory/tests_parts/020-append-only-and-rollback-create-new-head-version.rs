
#[test]
fn append_only_and_rollback_create_new_head_version() {
    let policy = DefaultVerityMemoryPolicy;
    let mut heap = UnifiedMemoryHeap::new(policy);
    let route = route();
    let cap = agent_token(
        "alpha",
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
