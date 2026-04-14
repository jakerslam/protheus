use crate::context_topology::ContextSpan;
use std::collections::BTreeSet;

pub fn contiguous_coverage(children: &[ContextSpan]) -> bool {
    if children.is_empty() {
        return false;
    }
    let mut sorted = children.to_vec();
    sorted.sort_by(|a, b| a.start_seq.cmp(&b.start_seq).then(a.end_seq.cmp(&b.end_seq)));
    for pair in sorted.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        if left.end_seq.saturating_add(1) != right.start_seq {
            return false;
        }
    }
    true
}

pub fn rollup_fidelity_score(parent: &ContextSpan, children: &[ContextSpan]) -> f32 {
    if children.is_empty() {
        return 0.0;
    }
    let contiguous = if contiguous_coverage(children) { 1.0 } else { 0.0 };
    let child_start = children.iter().map(|row| row.start_seq).min().unwrap_or(0);
    let child_end = children.iter().map(|row| row.end_seq).max().unwrap_or(0);
    let exact_bounds = if parent.start_seq == child_start && parent.end_seq == child_end {
        1.0
    } else {
        0.0
    };
    let task_survival = set_survival(
        &children
            .iter()
            .flat_map(|row| row.task_refs.iter().cloned())
            .collect::<BTreeSet<_>>(),
        &parent.task_refs.iter().cloned().collect::<BTreeSet<_>>(),
    );
    let open_loop_survival = set_survival(
        &children
            .iter()
            .flat_map(|row| row.open_loops.iter().cloned())
            .collect::<BTreeSet<_>>(),
        &parent.open_loops.iter().cloned().collect::<BTreeSet<_>>(),
    );
    let decision_survival = set_survival(
        &children
            .iter()
            .flat_map(|row| row.decisions.iter().cloned())
            .collect::<BTreeSet<_>>(),
        &parent.decisions.iter().cloned().collect::<BTreeSet<_>>(),
    );
    let constraint_survival = set_survival(
        &children
            .iter()
            .flat_map(|row| row.constraints.iter().cloned())
            .collect::<BTreeSet<_>>(),
        &parent.constraints.iter().cloned().collect::<BTreeSet<_>>(),
    );
    let memory_ref_survival = set_survival(
        &children
            .iter()
            .flat_map(|row| row.memory_version_refs.iter().cloned())
            .collect::<BTreeSet<_>>(),
        &parent
            .memory_version_refs
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
    );

    (
        contiguous
            + exact_bounds
            + task_survival
            + open_loop_survival
            + decision_survival
            + constraint_survival
            + memory_ref_survival
    ) / 7.0
}

fn set_survival(required: &BTreeSet<String>, present: &BTreeSet<String>) -> f32 {
    if required.is_empty() {
        return 1.0;
    }
    let preserved = required.iter().filter(|row| present.contains(*row)).count();
    preserved as f32 / required.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_topology::{ContextSpan, ContextSpanStatus};

    fn span(id: &str, start: u64, end: u64, tasks: &[&str]) -> ContextSpan {
        ContextSpan {
            span_id: id.to_string(),
            session_id: "s".to_string(),
            level: 0,
            status: ContextSpanStatus::Sealed,
            start_seq: start,
            end_seq: end,
            child_refs: vec![],
            summary: "".to_string(),
            decisions: vec![],
            constraints: vec![],
            open_loops: vec![],
            entities: vec![],
            task_refs: tasks.iter().map(|row| row.to_string()).collect(),
            memory_version_refs: vec![],
            token_count: 10,
            heat_score: 0.5,
            fidelity_score: 1.0,
            receipt_id: "".to_string(),
            lineage_refs: vec![],
        }
    }

    #[test]
    fn contiguous_coverage_requires_no_gaps() {
        let ok = vec![span("a", 1, 3, &[]), span("b", 4, 5, &[])];
        assert!(contiguous_coverage(ok.as_slice()));
        let gap = vec![span("a", 1, 3, &[]), span("b", 5, 6, &[])];
        assert!(!contiguous_coverage(gap.as_slice()));
    }

    #[test]
    fn rollup_fidelity_penalizes_missing_task_refs() {
        let children = vec![span("a", 1, 2, &["t1"]), span("b", 3, 4, &["t2"])];
        let mut parent = span("p", 1, 4, &["t1", "t2"]);
        let full = rollup_fidelity_score(&parent, children.as_slice());
        assert!(full >= 0.99);
        parent.task_refs = vec!["t1".to_string()];
        let reduced = rollup_fidelity_score(&parent, children.as_slice());
        assert!(reduced < full);
    }
}

