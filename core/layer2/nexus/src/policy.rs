use crate::deterministic_hash;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum VerityClass {
    Low,
    Standard,
    High,
    Critical,
}

impl VerityClass {
    pub fn permits(self, required: Self) -> bool {
        self >= required
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustClass {
    InternalControl,
    InterModuleData,
    ClientIngressBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NexusFeatureFlags {
    pub hierarchical_nexus_enabled: bool,
    pub coexist_with_flat_routing: bool,
}

impl Default for NexusFeatureFlags {
    fn default() -> Self {
        Self {
            hierarchical_nexus_enabled: false,
            coexist_with_flat_routing: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDecisionRef {
    pub decision_id: String,
    pub allow: bool,
    pub reason: String,
}

impl PolicyDecisionRef {
    fn build(reason: impl Into<String>, allow: bool, context: &PolicyEvaluationContext) -> Self {
        let reason = reason.into();
        let decision_id = format!("policy_{}", deterministic_hash(&(context, &reason, allow)));
        Self {
            decision_id,
            allow,
            reason,
        }
    }

    pub fn allow(reason: impl Into<String>, context: &PolicyEvaluationContext) -> Self {
        Self::build(reason, true, context)
    }

    pub fn deny(reason: impl Into<String>, context: &PolicyEvaluationContext) -> Self {
        Self::build(reason, false, context)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyEvaluationContext {
    pub issuer: String,
    pub source: String,
    pub target: String,
    pub schema_ids: Vec<String>,
    pub verbs: Vec<String>,
    pub required_verity: VerityClass,
    pub template_id: Option<String>,
}

pub trait NexusPolicyGate {
    fn evaluate(&self, context: &PolicyEvaluationContext) -> PolicyDecisionRef;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultNexusPolicy {
    allowed_control_plane_schemas: BTreeSet<String>,
    allowed_schema_prefixes: Vec<String>,
    blocked_pairs: BTreeSet<String>,
    client_ingress_allowed_targets: BTreeSet<String>,
    max_ttl_ms_by_verity: BTreeMap<VerityClass, u64>,
}

impl Default for DefaultNexusPolicy {
    fn default() -> Self {
        let mut allowed = BTreeSet::new();
        allowed.insert("nexus.registration".to_string());
        allowed.insert("nexus.template".to_string());
        allowed.insert("nexus.lease".to_string());
        allowed.insert("nexus.lifecycle".to_string());
        allowed.insert("nexus.plasticity".to_string());

        let mut max_ttl_ms_by_verity = BTreeMap::new();
        max_ttl_ms_by_verity.insert(VerityClass::Low, 15 * 60 * 1000);
        max_ttl_ms_by_verity.insert(VerityClass::Standard, 60 * 60 * 1000);
        max_ttl_ms_by_verity.insert(VerityClass::High, 4 * 60 * 60 * 1000);
        max_ttl_ms_by_verity.insert(VerityClass::Critical, 24 * 60 * 60 * 1000);

        Self {
            allowed_control_plane_schemas: allowed,
            allowed_schema_prefixes: vec![
                "nexus.".to_string(),
                "module.".to_string(),
                "stomach.".to_string(),
                "context_stacks.".to_string(),
                "client_ingress.".to_string(),
            ],
            blocked_pairs: BTreeSet::new(),
            client_ingress_allowed_targets: ["context_stacks".to_string()].into_iter().collect(),
            max_ttl_ms_by_verity,
        }
    }
}

impl DefaultNexusPolicy {
    pub fn max_ttl_ms(&self, class: VerityClass) -> u64 {
        self.max_ttl_ms_by_verity
            .get(&class)
            .copied()
            .unwrap_or(60 * 60 * 1000)
    }

    pub fn block_pair(&mut self, source: &str, target: &str) {
        self.blocked_pairs.insert(format!("{source}->{target}"));
    }

    pub fn allow_client_ingress_target(&mut self, target: &str) {
        let cleaned = target.trim();
        if cleaned.is_empty() {
            return;
        }
        self.client_ingress_allowed_targets
            .insert(cleaned.to_string());
    }
}

impl NexusPolicyGate for DefaultNexusPolicy {
    fn evaluate(&self, context: &PolicyEvaluationContext) -> PolicyDecisionRef {
        if context.schema_ids.is_empty() || context.verbs.is_empty() {
            return PolicyDecisionRef::deny("policy_context_incomplete", context);
        }
        let schema_allowed = |schema: &str| {
            self.allowed_control_plane_schemas.contains(schema)
                || self
                    .allowed_schema_prefixes
                    .iter()
                    .any(|prefix| schema.starts_with(prefix))
        };
        if context
            .schema_ids
            .iter()
            .any(|schema| !schema_allowed(schema))
        {
            return PolicyDecisionRef::deny("schema_not_allowlisted", context);
        }
        let control_plane_only = context
            .schema_ids
            .iter()
            .all(|schema| schema.starts_with("nexus."));
        if context.source == "client_ingress"
            && !control_plane_only
            && !self
                .client_ingress_allowed_targets
                .contains(&context.target)
        {
            return PolicyDecisionRef::deny("client_ingress_domain_boundary", context);
        }
        if self
            .blocked_pairs
            .contains(&format!("{}->{}", context.source, context.target))
        {
            return PolicyDecisionRef::deny("source_target_pair_blocked", context);
        }
        PolicyDecisionRef::allow("policy_allow", context)
    }
}
