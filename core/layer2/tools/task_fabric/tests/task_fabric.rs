// Layer ownership: tests (regression proof for task_fabric authoritative contracts).
use infring_task_fabric_core_v1::{
    now_ms, phase_task_id, root_task_id, AllowAllVerityGate, Blocker, BlockerKind, DependencyEdge,
    LifecycleStatus, MutationEnvelope, MutationKind, NexusConduitRequest, Task, TaskFabric,
    VerityGate,
};
use serde_json::json;

fn sample_task(scope: &str, id: &str, title: &str) -> Task {
    let ts = now_ms();
    Task {
        id: id.to_string(),
        title: title.to_string(),
        lifecycle_status: LifecycleStatus::Pending,
        parent_id: None,
        priority: 50,
        owner: Some("owner".to_string()),
        assignee: None,
        progress_pct: Some(0),
        tags: vec!["unit".to_string()],
        linked_receipts: Vec::new(),
        metadata: json!({}),
        scope_id: scope.to_string(),
        blockers: Vec::new(),
        related_links: Vec::new(),
        created_at: ts,
        updated_at: ts,
        started_at: None,
        completed_at: None,
        last_heartbeat_at: None,
        lease_expires_at: None,
        revision_id: 0,
    }
}

fn envelope(kind: MutationKind, key: &str) -> MutationEnvelope {
    MutationEnvelope {
        actor: "tester".to_string(),
        trace_id: format!("trace-{key}"),
        idempotency_key: key.to_string(),
        proof_refs: vec![format!("proof:{key}")],
        expected_revision: None,
        now_ms: now_ms(),
        mutation_kind: kind,
        payload: json!({}),
    }
}

#[test]
fn task_creation_and_dependency_cycle_protection() {
    let mut fabric = TaskFabric::new("scope-a");
    let gate = AllowAllVerityGate;
    fabric
        .submit_task(
            sample_task("scope-a", "parent", "Parent"),
            envelope(MutationKind::CreateTask, "c1"),
            &gate,
        )
        .expect("create parent");
    fabric
        .submit_task(
            sample_task("scope-a", "child", "Child"),
            envelope(MutationKind::CreateTask, "c2"),
            &gate,
        )
        .expect("create child");
    fabric
        .graph
        .set_parent("child", "parent")
        .expect("parent link");
    fabric
        .add_dependency(
            DependencyEdge {
                task_id: "child".to_string(),
                depends_on_task_id: "parent".to_string(),
            },
            envelope(MutationKind::AddDependency, "d1"),
            &gate,
        )
        .expect("dependency");
    let cycle = fabric.add_dependency(
        DependencyEdge {
            task_id: "parent".to_string(),
            depends_on_task_id: "child".to_string(),
        },
        envelope(MutationKind::AddDependency, "d2"),
        &gate,
    );
    assert!(cycle.is_err());
}

#[test]
fn status_transition_and_receipt_lineage() {
    let mut fabric = TaskFabric::new("scope-a");
    let gate = AllowAllVerityGate;
    fabric
        .submit_task(
            sample_task("scope-a", "t1", "Task"),
            envelope(MutationKind::CreateTask, "s1"),
            &gate,
        )
        .expect("create");
    let before = fabric.graph.task("t1").map(|v| v.revision_id).unwrap_or(0);
    let event = fabric
        .transition_status(
            "t1",
            LifecycleStatus::InProgress,
            MutationEnvelope {
                expected_revision: Some(before),
                payload: json!({"next_status":"in-progress"}),
                ..envelope(MutationKind::UpdateStatus, "s2")
            },
            &gate,
        )
        .expect("status update");
    assert_eq!(event.next_revision, Some(before + 1));
    assert!(!fabric.receipts.is_empty());
    assert!(fabric.receipts[0]
        .dna_lineage
        .iter()
        .any(|row| row.starts_with("scope:")));
}

#[test]
fn stomach_template_integration_creates_phases() {
    let mut fabric = TaskFabric::new("scope-a");
    let gate = AllowAllVerityGate;
    fabric
        .integrate_stomach_template(
            "module-x",
            Some("owner".to_string()),
            Some("assignee".to_string()),
            envelope(MutationKind::CreateTask, "stomach"),
            &gate,
        )
        .expect("integrate");
    assert!(fabric.graph.task(&root_task_id("module-x")).is_some());
    assert!(fabric
        .graph
        .task(&phase_task_id(
            "module-x",
            infring_task_fabric_core_v1::StomachPhase::Ingested
        ))
        .is_some());
    assert!(
        fabric
            .graph
            .dependencies_of(&phase_task_id(
                "module-x",
                infring_task_fabric_core_v1::StomachPhase::Analyzed
            ))
            .len()
            >= 1
    );
}

