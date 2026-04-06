use crate::deterministic_hash;
use crate::now_ms;
use crate::policy::{PolicyDecisionRef, TrustClass, VerityClass};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevocationCause {
    PolicyChanged,
    SourceQuiesced,
    TargetQuiesced,
    SourceDetached,
    TargetDetached,
    RegistrationLost,
    Expired,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteLeaseCapability {
    pub lease_id: String,
    pub source: String,
    pub target: String,
    pub schema_ids: Vec<String>,
    pub verbs: Vec<String>,
    pub required_verity: VerityClass,
    pub trust_class: TrustClass,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub receipt_id: String,
    pub policy_decision_ref: PolicyDecisionRef,
    pub template_id: Option<String>,
    pub template_version: Option<u32>,
    pub revoked_at_ms: Option<u64>,
    pub revocation_cause: Option<RevocationCause>,
}

impl RouteLeaseCapability {
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        schema_ids: Vec<String>,
        verbs: Vec<String>,
        required_verity: VerityClass,
        trust_class: TrustClass,
        ttl_ms: u64,
        receipt_id: impl Into<String>,
        policy_decision_ref: PolicyDecisionRef,
        template_id: Option<String>,
        template_version: Option<u32>,
    ) -> Self {
        let source = source.into();
        let target = target.into();
        let issued_at_ms = now_ms();
        let safe_ttl = ttl_ms.max(1);
        let expires_at_ms = issued_at_ms.saturating_add(safe_ttl);
        let lease_id = format!(
            "lease_{}",
            deterministic_hash(&(
                source.as_str(),
                target.as_str(),
                &schema_ids,
                &verbs,
                required_verity,
                trust_class,
                issued_at_ms,
                expires_at_ms
            ))
        );
        Self {
            lease_id,
            source,
            target,
            schema_ids,
            verbs,
            required_verity,
            trust_class,
            issued_at_ms,
            expires_at_ms,
            receipt_id: receipt_id.into(),
            policy_decision_ref,
            template_id,
            template_version,
            revoked_at_ms: None,
            revocation_cause: None,
        }
    }

    pub fn is_expired(&self, now_ms: u64) -> bool {
        now_ms > self.expires_at_ms
    }

    pub fn is_revoked(&self) -> bool {
        self.revoked_at_ms.is_some()
    }

    pub fn is_active(&self, now_ms: u64) -> bool {
        !self.is_revoked() && !self.is_expired(now_ms)
    }

    pub fn revoke(&mut self, cause: RevocationCause, at_ms: u64) {
        if self.revoked_at_ms.is_none() {
            self.revoked_at_ms = Some(at_ms);
            self.revocation_cause = Some(cause);
        }
    }

    pub fn authorizes(&self, input: &LeaseAuthorizationInput) -> Result<(), String> {
        if !self.is_active(input.now_ms) {
            return Err("lease_inactive".to_string());
        }
        if self.source != input.source || self.target != input.target {
            return Err("lease_route_mismatch".to_string());
        }
        if !self
            .schema_ids
            .iter()
            .any(|schema| schema == &input.schema_id)
        {
            return Err("lease_schema_denied".to_string());
        }
        if !self.verbs.iter().any(|verb| verb == &input.verb) {
            return Err("lease_verb_denied".to_string());
        }
        if !input.offered_verity.permits(self.required_verity) {
            return Err("lease_verity_insufficient".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseAuthorizationInput {
    pub source: String,
    pub target: String,
    pub schema_id: String,
    pub verb: String,
    pub offered_verity: VerityClass,
    pub now_ms: u64,
}
