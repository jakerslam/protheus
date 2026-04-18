use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const INFRING_DETACH_CONTRACT_ID_INFRING_TYPES: &str = "V6-INFRING-DETACH-001.6";

const MAX_AGENT_ID_LEN: usize = 96;
const MAX_AGENT_NAME_LEN: usize = 120;
const MAX_PROVIDER_TOKEN_LEN: usize = 64;
const MAX_MODEL_TOKEN_LEN: usize = 128;
const MAX_METADATA_KEY_LEN: usize = 64;
const MAX_METADATA_VALUE_LEN: usize = 512;
const MAX_TOOL_ID_LEN: usize = 64;

pub fn strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                ch,
                '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
            )
        })
        .collect()
}

fn is_valid_manifest_token(raw: &str, max_len: usize, allow_spaces: bool) -> bool {
    let normalized: String = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    let normalized = normalized.trim();
    if normalized.is_empty() || normalized.chars().count() > max_len {
        return false;
    }
    if !allow_spaces && normalized.chars().any(char::is_whitespace) {
        return false;
    }
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedBlobManifestEntry {
    pub id: String,
    pub hash: String,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct RawSignedBlobManifestEntry {
    pub id: String,
    pub hash: String,
    pub version: u32,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedSignedBlobManifestEntry {
    pub id: String,
    pub hash: String,
    pub version: u32,
    pub signature: String,
}

pub fn normalize_blob_id(raw: &str, max_len: usize) -> Option<String> {
    let normalized: String = strip_invisible_unicode(raw)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect();
    let normalized = normalized.trim();
    if normalized.is_empty() || normalized.len() > max_len {
        return None;
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/'))
    {
        return None;
    }
    Some(normalized.to_string())
}

pub fn normalize_sha256_hash(raw: &str) -> Option<String> {
    let normalized = strip_invisible_unicode(raw).trim().to_ascii_lowercase();
    if normalized.len() != 64 || !normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    Some(normalized)
}

pub fn decode_normalized_blob_manifest(
    bytes: &[u8],
    max_blob_id_len: usize,
) -> Result<Vec<NormalizedBlobManifestEntry>, String> {
    let rows: Vec<NormalizedBlobManifestEntry> =
        serde_json::from_slice(bytes).map_err(|err| err.to_string())?;
    let mut merged = BTreeMap::<String, NormalizedBlobManifestEntry>::new();
    for row in rows {
        let id = normalize_blob_id(&row.id, max_blob_id_len)
            .ok_or_else(|| "manifest_blob_id_invalid".to_string())?;
        let hash = normalize_sha256_hash(&row.hash)
            .ok_or_else(|| "manifest_blob_hash_invalid".to_string())?;
        let normalized = NormalizedBlobManifestEntry {
            id: id.clone(),
            hash,
            version: row.version,
        };
        match merged.get(&id) {
            Some(existing) if existing.version >= normalized.version => {}
            _ => {
                merged.insert(id, normalized);
            }
        }
    }
    Ok(merged.into_values().collect())
}

pub fn compute_blob_manifest_signature(
    id: &str,
    hash: &str,
    version: u32,
    signing_key: &str,
) -> String {
    let to_sign = format!("{id}:{hash}:{version}:{signing_key}");
    let mut hasher = Sha256::new();
    hasher.update(to_sign.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn decode_normalized_signed_bincode_blob_manifest(
    bytes: &[u8],
    max_blob_id_len: usize,
    signing_key: &str,
) -> Result<Vec<NormalizedSignedBlobManifestEntry>, String> {
    let rows: Vec<RawSignedBlobManifestEntry> =
        bincode::deserialize(bytes).map_err(|err| err.to_string())?;
    let mut normalized = Vec::with_capacity(rows.len());
    for row in rows {
        let id = normalize_blob_id(&row.id, max_blob_id_len)
            .ok_or_else(|| "manifest_blob_id_invalid".to_string())?;
        let hash = normalize_sha256_hash(&row.hash)
            .ok_or_else(|| "manifest_blob_hash_invalid".to_string())?;
        let signature = row
            .signature
            .as_deref()
            .and_then(normalize_sha256_hash)
            .ok_or_else(|| "manifest_signature_invalid".to_string())?;
        let expected = compute_blob_manifest_signature(&id, &hash, row.version, signing_key);
        if signature != expected {
            return Err(format!("manifest_signature_mismatch:{id}"));
        }
        normalized.push(NormalizedSignedBlobManifestEntry {
            id,
            hash,
            version: row.version,
            signature,
        });
    }
    Ok(normalized)
}

pub fn decode_signed_bincode_blob_manifest_with_adapter<T, E, F, G>(
    bytes: &[u8],
    max_blob_id_len: usize,
    signing_key: &str,
    adapt_entry: F,
    map_error: G,
) -> Result<Vec<T>, E>
where
    F: FnMut(NormalizedSignedBlobManifestEntry) -> T,
    G: FnOnce(String) -> E,
{
    match decode_normalized_signed_bincode_blob_manifest(bytes, max_blob_id_len, signing_key) {
        Ok(rows) => Ok(rows.into_iter().map(adapt_entry).collect()),
        Err(err) => Err(map_error(err)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Created,
    Running,
    Suspended,
    Terminated,
    Crashed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    Observe,
    Assist,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleMode {
    Reactive,
    Periodic,
    Proactive,
    Continuous,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolProfile {
    Minimal,
    Standard,
    Elevated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelSpecialty {
    General,
    Coding,
    Research,
    Reasoning,
    Vision,
    Planning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentIdentity {
    pub name: String,
    pub avatar_path: Option<String>,
    pub emoji: Option<String>,
    pub vibe: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceQuota {
    pub max_context_tokens: u32,
    pub max_tool_calls_per_turn: u16,
    pub max_wall_time_seconds: u32,
    pub max_parallel_tasks: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestCapabilities {
    pub can_spawn_agents: bool,
    pub can_write_files: bool,
    pub can_execute_commands: bool,
    pub can_browse_web: bool,
    pub can_use_marketplace: bool,
    pub can_manage_identity: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Option<Value>,
    pub required_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolConfig {
    pub enabled: bool,
    pub timeout_seconds: u32,
    pub retry_limit: u8,
    pub profile: ToolProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub context_window_tokens: u32,
    pub is_local: bool,
    pub params_billion: Option<f64>,
    pub price_input_per_1k: Option<f64>,
    pub price_output_per_1k: Option<f64>,
    pub specialty: ModelSpecialty,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelRef {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelRoutingConfig {
    pub default_model: ModelRef,
    pub fallback_models: Vec<ModelRef>,
    pub by_workload: BTreeMap<String, ModelRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionLabel(String);

impl SessionLabel {
    pub fn new(raw: &str) -> Result<Self, String> {
        let normalized = strip_invisible_unicode(raw);
        let trimmed = normalized.trim();
        if trimmed.is_empty() {
            return Err("session_label_empty".to_string());
        }
        if trimmed.len() > 64 {
            return Err("session_label_too_long".to_string());
        }
        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
        {
            return Err("session_label_invalid_chars".to_string());
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentManifest {
    pub agent_id: String,
    pub created_at: String,
    pub identity: AgentIdentity,
    pub mode: AgentMode,
    pub schedule_mode: ScheduleMode,
    pub capabilities: ManifestCapabilities,
    pub quota: ResourceQuota,
    pub routing: ModelRoutingConfig,
    pub models: Vec<ModelConfig>,
    pub tools: BTreeMap<String, ToolConfig>,
    pub labels: Vec<SessionLabel>,
    pub metadata: BTreeMap<String, String>,
}

impl AgentManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.created_at.trim().is_empty() {
            return Err("agent_manifest_missing_created_at".to_string());
        }
        if !is_valid_manifest_token(&self.agent_id, MAX_AGENT_ID_LEN, false) {
            return Err("agent_manifest_invalid_agent_id".to_string());
        }
        if !is_valid_manifest_token(&self.identity.name, MAX_AGENT_NAME_LEN, true) {
            return Err("agent_manifest_invalid_name".to_string());
        }
        if self.models.is_empty() {
            return Err("agent_manifest_missing_models".to_string());
        }
        if !is_valid_manifest_token(
            &self.routing.default_model.provider,
            MAX_PROVIDER_TOKEN_LEN,
            false,
        ) || !is_valid_manifest_token(
            &self.routing.default_model.model,
            MAX_MODEL_TOKEN_LEN,
            false,
        ) {
            return Err("agent_manifest_invalid_default_model".to_string());
        }
        for model in &self.models {
            if !is_valid_manifest_token(&model.provider, MAX_PROVIDER_TOKEN_LEN, false)
                || !is_valid_manifest_token(&model.model, MAX_MODEL_TOKEN_LEN, false)
            {
                return Err("agent_manifest_invalid_model_entry".to_string());
            }
        }
        for fallback in &self.routing.fallback_models {
            if !is_valid_manifest_token(&fallback.provider, MAX_PROVIDER_TOKEN_LEN, false)
                || !is_valid_manifest_token(&fallback.model, MAX_MODEL_TOKEN_LEN, false)
            {
                return Err("agent_manifest_invalid_fallback_model".to_string());
            }
        }
        for (workload, model) in &self.routing.by_workload {
            if !is_valid_manifest_token(workload, MAX_METADATA_KEY_LEN, false) {
                return Err("agent_manifest_invalid_workload_key".to_string());
            }
            if !is_valid_manifest_token(&model.provider, MAX_PROVIDER_TOKEN_LEN, false)
                || !is_valid_manifest_token(&model.model, MAX_MODEL_TOKEN_LEN, false)
            {
                return Err("agent_manifest_invalid_workload_model".to_string());
            }
        }
        for (key, value) in &self.metadata {
            if !is_valid_manifest_token(key, MAX_METADATA_KEY_LEN, false) {
                return Err("agent_manifest_invalid_metadata_key".to_string());
            }
            if !is_valid_manifest_token(value, MAX_METADATA_VALUE_LEN, true) {
                return Err("agent_manifest_invalid_metadata_value".to_string());
            }
        }
        let mut seen_labels = BTreeSet::<String>::new();
        for label in &self.labels {
            let normalized = label.as_str().to_ascii_lowercase();
            if !seen_labels.insert(normalized) {
                return Err("agent_manifest_duplicate_label".to_string());
            }
        }
        for (tool_id, tool_cfg) in &self.tools {
            if !is_valid_manifest_token(tool_id, MAX_TOOL_ID_LEN, false) {
                return Err("agent_manifest_invalid_tool_id".to_string());
            }
            if tool_cfg.timeout_seconds == 0 {
                return Err("agent_manifest_invalid_tool_timeout".to_string());
            }
        }
        if self.quota.max_context_tokens == 0 {
            return Err("agent_manifest_invalid_context_quota".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentEntry {
    pub manifest: AgentManifest,
    pub state: AgentState,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infring_detach_contract_id_is_bound_to_infring_types_authority() {
        assert_eq!(
            INFRING_DETACH_CONTRACT_ID_INFRING_TYPES,
            "V6-INFRING-DETACH-001.6"
        );
    }

    #[test]
    fn session_label_enforces_contract() {
        assert!(SessionLabel::new("agent.main_01").is_ok());
        assert!(SessionLabel::new("bad label").is_err());
        assert!(SessionLabel::new("").is_err());
    }

    #[test]
    fn manifest_validation_rejects_missing_models() {
        let manifest = AgentManifest {
            agent_id: "agent-main".to_string(),
            created_at: "2026-03-26T00:00:00Z".to_string(),
            identity: AgentIdentity {
                name: "Main".to_string(),
                avatar_path: None,
                emoji: Some("bot".to_string()),
                vibe: Some("direct".to_string()),
            },
            mode: AgentMode::Assist,
            schedule_mode: ScheduleMode::Reactive,
            capabilities: ManifestCapabilities {
                can_spawn_agents: true,
                can_write_files: true,
                can_execute_commands: true,
                can_browse_web: true,
                can_use_marketplace: true,
                can_manage_identity: true,
            },
            quota: ResourceQuota {
                max_context_tokens: 16_384,
                max_tool_calls_per_turn: 32,
                max_wall_time_seconds: 600,
                max_parallel_tasks: 8,
            },
            routing: ModelRoutingConfig {
                default_model: ModelRef {
                    provider: "ollama".to_string(),
                    model: "qwen2.5:7b".to_string(),
                },
                fallback_models: Vec::new(),
                by_workload: BTreeMap::new(),
            },
            models: Vec::new(),
            tools: BTreeMap::new(),
            labels: vec![SessionLabel::new("main").expect("label")],
            metadata: BTreeMap::new(),
        };

        let err = manifest.validate().expect_err("missing models should fail");
        assert_eq!(err, "agent_manifest_missing_models");
    }

    #[test]
    fn manifest_validation_accepts_valid_shape() {
        let manifest = AgentManifest {
            agent_id: "agent-main".to_string(),
            created_at: "2026-03-26T00:00:00Z".to_string(),
            identity: AgentIdentity {
                name: "Main".to_string(),
                avatar_path: None,
                emoji: Some("bot".to_string()),
                vibe: Some("friendly".to_string()),
            },
            mode: AgentMode::Assist,
            schedule_mode: ScheduleMode::Reactive,
            capabilities: ManifestCapabilities {
                can_spawn_agents: true,
                can_write_files: true,
                can_execute_commands: true,
                can_browse_web: true,
                can_use_marketplace: true,
                can_manage_identity: true,
            },
            quota: ResourceQuota {
                max_context_tokens: 16_384,
                max_tool_calls_per_turn: 32,
                max_wall_time_seconds: 600,
                max_parallel_tasks: 8,
            },
            routing: ModelRoutingConfig {
                default_model: ModelRef {
                    provider: "ollama".to_string(),
                    model: "qwen2.5:7b".to_string(),
                },
                fallback_models: vec![ModelRef {
                    provider: "openai".to_string(),
                    model: "gpt-5".to_string(),
                }],
                by_workload: BTreeMap::from([(
                    "coding".to_string(),
                    ModelRef {
                        provider: "ollama".to_string(),
                        model: "qwen2.5-coder:14b".to_string(),
                    },
                )]),
            },
            models: vec![ModelConfig {
                provider: "ollama".to_string(),
                model: "qwen2.5:7b".to_string(),
                context_window_tokens: 32_768,
                is_local: true,
                params_billion: Some(7.0),
                price_input_per_1k: None,
                price_output_per_1k: None,
                specialty: ModelSpecialty::General,
            }],
            tools: BTreeMap::from([(
                "terminal".to_string(),
                ToolConfig {
                    enabled: true,
                    timeout_seconds: 120,
                    retry_limit: 1,
                    profile: ToolProfile::Standard,
                },
            )]),
            labels: vec![SessionLabel::new("main").expect("label")],
            metadata: BTreeMap::new(),
        };

        manifest.validate().expect("valid manifest should pass");
    }
}