#[test]
fn named_queries_support_low_token_operations() {
    let mut fabric = TaskFabric::new("scope-a");
    let gate = AllowAllVerityGate;
    fabric
        .submit_task(
            sample_task("scope-a", "task-a", "Task A"),
            envelope(MutationKind::CreateTask, "q1"),
            &gate,
        )
        .expect("create");
    let next = fabric
        .query_via_hierarchical_nexus(
            NexusConduitRequest {
                operation: "next_runnable".to_string(),
                args: json!({"scope":"scope-a"}),
            },
            now_ms(),
        )
        .expect("query next");
    assert_eq!(
        next.get("task")
            .and_then(|row| row.get("id"))
            .and_then(|row| row.as_str()),
        Some("task-a")
    );
    let summary = fabric
        .query_via_hierarchical_nexus(
            NexusConduitRequest {
                operation: "summary".to_string(),
                args: json!({"scope":"scope-a"}),
            },
            now_ms(),
        )
        .expect("summary");
    assert_eq!(summary.get("total").and_then(|row| row.as_u64()), Some(1));
}

#[test]
fn lease_heartbeat_and_concurrency_contracts_hold() {
    let mut fabric = TaskFabric::new("scope-a");
    let gate = AllowAllVerityGate;
    fabric
        .submit_task(
            sample_task("scope-a", "lease-task", "Lease Task"),
            envelope(MutationKind::CreateTask, "l1"),
            &gate,
        )
        .expect("create");
    let rev = fabric
        .graph
        .task("lease-task")
        .map(|v| v.revision_id)
        .unwrap_or(0);
    fabric
        .claim_lease(
            "lease-task",
            "agent-a",
            60_000,
            MutationEnvelope {
                expected_revision: Some(rev),
                ..envelope(MutationKind::ClaimLease, "l2")
            },
            &gate,
        )
        .expect("claim");
    let mismatch = fabric.heartbeat(
        "lease-task",
        30_000,
        MutationEnvelope {
            expected_revision: Some(rev),
            ..envelope(MutationKind::Heartbeat, "l3")
        },
        &gate,
    );
    assert!(mismatch.is_err());
    let same = fabric
        .claim_lease(
            "lease-task",
            "agent-a",
            60_000,
            MutationEnvelope {
                expected_revision: None,
                ..envelope(MutationKind::ClaimLease, "same-key")
            },
            &gate,
        )
        .expect("first");
    let deduped = fabric
        .claim_lease(
            "lease-task",
            "agent-a",
            60_000,
            MutationEnvelope {
                expected_revision: None,
                ..envelope(MutationKind::ClaimLease, "same-key")
            },
            &gate,
        )
        .expect("second");
    assert_eq!(same.event_id, deduped.event_id);
}

#[test]
fn failed_tasks_are_terminal() {
    let mut fabric = TaskFabric::new("scope-a");
    let gate = AllowAllVerityGate;
    fabric
        .submit_task(
            sample_task("scope-a", "terminal-failed", "Terminal Failed"),
            envelope(MutationKind::CreateTask, "tf1"),
            &gate,
        )
        .expect("create");
    let rev = fabric
        .graph
        .task("terminal-failed")
        .map(|v| v.revision_id)
        .unwrap_or(0);
    fabric
        .transition_status(
            "terminal-failed",
            LifecycleStatus::Failed,
            MutationEnvelope {
                expected_revision: Some(rev),
                ..envelope(MutationKind::UpdateStatus, "tf2")
            },
            &gate,
        )
        .expect("fail");
    let blocked = fabric.transition_status(
        "terminal-failed",
        LifecycleStatus::InProgress,
        MutationEnvelope {
            expected_revision: None,
            ..envelope(MutationKind::UpdateStatus, "tf3")
        },
        &gate,
    );
    assert!(blocked.is_err());
}

#[test]
fn cancelled_tasks_are_terminal() {
    let mut fabric = TaskFabric::new("scope-a");
    let gate = AllowAllVerityGate;
    fabric
        .submit_task(
            sample_task("scope-a", "terminal-cancelled", "Terminal Cancelled"),
            envelope(MutationKind::CreateTask, "tc1"),
            &gate,
        )
        .expect("create");
    let rev = fabric
        .graph
        .task("terminal-cancelled")
        .map(|v| v.revision_id)
        .unwrap_or(0);
    fabric
        .transition_status(
            "terminal-cancelled",
            LifecycleStatus::Cancelled,
            MutationEnvelope {
                expected_revision: Some(rev),
                ..envelope(MutationKind::UpdateStatus, "tc2")
            },
            &gate,
        )
        .expect("cancel");
    let blocked = fabric.transition_status(
        "terminal-cancelled",
        LifecycleStatus::InProgress,
        MutationEnvelope {
            expected_revision: None,
            ..envelope(MutationKind::UpdateStatus, "tc3")
        },
        &gate,
    );
    assert!(blocked.is_err());
}

