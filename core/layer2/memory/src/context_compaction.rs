use crate::context_fidelity::rollup_fidelity_score;
use crate::context_topology::{ContextSpan, ContextSpanStatus, DEFAULT_FANOUT_TARGET};
use crate::deterministic_hash;
use std::collections::BTreeSet;

pub const DEFAULT_LEVEL0_TOKEN_TARGET: u32 = 2200;
pub const MIN_ROLLUP_FIDELITY: f32 = 0.65;

pub fn should_seal_level0(
    active_span: &ContextSpan,
    semantic_boundary: bool,
    workflow_boundary: bool,
) -> bool {
    if !matches!(active_span.status, ContextSpanStatus::Active) {
        return false;
    }
    if semantic_boundary || workflow_boundary {
        return true;
    }
    active_span.child_refs.len() >= DEFAULT_FANOUT_TARGET
        || active_span.token_count >= DEFAULT_LEVEL0_TOKEN_TARGET
}

pub fn build_rollup_parent(
    session_id: &str,
    level: u32,
    children: &[ContextSpan],
) -> Result<ContextSpan, String> {
    if children.is_empty() {
        return Err("context_rollup_requires_children".to_string());
    }
    let mut ordered = children.to_vec();
    ordered.sort_by(|a, b| {
        a.start_seq
            .cmp(&b.start_seq)
            .then(a.end_seq.cmp(&b.end_seq))
    });
    let start_seq = ordered.first().map(|row| row.start_seq).unwrap_or(0);
    let end_seq = ordered.last().map(|row| row.end_seq).unwrap_or(0);
    let child_refs = ordered
        .iter()
        .map(|row| row.span_id.clone())
        .collect::<Vec<_>>();
    let decisions = dedupe_sorted(ordered.iter().flat_map(|row| row.decisions.iter().cloned()));
    let constraints = dedupe_sorted(
        ordered
            .iter()
            .flat_map(|row| row.constraints.iter().cloned()),
    );
    let open_loops = dedupe_sorted(
        ordered
            .iter()
            .flat_map(|row| row.open_loops.iter().cloned()),
    );
    let entities = dedupe_sorted(ordered.iter().flat_map(|row| row.entities.iter().cloned()));
    let task_refs = dedupe_sorted(ordered.iter().flat_map(|row| row.task_refs.iter().cloned()));
    let memory_version_refs = dedupe_sorted(
        ordered
            .iter()
            .flat_map(|row| row.memory_version_refs.iter().cloned()),
    );
    let lineage_refs = dedupe_sorted(
        ordered
            .iter()
            .flat_map(|row| row.lineage_refs.iter().cloned()),
    );
    let token_count = ordered.iter().map(|row| row.token_count).sum::<u32>();
    let heat_score =
        ordered.iter().map(|row| row.heat_score).sum::<f32>() / ordered.len() as f32 * 0.88;
    let span_id = format!(
        "ctx_span_{}",
        &deterministic_hash(&(
            session_id.to_string(),
            level,
            start_seq,
            end_seq,
            child_refs.clone()
        ))[..24]
    );
    let summary = format!(
        "context rollup level={} covers {}-{} from {} children",
        level,
        start_seq,
        end_seq,
        child_refs.len()
    );

    let mut parent = ContextSpan {
        span_id,
        session_id: session_id.to_string(),
        level,
        status: ContextSpanStatus::Sealed,
        start_seq,
        end_seq,
        child_refs,
        summary,
        decisions,
        constraints,
        open_loops,
        entities,
        task_refs,
        memory_version_refs,
        token_count,
        heat_score,
        fidelity_score: 0.0,
        receipt_id: String::new(),
        lineage_refs,
    };
    let fidelity = rollup_fidelity_score(&parent, ordered.as_slice());
    if fidelity < MIN_ROLLUP_FIDELITY {
        return Err("context_rollup_fidelity_below_threshold".to_string());
    }
    parent.fidelity_score = fidelity;
    Ok(parent)
}

pub fn contiguous_exact_coverage(parent: &ContextSpan, children: &[ContextSpan]) -> bool {
    if children.is_empty() {
        return false;
    }
    let mut sorted = children.to_vec();
    sorted.sort_by(|a, b| {
        a.start_seq
            .cmp(&b.start_seq)
            .then(a.end_seq.cmp(&b.end_seq))
    });
    let start_ok = sorted.first().map(|row| row.start_seq) == Some(parent.start_seq);
    let end_ok = sorted.last().map(|row| row.end_seq) == Some(parent.end_seq);
    if !(start_ok && end_ok) {
        return false;
    }
    for pair in sorted.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        if left.end_seq.saturating_add(1) != right.start_seq {
            return false;
        }
    }
    true
}

fn dedupe_sorted(iter: impl Iterator<Item = String>) -> Vec<String> {
    let mut set = BTreeSet::<String>::new();
    for item in iter {
        if !item.trim().is_empty() {
            set.insert(item);
        }
    }
    set.into_iter().collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(id: &str, start: u64, end: u64) -> ContextSpan {
        ContextSpan {
            span_id: id.to_string(),
            session_id: "s".to_string(),
            level: 0,
            status: ContextSpanStatus::Sealed,
            start_seq: start,
            end_seq: end,
            child_refs: vec![],
            summary: String::new(),
            decisions: vec!["d".to_string()],
            constraints: vec!["c".to_string()],
            open_loops: vec!["o".to_string()],
            entities: vec!["e".to_string()],
            task_refs: vec!["task".to_string()],
            memory_version_refs: vec!["mem".to_string()],
            token_count: 20,
            heat_score: 0.7,
            fidelity_score: 1.0,
            receipt_id: String::new(),
            lineage_refs: vec!["lin".to_string()],
        }
    }

    #[test]
    fn contiguous_exact_coverage_rejects_gaps() {
        let a = span("a", 1, 2);
        let b = span("b", 4, 5);
        let parent = build_rollup_parent("s", 1, &[a.clone(), b.clone()])
            .expect("parent still builds; gaps are validated separately");
        assert!(!contiguous_exact_coverage(&parent, &[a, b]));
    }

    #[test]
    fn rollup_builds_parent_with_preserved_refs() {
        let a = span("a", 1, 2);
        let b = span("b", 3, 4);
        let parent = build_rollup_parent("s", 1, &[a, b]).expect("rollup");
        assert_eq!(parent.start_seq, 1);
        assert_eq!(parent.end_seq, 4);
        assert!(parent.task_refs.contains(&"task".to_string()));
        assert!(parent.fidelity_score >= MIN_ROLLUP_FIDELITY);
    }
}
