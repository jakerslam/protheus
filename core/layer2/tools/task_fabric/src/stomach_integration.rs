use crate::task_graph::{DependencyEdge, LifecycleStatus, Task};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StomachPhase {
    Ingested,
    Analyzed,
    Proposed,
    Verified,
    Assimilated,
    Burned,
}

pub const STOMACH_TEMPLATE: [StomachPhase; 6] = [
    StomachPhase::Ingested,
    StomachPhase::Analyzed,
    StomachPhase::Proposed,
    StomachPhase::Verified,
    StomachPhase::Assimilated,
    StomachPhase::Burned,
];

pub fn root_task_id(item_key: &str) -> String {
    format!("stomach::{item_key}")
}

pub fn phase_task_id(item_key: &str, phase: StomachPhase) -> String {
    format!(
        "{}::{}",
        root_task_id(item_key),
        format!("{phase:?}").to_ascii_lowercase()
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StomachTemplateBundle {
    pub root: Task,
    pub phases: Vec<Task>,
    pub dependencies: Vec<DependencyEdge>,
}

pub fn build_stomach_template(
    scope_id: &str,
    item_key: &str,
    owner: Option<String>,
    assignee: Option<String>,
    now_ms: u64,
) -> StomachTemplateBundle {
    let root_id = root_task_id(item_key);
    let root = Task {
        id: root_id.clone(),
        title: format!("Assimilate {item_key}"),
        lifecycle_status: LifecycleStatus::InProgress,
        parent_id: None,
        priority: 100,
        owner: owner.clone(),
        assignee: assignee.clone(),
        progress_pct: Some(0),
        tags: vec!["stomach".to_string(), "assimilation".to_string()],
        linked_receipts: Vec::new(),
        metadata: json!({
            "template": "stomach_assimilation_v1",
            "item_key": item_key
        }),
        scope_id: scope_id.to_string(),
        blockers: Vec::new(),
        related_links: Vec::new(),
        created_at: now_ms,
        updated_at: now_ms,
        started_at: Some(now_ms),
        completed_at: None,
        last_heartbeat_at: Some(now_ms),
        lease_expires_at: None,
        revision_id: 0,
    };
    let mut phases = Vec::<Task>::new();
    let mut dependencies = Vec::<DependencyEdge>::new();
    let mut previous: Option<String> = None;
    for phase in STOMACH_TEMPLATE {
        let phase_id = phase_task_id(item_key, phase);
        let task = Task {
            id: phase_id.clone(),
            title: format!(
                "{} {item_key}",
                format!("{phase:?}")
                    .chars()
                    .flat_map(|c| c.to_lowercase())
                    .collect::<String>()
                    .replace('_', " ")
            ),
            lifecycle_status: LifecycleStatus::Pending,
            parent_id: Some(root_id.clone()),
            priority: 90,
            owner: owner.clone(),
            assignee: assignee.clone(),
            progress_pct: Some(0),
            tags: vec!["stomach".to_string(), "assimilation-phase".to_string()],
            linked_receipts: Vec::new(),
            metadata: json!({
                "template": "stomach_assimilation_v1",
                "item_key": item_key,
                "phase": format!("{phase:?}").to_ascii_lowercase()
            }),
            scope_id: scope_id.to_string(),
            blockers: Vec::new(),
            related_links: Vec::new(),
            created_at: now_ms,
            updated_at: now_ms,
            started_at: None,
            completed_at: None,
            last_heartbeat_at: None,
            lease_expires_at: None,
            revision_id: 0,
        };
        if let Some(depends_on) = previous.clone() {
            dependencies.push(DependencyEdge {
                task_id: phase_id.clone(),
                depends_on_task_id: depends_on,
            });
        }
        previous = Some(phase_id);
        phases.push(task);
    }
    StomachTemplateBundle {
        root,
        phases,
        dependencies,
    }
}
