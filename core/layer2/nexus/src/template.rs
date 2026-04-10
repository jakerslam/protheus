use crate::deterministic_hash;
use crate::policy::{TrustClass, VerityClass};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const TEMPLATE_ERR_ID_MISSING: &str = "template_id_missing";
const TEMPLATE_ERR_VERSION_INVALID: &str = "template_version_invalid";
const TEMPLATE_ERR_ROUTE_MISSING: &str = "template_route_missing";
const TEMPLATE_ERR_SCHEMA_OR_VERB_MISSING: &str = "template_schema_or_verb_missing";
const TEMPLATE_ERR_MISSING: &str = "template_missing";

fn template_key(template_id: &str, version: u32) -> String {
    format!("{template_id}:{version}")
}

fn validate_template(template: &ConnectionTemplate) -> Result<(), &'static str> {
    if template.template_id.trim().is_empty() {
        return Err(TEMPLATE_ERR_ID_MISSING);
    }
    if template.version == 0 {
        return Err(TEMPLATE_ERR_VERSION_INVALID);
    }
    if template.source.trim().is_empty() || template.target.trim().is_empty() {
        return Err(TEMPLATE_ERR_ROUTE_MISSING);
    }
    if template.schema_ids.is_empty() || template.verbs.is_empty() {
        return Err(TEMPLATE_ERR_SCHEMA_OR_VERB_MISSING);
    }
    Ok(())
}

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
        template_key(self.template_id.as_str(), self.version)
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
        validate_template(&template).map_err(str::to_string)?;
        self.templates.insert(template.key(), template);
        Ok(())
    }

    pub fn get(&self, template_id: &str, version: u32) -> Option<&ConnectionTemplate> {
        self.templates
            .get(template_key(template_id, version).as_str())
    }

    pub fn instantiate(
        &self,
        template_id: &str,
        version: u32,
    ) -> Result<ConnectionTemplate, String> {
        self.get(template_id, version)
            .cloned()
            .ok_or_else(|| TEMPLATE_ERR_MISSING.to_string())
    }

    pub fn list(&self) -> Vec<ConnectionTemplate> {
        self.templates.values().cloned().collect()
    }
}
