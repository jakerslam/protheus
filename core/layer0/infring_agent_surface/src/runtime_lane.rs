use crate::agent::{AgentBuildError, AgentBuilder, AgentExecutionContext, AgentRunResult};
use crate::capability_pack::CapabilityPackCatalog;
use crate::provider::{ProviderClientRegistry, ProviderError};
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
    let catalog = CapabilityPackCatalog::new();
    let mut builder = AgentBuilder::new(request.name)
        .initial_prompt(request.initial_prompt)
        .metadata(request.metadata);
    if let Some(value) = request.preamble {
        builder = builder.preamble(value);
    }
    if let Some(value) = request.provider {
        builder = builder.provider(value);
    }
    if let Some(value) = request.model {
        builder = builder.model(value);
    }
    if let Some(value) = request.lifespan_seconds {
        builder = builder.lifespan_seconds(value);
    }
    for tool in request.tools {
        builder = builder.tool(tool);
    }
    for pack in request.capability_packs {
        builder = builder.capability_pack(pack);
    }
    let contract = builder.build().map_err(RuntimeLaneError::Build)?;
    let contract = contract.with_default_schedule_from_packs(&catalog);
    let context = AgentExecutionContext::new(providers, Some(&catalog));
    let run: AgentRunResult = contract
        .run_once(&context)
        .map_err(RuntimeLaneError::Provider)?;
    Ok(RuntimeLaneResponse {
        ok: true,
        contract: json!({
            "name": contract.name,
            "provider": contract.provider,
            "tool_count": contract.resolved_tools(Some(&catalog)).len(),
            "capability_packs": contract.capability_packs,
            "schedule": contract.schedule,
            "lifespan_seconds": contract.lifespan_seconds,
        }),
        receipt: run.receipt,
        trace_summary: json!({
            "trace_id": run.trace.trace_id,
            "event_count": run.trace.events.len(),
            "agent_name": run.trace.agent_name,
        }),
        output: run.response.output,
        error: None,
    })
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
    }
}

