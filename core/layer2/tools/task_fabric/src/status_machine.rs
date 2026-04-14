// Layer ownership: core/layer2/tools/task_fabric (authoritative task graph primitive).
use crate::task_graph::{LifecycleStatus, Task};

pub fn can_transition(from: LifecycleStatus, to: LifecycleStatus) -> bool {
    use LifecycleStatus::*;
    match (from, to) {
        (Pending, InProgress | Failed | Cancelled) => true,
        (InProgress, Review | Completed | Failed | Cancelled) => true,
        (Review, InProgress | Completed | Failed | Cancelled) => true,
        (a, b) if a == b => true,
        _ => false,
    }
}

pub fn apply_transition(task: &mut Task, to: LifecycleStatus, now_ms: u64) -> Result<(), String> {
    if !can_transition(task.lifecycle_status, to) {
        return Err("invalid_lifecycle_transition".to_string());
    }
    if matches!(to, LifecycleStatus::InProgress) && task.started_at.is_none() {
        task.started_at = Some(now_ms);
    }
    if matches!(to, LifecycleStatus::Completed) {
        task.progress_pct = Some(100);
        task.completed_at = Some(now_ms);
    }
    if matches!(to, LifecycleStatus::Cancelled | LifecycleStatus::Failed) {
        task.completed_at = Some(now_ms);
    }
    task.lifecycle_status = to;
    task.updated_at = now_ms;
    task.revision_id = task.revision_id.saturating_add(1);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_and_cancelled_states_do_not_reopen() {
        assert!(!can_transition(
            LifecycleStatus::Failed,
            LifecycleStatus::InProgress
        ));
        assert!(!can_transition(
            LifecycleStatus::Failed,
            LifecycleStatus::Cancelled
        ));
        assert!(!can_transition(
            LifecycleStatus::Cancelled,
            LifecycleStatus::InProgress
        ));
        assert!(!can_transition(
            LifecycleStatus::Cancelled,
            LifecycleStatus::Review
        ));
    }
}
