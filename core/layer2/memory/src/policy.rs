use crate::schemas::{
    CapabilityAction, CapabilityToken, MemoryScope, OwnerConsentMode, OwnerExportRedactionPolicy,
    OwnerScopeSettings, TrustState,
};
use crate::{deterministic_hash, now_ms};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyAction {
    Read,
    Write,
    Promote,
    Canonicalize,
    MaterializeContext,
    TaskFabricMutate,
    ExportOwner,
}

#[derive(Debug, Clone)]
pub struct MemoryPolicyRequest {
    pub principal_id: String,
    pub action: PolicyAction,
    pub source_scope: MemoryScope,
    pub target_scope: Option<MemoryScope>,
    pub trust_state: Option<TrustState>,
    pub capability: Option<CapabilityToken>,
    pub owner_settings: OwnerScopeSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryPolicyDecision {
    pub allow: bool,
    pub decision_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityValidationResult {
    pub valid: bool,
    pub reason: String,
}

pub trait MemoryPolicyGate {
    fn evaluate(&self, request: &MemoryPolicyRequest) -> MemoryPolicyDecision;
}

#[derive(Debug, Clone, Default)]
pub struct DefaultVerityMemoryPolicy;

impl DefaultVerityMemoryPolicy {
    fn validate_capability(
        &self,
        capability: Option<&CapabilityToken>,
        principal_id: &str,
        action: CapabilityAction,
        scope: &MemoryScope,
    ) -> CapabilityValidationResult {
        let Some(token) = capability else {
            return CapabilityValidationResult {
                valid: false,
                reason: "capability_missing".to_string(),
            };
        };
        if token.expires_at_ms <= now_ms() {
            return CapabilityValidationResult {
                valid: false,
                reason: "capability_expired".to_string(),
            };
        }
        if token.principal_id != principal_id && token.principal_id != "*" {
            return CapabilityValidationResult {
                valid: false,
                reason: "capability_principal_mismatch".to_string(),
            };
        }
        if !token.allowed_actions.iter().any(|row| row == &action) {
            return CapabilityValidationResult {
                valid: false,
                reason: "capability_action_missing".to_string(),
            };
        }
        if !token.scopes.iter().any(|row| row == scope) {
            return CapabilityValidationResult {
                valid: false,
                reason: "capability_scope_missing".to_string(),
            };
        }
        CapabilityValidationResult {
            valid: true,
            reason: "capability_valid".to_string(),
        }
    }

    fn canonicalize_authority_allows(&self, principal_id: &str, scope: &MemoryScope) -> bool {
        match scope {
            MemoryScope::Public => true,
            MemoryScope::Agent(agent_id) => {
                principal_id == agent_id || principal_id == format!("agent:{agent_id}")
            }
            MemoryScope::Swarm(swarm_id) => {
                principal_id == format!("swarm_steward:{swarm_id}") || principal_id == *swarm_id
            }
            MemoryScope::Core => principal_id.starts_with("core:") || principal_id == "verity",
            MemoryScope::Owner => {
                principal_id == "owner" || principal_id.starts_with("owner_steward:")
            }
        }
    }
}

impl MemoryPolicyGate for DefaultVerityMemoryPolicy {
    fn evaluate(&self, request: &MemoryPolicyRequest) -> MemoryPolicyDecision {
        let (allow, reason) = match request.action {
            PolicyAction::Read => {
                let cap = self.validate_capability(
                    request.capability.as_ref(),
                    request.principal_id.as_str(),
                    CapabilityAction::Read,
                    &request.source_scope,
                );
                (cap.valid, cap.reason)
            }
            PolicyAction::Write => {
                let cap = self.validate_capability(
                    request.capability.as_ref(),
                    request.principal_id.as_str(),
                    CapabilityAction::Write,
                    &request.source_scope,
                );
                (cap.valid, cap.reason)
            }
            PolicyAction::MaterializeContext => {
                let cap = self.validate_capability(
                    request.capability.as_ref(),
                    request.principal_id.as_str(),
                    CapabilityAction::MaterializeContext,
                    &request.source_scope,
                );
                (cap.valid, cap.reason)
            }
            PolicyAction::TaskFabricMutate => {
                let cap = self.validate_capability(
                    request.capability.as_ref(),
                    request.principal_id.as_str(),
                    CapabilityAction::TaskFabricMutate,
                    &request.source_scope,
                );
                (cap.valid, cap.reason)
            }
            PolicyAction::Canonicalize => {
                let cap = self.validate_capability(
                    request.capability.as_ref(),
                    request.principal_id.as_str(),
                    CapabilityAction::Canonicalize,
                    &request.source_scope,
                );
                if !cap.valid {
                    (false, cap.reason)
                } else if request.trust_state != Some(TrustState::Validated) {
                    (false, "canonicalize_requires_validated_state".to_string())
                } else if !self.canonicalize_authority_allows(
                    request.principal_id.as_str(),
                    &request.source_scope,
                ) {
                    (false, "canonicalize_authority_denied".to_string())
                } else {
                    (true, "policy_allow".to_string())
                }
            }
            PolicyAction::Promote => {
                let cap = self.validate_capability(
                    request.capability.as_ref(),
                    request.principal_id.as_str(),
                    CapabilityAction::Promote,
                    &request.source_scope,
                );
                if !cap.valid {
                    (false, cap.reason)
                } else if let Some(target_scope) = request.target_scope.as_ref() {
                    let can = self.validate_capability(
                        request.capability.as_ref(),
                        request.principal_id.as_str(),
                        CapabilityAction::Canonicalize,
                        target_scope,
                    );
                    if !can.valid {
                        (false, format!("promotion_target_denied:{}", can.reason))
                    } else if matches!(target_scope, MemoryScope::Owner)
                        && request.owner_settings.consent_mode == OwnerConsentMode::Restricted
                    {
                        (false, "owner_consent_mode_restricted".to_string())
                    } else if !self
                        .canonicalize_authority_allows(request.principal_id.as_str(), target_scope)
                    {
                        (false, "promotion_authority_denied".to_string())
                    } else {
                        (true, "policy_allow".to_string())
                    }
                } else {
                    (true, "policy_allow".to_string())
                }
            }
            PolicyAction::ExportOwner => {
                let cap = self.validate_capability(
                    request.capability.as_ref(),
                    request.principal_id.as_str(),
                    CapabilityAction::ExportOwnerRaw,
                    &MemoryScope::Owner,
                );
                if !cap.valid {
                    (false, cap.reason)
                } else if request.owner_settings.export_redaction_policy
                    != OwnerExportRedactionPolicy::AllowFull
                {
                    (false, "owner_raw_export_denied_by_policy".to_string())
                } else {
                    (true, "policy_allow".to_string())
                }
            }
        };
        let decision_id = format!(
            "policy_{}",
            &deterministic_hash(&(
                request.principal_id.clone(),
                format!("{:?}", request.action),
                request.source_scope.label(),
                request
                    .target_scope
                    .as_ref()
                    .map(|row| row.label())
                    .unwrap_or_default(),
                reason.clone()
            ))[..24]
        );
        MemoryPolicyDecision {
            allow,
            decision_id,
            reason,
        }
    }
}
