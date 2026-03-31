use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const INFRING_DETACH_CONTRACT_ID_INFRING_TYPES: &str = "V6-INFRING-DETACH-001.6";

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
        let trimmed = raw.trim();
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
        if self.agent_id.trim().is_empty() {
            return Err("agent_manifest_missing_agent_id".to_string());
        }
        if self.identity.name.trim().is_empty() {
            return Err("agent_manifest_missing_name".to_string());
        }
        if self.models.is_empty() {
            return Err("agent_manifest_missing_models".to_string());
        }
        if self.routing.default_model.provider.trim().is_empty()
            || self.routing.default_model.model.trim().is_empty()
        {
            return Err("agent_manifest_invalid_default_model".to_string());
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
