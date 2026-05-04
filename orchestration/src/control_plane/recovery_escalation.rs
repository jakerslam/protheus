// Layer ownership: orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

pub struct RecoveryEscalationContract;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatBackendFailureKind {
    BackendUnavailable,
    ProviderSyncFailed,
    LaneTimeout,
    FinalResponseHandoffLost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatBackendFailure {
    pub kind: ChatBackendFailureKind,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatRecoveryAction {
    RetryWithModel { model_id: String },
    Escalate { reason: String },
    NoRecovery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRecoveryPlan {
    pub action: ChatRecoveryAction,
    pub failure: Option<ChatBackendFailure>,
    pub telemetry_note: String,
}

impl SubdomainContract for RecoveryEscalationContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn classify_chat_backend_failure(text: &str) -> Option<ChatBackendFailure> {
    let raw = text.trim();
    if raw.is_empty() {
        return None;
    }
    let lower = raw.to_lowercase();
    if lower
        == "i lost the final response handoff for this turn. context is still intact, and i can continue from exactly where this left off."
        || lower.starts_with("completed tool steps:")
    {
        return None;
    }

    let kind = if contains_any(
        &lower,
        &[
            "hosted_model_provider_sync_failed",
            "provider-sync",
            "switch-provider",
        ],
    ) {
        ChatBackendFailureKind::ProviderSyncFailed
    } else if contains_any(&lower, &["lane_timeout_1500ms"]) {
        ChatBackendFailureKind::LaneTimeout
    } else if contains_any(
        &lower,
        &[
            "did not receive a final answer",
            "lost the final response handoff",
        ],
    ) {
        ChatBackendFailureKind::FinalResponseHandoffLost
    } else if contains_any(
        &lower,
        &[
            "couldn't reach a chat model backend",
            "could not reach a chat model backend",
            "start ollama",
            "configure app-plane",
            "model backend unavailable",
            "no chat model backend",
            "app_plane_chat_ui",
        ],
    ) {
        ChatBackendFailureKind::BackendUnavailable
    } else {
        return None;
    };

    Some(ChatBackendFailure {
        kind,
        summary: compact_summary(raw, 220),
    })
}

pub fn recommend_chat_failover(
    failure_text: &str,
    active_model_ids: &[String],
    candidate_model_ids: &[String],
) -> ChatRecoveryPlan {
    let Some(failure) = classify_chat_backend_failure(failure_text) else {
        return ChatRecoveryPlan {
            action: ChatRecoveryAction::NoRecovery,
            failure: None,
            telemetry_note: "no recoverable chat backend failure detected".to_string(),
        };
    };

    let active = model_variant_set(active_model_ids);
    let auto_selected = active.contains("auto");
    let mut seen = std::collections::HashSet::new();
    let target = candidate_model_ids.iter().find_map(|candidate| {
        let normalized = candidate.trim();
        if normalized.is_empty() || normalized.eq_ignore_ascii_case("auto") {
            return None;
        }
        let variants = model_variant_set(&[normalized.to_string()]);
        if variants.iter().any(|variant| active.contains(variant)) {
            return None;
        }
        let dedupe_key = normalized.to_lowercase();
        if !seen.insert(dedupe_key) {
            return None;
        }
        Some(normalized.to_string())
    });

    match target {
        Some(model_id) if auto_selected => ChatRecoveryPlan {
            action: ChatRecoveryAction::RetryWithModel { model_id },
            failure: Some(failure),
            telemetry_note: "retry with first non-active model candidate because Auto is selected"
                .to_string(),
        },
        Some(_) => ChatRecoveryPlan {
            action: ChatRecoveryAction::Escalate {
                reason: "model switch requires Auto selection or explicit user request".to_string(),
            },
            failure: Some(failure),
            telemetry_note: "escalate because explicit selected model cannot be changed automatically"
                .to_string(),
        },
        None => ChatRecoveryPlan {
            action: ChatRecoveryAction::Escalate {
                reason: "no alternate model candidate available".to_string(),
            },
            failure: Some(failure),
            telemetry_note: "escalate because failover has no admissible target".to_string(),
        },
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn compact_summary(raw: &str, max_chars: usize) -> String {
    let summary = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if summary.chars().count() <= max_chars {
        return summary;
    }
    let keep = max_chars.saturating_sub(3);
    format!("{}...", summary.chars().take(keep).collect::<String>())
}

fn model_variant_set(values: &[String]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for value in values {
        let raw = value.trim();
        if raw.is_empty() {
            continue;
        }
        out.insert(raw.to_lowercase());
        if let Some(tail) = raw.rsplit('/').next() {
            if !tail.is_empty() {
                out.insert(tail.to_lowercase());
            }
        }
    }
    out
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "recovery_escalation",
        legacy_module_bindings: &["recovery", "clarification", "posture"],
        allowed_kernel_inputs: &[
            "execution_observation_snapshot",
            "core_probe_envelope",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "recovery_recommendation_envelope",
            "clarification_request_envelope",
            "degradation_projection",
        ],
        message_boundaries: &[
            "recovery_to_packaging_boundary",
            "recovery_to_kernel_recommendation_boundary",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_chat_backend_failures_without_claiming_tool_summaries() {
        let failure = classify_chat_backend_failure(
            "Could not reach a chat model backend. Start Ollama or configure app-plane.",
        )
        .expect("backend outage should be recoverable");

        assert_eq!(failure.kind, ChatBackendFailureKind::BackendUnavailable);
        assert!(failure.summary.contains("Could not reach"));
        assert!(classify_chat_backend_failure("Completed tool steps: web search").is_none());
    }

    #[test]
    fn escalates_instead_of_switching_explicit_selected_model() {
        let plan = recommend_chat_failover(
            "hosted_model_provider_sync_failed",
            &["openai/gpt-5.4".to_string()],
            &[
                "gpt-5.4".to_string(),
                "anthropic/claude-4.2".to_string(),
                "anthropic/claude-4.2".to_string(),
            ],
        );

        assert_eq!(
            plan.action,
            ChatRecoveryAction::Escalate {
                reason: "model switch requires Auto selection or explicit user request".to_string()
            }
        );
        assert_eq!(
            plan.failure.expect("failure should be carried").kind,
            ChatBackendFailureKind::ProviderSyncFailed
        );
    }

    #[test]
    fn auto_selection_can_recommend_first_non_active_candidate_for_failover() {
        let plan = recommend_chat_failover(
            "hosted_model_provider_sync_failed",
            &["auto".to_string()],
            &[
                "anthropic/claude-4.2".to_string(),
                "anthropic/claude-4.2".to_string(),
            ],
        );

        assert_eq!(
            plan.action,
            ChatRecoveryAction::RetryWithModel {
                model_id: "anthropic/claude-4.2".to_string()
            }
        );
    }

    #[test]
    fn escalates_when_recovery_has_no_admissible_candidate() {
        let plan = recommend_chat_failover(
            "lane_timeout_1500ms",
            &["local/kimi".to_string()],
            &["kimi".to_string(), "auto".to_string()],
        );

        assert_eq!(
            plan.action,
            ChatRecoveryAction::Escalate {
                reason: "no alternate model candidate available".to_string()
            }
        );
    }
}
