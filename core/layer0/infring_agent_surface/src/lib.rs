pub mod agent;
pub mod capability_pack;
pub mod mcp;
pub mod provider;
pub mod runtime_lane;
pub mod scheduler;
pub mod telemetry;
pub mod template;

pub use agent::{AgentBuildError, AgentBuilder, AgentContract, AgentExecutionContext, AgentRunResult};
pub use capability_pack::{CapabilityPackCatalog, CapabilityPackSpec, ResearchCapabilityPack, WebOpsCapabilityPack};
pub use infring_agent_derive::{infring_agent, infring_tool};
pub use mcp::{mcp_handshake_receipt, McpBridge, McpServerConfig, McpTool};
pub use provider::{
    LocalEchoProvider, ProviderClient, ProviderClientRegistry, ProviderError, ProviderErrorCode,
    ProviderRequest, ProviderResponse,
};
pub use runtime_lane::{
    run_runtime_lane, run_runtime_lane_with_registry, RuntimeLaneRequest, RuntimeLaneResponse,
};
pub use scheduler::{ScheduleEntry, SchedulePlan, Scheduler};
pub use telemetry::{ReceiptEvent, ReceiptSpan, ReceiptTraceSink, ReceiptVisualizer};
pub use template::{
    default_template_dir, scaffold_template, TemplateKind, TemplateScaffoldOptions,
    TemplateScaffoldResult,
};

#[macro_export]
macro_rules! agent {
    ($name:expr) => {{
        $crate::AgentBuilder::new($name)
    }};
    (
        $name:expr,
        preamble = $preamble:expr,
        provider = $provider:expr,
        tools = [$($tool:expr),* $(,)?]
    ) => {{
        let mut builder = $crate::AgentBuilder::new($name)
            .preamble($preamble)
            .provider($provider);
        $(
            builder = builder.tool($tool);
        )*
        builder
    }};
}
