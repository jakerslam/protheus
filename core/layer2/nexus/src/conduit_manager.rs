use crate::deterministic_hash;
use crate::now_ms;
use crate::policy::TrustClass;
use conduit::ConduitPolicy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
