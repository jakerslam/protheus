use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub type TaskId = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleStatus {
    Pending,
    InProgress,
    Review,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReadinessStatus {
    Runnable,
    Blocked,
    Leased,
    Stale,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockerKind {
    Task,
    External,
    Policy,
    Resource,
    HumanApproval,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Blocker {
    pub blocker_id: String,
    pub kind: BlockerKind,
    pub reference_id: Option<String>,
    pub reason: String,
    pub resolved: bool,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelatedLink {
    pub target_task_id: TaskId,
    pub relation: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyEdge {
    pub task_id: TaskId,
    pub depends_on_task_id: TaskId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub lifecycle_status: LifecycleStatus,
    pub parent_id: Option<TaskId>,
    pub priority: i32,
    pub owner: Option<String>,
    pub assignee: Option<String>,
    pub progress_pct: Option<u8>,
    pub tags: Vec<String>,
    pub linked_receipts: Vec<String>,
    pub metadata: Value,
    pub scope_id: String,
    pub blockers: Vec<Blocker>,
    pub related_links: Vec<RelatedLink>,
    pub created_at: u64,
    pub updated_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub last_heartbeat_at: Option<u64>,
    pub lease_expires_at: Option<u64>,
    pub revision_id: u64,
}

impl Task {
    pub fn unresolved_blockers(&self) -> bool {
        self.blockers.iter().any(|row| !row.resolved)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TaskGraph {
    pub scope_id: String,
    pub tasks: BTreeMap<TaskId, Task>,
    pub children: BTreeMap<TaskId, BTreeSet<TaskId>>,
    pub dependencies: BTreeMap<TaskId, BTreeSet<TaskId>>,
}

impl TaskGraph {
    pub fn new(scope_id: impl Into<String>) -> Self {
        Self {
            scope_id: scope_id.into(),
            ..Self::default()
        }
    }

    pub fn task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    pub fn task_mut(&mut self, task_id: &str) -> Option<&mut Task> {
        self.tasks.get_mut(task_id)
    }

    pub fn insert_task(&mut self, task: Task) -> Result<(), String> {
        if task.scope_id != self.scope_id {
            return Err("task_scope_mismatch".to_string());
        }
        if self.tasks.contains_key(&task.id) {
            return Err("task_id_already_exists".to_string());
        }
        self.tasks.insert(task.id.clone(), task);
        Ok(())
    }

    pub fn set_parent(&mut self, child_id: &str, parent_id: &str) -> Result<(), String> {
        if child_id == parent_id {
            return Err("parent_cycle_detected".to_string());
        }
        if !self.tasks.contains_key(child_id) || !self.tasks.contains_key(parent_id) {
            return Err("task_not_found".to_string());
        }
        if self.parent_chain_contains(parent_id, child_id) {
            return Err("parent_cycle_detected".to_string());
        }
        if let Some(task) = self.tasks.get_mut(child_id) {
            task.parent_id = Some(parent_id.to_string());
        }
        self.children
            .entry(parent_id.to_string())
            .or_default()
            .insert(child_id.to_string());
        Ok(())
    }

    fn parent_chain_contains(&self, start: &str, needle: &str) -> bool {
        let mut cursor = Some(start.to_string());
        let mut seen = BTreeSet::<String>::new();
        while let Some(current) = cursor {
            if current == needle {
                return true;
            }
            if !seen.insert(current.clone()) {
                return true;
            }
            cursor = self
                .tasks
                .get(&current)
                .and_then(|task| task.parent_id.clone());
        }
        false
    }

    pub fn add_dependency(&mut self, edge: DependencyEdge) -> Result<(), String> {
        if edge.task_id == edge.depends_on_task_id {
            return Err("dependency_cycle_detected".to_string());
        }
        if !self.tasks.contains_key(&edge.task_id)
            || !self.tasks.contains_key(&edge.depends_on_task_id)
        {
            return Err("task_not_found".to_string());
        }
        self.dependencies
            .entry(edge.task_id.clone())
            .or_default()
            .insert(edge.depends_on_task_id.clone());
        if self.validate_dependency_acyclic().is_err() {
            if let Some(rows) = self.dependencies.get_mut(&edge.task_id) {
                rows.remove(&edge.depends_on_task_id);
            }
            return Err("dependency_cycle_detected".to_string());
        }
        Ok(())
    }

    pub fn dependencies_of(&self, task_id: &str) -> Vec<String> {
        self.dependencies
            .get(task_id)
            .map(|rows| rows.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    pub fn children_of(&self, task_id: &str) -> Vec<String> {
        self.children
            .get(task_id)
            .map(|rows| rows.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    pub fn validate_dependency_acyclic(&self) -> Result<(), String> {
        fn visit(
            node: &str,
            graph: &TaskGraph,
            temp: &mut BTreeSet<String>,
            done: &mut BTreeSet<String>,
        ) -> Result<(), String> {
            if done.contains(node) {
                return Ok(());
            }
            if !temp.insert(node.to_string()) {
                return Err("dependency_cycle_detected".to_string());
            }
            for dep in graph.dependencies_of(node) {
                visit(&dep, graph, temp, done)?;
            }
            temp.remove(node);
            done.insert(node.to_string());
            Ok(())
        }

        let mut temp = BTreeSet::<String>::new();
        let mut done = BTreeSet::<String>::new();
        for id in self.tasks.keys() {
            visit(id, self, &mut temp, &mut done)?;
        }
        Ok(())
    }

    pub fn derive_readiness(
        &self,
        task_id: &str,
        now_ms: u64,
        stale_after_ms: u64,
    ) -> Option<ReadinessStatus> {
        let task = self.tasks.get(task_id)?;
        let leased = task.lease_expires_at.map(|ts| ts > now_ms).unwrap_or(false);
        if leased {
            return Some(ReadinessStatus::Leased);
        }
        let stale = task
            .last_heartbeat_at
            .map(|ts| now_ms.saturating_sub(ts) > stale_after_ms)
            .unwrap_or(false);
        if stale {
            return Some(ReadinessStatus::Stale);
        }
        if task.unresolved_blockers() {
            return Some(ReadinessStatus::Blocked);
        }
        let has_blocking_dep = self.dependencies_of(task_id).iter().any(|dep| {
            self.tasks
                .get(dep)
                .map(|v| v.lifecycle_status != LifecycleStatus::Completed)
                .unwrap_or(true)
        });
        if has_blocking_dep {
            return Some(ReadinessStatus::Blocked);
        }
        match task.lifecycle_status {
            LifecycleStatus::Pending | LifecycleStatus::InProgress | LifecycleStatus::Review => {
                Some(ReadinessStatus::Runnable)
            }
            LifecycleStatus::Completed | LifecycleStatus::Failed | LifecycleStatus::Cancelled => {
                Some(ReadinessStatus::Blocked)
            }
        }
    }
}
