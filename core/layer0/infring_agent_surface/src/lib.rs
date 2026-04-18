pub mod agent;
pub mod capability_pack;
pub mod mcp;
pub mod provider;
pub mod rbac_memory;
pub mod realtime_voice;
pub mod runtime_lane;
pub mod scheduler;
pub mod telemetry;
pub mod template;
pub mod wasm_sandbox;
pub mod merkle_receipt;

pub use agent::{AgentBuildError, AgentBuilder, AgentContract, AgentExecutionContext, AgentRunResult};
pub use capability_pack::{CapabilityPackCatalog, CapabilityPackSpec, ResearchCapabilityPack, WebOpsCapabilityPack};
pub use infring_agent_derive::{infring_agent, infring_tool};
pub use mcp::{mcp_handshake_receipt, McpBridge, McpServerConfig, McpTool};
pub use provider::{
    LocalEchoProvider, ProviderClient, ProviderClientRegistry, ProviderError, ProviderErrorCode,
    ProviderRequest, ProviderResponse,
};
pub use rbac_memory::{
    memory_read_allowed, memory_write_allowed, permission_manifest_from_value, PermissionManifest,
    PermissionTrit,
};
pub use realtime_voice::{
    normalize_voice_session_request, voice_session_contract, VoiceSessionRequest,
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
pub use wasm_sandbox::{
    evaluate_wasm_policy, wasm_policy_from_value, wasm_policy_snapshot, WasmPolicyDecision,
    WasmSandboxPolicy,
};
pub use merkle_receipt::{
    merkle_receipt_options_from_value, merkle_receipt_payload, MerkleReceiptOptions,
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
