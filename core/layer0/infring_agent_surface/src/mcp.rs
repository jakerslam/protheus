use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub id: String,
    pub transport: String,
    pub endpoint: String,
    pub auth_token_env: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct McpBridge {
    servers: BTreeMap<String, McpServerConfig>,
    tools_by_server: BTreeMap<String, Vec<McpTool>>,
}

impl McpBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_server(&mut self, config: McpServerConfig) {
        self.servers.insert(config.id.clone(), config);
    }

    pub fn register_tools(&mut self, server_id: &str, tools: Vec<McpTool>) {
        self.tools_by_server.insert(server_id.to_string(), tools);
    }

    pub fn server(&self, id: &str) -> Option<&McpServerConfig> {
        self.servers.get(id)
    }

    pub fn tools_for(&self, server_id: &str) -> Vec<McpTool> {
        self.tools_by_server
            .get(server_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn tool_descriptors(&self) -> Value {
        let mut descriptors = Vec::<Value>::new();
        for (server_id, tools) in &self.tools_by_server {
            for tool in tools {
                descriptors.push(json!({
                    "server_id": server_id,
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.input_schema,
                }));
            }
        }
        json!({
            "type": "mcp_tool_registry",
            "servers": self.servers.values().collect::<Vec<_>>(),
            "tools": descriptors,
        })
    }

    pub fn load_server_from_env(
        &mut self,
        prefix: &str,
        env: &HashMap<String, String>,
    ) -> Option<McpServerConfig> {
        let id_key = format!("{prefix}_ID");
        let endpoint_key = format!("{prefix}_ENDPOINT");
        let transport_key = format!("{prefix}_TRANSPORT");
        let token_env_key = format!("{prefix}_AUTH_TOKEN_ENV");
        let id = env.get(&id_key)?.trim();
        let endpoint = env.get(&endpoint_key)?.trim();
        if id.is_empty() || endpoint.is_empty() {
            return None;
        }
        let transport = env
            .get(&transport_key)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "stdio".to_string());
        let auth_token_env = env
            .get(&token_env_key)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let config = McpServerConfig {
            id: id.to_string(),
            transport,
            endpoint: endpoint.to_string(),
            auth_token_env,
            metadata: BTreeMap::new(),
        };
        self.register_server(config.clone());
        Some(config)
    }
}

pub fn mcp_handshake_receipt(
    server: &McpServerConfig,
    status: &str,
    detail: Option<&str>,
) -> Value {
    json!({
        "type": "mcp_handshake",
        "server_id": server.id,
        "transport": server.transport,
        "endpoint": server.endpoint,
        "status": status,
        "detail": detail.unwrap_or(""),
    })
}

