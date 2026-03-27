use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PromotionStage {
    ApprenticeMode,
    ShadowMode,
    FullIntegration,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainmentPermissions {
    pub max_train_minutes: u64,
    pub allow_network: bool,
    pub allowed_providers: Vec<String>,
    pub max_context_tokens: u32,
    pub require_human_signoff_for_full_integration: bool,
}

impl Default for ContainmentPermissions {
    fn default() -> Self {
        Self {
            max_train_minutes: 30,
            allow_network: false,
            allowed_providers: Vec::new(),
            max_context_tokens: 32_768,
            require_human_signoff_for_full_integration: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyGate {
    pub id: String,
    pub enabled: bool,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyGates {
    pub execution_mode: String,
    pub gates: Vec<PolicyGate>,
}

impl Default for PolicyGates {
    fn default() -> Self {
        Self {
            execution_mode: "sandboxed".to_string(),
            gates: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeedModelArtifact {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub required: bool,
    pub params_billion: Option<f64>,
    pub context_tokens: Option<u32>,
    pub specialty: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SeedManifest {
    pub artifacts: Vec<SeedModelArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpecialistTrainingSpec {
    pub specialist_id: String,
    pub seed_id: String,
    pub provider: String,
    pub model: String,
    pub tier: String,
    pub max_train_minutes: u64,
    pub promotion_stage: PromotionStage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpecialistTrainingPlan {
    pub generated_at: String,
    pub specialists: Vec<SpecialistTrainingSpec>,
    pub max_train_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuarantineDecision {
    pub artifact_id: String,
    pub quarantined: bool,
    pub reasons: Vec<String>,
}

pub fn containment_permissions_from_value(raw: &Value) -> ContainmentPermissions {
    let mut out = ContainmentPermissions::default();
    if let Some(v) = raw.get("max_train_minutes").and_then(Value::as_u64) {
        out.max_train_minutes = v.max(1);
    }
    if let Some(v) = raw.get("allow_network").and_then(Value::as_bool) {
        out.allow_network = v;
    }
    if let Some(v) = raw.get("max_context_tokens").and_then(Value::as_u64) {
        out.max_context_tokens = v.clamp(1, u32::MAX as u64) as u32;
    }
    if let Some(v) = raw
        .get("require_human_signoff_for_full_integration")
        .and_then(Value::as_bool)
    {
        out.require_human_signoff_for_full_integration = v;
    }
    if let Some(rows) = raw.get("allowed_providers").and_then(Value::as_array) {
        out.allowed_providers = rows
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
    }
    out
}

pub fn policy_gates_from_value(raw: &Value) -> PolicyGates {
    let mut out = PolicyGates::default();
    if let Some(mode) = raw.get("execution_mode").and_then(Value::as_str) {
        let mode = mode.trim().to_lowercase();
        if !mode.is_empty() {
            out.execution_mode = mode;
        }
    }
    if let Some(rows) = raw.get("gates").and_then(Value::as_array) {
        out.gates = rows
            .iter()
            .filter_map(|row| {
                let id = row.get("id").and_then(Value::as_str)?.trim();
                if id.is_empty() {
                    return None;
                }
                let action = row
                    .get("action")
                    .and_then(Value::as_str)
                    .unwrap_or("deny")
                    .trim()
                    .to_lowercase();
                Some(PolicyGate {
                    id: id.to_string(),
                    enabled: row.get("enabled").and_then(Value::as_bool).unwrap_or(true),
                    action,
                })
            })
            .collect::<Vec<_>>();
    }
    out
}

pub fn seed_manifest_from_value(raw: &Value) -> SeedManifest {
    let artifacts = raw
        .get("artifacts")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let id = row.get("id").and_then(Value::as_str)?.trim();
                    let provider = row.get("provider").and_then(Value::as_str)?.trim();
                    let model = row.get("model").and_then(Value::as_str)?.trim();
                    if id.is_empty() || provider.is_empty() || model.is_empty() {
                        return None;
                    }
                    Some(SeedModelArtifact {
                        id: id.to_string(),
                        provider: provider.to_lowercase(),
                        model: model.to_string(),
                        required: row
                            .get("required")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        params_billion: row.get("params_billion").and_then(Value::as_f64),
                        context_tokens: row
                            .get("context_tokens")
                            .and_then(Value::as_u64)
                            .map(|v| v.clamp(1, u32::MAX as u64) as u32),
                        specialty: row
                            .get("specialty")
                            .and_then(Value::as_str)
                            .map(|v| v.trim().to_string())
                            .filter(|v| !v.is_empty()),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    SeedManifest { artifacts }
}

pub fn build_specialist_training_plan(
    generated_at: &str,
    manifest: &SeedManifest,
    permissions: &ContainmentPermissions,
) -> SpecialistTrainingPlan {
    let specialists = manifest
        .artifacts
        .iter()
        .map(|artifact| SpecialistTrainingSpec {
            specialist_id: format!("nursery-{}", artifact.id),
            seed_id: artifact.id.clone(),
            provider: artifact.provider.clone(),
            model: artifact.model.clone(),
            tier: if artifact.required {
                "primary".to_string()
            } else {
                "shadow".to_string()
            },
            max_train_minutes: permissions.max_train_minutes,
            promotion_stage: if artifact.required {
                PromotionStage::ApprenticeMode
            } else {
                PromotionStage::ShadowMode
            },
        })
        .collect::<Vec<_>>();

    SpecialistTrainingPlan {
        generated_at: generated_at.to_string(),
        specialists,
        max_train_minutes: permissions.max_train_minutes,
    }
}

pub fn evaluate_quarantine(
    manifest: &SeedManifest,
    permissions: &ContainmentPermissions,
    gates: &PolicyGates,
) -> Vec<QuarantineDecision> {
    let allowlist = permissions
        .allowed_providers
        .iter()
        .map(|row| row.to_lowercase())
        .collect::<Vec<_>>();

    manifest
        .artifacts
        .iter()
        .map(|artifact| {
            let mut reasons = Vec::<String>::new();
            if !allowlist.is_empty() && !allowlist.contains(&artifact.provider.to_lowercase()) {
                reasons.push("provider_not_in_allowlist".to_string());
            }
            if artifact.model.trim().is_empty() {
                reasons.push("missing_model_name".to_string());
            }
            if let Some(tokens) = artifact.context_tokens {
                if tokens > permissions.max_context_tokens {
                    reasons.push("context_tokens_exceed_policy".to_string());
                }
            }
            if gates.execution_mode == "sandboxed"
                && !permissions.allow_network
                && artifact.provider != "ollama"
                && artifact.provider != "llama_cpp"
            {
                reasons.push("network_provider_blocked_in_sandbox_mode".to_string());
            }
            QuarantineDecision {
                artifact_id: artifact.id.clone(),
                quarantined: !reasons.is_empty(),
                reasons,
            }
        })
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn training_plan_builds_for_all_seed_artifacts() {
        let seed = seed_manifest_from_value(&json!({
            "artifacts": [
                {"id": "tiny", "provider": "ollama", "model": "qwen2.5:3b", "required": true},
                {"id": "shadow", "provider": "openai", "model": "gpt-5-mini", "required": false}
            ]
        }));
        let permissions = containment_permissions_from_value(&json!({
            "max_train_minutes": 45
        }));

        let plan = build_specialist_training_plan("2026-03-26T00:00:00Z", &seed, &permissions);
        assert_eq!(plan.specialists.len(), 2);
        assert_eq!(plan.max_train_minutes, 45);
        assert_eq!(
            plan.specialists[0].promotion_stage,
            PromotionStage::ApprenticeMode
        );
        assert_eq!(
            plan.specialists[1].promotion_stage,
            PromotionStage::ShadowMode
        );
    }

    #[test]
    fn quarantine_rejects_non_allowlisted_provider_when_policy_is_strict() {
        let seed = seed_manifest_from_value(&json!({
            "artifacts": [
                {"id": "cloud", "provider": "openai", "model": "gpt-5-mini", "context_tokens": 8192}
            ]
        }));
        let permissions = containment_permissions_from_value(&json!({
            "allow_network": false,
            "allowed_providers": ["ollama"],
            "max_context_tokens": 4096
        }));
        let gates = policy_gates_from_value(&json!({"execution_mode": "sandboxed"}));

        let decisions = evaluate_quarantine(&seed, &permissions, &gates);
        assert_eq!(decisions.len(), 1);
        assert!(decisions[0].quarantined);
        assert!(decisions[0]
            .reasons
            .iter()
            .any(|reason| reason == "provider_not_in_allowlist"));
        assert!(decisions[0]
            .reasons
            .iter()
            .any(|reason| reason == "context_tokens_exceed_policy"));
    }

    #[test]
    fn quarantine_allows_local_model_within_policy_bounds() {
        let seed = seed_manifest_from_value(&json!({
            "artifacts": [
                {"id": "local", "provider": "ollama", "model": "qwen2.5:7b", "context_tokens": 4096}
            ]
        }));
        let permissions = containment_permissions_from_value(&json!({
            "allow_network": false,
            "allowed_providers": ["ollama"],
            "max_context_tokens": 8192
        }));
        let gates = policy_gates_from_value(&json!({"execution_mode": "sandboxed"}));

        let decisions = evaluate_quarantine(&seed, &permissions, &gates);
        assert_eq!(decisions.len(), 1);
        assert!(!decisions[0].quarantined);
    }
}
