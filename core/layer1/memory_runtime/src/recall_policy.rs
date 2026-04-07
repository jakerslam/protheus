// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer1/memory_runtime::recall_policy (authoritative)

pub const DEFAULT_RECALL_TOP: usize = 5;
pub const MAX_RECALL_TOP: usize = 50;
pub const DEFAULT_MAX_FILES: usize = 1;
pub const MAX_MAX_FILES: usize = 20;
pub const DEFAULT_EXPAND_LINES: usize = 0;
pub const MAX_EXPAND_LINES: usize = 300;
pub const DEFAULT_INDEX_MAX_AGE_MS: u64 = 1_200_000;
pub const DEFAULT_BOOTSTRAP_HYDRATION_TOKEN_CAP: u32 = 48;
pub const DEFAULT_BURN_THRESHOLD_TOKENS: u32 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailClosedMode {
    Reject,
    Trim,
}

impl FailClosedMode {
    pub fn from_raw(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "trim" | "cap" | "clamp" => Self::Trim,
            _ => Self::Reject,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecallBudgetInput {
    pub requested_top: usize,
    pub requested_max_files: usize,
    pub requested_expand_lines: usize,
    pub mode: FailClosedMode,
    pub max_top: usize,
    pub max_files: usize,
    pub max_expand_lines: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecallBudgetDecision {
    pub ok: bool,
    pub reason_code: &'static str,
    pub trimmed: bool,
    pub effective_top: usize,
    pub effective_max_files: usize,
    pub effective_expand_lines: usize,
}

pub fn enforce_recall_budget(input: &RecallBudgetInput) -> RecallBudgetDecision {
    let exceeds = input.requested_top > input.max_top
        || input.requested_max_files > input.max_files
        || input.requested_expand_lines > input.max_expand_lines;
    if exceeds && matches!(input.mode, FailClosedMode::Reject) {
        return RecallBudgetDecision {
            ok: false,
            reason_code: "recall_budget_exceeded",
            trimmed: false,
            effective_top: input.max_top,
            effective_max_files: input.max_files,
            effective_expand_lines: input.max_expand_lines,
        };
    }

    let effective_top = input.requested_top.clamp(1, input.max_top);
    let effective_max_files = input.requested_max_files.clamp(1, input.max_files);
    let effective_expand_lines = input
        .requested_expand_lines
        .clamp(0, input.max_expand_lines);
    RecallBudgetDecision {
        ok: true,
        reason_code: if exceeds {
            "recall_budget_trimmed"
        } else {
            "within_budget"
        },
        trimmed: exceeds,
        effective_top,
        effective_max_files,
        effective_expand_lines,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexFirstDecision {
    pub ok: bool,
    pub reason_code: &'static str,
}

pub fn enforce_index_first(index_sources: &[String], entries_total: usize) -> IndexFirstDecision {
    if entries_total == 0 {
        return IndexFirstDecision {
            ok: false,
            reason_code: "index_entries_missing",
        };
    }
    if index_sources.is_empty() {
        return IndexFirstDecision {
            ok: false,
            reason_code: "index_sources_missing",
        };
    }
    IndexFirstDecision {
        ok: true,
        reason_code: "index_authority_ok",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeOnlyDecision {
    pub ok: bool,
    pub reason_code: &'static str,
}

pub fn enforce_node_only(node_id: &str, uid: &str) -> NodeOnlyDecision {
    if node_id.trim().is_empty() && uid.trim().is_empty() {
        return NodeOnlyDecision {
            ok: false,
            reason_code: "missing_node_or_uid",
        };
    }
    NodeOnlyDecision {
        ok: true,
        reason_code: "node_scoped_lookup",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HydrationGuardInput {
    pub bootstrap: bool,
    pub lazy_hydration: bool,
    pub estimated_hydration_tokens: u32,
    pub max_bootstrap_tokens: u32,
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HydrationGuardDecision {
    pub ok: bool,
    pub reason_code: &'static str,
}

pub fn enforce_hydration_guard(input: &HydrationGuardInput) -> HydrationGuardDecision {
    if !input.bootstrap {
        return HydrationGuardDecision {
            ok: true,
            reason_code: "not_bootstrap",
        };
    }
    if input.force {
        return HydrationGuardDecision {
            ok: true,
            reason_code: "bootstrap_force_override",
        };
    }
    if !input.lazy_hydration {
        return HydrationGuardDecision {
            ok: false,
            reason_code: "bootstrap_requires_lazy_hydration",
        };
    }
    if input.estimated_hydration_tokens > input.max_bootstrap_tokens {
        return HydrationGuardDecision {
            ok: false,
            reason_code: "bootstrap_hydration_token_cap_exceeded",
        };
    }
    HydrationGuardDecision {
        ok: true,
        reason_code: "bootstrap_hydration_ok",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshnessDecision {
    pub ok: bool,
    pub stale: bool,
    pub reason_code: &'static str,
    pub age_ms: Option<u64>,
    pub threshold_ms: u64,
}

pub fn enforce_index_freshness(
    now_ms: u64,
    newest_mtime_ms: Option<u64>,
    max_age_ms: u64,
    allow_stale: bool,
) -> FreshnessDecision {
    let threshold = max_age_ms.max(1);
    let Some(newest) = newest_mtime_ms else {
        return FreshnessDecision {
            ok: allow_stale,
            stale: true,
            reason_code: if allow_stale {
                "freshness_timestamp_missing_allowed"
            } else {
                "freshness_timestamp_missing_blocked"
            },
            age_ms: None,
            threshold_ms: threshold,
        };
    };

    let age = now_ms.saturating_sub(newest);
    let stale = age > threshold;
    if stale && !allow_stale {
        return FreshnessDecision {
            ok: false,
            stale: true,
            reason_code: "index_stale_blocked",
            age_ms: Some(age),
            threshold_ms: threshold,
        };
    }

    FreshnessDecision {
        ok: true,
        stale,
        reason_code: if stale {
            "index_stale_allowed"
        } else {
            "index_fresh"
        },
        age_ms: Some(age),
        threshold_ms: threshold,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankingInvariantDecision {
    pub ok: bool,
    pub reason_code: &'static str,
}

pub fn enforce_descending_ranking(scores: &[f64], ids: &[String]) -> RankingInvariantDecision {
    if scores.len() != ids.len() {
        return RankingInvariantDecision {
            ok: false,
            reason_code: "ranking_shape_mismatch",
        };
    }
    if scores.is_empty() {
        return RankingInvariantDecision {
            ok: true,
            reason_code: "ranking_empty_ok",
        };
    }
    for i in 0..scores.len() {
        if !scores[i].is_finite() {
            return RankingInvariantDecision {
                ok: false,
                reason_code: "ranking_non_finite_score",
            };
        }
        if i == 0 {
            continue;
        }
        if scores[i] > scores[i - 1] {
            return RankingInvariantDecision {
                ok: false,
                reason_code: "ranking_not_descending",
            };
        }
        if (scores[i] - scores[i - 1]).abs() <= f64::EPSILON && ids[i] < ids[i - 1] {
            return RankingInvariantDecision {
                ok: false,
                reason_code: "ranking_tie_not_stable",
            };
        }
    }
    RankingInvariantDecision {
        ok: true,
        reason_code: "ranking_descending_stable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_mode_fails_when_budget_exceeded() {
        let out = enforce_recall_budget(&RecallBudgetInput {
            requested_top: 99,
            requested_max_files: 3,
            requested_expand_lines: 400,
            mode: FailClosedMode::Reject,
            max_top: 50,
            max_files: 2,
            max_expand_lines: 300,
        });
        assert!(!out.ok);
        assert_eq!(out.reason_code, "recall_budget_exceeded");
    }

    #[test]
    fn trim_mode_caps_values() {
        let out = enforce_recall_budget(&RecallBudgetInput {
            requested_top: 99,
            requested_max_files: 30,
            requested_expand_lines: 999,
            mode: FailClosedMode::Trim,
            max_top: 50,
            max_files: 20,
            max_expand_lines: 300,
        });
        assert!(out.ok);
        assert!(out.trimmed);
        assert_eq!(out.effective_top, 50);
        assert_eq!(out.effective_max_files, 20);
        assert_eq!(out.effective_expand_lines, 300);
    }

    #[test]
    fn index_first_requires_sources_and_entries() {
        assert!(!enforce_index_first(&[], 3).ok);
        assert!(!enforce_index_first(&["sqlite:ok".to_string()], 0).ok);
        assert!(enforce_index_first(&["sqlite:ok".to_string()], 3).ok);
    }

    #[test]
    fn node_only_requires_node_or_uid() {
        assert!(!enforce_node_only("", "").ok);
        assert!(enforce_node_only("node-1", "").ok);
        assert!(enforce_node_only("", "UID123").ok);
    }

    #[test]
    fn bootstrap_requires_lazy_hydration_and_cap() {
        let eager = enforce_hydration_guard(&HydrationGuardInput {
            bootstrap: true,
            lazy_hydration: false,
            estimated_hydration_tokens: 10,
            max_bootstrap_tokens: 48,
            force: false,
        });
        assert!(!eager.ok);

        let over_cap = enforce_hydration_guard(&HydrationGuardInput {
            bootstrap: true,
            lazy_hydration: true,
            estimated_hydration_tokens: 80,
            max_bootstrap_tokens: 48,
            force: false,
        });
        assert!(!over_cap.ok);

        let good = enforce_hydration_guard(&HydrationGuardInput {
            bootstrap: true,
            lazy_hydration: true,
            estimated_hydration_tokens: 30,
            max_bootstrap_tokens: 48,
            force: false,
        });
        assert!(good.ok);
    }

    #[test]
    fn freshness_fails_closed_without_override() {
        let decision = enforce_index_freshness(10_000, Some(1), 50, false);
        assert!(!decision.ok);
        assert!(decision.stale);
        assert_eq!(decision.reason_code, "index_stale_blocked");
    }

    #[test]
    fn freshness_allows_override() {
        let decision = enforce_index_freshness(10_000, Some(1), 50, true);
        assert!(decision.ok);
        assert!(decision.stale);
        assert_eq!(decision.reason_code, "index_stale_allowed");
    }

    #[test]
    fn ranking_requires_descending_and_stable_ties() {
        let good_scores = vec![10.0, 9.0, 9.0, 8.0];
        let good_ids = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ];
        assert!(enforce_descending_ranking(&good_scores, &good_ids).ok);

        let bad_scores = vec![10.0, 11.0];
        let bad_ids = vec!["a".to_string(), "b".to_string()];
        assert!(!enforce_descending_ranking(&bad_scores, &bad_ids).ok);
    }
}