struct DenyGate;

impl VerityGate for DenyGate {
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

#[test]
fn high_risk_mutations_require_synchronous_verity() {
    let mut fabric = TaskFabric::new("scope-a");
    let allow = AllowAllVerityGate;
    fabric
        .submit_task(
            sample_task("scope-a", "risk-task", "Risk Task"),
            envelope(MutationKind::CreateTask, "r1"),
            &allow,
        )
        .expect("create");
    let deny = DenyGate;
    let out = fabric.transition_status(
        "risk-task",
        LifecycleStatus::Cancelled,
        MutationEnvelope {
            payload: json!({"next_status":"cancelled"}),
            ..envelope(MutationKind::UpdateStatus, "r2")
        },
        &deny,
    );
    assert!(out.is_err());
}

#[test]
fn blockers_are_typed_and_survive_query() {
    let mut fabric = TaskFabric::new("scope-a");
    let gate = AllowAllVerityGate;
    let mut task = sample_task("scope-a", "b-task", "Blocked");
    task.blockers.push(Blocker {
        blocker_id: "blk-1".to_string(),
        kind: BlockerKind::External,
        reference_id: Some("ticket-123".to_string()),
        reason: "awaiting external approval".to_string(),
        resolved: false,
        metadata: json!({"team":"compliance"}),
    });
    fabric
        .submit_task(task, envelope(MutationKind::CreateTask, "b1"), &gate)
        .expect("create");
    let blocked = fabric
        .query_via_hierarchical_nexus(
            NexusConduitRequest {
                operation: "blocked_by".to_string(),
                args: json!({"task_id":"b-task"}),
            },
            now_ms(),
        )
        .expect("blocked query");
    assert_eq!(
        blocked
            .get("blockers")
            .and_then(|row| row.as_array())
            .map(|rows| rows.len()),
        Some(1)
    );
}

#[test]
fn mutation_requires_proof_refs() {
    let mut fabric = TaskFabric::new("scope-a");
    let gate = AllowAllVerityGate;
    let out = fabric.submit_task(
        sample_task("scope-a", "p-task", "Proof required"),
        MutationEnvelope {
            proof_refs: Vec::new(),
            ..envelope(MutationKind::CreateTask, "proof-required")
        },
        &gate,
    );
    assert!(out.is_err());
    assert_eq!(out.err().unwrap_or_default(), "proof_refs_required");
}

#[test]
fn terminal_tasks_are_counted_separately_and_excluded_from_stale_views() {
    let mut fabric = TaskFabric::new("scope-a");
    let gate = AllowAllVerityGate;
    fabric
        .submit_task(
            sample_task("scope-a", "terminal-summary", "Terminal Summary"),
            envelope(MutationKind::CreateTask, "ts1"),
            &gate,
        )
        .expect("create");
    let rev = fabric
        .graph
        .task("terminal-summary")
        .map(|v| v.revision_id)
        .unwrap_or(0);
    let heartbeat_ms = now_ms().saturating_sub(120_000);
    fabric
        .transition_status(
            "terminal-summary",
            LifecycleStatus::Failed,
            MutationEnvelope {
                expected_revision: Some(rev),
                now_ms: heartbeat_ms,
                ..envelope(MutationKind::UpdateStatus, "ts2")
            },
            &gate,
        )
        .expect("fail");
    fabric
        .graph
        .tasks
        .get_mut("terminal-summary")
        .expect("task")
        .last_heartbeat_at = Some(heartbeat_ms);

    let summary = fabric
        .query_via_hierarchical_nexus(
            NexusConduitRequest {
                operation: "summary".to_string(),
                args: json!({"scope":"scope-a"}),
            },
            now_ms(),
        )
        .expect("summary");
    assert_eq!(
        summary
            .get("readiness")
            .and_then(|row| row.get("terminal"))
            .and_then(|row| row.as_u64()),
        Some(1)
    );

    let stale = fabric
        .query_via_hierarchical_nexus(
            NexusConduitRequest {
                operation: "stale_tasks".to_string(),
                args: json!({"scope":"scope-a","age_ms":60_000}),
            },
            now_ms(),
        )
        .expect("stale");
    assert_eq!(
        stale
            .get("tasks")
            .and_then(|row| row.as_array())
            .map(|rows| rows.len()),
        Some(0)
    );
}
