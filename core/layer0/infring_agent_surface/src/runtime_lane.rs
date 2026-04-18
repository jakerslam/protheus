use crate::agent::{AgentBuildError, AgentBuilder, AgentExecutionContext, AgentRunResult};
use crate::capability_pack::CapabilityPackCatalog;
use crate::merkle_receipt::{merkle_receipt_options_from_value, merkle_receipt_payload};
use crate::provider::{ProviderClientRegistry, ProviderError};
use crate::rbac_memory::{
    memory_read_allowed, memory_write_allowed, permission_manifest_from_value,
    permission_manifest_snapshot, permission_for, PermissionTrit,
};
use crate::realtime_voice::{normalize_voice_session_request, voice_session_contract};
use crate::wasm_sandbox::{
    evaluate_wasm_policy, wasm_policy_from_value, wasm_policy_snapshot, WasmPolicyDecision,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeLaneRequest {
    pub name: String,
    pub preamble: Option<String>,
    pub initial_prompt: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub capability_packs: Vec<String>,
    pub lifespan_seconds: Option<u64>,
    pub metadata: Value,
    #[serde(default)]
    pub permissions_manifest: Option<Value>,
    #[serde(default)]
    pub wasm_sandbox: Option<Value>,
    #[serde(default)]
    pub voice_session: Option<Value>,
    #[serde(default)]
    pub receipt_merkle: Option<Value>,
    #[serde(default)]
    pub previous_receipt_root: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeLaneResponse {
    pub ok: bool,
    pub contract: Value,
    pub receipt: Value,
    pub trace_summary: Value,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum RuntimeLaneError {
    Build(AgentBuildError),
    Provider(ProviderError),
}

impl std::fmt::Display for RuntimeLaneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Build(error) => write!(f, "build:{}", error),
            Self::Provider(error) => write!(f, "provider:{}", error.message),
        }
    }
}

impl std::error::Error for RuntimeLaneError {}

pub fn run_runtime_lane(request: RuntimeLaneRequest) -> Result<RuntimeLaneResponse, RuntimeLaneError> {
    let providers = ProviderClientRegistry::with_builtin();
    run_runtime_lane_with_registry(request, &providers)
}

pub fn run_runtime_lane_with_registry(
    request: RuntimeLaneRequest,
    providers: &ProviderClientRegistry,
) -> Result<RuntimeLaneResponse, RuntimeLaneError> {
    let RuntimeLaneRequest {
        name,
        preamble,
        initial_prompt,
        provider,
        model,
        tools,
        capability_packs,
        lifespan_seconds,
        metadata,
        permissions_manifest,
        wasm_sandbox,
        voice_session,
        receipt_merkle,
        previous_receipt_root,
    } = request;

    let permissions = permission_manifest_from_value(permissions_manifest.as_ref());
    if tools.iter().any(|tool| tool == "memory.read") && !memory_read_allowed(&permissions) {
        return Ok(runtime_lane_fail_closed(
            "runtime_lane_memory_read_denied",
            json!({
                "permission": "memory.read",
                "permission_state": permission_trit_code(permission_for(&permissions, "memory.read")),
            }),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
        ));
    }
    if tools.iter().any(|tool| tool == "memory.write") && !memory_write_allowed(&permissions) {
        return Ok(runtime_lane_fail_closed(
            "runtime_lane_memory_write_denied",
            json!({
                "permission": "memory.write",
                "permission_state": permission_trit_code(permission_for(&permissions, "memory.write")),
            }),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
        ));
    }

    let wasm_policy = wasm_policy_from_value(wasm_sandbox.as_ref());
    let requested_modules = runtime_requested_wasm_modules(&tools, &metadata);
    let requests_network = runtime_requests_network(&tools, &metadata);
    match evaluate_wasm_policy(&wasm_policy, &requested_modules, requests_network) {
        WasmPolicyDecision::Allowed => {}
        WasmPolicyDecision::Blocked(error_code) => {
            return Ok(runtime_lane_fail_closed(
                &error_code,
                json!({
                    "requested_modules": requested_modules,
                    "requests_network": requests_network
                }),
                &permissions,
                wasm_sandbox.as_ref(),
                voice_session.as_ref(),
            ));
        }
    }

    let voice_request = normalize_voice_session_request(voice_session.as_ref());
    if voice_session.is_some() && voice_request.is_none() {
        return Ok(runtime_lane_fail_closed(
            "runtime_lane_voice_contract_invalid",
            json!({"voice_session": voice_session}),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
        ));
    }

    let merkle_options = merkle_receipt_options_from_value(receipt_merkle.as_ref());
    let catalog = CapabilityPackCatalog::new();
    let mut builder = AgentBuilder::new(name)
        .initial_prompt(initial_prompt)
        .metadata(metadata.clone());
    if let Some(value) = preamble {
        builder = builder.preamble(value);
    }
    if let Some(value) = provider {
        builder = builder.provider(value);
    }
    if let Some(value) = model {
        builder = builder.model(value);
    }
    if let Some(value) = lifespan_seconds {
        builder = builder.lifespan_seconds(value);
    }
    for tool in tools.clone() {
        builder = builder.tool(tool);
    }
    for pack in capability_packs.clone() {
        builder = builder.capability_pack(pack);
    }
    let contract = builder.build().map_err(RuntimeLaneError::Build)?;
    let contract = contract.with_default_schedule_from_packs(&catalog);
    let context = AgentExecutionContext::new(providers, Some(&catalog));
    let run: AgentRunResult = contract
        .run_once(&context)
        .map_err(RuntimeLaneError::Provider)?;
    let merkle = merkle_receipt_payload(
        &run.receipt,
        previous_receipt_root.as_deref(),
        &merkle_options,
    );
    let voice = voice_request
        .as_ref()
        .map(|request| {
            voice_session_contract(
                request,
                permission_for(&permissions, "voice.realtime") == PermissionTrit::Allow,
            )
        })
        .unwrap_or(Value::Null);
    Ok(RuntimeLaneResponse {
        ok: true,
        contract: json!({
            "name": contract.name,
            "provider": contract.provider,
            "tool_count": contract.resolved_tools(Some(&catalog)).len(),
            "tools": tools,
            "capability_packs": capability_packs,
            "schedule": contract.schedule,
            "lifespan_seconds": contract.lifespan_seconds,
            "permissions_manifest": permission_manifest_snapshot(&permissions),
            "wasm_sandbox": wasm_policy_snapshot(&wasm_policy),
            "voice_session": voice,
            "receipt_merkle": merkle,
        }),
        receipt: run.receipt,
        trace_summary: json!({
            "trace_id": run.trace.trace_id,
            "event_count": run.trace.events.len(),
            "agent_name": run.trace.agent_name,
            "wasm_modules": requested_modules,
            "requests_network": requests_network,
        }),
        output: run.response.output,
        error: None,
    })
}

