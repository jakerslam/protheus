use crate::agent::{AgentBuildError, AgentBuilder, AgentExecutionContext, AgentRunResult};
use crate::capability_pack::CapabilityPackCatalog;
use crate::merkle_receipt::{merkle_receipt_options_from_value, merkle_receipt_payload};
use crate::provider::{ProviderClientRegistry, ProviderError};
use crate::rbac_memory::{
    memory_read_allowed, memory_write_allowed, permission_manifest_from_value,
    permission_manifest_from_value_with_inheritance,
    permission_manifest_snapshot, permission_for, PermissionTrit,
};
use crate::realtime_voice::{normalize_voice_session_request, voice_session_contract};
use crate::runtime_state::{
    runtime_lane_state_load, runtime_lane_state_mark_schedule_failure,
    runtime_lane_state_mark_schedule_success, runtime_lane_state_path,
    runtime_lane_state_record_denied_action, runtime_lane_state_record_merkle_continuity_failure,
    runtime_lane_state_release_gate_counters, runtime_lane_state_save, RuntimeLaneDurableState,
};
use crate::scheduler::SchedulePlan;
use crate::wasm_sandbox::{
    evaluate_wasm_execution_boundary, evaluate_wasm_policy, wasm_policy_from_value,
    wasm_policy_snapshot, WasmPolicyDecision,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

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
    #[serde(default)]
    pub schedule_interval_seconds: Option<u64>,
    #[serde(default)]
    pub schedule_max_runs: Option<u32>,
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
        schedule_interval_seconds,
        schedule_max_runs,
    } = request;

    let state_path = runtime_lane_state_path(&metadata);
    let mut durable_state = runtime_lane_state_load(&state_path);
    let parent_permissions_manifest =
        permission_manifest_from_value(metadata.get("parent_permissions_manifest"));
    let permissions_template = metadata
        .get("permissions_template")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let permissions = permission_manifest_from_value_with_inheritance(
        permissions_manifest.as_ref(),
        permissions_template,
        Some(&parent_permissions_manifest),
    );
    let catalog = CapabilityPackCatalog::new();
    let required_pack_permissions = catalog.required_permissions_for_packs(&capability_packs);
    for permission in &required_pack_permissions {
        let state = permission_for(&permissions, permission);
        if state != PermissionTrit::Allow {
            return Ok(runtime_lane_fail_closed_with_state(
                "runtime_lane_pack_permission_denied",
                json!({
                    "permission": permission,
                    "permission_state": permission_trit_code(state),
                }),
                &permissions,
                wasm_sandbox.as_ref(),
                voice_session.as_ref(),
                &state_path,
                &mut durable_state,
            ));
        }
    }
    if tools.iter().any(|tool| tool == "memory.read") && !memory_read_allowed(&permissions) {
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_memory_read_denied",
            json!({
                "permission": "memory.read",
                "permission_state": permission_trit_code(permission_for(&permissions, "memory.read")),
            }),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }
    if tools.iter().any(|tool| tool == "memory.write") && !memory_write_allowed(&permissions) {
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_memory_write_denied",
            json!({
                "permission": "memory.write",
                "permission_state": permission_trit_code(permission_for(&permissions, "memory.write")),
            }),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }

    let wasm_policy = wasm_policy_from_value(wasm_sandbox.as_ref());
    let requested_modules = runtime_requested_wasm_modules(&tools, &metadata);
    let requests_network = runtime_requests_network(&tools, &metadata);
    match evaluate_wasm_policy(&wasm_policy, &requested_modules, requests_network) {
        WasmPolicyDecision::Allowed => {}
        WasmPolicyDecision::Blocked(error_code) => {
            return Ok(runtime_lane_fail_closed_with_state(
                &error_code,
                json!({
                    "requested_modules": requested_modules,
                    "requests_network": requests_network
                }),
                &permissions,
                wasm_sandbox.as_ref(),
                voice_session.as_ref(),
                &state_path,
                &mut durable_state,
            ));
        }
    }

    let voice_request = normalize_voice_session_request(voice_session.as_ref());
    if voice_session.is_some() && voice_request.is_none() {
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_voice_contract_invalid",
            json!({"voice_session": voice_session}),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }

    let merkle_options = merkle_receipt_options_from_value(receipt_merkle.as_ref());
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
    if schedule_interval_seconds == Some(0) {
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_schedule_interval_invalid",
            json!({"schedule_interval_seconds": schedule_interval_seconds}),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }
    if schedule_max_runs == Some(0) {
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_schedule_max_runs_invalid",
            json!({"schedule_max_runs": schedule_max_runs}),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }
    if schedule_interval_seconds.is_some() || schedule_max_runs.is_some() {
        builder = builder.schedule(SchedulePlan {
            interval_seconds: schedule_interval_seconds.unwrap_or(300),
            jitter_seconds: 15,
            max_runs: schedule_max_runs,
        });
    }
    for tool in tools.clone() {
        builder = builder.tool(tool);
    }
    for pack in capability_packs.clone() {
        builder = builder.capability_pack(pack);
    }
    let contract = builder.build().map_err(RuntimeLaneError::Build)?;
    let contract = contract.with_default_schedule_from_packs(&catalog);
    let resolved_tools = contract.resolved_tools(Some(&catalog));
    let wasm_execution_fuel_used = metadata
        .get("wasm_execution")
        .and_then(|value| value.get("fuel_used"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let wasm_execution_elapsed_ms = metadata
        .get("wasm_execution")
        .and_then(|value| value.get("elapsed_ms"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    for tool in &resolved_tools {
        if let Some(module_id) = tool.strip_prefix("wasm.") {
            match evaluate_wasm_execution_boundary(
                &wasm_policy,
                module_id,
                wasm_execution_fuel_used,
                wasm_execution_elapsed_ms,
                requests_network,
            ) {
                WasmPolicyDecision::Allowed => {}
                WasmPolicyDecision::Blocked(error_code) => {
                    if let Some(plan) = &contract.schedule {
                        let pack_id = contract
                            .capability_packs
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "runtime".to_string());
                        runtime_lane_state_mark_schedule_failure(
                            &mut durable_state,
                            contract.name.as_str(),
                            pack_id.as_str(),
                            plan,
                            error_code.as_str(),
                        );
                    }
                    return Ok(runtime_lane_fail_closed_with_state(
                        error_code.as_str(),
                        json!({
                            "boundary": "wasm_execution",
                            "module_id": module_id,
                            "fuel_used": wasm_execution_fuel_used,
                            "elapsed_ms": wasm_execution_elapsed_ms,
                            "requests_network": requests_network,
                        }),
                        &permissions,
                        wasm_sandbox.as_ref(),
                        voice_session.as_ref(),
                        &state_path,
                        &mut durable_state,
                    ));
                }
            }
        }
    }
    let context = AgentExecutionContext::new(providers, Some(&catalog));
    let run: AgentRunResult = match contract.run_once(&context) {
        Ok(result) => result,
        Err(error) => {
            if let Some(plan) = &contract.schedule {
                let pack_id = contract
                    .capability_packs
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "runtime".to_string());
                runtime_lane_state_mark_schedule_failure(
                    &mut durable_state,
                    contract.name.as_str(),
                    pack_id.as_str(),
                    plan,
                    error.code.as_str(),
                );
            }
            let _ = runtime_lane_state_save(&state_path, &durable_state);
            return Err(RuntimeLaneError::Provider(error));
        }
    };
    let persisted_previous_root = durable_state
        .merkle_roots
        .get(contract.name.as_str())
        .cloned();
    if let (Some(requested), Some(persisted)) = (
        previous_receipt_root.as_deref(),
        persisted_previous_root.as_deref(),
    ) {
        if requested != persisted {
            runtime_lane_state_record_merkle_continuity_failure(&mut durable_state);
        }
    }
    let effective_previous_root = previous_receipt_root
        .as_deref()
        .or(persisted_previous_root.as_deref());
    let merkle = merkle_receipt_payload(&run.receipt, effective_previous_root, &merkle_options);
    if let Some(root) = merkle.get("root").and_then(Value::as_str) {
        durable_state
            .merkle_roots
            .insert(contract.name.clone(), root.to_string());
    }
    if let Some(plan) = &contract.schedule {
        let pack_id = contract
            .capability_packs
            .first()
            .cloned()
            .unwrap_or_else(|| "runtime".to_string());
        runtime_lane_state_mark_schedule_success(
            &mut durable_state,
            contract.name.as_str(),
            pack_id.as_str(),
            plan,
        );
    }
    let state_persist_error = runtime_lane_state_save(&state_path, &durable_state);
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
            "capability_profiles": catalog.autonomy_profiles_for_packs(&contract.capability_packs),
            "required_permissions": required_pack_permissions,
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
            "release_gate_counters": runtime_lane_state_release_gate_counters(&durable_state),
            "state_path": state_path.display().to_string(),
            "state_persist_error": state_persist_error,
        }),
        output: run.response.output,
        error: None,
    })
}

fn runtime_lane_fail_closed_with_state(
    error_code: &str,
    details: Value,
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
) -> RuntimeLaneResponse {
    runtime_lane_state_record_denied_action(durable_state, error_code);
    let state_persist_error = runtime_lane_state_save(state_path, durable_state);
    RuntimeLaneResponse {
        ok: false,
        contract: json!({
            "permissions_manifest": permission_manifest_snapshot(permissions),
            "wasm_sandbox": wasm_policy_snapshot(&wasm_policy_from_value(wasm_sandbox)),
            "voice_session_requested": voice_session.is_some(),
            "state_path": state_path.display().to_string(),
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
            "release_gate_counters": runtime_lane_state_release_gate_counters(durable_state),
            "state_persist_error": state_persist_error,
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
