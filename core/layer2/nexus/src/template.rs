use crate::deterministic_hash;
use crate::policy::{TrustClass, VerityClass};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionTemplate {
    pub template_id: String,
    pub version: u32,
    pub source: String,
    pub target: String,
    pub schema_ids: Vec<String>,
    pub verbs: Vec<String>,
    pub required_verity: VerityClass,
    pub trust_class: TrustClass,
    pub default_ttl_ms: u64,
}

impl ConnectionTemplate {
    pub fn key(&self) -> String {
        format!("{}:{}", self.template_id, self.version)
    }

    pub fn template_fingerprint(&self) -> String {
        deterministic_hash(self)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateRegistry {
    templates: BTreeMap<String, ConnectionTemplate>,
}

impl TemplateRegistry {
    pub fn upsert(&mut self, template: ConnectionTemplate) -> Result<(), String> {
        if template.template_id.trim().is_empty() {
            return Err("template_id_missing".to_string());
        }
        if template.version == 0 {
            return Err("template_version_invalid".to_string());
        }
        if template.source.trim().is_empty() || template.target.trim().is_empty() {
            return Err("template_route_missing".to_string());
        }
        if template.schema_ids.is_empty() || template.verbs.is_empty() {
            return Err("template_schema_or_verb_missing".to_string());
        }
        self.templates.insert(template.key(), template);
        Ok(())
    }

    pub fn get(&self, template_id: &str, version: u32) -> Option<&ConnectionTemplate> {
        self.templates.get(&format!("{template_id}:{version}"))
    }

    pub fn instantiate(
        &self,
        template_id: &str,
        version: u32,
    ) -> Result<ConnectionTemplate, String> {
        self.get(template_id, version)
            .cloned()
            .ok_or_else(|| "template_missing".to_string())
    }

    pub fn list(&self) -> Vec<ConnectionTemplate> {
        self.templates.values().cloned().collect()
    }
}
