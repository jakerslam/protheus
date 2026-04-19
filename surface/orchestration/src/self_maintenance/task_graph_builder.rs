use crate::self_maintenance::contracts::{ClaimBundle, ClaimStatus, RemediationClass};
use infring_task_fabric_core_v1::{
    Blocker, BlockerKind, DependencyEdge, LifecycleStatus, RelatedLink, Task,
};
use serde_json::json;

#[derive(Debug, Clone)]
pub struct MaintenanceTaskGraph {
    pub tasks: Vec<Task>,
    pub dependencies: Vec<DependencyEdge>,
}

pub fn build_task_graph_from_claim_bundle(
    claim_bundle: &ClaimBundle,
    scope_id: &str,
    now_ms: u64,
) -> MaintenanceTaskGraph {
    let root_task_id = format!("maintenance-root-{}", claim_bundle.claim_bundle_id);
    let mut tasks = vec![new_task(NewTaskParams {
        id: root_task_id.as_str(),
        title: "governed self-maintenance cycle",
        scope_id,
        now_ms,
        parent_id: None,
        tags: vec!["self_maintenance".to_string(), "root".to_string()],
        blockers: Vec::new(),
        related_links: Vec::new(),
        metadata: json!({
            "owner":"system",
            "claim_bundle_id": claim_bundle.claim_bundle_id,
        }),
    })];
    let mut dependencies = Vec::<DependencyEdge>::new();

    for claim in &claim_bundle.claims {
        let task_id = format!("maintenance-claim-{}", claim.claim_id);
        let mut blockers = Vec::<Blocker>::new();
        if claim.status == ClaimStatus::Unsupported {
            blockers.push(Blocker {
                blocker_id: format!("blocker-unsupported-{}", claim.claim_id),
                kind: BlockerKind::Policy,
                reference_id: Some(claim.claim_id.clone()),
                reason: "claim confidence below support threshold".to_string(),
                resolved: false,
                metadata: json!({"claim_status":"unsupported"}),
            });
        }
        if claim.status == ClaimStatus::Conflicting {
            blockers.push(Blocker {
                blocker_id: format!("blocker-conflict-{}", claim.claim_id),
                kind: BlockerKind::External,
                reference_id: Some(claim.claim_id.clone()),
                reason: "conflicting evidence requires review".to_string(),
                resolved: false,
                metadata: json!({"conflict_refs": claim.conflict_refs}),
            });
        }

        let related_links = vec![RelatedLink {
            target_task_id: root_task_id.clone(),
            relation: "maintenance_cycle_member".to_string(),
            metadata: json!({
                "claim_id": claim.claim_id,
                "remediation_class": format!("{:?}", claim.remediation_class).to_ascii_lowercase()
            }),
        }];

        tasks.push(new_task(NewTaskParams {
            id: task_id.as_str(),
            title: claim.text.as_str(),
            scope_id,
            now_ms,
            parent_id: Some(root_task_id.clone()),
            tags: vec![
                "self_maintenance".to_string(),
                "claim".to_string(),
                remediation_tag(claim.remediation_class),
            ],
            blockers,
            related_links,
            metadata: json!({
                "owner":"system",
                "claim_id": claim.claim_id,
                "claim_type": format!("{:?}", claim.claim_type).to_ascii_lowercase(),
            }),
        }));
        dependencies.push(DependencyEdge {
            task_id,
            depends_on_task_id: root_task_id.clone(),
        });
    }

    MaintenanceTaskGraph {
        tasks,
        dependencies,
    }
}

struct NewTaskParams<'a> {
    id: &'a str,
    title: &'a str,
    scope_id: &'a str,
    now_ms: u64,
    parent_id: Option<String>,
    tags: Vec<String>,
    blockers: Vec<Blocker>,
    related_links: Vec<RelatedLink>,
    metadata: serde_json::Value,
}

fn new_task(params: NewTaskParams<'_>) -> Task {
    Task {
        id: params.id.to_string(),
        title: params.title.to_string(),
        lifecycle_status: LifecycleStatus::Pending,
        parent_id: params.parent_id,
        priority: 90,
        owner: Some("system".to_string()),
        assignee: None,
        progress_pct: Some(0),
        tags: params.tags,
        linked_receipts: Vec::new(),
        metadata: params.metadata,
        scope_id: params.scope_id.to_string(),
        blockers: params.blockers,
        related_links: params.related_links,
        created_at: params.now_ms,
        updated_at: params.now_ms,
        started_at: None,
        completed_at: None,
        last_heartbeat_at: None,
        lease_expires_at: None,
        revision_id: 0,
    }
}

fn remediation_tag(class: RemediationClass) -> String {
    match class {
        RemediationClass::DocsDriftFix => "docs_drift_fix".to_string(),
        RemediationClass::PathCorrection => "path_correction".to_string(),
        RemediationClass::CleanupTask => "cleanup_task".to_string(),
        RemediationClass::BacklogHygiene => "backlog_hygiene".to_string(),
        RemediationClass::Unsafe => "unsafe".to_string(),
    }
}
