use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityPackSpec {
    pub id: String,
    pub description: String,
    pub tools: Vec<String>,
    pub default_interval_seconds: u64,
}

pub trait StaticCapabilityPack {
    fn spec() -> CapabilityPackSpec;
}

pub struct ResearchCapabilityPack;
pub struct WebOpsCapabilityPack;

impl StaticCapabilityPack for ResearchCapabilityPack {
    fn spec() -> CapabilityPackSpec {
        CapabilityPackSpec {
            id: "research".to_string(),
            description: "Research and synthesis flow with web + memory reads".to_string(),
            tools: vec![
                "web.search".to_string(),
                "web.fetch".to_string(),
                "memory.read".to_string(),
                "summary.synthesize".to_string(),
            ],
            default_interval_seconds: 600,
        }
    }
}

impl StaticCapabilityPack for WebOpsCapabilityPack {
    fn spec() -> CapabilityPackSpec {
        CapabilityPackSpec {
            id: "web-ops".to_string(),
            description: "Operational health checks for tooling and workflow routes".to_string(),
            tools: vec![
                "web.search".to_string(),
                "web.fetch".to_string(),
                "tool.health_probe".to_string(),
                "receipt.inspect".to_string(),
            ],
            default_interval_seconds: 300,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CapabilityPackCatalog {
    packs: BTreeMap<String, CapabilityPackSpec>,
}

impl CapabilityPackCatalog {
    pub fn new() -> Self {
        let mut catalog = Self::default();
        catalog.register(ResearchCapabilityPack::spec());
        catalog.register(WebOpsCapabilityPack::spec());
        catalog
    }

    pub fn register(&mut self, pack: CapabilityPackSpec) {
        self.packs.insert(pack.id.clone(), pack);
    }

    pub fn get(&self, id: &str) -> Option<&CapabilityPackSpec> {
        self.packs.get(id)
    }

    pub fn all(&self) -> Vec<CapabilityPackSpec> {
        self.packs.values().cloned().collect()
    }

    pub fn default_interval_for_pack(&self, id: &str) -> Option<u64> {
        self.get(id).map(|pack| pack.default_interval_seconds)
    }

    pub fn expand_tools(&self, pack_ids: &[String], explicit_tools: &[String]) -> Vec<String> {
        let mut out = BTreeSet::<String>::new();
        for tool in explicit_tools {
            let token = tool.trim();
            if !token.is_empty() {
                out.insert(token.to_string());
            }
        }
        for pack_id in pack_ids {
            if let Some(pack) = self.get(pack_id) {
                for tool in &pack.tools {
                    out.insert(tool.clone());
                }
            }
        }
        out.into_iter().collect()
    }
}

