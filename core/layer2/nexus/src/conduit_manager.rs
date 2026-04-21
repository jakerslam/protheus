use crate::deterministic_hash;
use crate::now_ms;
use crate::policy::TrustClass;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_CONDUIT_MESSAGE_TYPES: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConduitRateLimitPolicy {
    pub window_ms: u64,
    pub per_client_max: u32,
    pub per_client_command_max: u32,
}

impl Default for ConduitRateLimitPolicy {
    fn default() -> Self {
        Self {
            window_ms: 1_000,
            per_client_max: 60,
            per_client_command_max: 20,
        }
    }
}

fn default_bridge_message_budget_max() -> usize {
    MAX_CONDUIT_MESSAGE_TYPES
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConduitPolicy {
    pub constitution_path: String,
    pub guard_registry_path: String,
    pub required_constitution_markers: Vec<String>,
    pub required_guard_checks: Vec<String>,
    pub command_required_capabilities: BTreeMap<String, String>,
    pub allow_policy_update_prefixes: Vec<String>,
    pub rate_limit: ConduitRateLimitPolicy,
    #[serde(default = "default_bridge_message_budget_max")]
    pub bridge_message_budget_max: usize,
}

impl Default for ConduitPolicy {
    fn default() -> Self {
        let mut capabilities = BTreeMap::new();
        capabilities.insert("start_agent".to_string(), "agent.lifecycle".to_string());
        capabilities.insert("stop_agent".to_string(), "agent.lifecycle".to_string());
        capabilities.insert(
            "query_receipt_chain".to_string(),
            "receipt.read".to_string(),
        );
        capabilities.insert("list_active_agents".to_string(), "system.read".to_string());
        capabilities.insert("get_system_status".to_string(), "system.read".to_string());
        capabilities.insert(
            "apply_policy_update".to_string(),
            "policy.update".to_string(),
        );
        capabilities.insert(
            "install_extension".to_string(),
            "extension.install".to_string(),
        );

        Self {
            constitution_path: "docs/workspace/AGENT-CONSTITUTION.md".to_string(),
            guard_registry_path: "client/runtime/config/guard_check_registry.json".to_string(),
            required_constitution_markers: vec![
                "Mind Sovereignty Covenant".to_string(),
                "RSI Guardrails".to_string(),
            ],
            required_guard_checks: vec![
                "contract_check".to_string(),
                "formal_invariant_engine".to_string(),
            ],
            command_required_capabilities: capabilities,
            allow_policy_update_prefixes: vec!["constitution_safe/".to_string()],
            rate_limit: ConduitRateLimitPolicy::default(),
            bridge_message_budget_max: MAX_CONDUIT_MESSAGE_TYPES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConduitBackedLink {
    pub link_id: String,
    pub source: String,
    pub target: String,
    pub trust_class: TrustClass,
    pub created_at_ms: u64,
    pub last_used_ms: u64,
    pub policy: ConduitPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConduitManager {
    links: BTreeMap<String, ConduitBackedLink>,
    idle_timeout_ms: u64,
}

impl Default for ConduitManager {
    fn default() -> Self {
        Self {
            links: BTreeMap::new(),
            idle_timeout_ms: 5 * 60 * 1000,
        }
    }
}

impl ConduitManager {
    pub fn with_idle_timeout_ms(idle_timeout_ms: u64) -> Self {
        Self {
            links: BTreeMap::new(),
            idle_timeout_ms: idle_timeout_ms.max(1),
        }
    }

    fn link_key(source: &str, target: &str) -> String {
        format!("{source}->{target}")
    }

    fn link_mut(&mut self, source: &str, target: &str) -> Option<&mut ConduitBackedLink> {
        self.links.get_mut(&Self::link_key(source, target))
    }

    fn create_link(
        source: &str,
        target: &str,
        trust_class: TrustClass,
        ts: u64,
    ) -> ConduitBackedLink {
        let link_id = format!(
            "conduit_{}",
            deterministic_hash(&(source, target, trust_class, ts))
        );
        ConduitBackedLink {
            link_id,
            source: source.to_string(),
            target: target.to_string(),
            trust_class,
            created_at_ms: ts,
            last_used_ms: ts,
            policy: ConduitPolicy::default(),
        }
    }

    pub fn has_link(&self, source: &str, target: &str) -> bool {
        self.links.contains_key(&Self::link_key(source, target))
    }

    pub fn ensure_link(
        &mut self,
        source: &str,
        target: &str,
        trust_class: TrustClass,
    ) -> (ConduitBackedLink, bool) {
        let key = Self::link_key(source, target);
        let ts = now_ms();
        if let Some(existing) = self.links.get_mut(&key) {
            existing.last_used_ms = ts;
            return (existing.clone(), false);
        }
        let link = Self::create_link(source, target, trust_class, ts);
        self.links.insert(key, link.clone());
        (link, true)
    }

    pub fn mark_used(&mut self, source: &str, target: &str, at_ms: u64) {
        if let Some(link) = self.link_mut(source, target) {
            link.last_used_ms = at_ms;
        }
    }

    pub fn remove_links_for_node(&mut self, node_id: &str) -> usize {
        let before = self.links.len();
        self.links
            .retain(|_, link| link.source != node_id && link.target != node_id);
        before.saturating_sub(self.links.len())
    }

    pub fn teardown_idle(&mut self, now_ms: u64) -> Vec<ConduitBackedLink> {
        let idle_timeout_ms = self.idle_timeout_ms;
        let mut removed = Vec::new();
        self.links.retain(|_, link| {
            let idle_ms = now_ms.saturating_sub(link.last_used_ms);
            if idle_ms > idle_timeout_ms {
                removed.push(link.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    pub fn list(&self) -> Vec<ConduitBackedLink> {
        self.links.values().cloned().collect()
    }
}
