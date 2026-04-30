// Layer ownership: orchestration (non-canonical orchestration coordination only).
use std::collections::HashSet;

use super::{SubdomainBoundary, SubdomainContract};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderIntent {
    NoModelsGuidance,
    SelectFailoverModel,
    SaveProviderKey,
    DeleteProviderKey,
    PollOAuth,
    ProjectProviderTest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCandidate {
    pub id: String,
    pub provider: Option<String>,
    pub available: bool,
    pub last_used_epoch_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCoordinationRequest {
    pub intent: ProviderIntent,
    pub provider_id: Option<String>,
    pub active_model_ids: Vec<String>,
    pub fallback_model_ids: Vec<String>,
    pub catalog: Vec<ModelCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderTestStatus {
    Ok,
    Error,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderTestProjection {
    pub provider_id: String,
    pub status: ProviderTestStatus,
    pub latency_ms: Option<u64>,
    pub error_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderCoordinationAction {
    RequestRuntimeCredentialMutation {
        operation: String,
        provider_id: String,
    },
    RequestRuntimeOAuthPoll {
        provider_id: String,
    },
    RecommendModelFailover {
        model_id: String,
    },
    ProjectNoModelsRecovery {
        action_kind: String,
    },
    ProjectProviderTest {
        provider_id: String,
        status: ProviderTestStatus,
    },
    Clarify {
        prompt: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCoordinationPlan {
    pub action: ProviderCoordinationAction,
    pub telemetry_note: String,
}

pub struct ModelProviderCoordinationContract;

impl SubdomainContract for ModelProviderCoordinationContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "model_provider_coordination",
        legacy_module_bindings: &[
            "chat_model_guidance_helpers",
            "chat_model_failover_helpers",
            "settings_view_provider_helpers",
            "wizard",
        ],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "execution_observation_snapshot",
            "policy_scope_snapshot",
            "capability_probe_snapshot",
        ],
        allowed_kernel_outputs: &[
            "model_selection_recommendation",
            "provider_recovery_projection",
            "provider_credential_mutation_recommendation",
            "provider_test_projection",
        ],
        message_boundaries: &[
            "provider_to_shell_projection_boundary",
            "provider_to_core_credential_boundary",
            "provider_to_model_selection_boundary",
        ],
    }
}

pub fn coordinate_provider_intent(
    request: &ProviderCoordinationRequest,
) -> ProviderCoordinationPlan {
    match request.intent {
        ProviderIntent::NoModelsGuidance => ProviderCoordinationPlan {
            action: ProviderCoordinationAction::ProjectNoModelsRecovery {
                action_kind: "model_discover".to_string(),
            },
            telemetry_note: "project no-models recovery guidance from orchestration".to_string(),
        },
        ProviderIntent::SelectFailoverModel => recommend_failover(request),
        ProviderIntent::SaveProviderKey => credential_mutation("save_key", request),
        ProviderIntent::DeleteProviderKey => credential_mutation("delete_key", request),
        ProviderIntent::PollOAuth => {
            let Some(provider_id) = request
                .provider_id
                .as_deref()
                .map(str::trim)
                .filter(|id| !id.is_empty())
            else {
                return clarify(
                    "oauth poll requires a provider id",
                    "missing provider id for oauth poll",
                );
            };
            ProviderCoordinationPlan {
                action: ProviderCoordinationAction::RequestRuntimeOAuthPoll {
                    provider_id: provider_id.to_string(),
                },
                telemetry_note: "request runtime-owned oauth poll projection".to_string(),
            }
        }
        ProviderIntent::ProjectProviderTest => {
            let Some(provider_id) = request
                .provider_id
                .as_deref()
                .map(str::trim)
                .filter(|id| !id.is_empty())
            else {
                return clarify(
                    "provider test projection requires a provider id",
                    "missing provider id for provider test",
                );
            };
            ProviderCoordinationPlan {
                action: ProviderCoordinationAction::ProjectProviderTest {
                    provider_id: provider_id.to_string(),
                    status: ProviderTestStatus::Unknown,
                },
                telemetry_note: "provider test result must render from runtime receipt".to_string(),
            }
        }
    }
}

pub fn project_provider_test_result(test: &ProviderTestProjection) -> ProviderCoordinationPlan {
    ProviderCoordinationPlan {
        action: ProviderCoordinationAction::ProjectProviderTest {
            provider_id: test.provider_id.clone(),
            status: test.status.clone(),
        },
        telemetry_note: match test.status {
            ProviderTestStatus::Ok => "project successful provider test receipt".to_string(),
            ProviderTestStatus::Error => "project provider test error receipt".to_string(),
            ProviderTestStatus::Blocked => "project blocked provider test receipt".to_string(),
            ProviderTestStatus::Unknown => "project unknown provider test receipt".to_string(),
        },
    }
}

fn recommend_failover(request: &ProviderCoordinationRequest) -> ProviderCoordinationPlan {
    let active = model_variant_set(&request.active_model_ids);
    let mut seen = HashSet::new();
    for candidate in request
        .fallback_model_ids
        .iter()
        .cloned()
        .chain(sorted_available_catalog_ids(&request.catalog))
    {
        let normalized = candidate.trim();
        if normalized.is_empty() || normalized.eq_ignore_ascii_case("auto") {
            continue;
        }
        let variants = model_variant_set(&[normalized.to_string()]);
        if variants.iter().any(|variant| active.contains(variant)) {
            continue;
        }
        if !seen.insert(normalized.to_lowercase()) {
            continue;
        }
        return ProviderCoordinationPlan {
            action: ProviderCoordinationAction::RecommendModelFailover {
                model_id: normalized.to_string(),
            },
            telemetry_note: "recommend first non-active failover model".to_string(),
        };
    }
    clarify(
        "no alternate model candidate available",
        "failover model recommendation had no admissible target",
    )
}

fn credential_mutation(
    operation: &str,
    request: &ProviderCoordinationRequest,
) -> ProviderCoordinationPlan {
    let Some(provider_id) = request
        .provider_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    else {
        return clarify(
            "provider credential mutation requires a provider id",
            "missing provider id for provider credential mutation",
        );
    };
    ProviderCoordinationPlan {
        action: ProviderCoordinationAction::RequestRuntimeCredentialMutation {
            operation: operation.to_string(),
            provider_id: provider_id.to_string(),
        },
        telemetry_note: "request runtime credential mutation with audit receipt".to_string(),
    }
}

fn sorted_available_catalog_ids(catalog: &[ModelCandidate]) -> Vec<String> {
    let mut rows = catalog
        .iter()
        .filter(|row| row.available && !row.id.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .last_used_epoch_ms
            .cmp(&left.last_used_epoch_ms)
            .then_with(|| {
                left.provider
                    .clone()
                    .unwrap_or_default()
                    .cmp(&right.provider.clone().unwrap_or_default())
            })
            .then_with(|| left.id.cmp(&right.id))
    });
    rows.into_iter().map(|row| row.id).collect()
}

fn model_variant_set(values: &[String]) -> HashSet<String> {
    let mut out = HashSet::new();
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

fn clarify(prompt: &str, note: &str) -> ProviderCoordinationPlan {
    ProviderCoordinationPlan {
        action: ProviderCoordinationAction::Clarify {
            prompt: prompt.to_string(),
        },
        telemetry_note: note.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(intent: ProviderIntent) -> ProviderCoordinationRequest {
        ProviderCoordinationRequest {
            intent,
            provider_id: Some("openai".to_string()),
            active_model_ids: vec!["openai/gpt-5.4".to_string()],
            fallback_model_ids: vec!["gpt-5.4".to_string(), "anthropic/claude-4.2".to_string()],
            catalog: vec![
                ModelCandidate {
                    id: "google/gemini-3".to_string(),
                    provider: Some("google".to_string()),
                    available: true,
                    last_used_epoch_ms: 50,
                },
                ModelCandidate {
                    id: "openai/gpt-5.4".to_string(),
                    provider: Some("openai".to_string()),
                    available: true,
                    last_used_epoch_ms: 100,
                },
            ],
        }
    }

    #[test]
    fn no_models_projects_recovery_without_shell_prose() {
        let plan = coordinate_provider_intent(&request(ProviderIntent::NoModelsGuidance));

        assert_eq!(
            plan.action,
            ProviderCoordinationAction::ProjectNoModelsRecovery {
                action_kind: "model_discover".to_string()
            }
        );
    }

    #[test]
    fn failover_skips_active_model_variants() {
        let plan = coordinate_provider_intent(&request(ProviderIntent::SelectFailoverModel));

        assert_eq!(
            plan.action,
            ProviderCoordinationAction::RecommendModelFailover {
                model_id: "anthropic/claude-4.2".to_string()
            }
        );
    }

    #[test]
    fn provider_key_write_requests_runtime_audit_receipt() {
        let plan = coordinate_provider_intent(&request(ProviderIntent::SaveProviderKey));

        assert_eq!(
            plan.action,
            ProviderCoordinationAction::RequestRuntimeCredentialMutation {
                operation: "save_key".to_string(),
                provider_id: "openai".to_string()
            }
        );
    }

    #[test]
    fn provider_key_delete_requests_runtime_audit_receipt() {
        let plan = coordinate_provider_intent(&request(ProviderIntent::DeleteProviderKey));

        assert_eq!(
            plan.action,
            ProviderCoordinationAction::RequestRuntimeCredentialMutation {
                operation: "delete_key".to_string(),
                provider_id: "openai".to_string()
            }
        );
    }

    #[test]
    fn oauth_poll_requests_runtime_projection() {
        let plan = coordinate_provider_intent(&request(ProviderIntent::PollOAuth));

        assert_eq!(
            plan.action,
            ProviderCoordinationAction::RequestRuntimeOAuthPoll {
                provider_id: "openai".to_string()
            }
        );
    }

    #[test]
    fn provider_test_result_projection_uses_receipt_status() {
        let plan = project_provider_test_result(&ProviderTestProjection {
            provider_id: "openai".to_string(),
            status: ProviderTestStatus::Ok,
            latency_ms: Some(42),
            error_present: false,
        });

        assert_eq!(
            plan.action,
            ProviderCoordinationAction::ProjectProviderTest {
                provider_id: "openai".to_string(),
                status: ProviderTestStatus::Ok
            }
        );
    }
}