fn runtime_lane_fail_closed(
    error_code: &str,
    details: Value,
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
) -> RuntimeLaneResponse {
    RuntimeLaneResponse {
        ok: false,
        contract: json!({
            "permissions_manifest": permission_manifest_snapshot(permissions),
            "wasm_sandbox": wasm_policy_snapshot(&wasm_policy_from_value(wasm_sandbox)),
            "voice_session_requested": voice_session.is_some(),
        }),
        receipt: json!({
            "type": "runtime_lane_receipt",
            "status": "fail_closed",
            "error_code": error_code,
            "details": details,
        }),
        trace_summary: json!({
            "status": "fail_closed",
            "error_code": error_code,
        }),
        output: String::new(),
        error: Some(error_code.to_string()),
    }
}

fn permission_trit_code(value: PermissionTrit) -> i8 {
    match value {
        PermissionTrit::Deny => -1,
        PermissionTrit::Ask => 0,
        PermissionTrit::Allow => 1,
    }
}

fn runtime_requested_wasm_modules(tools: &[String], metadata: &Value) -> Vec<String> {
    let mut modules = Vec::<String>::new();
    for tool in tools {
        if let Some(module) = tool.strip_prefix("wasm.") {
            let normalized = module.trim().to_ascii_lowercase();
            if !normalized.is_empty() {
                modules.push(normalized);
            }
        }
    }
    if let Some(items) = metadata.get("wasm_modules").and_then(Value::as_array) {
        for item in items {
            let Some(text) = item.as_str() else {
                continue;
            };
            let normalized = text.trim().to_ascii_lowercase();
            if !normalized.is_empty() {
                modules.push(normalized);
            }
        }
    }
    modules.sort();
    modules.dedup();
    modules
}

fn runtime_requests_network(tools: &[String], metadata: &Value) -> bool {
    if metadata
        .get("wasm_requests_network")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return true;
    }
    tools
        .iter()
        .any(|tool| matches!(tool.as_str(), "web.search" | "web.fetch" | "network.request"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_lane_executes_with_capability_pack_defaults() {
        let response = run_runtime_lane(RuntimeLaneRequest {
            name: "lane-agent".to_string(),
            preamble: Some("You are concise.".to_string()),
            initial_prompt: "Summarize system status in one line.".to_string(),
            provider: Some("local-echo".to_string()),
            model: None,
            tools: vec!["web.search".to_string()],
            capability_packs: vec!["research".to_string()],
            lifespan_seconds: Some(120),
            metadata: json!({"lane":"runtime"}),
            permissions_manifest: Some(json!({
                "grants": {
                    "voice.realtime": 1
                }
            })),
            wasm_sandbox: Some(json!({
                "enabled": true,
                "allow_network": true,
                "allowed_modules": ["planner.module"]
            })),
            voice_session: Some(json!({
                "transport": "webrtc",
                "provider": "realtime",
                "model": "gpt-realtime"
            })),
            receipt_merkle: Some(json!({
                "enabled": true,
                "seed": "wave4"
            })),
            previous_receipt_root: Some("root0".to_string()),
        })
        .expect("runtime lane");
        assert!(response.ok);
        assert_eq!(
            response
                .receipt
                .get("type")
                .and_then(serde_json::Value::as_str),
            Some("agent_run_receipt")
        );
        assert!(
            response
                .contract
                .get("receipt_merkle")
                .and_then(|value| value.get("root"))
                .and_then(Value::as_str)
                .is_some()
        );
    }

    #[test]
    fn runtime_lane_fail_closes_when_memory_write_permission_is_not_allowed() {
        let response = run_runtime_lane(RuntimeLaneRequest {
            name: "lane-agent".to_string(),
            preamble: Some("You are concise.".to_string()),
            initial_prompt: "Try mutation".to_string(),
            provider: Some("local-echo".to_string()),
            model: None,
            tools: vec!["memory.write".to_string()],
            capability_packs: vec![],
            lifespan_seconds: Some(120),
            metadata: json!({"lane":"runtime"}),
            permissions_manifest: Some(json!({
                "grants": {
                    "memory.write": 0
                }
            })),
            wasm_sandbox: None,
            voice_session: None,
            receipt_merkle: None,
            previous_receipt_root: None,
        })
        .expect("runtime lane");
        assert!(!response.ok);
        assert_eq!(
            response.error.as_deref(),
            Some("runtime_lane_memory_write_denied")
        );
    }
}
