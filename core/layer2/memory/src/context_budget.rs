use crate::context_atoms::ContextAtom;
use crate::context_topology::{ContextFrontier, ContextPressureState, ContextSpan, ContextSpanStatus};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const DEFAULT_PINNED_TOKEN_ESTIMATE: u32 = 32;
const HOT_TAIL_COUNT: usize = 4;
const DESCEND_HEAT_THRESHOLD: f32 = 0.62;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBudgetRequest {
    pub session_id: String,
    pub budget_tokens: u32,
    pub pinned_anchor_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextBudgetReport {
    pub budget_tokens: u32,
    pub used_tokens: u32,
    pub hot_tokens: u32,
    pub warm_tokens: u32,
    pub cool_tokens: u32,
    pub cold_tokens: u32,
    pub pinned_tokens: u32,
    pub fidelity_score: f32,
    pub pressure_state: ContextPressureState,
}

pub fn build_frontier(
    request: &ContextBudgetRequest,
    atoms: &[ContextAtom],
    spans: &[ContextSpan],
    previous_frontier: Option<&ContextFrontier>,
) -> (ContextFrontier, ContextBudgetReport) {
    let mut used_tokens = 0u32;
    let mut hot_tokens = 0u32;
    let mut warm_tokens = 0u32;
    let mut cool_tokens = 0u32;
    let mut cold_tokens = 0u32;
    let pinned_tokens =
        DEFAULT_PINNED_TOKEN_ESTIMATE.saturating_mul(request.pinned_anchor_refs.len() as u32);

    let mut hot_atom_refs = Vec::<String>::new();
    let mut warm_span_refs = Vec::<String>::new();
    let mut cool_span_refs = Vec::<String>::new();
    let mut cold_span_refs = Vec::<String>::new();

    let budget = request.budget_tokens.max(1);
    used_tokens = used_tokens.saturating_add(pinned_tokens.min(budget));
    let hot_atoms = select_hot_atoms(atoms);
    for atom in hot_atoms {
        if used_tokens.saturating_add(atom.token_count) > budget {
            break;
        }
        hot_tokens = hot_tokens.saturating_add(atom.token_count);
        used_tokens = used_tokens.saturating_add(atom.token_count);
        hot_atom_refs.push(atom.atom_id.clone());
    }

    let mut sealed_by_level = BTreeMap::<u32, Vec<&ContextSpan>>::new();
    for span in spans {
        if !matches!(span.status, ContextSpanStatus::Sealed) {
            continue;
        }
        sealed_by_level.entry(span.level).or_default().push(span);
    }
    for rows in sealed_by_level.values_mut() {
        rows.sort_by(|a, b| {
            b.heat_score
                .total_cmp(&a.heat_score)
                .then(b.end_seq.cmp(&a.end_seq))
        });
    }

    for span in sealed_by_level.get(&0).cloned().unwrap_or_default() {
        if used_tokens.saturating_add(span.token_count) > budget {
            continue;
        }
        warm_tokens = warm_tokens.saturating_add(span.token_count);
        used_tokens = used_tokens.saturating_add(span.token_count);
        warm_span_refs.push(span.span_id.clone());
    }

    for level in [1u32, 2u32] {
        for span in sealed_by_level.get(&level).cloned().unwrap_or_default() {
            if used_tokens.saturating_add(span.token_count) > budget {
                continue;
            }
            cool_tokens = cool_tokens.saturating_add(span.token_count);
            used_tokens = used_tokens.saturating_add(span.token_count);
            cool_span_refs.push(span.span_id.clone());
        }
    }

    for (level, rows) in &sealed_by_level {
        if *level < 3 {
            continue;
        }
        for span in rows {
            if previous_pressure_blocks_descent(previous_frontier, span.heat_score) {
                continue;
            }
            if used_tokens.saturating_add(span.token_count) > budget {
                continue;
            }
            cold_tokens = cold_tokens.saturating_add(span.token_count);
            used_tokens = used_tokens.saturating_add(span.token_count);
            cold_span_refs.push(span.span_id.clone());
        }
    }

    let fidelity_score = weighted_fidelity(spans, &warm_span_refs, &cool_span_refs, &cold_span_refs);
    let pressure_state = pressure_state(budget, used_tokens);
    let frontier = ContextFrontier {
        session_id: request.session_id.clone(),
        budget_tokens: budget,
        used_tokens,
        hot_atom_refs,
        warm_span_refs,
        cool_span_refs,
        cold_span_refs,
        pinned_anchor_refs: dedupe(request.pinned_anchor_refs.clone()),
        pressure_state: pressure_state.clone(),
        fidelity_score,
    };
    let report = ContextBudgetReport {
        budget_tokens: budget,
        used_tokens,
        hot_tokens,
        warm_tokens,
        cool_tokens,
        cold_tokens,
        pinned_tokens: pinned_tokens.min(budget),
        fidelity_score,
        pressure_state,
    };
    (frontier, report)
}

fn select_hot_atoms(atoms: &[ContextAtom]) -> Vec<&ContextAtom> {
    let mut sorted = atoms.iter().collect::<Vec<_>>();
    sorted.sort_by(|a, b| b.sequence_no.cmp(&a.sequence_no));
    sorted.into_iter().take(HOT_TAIL_COUNT).collect::<Vec<_>>()
}

fn weighted_fidelity(
    spans: &[ContextSpan],
    warm_refs: &[String],
    cool_refs: &[String],
    cold_refs: &[String],
) -> f32 {
    let selected = warm_refs
        .iter()
        .chain(cool_refs.iter())
        .chain(cold_refs.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    if selected.is_empty() {
        return 1.0;
    }
    let mut total = 0.0f32;
    let mut count = 0usize;
    for span in spans {
        if selected.contains(&span.span_id) {
            total += span.fidelity_score;
            count += 1;
        }
    }
    if count == 0 {
        1.0
    } else {
        (total / count as f32).clamp(0.0, 1.0)
    }
}

fn pressure_state(budget_tokens: u32, used_tokens: u32) -> ContextPressureState {
    let ratio = used_tokens as f32 / budget_tokens.max(1) as f32;
    if ratio >= 0.92 {
        ContextPressureState::High
    } else if ratio >= 0.70 {
        ContextPressureState::Medium
    } else {
        ContextPressureState::Low
    }
}

fn previous_pressure_blocks_descent(
    previous_frontier: Option<&ContextFrontier>,
    candidate_heat: f32,
) -> bool {
    match previous_frontier.map(|row| &row.pressure_state) {
        Some(ContextPressureState::High) => candidate_heat < DESCEND_HEAT_THRESHOLD,
        _ => false,
    }
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut out = values
        .into_iter()
        .filter(|row| !row.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_atoms::{ContextAtom, ContextAtomSourceKind};

    fn atom(seq: u64, tokens: u32) -> ContextAtom {
        ContextAtom {
            atom_id: format!("a{seq}"),
            session_id: "s".to_string(),
            sequence_no: seq,
            source_kind: ContextAtomSourceKind::InteractionUnit,
            source_ref: "r".to_string(),
            token_count: tokens,
            timestamp_ms: seq,
            task_refs: vec![],
            memory_version_refs: vec![],
            lineage_refs: vec![],
        }
    }

    fn span(id: &str, level: u32, tokens: u32, heat: f32) -> ContextSpan {
        ContextSpan {
            span_id: id.to_string(),
            session_id: "s".to_string(),
            level,
            status: ContextSpanStatus::Sealed,
            start_seq: 1,
            end_seq: 2,
            child_refs: vec![],
            summary: String::new(),
            decisions: vec![],
            constraints: vec![],
            open_loops: vec![],
            entities: vec![],
            task_refs: vec![],
            memory_version_refs: vec![],
            token_count: tokens,
            heat_score: heat,
            fidelity_score: 0.9,
            receipt_id: String::new(),
            lineage_refs: vec![],
        }
    }

    #[test]
    fn budget_frontier_prioritizes_hot_then_warm() {
        let req = ContextBudgetRequest {
            session_id: "s".to_string(),
            budget_tokens: 180,
            pinned_anchor_refs: vec!["pin".to_string()],
        };
        let atoms = vec![atom(1, 20), atom(2, 20), atom(3, 20), atom(4, 20), atom(5, 20)];
        let spans = vec![span("w1", 0, 40, 0.8), span("c1", 1, 60, 0.7)];
        let (frontier, report) = build_frontier(&req, &atoms, &spans, None);
        assert!(!frontier.hot_atom_refs.is_empty());
        assert!(!frontier.warm_span_refs.is_empty());
        assert!(report.used_tokens <= req.budget_tokens);
    }

    #[test]
    fn previous_high_pressure_requires_hotter_cold_descent() {
        let req = ContextBudgetRequest {
            session_id: "s".to_string(),
            budget_tokens: 400,
            pinned_anchor_refs: vec![],
        };
        let atoms = vec![atom(1, 30)];
        let spans = vec![span("cold-low", 3, 60, 0.2), span("cold-high", 3, 60, 0.9)];
        let previous = ContextFrontier {
            session_id: "s".to_string(),
            budget_tokens: 100,
            used_tokens: 100,
            hot_atom_refs: vec![],
            warm_span_refs: vec![],
            cool_span_refs: vec![],
            cold_span_refs: vec![],
            pinned_anchor_refs: vec![],
            pressure_state: ContextPressureState::High,
            fidelity_score: 1.0,
        };
        let (frontier, _) = build_frontier(&req, &atoms, &spans, Some(&previous));
        assert!(frontier.cold_span_refs.contains(&"cold-high".to_string()));
        assert!(!frontier.cold_span_refs.contains(&"cold-low".to_string()));
    }
}
