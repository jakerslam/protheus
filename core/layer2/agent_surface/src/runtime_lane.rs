// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime surface coordination.
use crate::agent::{AgentBuildError, AgentBuilder, AgentExecutionContext, AgentRunResult};
use crate::capability_pack::CapabilityPackCatalog;
use crate::merkle_receipt::{merkle_receipt_options_from_value, merkle_receipt_payload};
use crate::native_tools::{NativeToolCall, NativeToolDispatcher, NativeToolReceipt};
use crate::provider::{ProviderClientRegistry, ProviderError};
use crate::rbac_memory::{
    memory_read_allowed, memory_write_allowed, permission_for, permission_manifest_from_value,
    permission_manifest_from_value_with_inheritance, permission_manifest_snapshot, PermissionTrit,
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
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

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

pub fn run_runtime_lane(
    request: RuntimeLaneRequest,
) -> Result<RuntimeLaneResponse, RuntimeLaneError> {
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
    let parent_permissions_snapshot = permission_manifest_snapshot(&parent_permissions_manifest);
    let parent_permissions_manifest_present = parent_permissions_snapshot
        .get("grants")
        .and_then(Value::as_object)
        .map(|grants| !grants.is_empty())
        .unwrap_or(false);
    let effective_permissions_snapshot = permission_manifest_snapshot(&permissions);
    let parent_permissions_patch_clamped = metadata
        .get("parent_permissions_patch_clamped")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let catalog = CapabilityPackCatalog::new();
    let required_pack_permissions = catalog.required_permissions_for_packs(&capability_packs);
    for permission in &required_pack_permissions {
        let state = permission_for(&permissions, permission);
        if state != PermissionTrit::Allow {
            let effective_state = permission_trit_code(state);
            let parent_state =
                permission_trit_code(permission_for(&parent_permissions_manifest, permission));
            return Ok(runtime_lane_fail_closed_with_state(
                "runtime_lane_pack_permission_denied",
                json!({
                    "permission": permission,
                    "permission_state": effective_state,
                    "enforcement_mode": "strict_fail_closed",
                    "blocked_permission_key_lineage": {
                        "permission": permission,
                        "effective_state": effective_state,
                        "parent_state": parent_state,
                        "lineage_chain": [
                            {"source": "effective_manifest", "state": effective_state},
                            {"source": "parent_manifest", "state": parent_state}
                        ]
                    },
                    "parent_permissions_manifest_present": parent_permissions_manifest_present,
                    "parent_permissions_patch_clamped": parent_permissions_patch_clamped,
                    "permissions_effective_snapshot": effective_permissions_snapshot.clone(),
                    "permissions_parent_snapshot": parent_permissions_snapshot.clone(),
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
        let effective_state = permission_trit_code(permission_for(&permissions, "memory.read"));
        let parent_state =
            permission_trit_code(permission_for(&parent_permissions_manifest, "memory.read"));
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_memory_read_denied",
            json!({
                "permission": "memory.read",
                "permission_state": effective_state,
                "enforcement_mode": "strict_fail_closed",
                "blocked_permission_key_lineage": {
                    "permission": "memory.read",
                    "effective_state": effective_state,
                    "parent_state": parent_state,
                    "lineage_chain": [
                        {"source": "effective_manifest", "state": effective_state},
                        {"source": "parent_manifest", "state": parent_state}
                    ]
                },
                "parent_permissions_manifest_present": parent_permissions_manifest_present,
                "parent_permissions_patch_clamped": parent_permissions_patch_clamped,
                "permissions_effective_snapshot": effective_permissions_snapshot.clone(),
                "permissions_parent_snapshot": parent_permissions_snapshot.clone(),
            }),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }
    if tools.iter().any(|tool| tool == "memory.write") && !memory_write_allowed(&permissions) {
        let effective_state = permission_trit_code(permission_for(&permissions, "memory.write"));
        let parent_state =
            permission_trit_code(permission_for(&parent_permissions_manifest, "memory.write"));
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_memory_write_denied",
            json!({
                "permission": "memory.write",
                "permission_state": effective_state,
                "enforcement_mode": "strict_fail_closed",
                "blocked_permission_key_lineage": {
                    "permission": "memory.write",
                    "effective_state": effective_state,
                    "parent_state": parent_state,
                    "lineage_chain": [
                        {"source": "effective_manifest", "state": effective_state},
                        {"source": "parent_manifest", "state": parent_state}
                    ]
                },
                "parent_permissions_manifest_present": parent_permissions_manifest_present,
                "parent_permissions_patch_clamped": parent_permissions_patch_clamped,
                "permissions_effective_snapshot": effective_permissions_snapshot.clone(),
                "permissions_parent_snapshot": parent_permissions_snapshot.clone(),
            }),
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        ));
    }
    if let Some((tool, permission)) = tools
        .iter()
        .filter_map(|tool| file_tool_permission(tool).map(|permission| (tool, permission)))
        .find(|(_, permission)| permission_for(&permissions, permission) != PermissionTrit::Allow)
    {
        let effective_state = permission_trit_code(permission_for(&permissions, permission));
        let parent_state =
            permission_trit_code(permission_for(&parent_permissions_manifest, permission));
        return Ok(runtime_lane_fail_closed_with_state(
            "runtime_lane_file_tool_permission_denied",
            json!({
                "tool": tool,
                "permission": permission,
                "permission_state": effective_state,
                "enforcement_mode": "strict_fail_closed",
                "blocked_permission_key_lineage": {
                    "permission": permission,
                    "effective_state": effective_state,
                    "parent_state": parent_state,
                    "lineage_chain": [
                        {"source": "effective_manifest", "state": effective_state},
                        {"source": "parent_manifest", "state": parent_state}
                    ]
                },
                "parent_permissions_manifest_present": parent_permissions_manifest_present,
                "parent_permissions_patch_clamped": parent_permissions_patch_clamped,
                "permissions_effective_snapshot": effective_permissions_snapshot.clone(),
                "permissions_parent_snapshot": parent_permissions_snapshot.clone(),
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

    if let Some(response) = runtime_lane_try_direct_mutation(
        &name,
        &initial_prompt,
        &metadata,
        &tools,
        &capability_packs,
        &required_pack_permissions,
        &permissions,
        wasm_sandbox.as_ref(),
        voice_session.as_ref(),
        receipt_merkle.as_ref(),
        previous_receipt_root.as_ref(),
        &state_path,
        &mut durable_state,
    ) {
        return Ok(response);
    }

    if let Some(response) = runtime_lane_try_deterministic_local_loop(
        &name,
        &initial_prompt,
        &metadata,
        &tools,
        &capability_packs,
        &required_pack_permissions,
        &permissions,
        wasm_sandbox.as_ref(),
        voice_session.as_ref(),
        receipt_merkle.as_ref(),
        previous_receipt_root.as_ref(),
        &state_path,
        &mut durable_state,
    ) {
        return Ok(response);
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
    if let Some((error_code, details)) =
        native_success_contract_violation(&metadata, &run.receipt, &run.response.output)
    {
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
        let mut response = runtime_lane_fail_closed_with_state(
            error_code.as_str(),
            details,
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        );
        runtime_lane_attach_agent_run_journal(&mut response, &run);
        return Ok(response);
    }
    if let Some((error_code, details)) =
        public_reasoning_contract_violation(&metadata, &run.receipt, &run.response.output)
    {
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
        let mut response = runtime_lane_fail_closed_with_state(
            error_code.as_str(),
            details,
            &permissions,
            wasm_sandbox.as_ref(),
            voice_session.as_ref(),
            &state_path,
            &mut durable_state,
        );
        runtime_lane_attach_agent_run_journal(&mut response, &run);
        return Ok(response);
    }
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
    let agent_status = run
        .receipt
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("ok")
        .to_string();
    let response_ok = agent_status == "ok";
    let response_error = if response_ok {
        None
    } else {
        Some(format!("runtime_lane_agent_status:{agent_status}"))
    };
    Ok(RuntimeLaneResponse {
        ok: response_ok,
        contract: json!({
            "name": contract.name,
            "provider": contract.provider,
            "agent_status": agent_status.clone(),
            "tool_count": contract.resolved_tools(Some(&catalog)).len(),
            "native_tool_call_count": run
                .receipt
                .get("native_tool_call_count")
                .and_then(Value::as_u64)
                .unwrap_or(0),
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
            "workflow": metadata
                .get("workflow")
                .cloned()
                .unwrap_or(Value::Null),
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
        error: response_error,
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

#[derive(Clone, Debug)]
struct DirectMutationCandidate {
    workspace_root: Option<PathBuf>,
    target_path: PathBuf,
    content: String,
    content_source: &'static str,
    overwrite: bool,
}

#[derive(Clone, Debug)]
enum DirectMutationGate {
    NotCandidate,
    Blocked {
        failure_code: &'static str,
        failure_message: String,
        needed_input: Option<String>,
        target_path: Option<String>,
    },
    Candidate(DirectMutationCandidate),
}

#[derive(Clone, Debug)]
struct DeterministicLocalLoopCandidate {
    workspace_root: PathBuf,
    actions: Vec<DeterministicLocalAction>,
    requires_validation: bool,
}

#[derive(Clone, Debug)]
enum DeterministicLocalAction {
    WriteFile {
        target_path: PathBuf,
        content: String,
        overwrite: bool,
    },
    CommandRun {
        cwd: PathBuf,
        cmd: Vec<String>,
        timeout_seconds: u64,
        max_output_bytes: u64,
    },
}

#[derive(Clone, Debug)]
enum DeterministicLocalLoopGate {
    NotCandidate,
    Blocked {
        failure_code: &'static str,
        failure_message: String,
        needed_input: Option<String>,
    },
    Candidate(DeterministicLocalLoopCandidate),
}

fn runtime_lane_try_direct_mutation(
    name: &str,
    prompt: &str,
    metadata: &Value,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
) -> Option<RuntimeLaneResponse> {
    let total_started = Instant::now();
    let gate_started = Instant::now();
    let gate = runtime_lane_direct_mutation_candidate(prompt, tools, capability_packs, permissions);
    let execution_shape_gate_ms = gate_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    match gate {
        DirectMutationGate::NotCandidate => None,
        DirectMutationGate::Blocked {
            failure_code,
            failure_message,
            needed_input,
            target_path,
        } => {
            let response = runtime_lane_fail_closed_with_state(
                "runtime_lane_direct_mutation_blocked",
                json!({
                    "lane": "structured_blocker",
                    "lane_reason": "direct_mutation_precondition_failed",
                    "failure_code": failure_code,
                    "failure_message": failure_message,
                    "needed_input": needed_input,
                    "target_path": target_path,
                    "phase_latency_ms": {
                        "workflow_load": 0,
                        "execution_shape_gate": execution_shape_gate_ms,
                        "provider_start": 0,
                        "model_call": 0,
                        "tool_dispatch": 0,
                        "mutation": 0,
                        "validation": 0,
                        "repair": 0,
                        "final_synthesis": 0,
                        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                    }
                }),
                permissions,
                wasm_sandbox,
                voice_session,
                state_path,
                durable_state,
            );
            Some(response)
        }
        DirectMutationGate::Candidate(candidate) => {
            let dispatch_started = Instant::now();
            let dispatcher = NativeToolDispatcher::new(&["file_write".to_string()]);
            let receipt = dispatcher.dispatch(NativeToolCall {
                id: "single_mutation_execution_1".to_string(),
                name: "file_write".to_string(),
                args: json!({
                    "path": candidate.target_path.display().to_string(),
                    "content": candidate.content.clone(),
                    "overwrite": candidate.overwrite,
                    "direct_mutation_lane": true,
                    "content_source": candidate.content_source,
                }),
            });
            let tool_dispatch_ms =
                dispatch_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
            Some(runtime_lane_direct_mutation_response(
                name,
                metadata,
                tools,
                capability_packs,
                required_pack_permissions,
                permissions,
                wasm_sandbox,
                voice_session,
                receipt_merkle,
                previous_receipt_root,
                state_path,
                durable_state,
                candidate,
                receipt,
                execution_shape_gate_ms,
                tool_dispatch_ms,
                total_started,
            ))
        }
    }
}

fn runtime_lane_try_deterministic_local_loop(
    name: &str,
    prompt: &str,
    metadata: &Value,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
) -> Option<RuntimeLaneResponse> {
    let total_started = Instant::now();
    let gate_started = Instant::now();
    let gate =
        runtime_lane_deterministic_local_loop_candidate(prompt, tools, capability_packs, permissions);
    let execution_shape_gate_ms = gate_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    match gate {
        DeterministicLocalLoopGate::NotCandidate => None,
        DeterministicLocalLoopGate::Blocked {
            failure_code,
            failure_message,
            needed_input,
        } => {
            let response = runtime_lane_fail_closed_with_state(
                "runtime_lane_deterministic_local_loop_blocked",
                json!({
                    "lane": "structured_blocker",
                    "lane_reason": "deterministic_local_loop_precondition_failed",
                    "failure_code": failure_code,
                    "failure_message": failure_message,
                    "needed_input": needed_input,
                    "phase_latency_ms": {
                        "workflow_load": 0,
                        "execution_shape_gate": execution_shape_gate_ms,
                        "provider_start": 0,
                        "model_call": 0,
                        "tool_dispatch": 0,
                        "mutation": 0,
                        "validation": 0,
                        "repair": 0,
                        "final_synthesis": 0,
                        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
                    }
                }),
                permissions,
                wasm_sandbox,
                voice_session,
                state_path,
                durable_state,
            );
            Some(response)
        }
        DeterministicLocalLoopGate::Candidate(candidate) => {
            let dispatch_started = Instant::now();
            let dispatcher =
                NativeToolDispatcher::new(&["file_write".to_string(), "command_run".to_string()]);
            let mut receipts = Vec::<NativeToolReceipt>::new();
            for (index, action) in candidate.actions.iter().enumerate() {
                let call = match action {
                    DeterministicLocalAction::WriteFile {
                        target_path,
                        content,
                        overwrite,
                    } => NativeToolCall {
                        id: format!("deterministic_local_loop_{}", index + 1),
                        name: "file_write".to_string(),
                        args: json!({
                            "path": target_path.display().to_string(),
                            "content": content,
                            "overwrite": overwrite,
                            "deterministic_local_loop": true,
                        }),
                    },
                    DeterministicLocalAction::CommandRun {
                        cwd,
                        cmd,
                        timeout_seconds,
                        max_output_bytes,
                    } => NativeToolCall {
                        id: format!("deterministic_local_loop_{}", index + 1),
                        name: "command_run".to_string(),
                        args: json!({
                            "cwd": cwd.display().to_string(),
                            "cmd": cmd,
                            "timeout_seconds": timeout_seconds,
                            "max_output_bytes": max_output_bytes,
                            "deterministic_local_loop": true,
                        }),
                    },
                };
                let receipt = dispatcher.dispatch(call);
                let should_stop = receipt.status != "ok"
                    || (receipt.tool_name == "command_run"
                        && !receipt
                            .result
                            .get("success")
                            .and_then(Value::as_bool)
                            .unwrap_or(true));
                receipts.push(receipt);
                if should_stop {
                    break;
                }
            }
            let tool_dispatch_ms =
                dispatch_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
            Some(runtime_lane_deterministic_local_loop_response(
                name,
                metadata,
                tools,
                capability_packs,
                required_pack_permissions,
                permissions,
                wasm_sandbox,
                voice_session,
                receipt_merkle,
                previous_receipt_root,
                state_path,
                durable_state,
                candidate,
                receipts,
                execution_shape_gate_ms,
                tool_dispatch_ms,
                total_started,
            ))
        }
    }
}

fn runtime_lane_deterministic_local_loop_response(
    name: &str,
    metadata: &Value,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
    candidate: DeterministicLocalLoopCandidate,
    receipts: Vec<NativeToolReceipt>,
    execution_shape_gate_ms: u64,
    tool_dispatch_ms: u64,
    total_started: Instant,
) -> RuntimeLaneResponse {
    let mutation_count = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok" && receipt.tool_name == "file_write")
        .count();
    let validation_receipts = receipts
        .iter()
        .filter(|receipt| receipt.tool_name == "command_run")
        .collect::<Vec<_>>();
    let validation_ok = validation_receipts
        .last()
        .map(|receipt| {
            receipt.status == "ok"
                && receipt
                    .result
                    .get("success")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
        })
        .unwrap_or(!candidate.requires_validation);
    let ok = !receipts.is_empty()
        && receipts.iter().all(|receipt| receipt.status == "ok")
        && mutation_count > 0
        && validation_ok;
    let changed_files = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok" && receipt.tool_name == "file_write")
        .filter_map(|receipt| {
            Some(json!({
                "path": receipt.result.get("path")?.as_str()?,
                "operation": if receipt
                    .result
                    .get("created")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "created"
                } else {
                    "written"
                },
                "receipt_ref": receipt.call_id,
            }))
        })
        .collect::<Vec<_>>();
    let receipt_refs = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .map(|receipt| receipt.call_id.clone())
        .collect::<Vec<_>>();
    let mutation_ms = receipts
        .iter()
        .filter(|receipt| receipt.tool_name == "file_write")
        .map(|receipt| receipt.duration_ms)
        .sum::<u64>();
    let validation_ms = receipts
        .iter()
        .filter(|receipt| receipt.tool_name == "command_run")
        .map(|receipt| receipt.duration_ms)
        .sum::<u64>();
    let final_synthesis_started = Instant::now();
    let validation_status = if candidate.requires_validation {
        if validation_ok {
            "passed"
        } else {
            "failed"
        }
    } else {
        "not_run"
    };
    let output = format!(
        "{} via deterministic_local_loop.\n\nChanged files:\n{}\n\nValidation: {validation_status}.\nReceipts: {}",
        if ok { "Completed" } else { "Stopped" },
        changed_files
            .iter()
            .filter_map(|item| item.get("path").and_then(Value::as_str))
            .map(|path| format!("- {path}"))
            .collect::<Vec<_>>()
            .join("\n"),
        receipt_refs.join(", ")
    );
    let final_synthesis_ms =
        final_synthesis_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let phase_latency_ms = json!({
        "workflow_load": 0,
        "execution_shape_gate": execution_shape_gate_ms,
        "provider_start": 0,
        "model_call": 0,
        "tool_dispatch": tool_dispatch_ms,
        "mutation": mutation_ms,
        "validation": validation_ms,
        "repair": 0,
        "final_synthesis": final_synthesis_ms,
        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    });
    let mut runtime_receipt = json!({
        "type": "runtime_lane_receipt",
        "status": if ok { "ok" } else { "deterministic_local_loop_failed" },
        "lane": "deterministic_local_loop",
        "lane_reason": "declared_local_action_manifest",
        "requires_model": false,
        "requires_discovery": false,
        "requires_validation": candidate.requires_validation,
        "target_scope": "declared_manifest",
        "mutation_safety": "safe_manifest_actions",
        "workspace_root": candidate.workspace_root.display().to_string(),
        "changed_file_summary": changed_files,
        "native_tool_call_count": receipts.len(),
        "native_tool_receipts": receipts,
        "receipt_refs": receipt_refs,
        "validation_status": validation_status,
        "phase_latency_ms": phase_latency_ms,
    });
    let merkle_options = merkle_receipt_options_from_value(receipt_merkle);
    let persisted_previous_root = durable_state.merkle_roots.get(name).cloned();
    let effective_previous_root = previous_receipt_root
        .map(String::as_str)
        .or(persisted_previous_root.as_deref());
    let merkle = merkle_receipt_payload(&runtime_receipt, effective_previous_root, &merkle_options);
    if let Some(root) = merkle.get("root").and_then(Value::as_str) {
        durable_state
            .merkle_roots
            .insert(name.to_string(), root.to_string());
    }
    let state_persist_error = runtime_lane_state_save(state_path, durable_state);
    if let Some(object) = runtime_receipt.as_object_mut() {
        object.insert("receipt_merkle".to_string(), merkle.clone());
    }
    RuntimeLaneResponse {
        ok,
        contract: json!({
            "name": name,
            "provider": Value::Null,
            "agent_status": if ok { "ok" } else { "deterministic_local_loop_failed" },
            "tool_count": tools.len(),
            "native_tool_call_count": runtime_receipt
                .get("native_tool_call_count")
                .cloned()
                .unwrap_or(Value::Null),
            "tools": tools,
            "capability_packs": capability_packs,
            "required_permissions": required_pack_permissions,
            "schedule": Value::Null,
            "lifespan_seconds": Value::Null,
            "permissions_manifest": permission_manifest_snapshot(permissions),
            "wasm_sandbox": wasm_policy_snapshot(&wasm_policy_from_value(wasm_sandbox)),
            "voice_session_requested": voice_session.is_some(),
            "receipt_merkle": merkle,
            "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
            "execution_shape": {
                "lane": "deterministic_local_loop",
                "confidence": 1.0,
                "requires_model": false,
                "requires_discovery": false,
                "requires_validation": candidate.requires_validation,
                "target_scope": "declared_manifest",
                "mutation_safety": "safe_manifest_actions",
                "escalation_reason": Value::Null
            }
        }),
        receipt: runtime_receipt,
        trace_summary: json!({
            "status": if ok { "ok" } else { "deterministic_local_loop_failed" },
            "lane": "deterministic_local_loop",
            "events": [
                "coding.task_contract.created",
                "coding.execution_shape.selected",
                "coding.local_action_loop.started",
                "coding.mutation.requested",
                if mutation_count > 0 { "coding.mutation.applied" } else { "coding.mutation.failed" },
                if candidate.requires_validation { "coding.validation.completed" } else { "coding.validation.skipped" },
                "coding.final_synthesis.completed"
            ],
            "phase_latency_ms": phase_latency_ms,
            "state_path": state_path.display().to_string(),
            "state_persist_error": state_persist_error,
            "release_gate_counters": runtime_lane_state_release_gate_counters(durable_state),
        }),
        output,
        error: if ok {
            None
        } else {
            Some("runtime_lane_deterministic_local_loop_failed".to_string())
        },
    }
}

fn runtime_lane_direct_mutation_response(
    name: &str,
    metadata: &Value,
    tools: &[String],
    capability_packs: &[String],
    required_pack_permissions: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
    wasm_sandbox: Option<&Value>,
    voice_session: Option<&Value>,
    receipt_merkle: Option<&Value>,
    previous_receipt_root: Option<&String>,
    state_path: &Path,
    durable_state: &mut RuntimeLaneDurableState,
    candidate: DirectMutationCandidate,
    receipt: NativeToolReceipt,
    execution_shape_gate_ms: u64,
    tool_dispatch_ms: u64,
    total_started: Instant,
) -> RuntimeLaneResponse {
    let ok = receipt.status == "ok";
    let changed_file = candidate.target_path.display().to_string();
    let mutation_ms = receipt.duration_ms;
    let final_synthesis_started = Instant::now();
    let output = if ok {
        format!(
            "Completed via direct_mutation.\n\nChanged files:\n- {changed_file}\n\nValidation: not run.\nReceipt: {}",
            receipt.call_id
        )
    } else {
        format!(
            "Direct mutation was blocked before provider startup.\n\nTarget: {changed_file}\nError: {}",
            receipt.error.clone().unwrap_or_else(|| "unknown_error".to_string())
        )
    };
    let final_synthesis_ms =
        final_synthesis_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let phase_latency_ms = json!({
        "workflow_load": 0,
        "execution_shape_gate": execution_shape_gate_ms,
        "provider_start": 0,
        "model_call": 0,
        "tool_dispatch": tool_dispatch_ms,
        "mutation": mutation_ms,
        "validation": 0,
        "repair": 0,
        "final_synthesis": final_synthesis_ms,
        "total": total_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    });
    let receipt_refs = if ok {
        vec![receipt.call_id.clone()]
    } else {
        Vec::new()
    };
    let mut runtime_receipt = json!({
        "type": "runtime_lane_receipt",
        "status": if ok { "ok" } else { "direct_mutation_failed" },
        "lane": "direct_mutation",
        "lane_reason": "explicit_content_and_safe_target_path",
        "requires_model": false,
        "requires_discovery": false,
        "requires_validation": false,
        "target_scope": if candidate.overwrite { "known_file" } else { "new_file" },
        "mutation_safety": if candidate.overwrite { "safe_overwrite_requested" } else { "safe_new" },
        "workspace_root": candidate
            .workspace_root
            .as_ref()
            .map(|path| path.display().to_string()),
        "changed_file_summary": if ok {
            json!([{
                "path": changed_file,
                "operation": if candidate.overwrite { "overwritten" } else { "created_or_written" },
                "receipt_ref": receipt.call_id
            }])
        } else {
            json!([])
        },
        "native_tool_call_count": 1,
        "native_tool_receipts": [receipt.clone()],
        "receipt_refs": receipt_refs,
        "phase_latency_ms": phase_latency_ms,
    });
    let merkle_options = merkle_receipt_options_from_value(receipt_merkle);
    let persisted_previous_root = durable_state.merkle_roots.get(name).cloned();
    let effective_previous_root = previous_receipt_root
        .map(String::as_str)
        .or(persisted_previous_root.as_deref());
    let merkle = merkle_receipt_payload(&runtime_receipt, effective_previous_root, &merkle_options);
    if let Some(root) = merkle.get("root").and_then(Value::as_str) {
        durable_state
            .merkle_roots
            .insert(name.to_string(), root.to_string());
    }
    let state_persist_error = runtime_lane_state_save(state_path, durable_state);
    if let Some(object) = runtime_receipt.as_object_mut() {
        object.insert("receipt_merkle".to_string(), merkle.clone());
    }
    RuntimeLaneResponse {
        ok,
        contract: json!({
            "name": name,
            "provider": Value::Null,
            "agent_status": if ok { "ok" } else { "direct_mutation_failed" },
            "tool_count": tools.len(),
            "native_tool_call_count": 1,
            "tools": tools,
            "capability_packs": capability_packs,
            "required_permissions": required_pack_permissions,
            "schedule": Value::Null,
            "lifespan_seconds": Value::Null,
            "permissions_manifest": permission_manifest_snapshot(permissions),
            "wasm_sandbox": wasm_policy_snapshot(&wasm_policy_from_value(wasm_sandbox)),
            "voice_session_requested": voice_session.is_some(),
            "receipt_merkle": merkle,
            "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
            "execution_shape": {
                "lane": "direct_mutation",
                "confidence": 1.0,
                "requires_model": false,
                "requires_discovery": false,
                "requires_validation": false,
                "target_scope": if candidate.overwrite { "known_file" } else { "new_file" },
                "mutation_safety": if candidate.overwrite { "safe_overwrite_requested" } else { "safe_new" },
                "escalation_reason": Value::Null
            }
        }),
        receipt: runtime_receipt,
        trace_summary: json!({
            "status": if ok { "ok" } else { "direct_mutation_failed" },
            "lane": "direct_mutation",
            "events": [
                "coding.task_contract.created",
                "coding.execution_shape.selected",
                "coding.mutation.requested",
                if ok { "coding.mutation.applied" } else { "coding.mutation.failed" },
                "coding.final_synthesis.completed"
            ],
            "phase_latency_ms": phase_latency_ms,
            "state_path": state_path.display().to_string(),
            "state_persist_error": state_persist_error,
            "release_gate_counters": runtime_lane_state_release_gate_counters(durable_state),
        }),
        output,
        error: if ok {
            None
        } else {
            Some("runtime_lane_direct_mutation_failed".to_string())
        },
    }
}

fn runtime_lane_direct_mutation_candidate(
    prompt: &str,
    tools: &[String],
    capability_packs: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
) -> DirectMutationGate {
    if !runtime_lane_direct_mutation_surface_enabled(tools, capability_packs) {
        return DirectMutationGate::NotCandidate;
    }
    if permission_for(permissions, "file.write") != PermissionTrit::Allow {
        return DirectMutationGate::NotCandidate;
    }
    let Some(content) = runtime_lane_extract_explicit_file_content(prompt) else {
        return DirectMutationGate::NotCandidate;
    };
    let workspace_root = runtime_lane_extract_workspace_root(prompt);
    let Some(raw_target) = runtime_lane_extract_target_file_path(prompt) else {
        return DirectMutationGate::NotCandidate;
    };
    let target_path = match runtime_lane_resolve_target_path(&raw_target, workspace_root.as_ref()) {
        Ok(path) => path,
        Err(error) => {
            return DirectMutationGate::Blocked {
                failure_code: "unsafe_or_unresolved_target_path",
                failure_message: error,
                needed_input: Some("Provide an explicit safe target path inside the workspace.".to_string()),
                target_path: Some(raw_target),
            };
        }
    };
    let overwrite = runtime_lane_prompt_requests_overwrite(prompt);
    if target_path.exists() && !overwrite {
        return DirectMutationGate::Blocked {
            failure_code: "unsafe_overwrite",
            failure_message: "Target already exists and overwrite was not explicitly requested.".to_string(),
            needed_input: Some("Confirm overwrite or choose a new target path.".to_string()),
            target_path: Some(target_path.display().to_string()),
        };
    }
    DirectMutationGate::Candidate(DirectMutationCandidate {
        workspace_root,
        target_path,
        content,
        content_source: "explicit_prompt_content",
        overwrite,
    })
}

fn runtime_lane_deterministic_local_loop_candidate(
    prompt: &str,
    tools: &[String],
    capability_packs: &[String],
    permissions: &crate::rbac_memory::PermissionManifest,
) -> DeterministicLocalLoopGate {
    if !runtime_lane_direct_mutation_surface_enabled(tools, capability_packs) {
        return DeterministicLocalLoopGate::NotCandidate;
    }
    let Some(manifest) = runtime_lane_extract_deterministic_manifest(prompt) else {
        return DeterministicLocalLoopGate::NotCandidate;
    };
    if permission_for(permissions, "file.write") != PermissionTrit::Allow {
        return DeterministicLocalLoopGate::Blocked {
            failure_code: "permission_denied",
            failure_message: "file.write permission is required for deterministic local loop mutations."
                .to_string(),
            needed_input: Some("Grant file.write or use a non-mutating workflow.".to_string()),
        };
    }
    let root_value = manifest
        .get("workspace_root")
        .or_else(|| manifest.get("project_root"))
        .or_else(|| manifest.get("repo_root"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(root_value) = root_value else {
        return DeterministicLocalLoopGate::Blocked {
            failure_code: "workspace_root_required",
            failure_message: "Deterministic local loop manifests require workspace_root."
                .to_string(),
            needed_input: Some("Add workspace_root to the deterministic local action manifest.".to_string()),
        };
    };
    let workspace_root = PathBuf::from(root_value);
    if !workspace_root.is_absolute() {
        return DeterministicLocalLoopGate::Blocked {
            failure_code: "absolute_workspace_root_required",
            failure_message: "workspace_root must be absolute.".to_string(),
            needed_input: Some("Use an absolute workspace_root path.".to_string()),
        };
    }
    let Some(actions_value) = manifest
        .get("actions")
        .or_else(|| manifest.get("files"))
        .and_then(Value::as_array)
    else {
        return DeterministicLocalLoopGate::Blocked {
            failure_code: "actions_required",
            failure_message: "Deterministic local loop manifests require actions or files."
                .to_string(),
            needed_input: Some("Add at least one write_file action with path and content.".to_string()),
        };
    };
    let mut actions = Vec::<DeterministicLocalAction>::new();
    for action in actions_value {
        let kind = action
            .get("type")
            .or_else(|| action.get("kind"))
            .or_else(|| action.get("action"))
            .and_then(Value::as_str)
            .unwrap_or("write_file")
            .trim()
            .to_ascii_lowercase();
        match kind.as_str() {
            "write_file" | "create_file" | "file_write" | "write" => {
                let Some(path) = action
                    .get("path")
                    .or_else(|| action.get("target_path"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "path_required",
                        failure_message: "write_file actions require path.".to_string(),
                        needed_input: Some("Add a path to each write_file action.".to_string()),
                    };
                };
                let Some(content) = action
                    .get("content")
                    .or_else(|| action.get("text"))
                    .or_else(|| action.get("body"))
                    .and_then(Value::as_str)
                else {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "content_required",
                        failure_message: "write_file actions require content.".to_string(),
                        needed_input: Some("Add content to each write_file action.".to_string()),
                    };
                };
                let target_path =
                    match runtime_lane_resolve_target_path(path, Some(&workspace_root)) {
                        Ok(path) => path,
                        Err(error) => {
                            return DeterministicLocalLoopGate::Blocked {
                                failure_code: "unsafe_or_unresolved_target_path",
                                failure_message: error,
                                needed_input: Some(
                                    "Use a safe path inside workspace_root.".to_string(),
                                ),
                            };
                        }
                    };
                let overwrite = action
                    .get("overwrite")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if target_path.exists() && !overwrite {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "unsafe_overwrite",
                        failure_message: format!(
                            "Target already exists and overwrite was not explicitly requested: {}",
                            target_path.display()
                        ),
                        needed_input: Some(
                            "Set overwrite=true or choose a new target path.".to_string(),
                        ),
                    };
                }
                actions.push(DeterministicLocalAction::WriteFile {
                    target_path,
                    content: ensure_trailing_newline(content.to_string()),
                    overwrite,
                });
            }
            "command_run" | "run_command" | "validate" | "validation" => {
                if permission_for(permissions, "command.run") != PermissionTrit::Allow {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "permission_denied",
                        failure_message: "command.run permission is required for validation actions."
                            .to_string(),
                        needed_input: Some(
                            "Grant command.run or remove validation actions.".to_string(),
                        ),
                    };
                }
                let cwd = action
                    .get("cwd")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| runtime_lane_resolve_target_path(value, Some(&workspace_root)))
                    .transpose();
                let cwd = match cwd {
                    Ok(Some(path)) => path,
                    Ok(None) => workspace_root.clone(),
                    Err(error) => {
                        return DeterministicLocalLoopGate::Blocked {
                            failure_code: "unsafe_or_unresolved_cwd",
                            failure_message: error,
                            needed_input: Some("Use a safe cwd inside workspace_root.".to_string()),
                        };
                    }
                };
                let Some(cmd) = runtime_lane_manifest_command(action) else {
                    return DeterministicLocalLoopGate::Blocked {
                        failure_code: "command_required",
                        failure_message: "command_run actions require cmd or command.".to_string(),
                        needed_input: Some("Add cmd as a string array or shell command string.".to_string()),
                    };
                };
                actions.push(DeterministicLocalAction::CommandRun {
                    cwd,
                    cmd,
                    timeout_seconds: action
                        .get("timeout_seconds")
                        .and_then(Value::as_u64)
                        .unwrap_or(30),
                    max_output_bytes: action
                        .get("max_output_bytes")
                        .and_then(Value::as_u64)
                        .unwrap_or(12000),
                });
            }
            other => {
                return DeterministicLocalLoopGate::Blocked {
                    failure_code: "unsupported_action_type",
                    failure_message: format!("Unsupported deterministic local loop action: {other}"),
                    needed_input: Some(
                        "Use write_file/create_file or command_run/validation actions.".to_string(),
                    ),
                };
            }
        }
    }
    if let Some(validation) = manifest.get("validation").filter(|value| value.is_object()) {
        if permission_for(permissions, "command.run") != PermissionTrit::Allow {
            return DeterministicLocalLoopGate::Blocked {
                failure_code: "permission_denied",
                failure_message: "command.run permission is required for validation.".to_string(),
                needed_input: Some("Grant command.run or remove validation.".to_string()),
            };
        }
        let Some(cmd) = runtime_lane_manifest_command(validation) else {
            return DeterministicLocalLoopGate::Blocked {
                failure_code: "command_required",
                failure_message: "validation requires cmd or command.".to_string(),
                needed_input: Some("Add validation.cmd as a string array or shell command string.".to_string()),
            };
        };
        actions.push(DeterministicLocalAction::CommandRun {
            cwd: workspace_root.clone(),
            cmd,
            timeout_seconds: validation
                .get("timeout_seconds")
                .and_then(Value::as_u64)
                .unwrap_or(30),
            max_output_bytes: validation
                .get("max_output_bytes")
                .and_then(Value::as_u64)
                .unwrap_or(12000),
        });
    }
    let write_count = actions
        .iter()
        .filter(|action| matches!(action, DeterministicLocalAction::WriteFile { .. }))
        .count();
    if write_count == 0 {
        return DeterministicLocalLoopGate::Blocked {
            failure_code: "mutation_action_required",
            failure_message: "Deterministic local loops require at least one write_file action."
                .to_string(),
            needed_input: Some("Add a write_file action with path and content.".to_string()),
        };
    }
    DeterministicLocalLoopGate::Candidate(DeterministicLocalLoopCandidate {
        workspace_root,
        requires_validation: actions
            .iter()
            .any(|action| matches!(action, DeterministicLocalAction::CommandRun { .. })),
        actions,
    })
}

fn runtime_lane_extract_deterministic_manifest(prompt: &str) -> Option<Value> {
    for block in runtime_lane_fenced_blocks(prompt) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&block) {
            if let Some(manifest) = parsed.get("deterministic_local_loop") {
                return Some(manifest.clone());
            }
            if parsed.get("actions").is_some() || parsed.get("files").is_some() {
                return Some(parsed);
            }
        }
    }
    None
}

fn runtime_lane_manifest_command(value: &Value) -> Option<Vec<String>> {
    let command = value.get("cmd").or_else(|| value.get("command"))?;
    if let Some(items) = command.as_array() {
        let out = items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    } else {
        let command = command.as_str()?.trim();
        if command.is_empty() {
            None
        } else {
            Some(vec!["sh".to_string(), "-lc".to_string(), command.to_string()])
        }
    }
}

fn runtime_lane_direct_mutation_surface_enabled(tools: &[String], capability_packs: &[String]) -> bool {
    capability_packs
        .iter()
        .any(|pack| pack.trim().eq_ignore_ascii_case("local-coding-files"))
        || tools.iter().any(|tool| {
            matches!(
                tool.trim().to_ascii_lowercase().as_str(),
                "file_write" | "write_file" | "workspace.write" | "workspace_write"
            )
        })
}

fn runtime_lane_extract_explicit_file_content(prompt: &str) -> Option<String> {
    if let Some(content) = runtime_lane_first_fenced_block(prompt) {
        return Some(content);
    }
    let lower = prompt.to_ascii_lowercase();
    let marker_index = lower
        .find("content:")
        .or_else(|| lower.find("contents:"))
        .or_else(|| lower.find("file text:"))?;
    let marker_end = prompt[marker_index..].find(':')? + marker_index + 1;
    let content = prompt[marker_end..]
        .trim_start_matches(|ch| matches!(ch, ' ' | '\t' | '\r' | '\n'));
    if content.trim().is_empty() {
        None
    } else {
        Some(ensure_trailing_newline(content.to_string()))
    }
}

fn runtime_lane_first_fenced_block(prompt: &str) -> Option<String> {
    let (_, after_open) = prompt.split_once("```")?;
    let (block, _) = after_open.split_once("```")?;
    let block = block.trim_start_matches('\n');
    let mut lines = block.lines();
    let first = lines.next().unwrap_or("");
    let content = if runtime_lane_looks_like_fence_language(first) {
        lines.collect::<Vec<_>>().join("\n")
    } else {
        block.to_string()
    };
    if content.trim().is_empty() {
        None
    } else {
        Some(ensure_trailing_newline(content))
    }
}

fn runtime_lane_fenced_blocks(prompt: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut rest = prompt;
    while let Some(open) = rest.find("```") {
        let after_open = &rest[open + 3..];
        let Some(close) = after_open.find("```") else {
            break;
        };
        let block = after_open[..close].trim_start_matches('\n');
        let mut lines = block.lines();
        let first = lines.next().unwrap_or("");
        let content = if runtime_lane_looks_like_fence_language(first) {
            lines.collect::<Vec<_>>().join("\n")
        } else {
            block.to_string()
        };
        if !content.trim().is_empty() {
            blocks.push(content);
        }
        rest = &after_open[close + 3..];
    }
    blocks
}

fn runtime_lane_looks_like_fence_language(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 24
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '+' | '#'))
}

fn ensure_trailing_newline(mut content: String) -> String {
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content
}

fn runtime_lane_extract_workspace_root(prompt: &str) -> Option<PathBuf> {
    for line in prompt.lines() {
        let lower = line.to_ascii_lowercase();
        if !(lower.contains("project root")
            || lower.contains("workspace root")
            || lower.contains("repo root"))
        {
            continue;
        }
        if let Some((_, value)) = line.split_once(':') {
            if let Some(path) = runtime_lane_first_absolute_path_token(value) {
                return Some(path);
            }
        }
        if let Some(path) = runtime_lane_first_absolute_path_token(line) {
            return Some(path);
        }
    }
    None
}

fn runtime_lane_first_absolute_path_token(text: &str) -> Option<PathBuf> {
    text.split_whitespace()
        .map(runtime_lane_clean_path_token)
        .filter(|token| token.starts_with('/'))
        .map(PathBuf::from)
        .next()
}

fn runtime_lane_extract_target_file_path(prompt: &str) -> Option<String> {
    let without_fences = runtime_lane_strip_fenced_blocks(prompt);
    for span in runtime_lane_inline_code_spans(&without_fences) {
        if runtime_lane_is_file_like_path(&span) {
            return Some(span);
        }
    }
    for line in without_fences.lines() {
        let lower = line.to_ascii_lowercase();
        if !(lower.contains("target")
            || lower.contains("file")
            || lower.contains("path")
            || lower.contains("create")
            || lower.contains("write"))
        {
            continue;
        }
        for token in line.split_whitespace().map(runtime_lane_clean_path_token) {
            if runtime_lane_is_file_like_path(&token) {
                return Some(token);
            }
        }
    }
    None
}

fn runtime_lane_strip_fenced_blocks(prompt: &str) -> String {
    let mut out = String::new();
    let mut rest = prompt;
    loop {
        let Some(open) = rest.find("```") else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 3..];
        let Some(close) = after_open.find("```") else {
            break;
        };
        rest = &after_open[close + 3..];
    }
    out
}

fn runtime_lane_inline_code_spans(text: &str) -> Vec<String> {
    let mut spans = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        let span = runtime_lane_clean_path_token(&after_start[..end]);
        if !span.is_empty() {
            spans.push(span);
        }
        rest = &after_start[end + 1..];
    }
    spans
}

fn runtime_lane_clean_path_token(token: &str) -> String {
    token
        .trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches(',')
        .trim_matches(';')
        .trim_matches(':')
        .trim_matches(')')
        .trim_matches('(')
        .trim_matches(']')
        .trim_matches('[')
        .to_string()
}

fn runtime_lane_is_file_like_path(value: &str) -> bool {
    if value.is_empty() || value.contains('\n') || value.ends_with('/') {
        return false;
    }
    if value.starts_with("http://") || value.starts_with("https://") {
        return false;
    }
    let path = Path::new(value);
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return true;
    }
    if path.extension().is_some() {
        return true;
    }
    value.starts_with("./") || value.starts_with('/') && value.rsplit('/').next().unwrap_or("").contains('.')
}

fn runtime_lane_resolve_target_path(
    raw_target: &str,
    workspace_root: Option<&PathBuf>,
) -> Result<PathBuf, String> {
    let raw_path = PathBuf::from(raw_target);
    if raw_path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("target_path_must_not_contain_parent_segments".to_string());
    }
    let target = if raw_path.is_absolute() {
        raw_path
    } else {
        let Some(root) = workspace_root else {
            return Err("relative_target_requires_workspace_root".to_string());
        };
        root.join(raw_path)
    };
    if let Some(root) = workspace_root {
        if !target.starts_with(root) {
            return Err("target_path_outside_workspace_root".to_string());
        }
    }
    Ok(target)
}

fn runtime_lane_prompt_requests_overwrite(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    lower.contains("overwrite")
        || lower.contains("replace the file")
        || lower.contains("replace file")
        || lower.contains("update the file")
}

fn runtime_lane_attach_agent_run_journal(response: &mut RuntimeLaneResponse, run: &AgentRunResult) {
    let native_tool_receipts = run
        .receipt
        .get("native_tool_receipts")
        .cloned()
        .unwrap_or(Value::Null);
    if let Some(object) = response.receipt.as_object_mut() {
        object.insert("agent_run_receipt".to_string(), run.receipt.clone());
        object.insert("native_tool_receipts".to_string(), native_tool_receipts);
        object.insert(
            "provider_output".to_string(),
            Value::String(run.response.output.clone()),
        );
        object.insert("provider_raw".to_string(), run.response.raw.clone());
    }
    if response.output.is_empty() {
        response.output = run.response.output.clone();
    }
}

fn permission_trit_code(value: PermissionTrit) -> i8 {
    match value {
        PermissionTrit::Deny => -1,
        PermissionTrit::Ask => 0,
        PermissionTrit::Allow => 1,
    }
}

fn file_tool_permission(tool: &str) -> Option<&'static str> {
    match tool.trim().to_ascii_lowercase().as_str() {
        "file_list" | "list_files" | "workspace.list" | "workspace_list" | "file_stat"
        | "stat_file" | "file_exists" | "workspace.stat" | "workspace_stat" => {
            Some("file.read")
        }
        "file_read" | "file_read_many" | "read_file" | "read_many_files" | "workspace.read"
        | "workspace.read_many" | "workspace_read" | "workspace_read_many" => Some("file.read"),
        "file_write" | "write_file" | "workspace.write" | "workspace_write" => {
            Some("file.write")
        }
        "file_patch" | "patch_file" | "apply_patch" | "workspace.patch" | "workspace_patch" => {
            Some("file.patch")
        }
        "command_run" | "run_command" | "command.run" | "shell.run" | "shell_run" => {
            Some("command.run")
        }
        _ => None,
    }
}

fn native_success_contract_violation(
    metadata: &Value,
    run_receipt: &Value,
    output: &str,
) -> Option<(String, Value)> {
    let criteria = metadata.get("native_success_criteria")?;
    if !criteria.is_object() {
        return None;
    }
    let requires_native_tool_use = criteria
        .get("requires_native_tool_use")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let requires_successful_mutation_receipt = criteria
        .get("requires_successful_mutation_receipt")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let requires_successful_discovery_receipt = criteria
        .get("requires_successful_discovery_receipt")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let min_successful_tool_receipts = criteria
        .get("min_successful_tool_receipts")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let min_successful_discovery_receipts = criteria
        .get("min_successful_discovery_receipts")
        .and_then(Value::as_u64)
        .unwrap_or(if requires_successful_discovery_receipt {
            1
        } else {
            0
        });
    let successful_discovery_tools = criteria
        .get("successful_discovery_tools")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(normalize_native_tool_name)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| vec!["file_list".to_string(), "file_stat".to_string()]);
    let successful_mutation_tools = criteria
        .get("successful_mutation_tools")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(normalize_native_tool_name)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| vec!["file_write".to_string(), "file_patch".to_string()]);

    let receipts = run_receipt
        .get("native_tool_receipts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let receipt_count = run_receipt
        .get("native_tool_call_count")
        .and_then(Value::as_u64)
        .unwrap_or(receipts.len() as u64);
    let successful_tool_receipt_count = receipts
        .iter()
        .filter(|receipt| receipt.get("status").and_then(Value::as_str) == Some("ok"))
        .count() as u64;
    let successful_discovery_receipt_count = receipts
        .iter()
        .filter(|receipt| receipt.get("status").and_then(Value::as_str) == Some("ok"))
        .filter(|receipt| {
            receipt
                .get("tool_name")
                .and_then(Value::as_str)
                .map(normalize_native_tool_name)
                .map(|tool| successful_discovery_tools.iter().any(|allowed| allowed == &tool))
                .unwrap_or(false)
        })
        .count() as u64;
    let successful_mutation_receipt_count = receipts
        .iter()
        .filter(|receipt| receipt.get("status").and_then(Value::as_str) == Some("ok"))
        .filter(|receipt| {
            receipt
                .get("tool_name")
                .and_then(Value::as_str)
                .map(normalize_native_tool_name)
                .map(|tool| successful_mutation_tools.iter().any(|allowed| allowed == &tool))
                .unwrap_or(false)
        })
        .count() as u64;

    let details = || {
        json!({
            "criteria": criteria,
            "native_tool_call_count": receipt_count,
            "successful_tool_receipt_count": successful_tool_receipt_count,
            "successful_discovery_receipt_count": successful_discovery_receipt_count,
            "successful_mutation_receipt_count": successful_mutation_receipt_count,
            "native_tool_receipt_summary": native_tool_receipt_summary(&receipts),
            "agent_output_preview": output.chars().take(1200).collect::<String>(),
            "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
            "enforcement_mode": "strict_fail_closed",
        })
    };

    if requires_native_tool_use && receipt_count == 0 {
        return Some((
            "runtime_lane_required_native_tool_use_missing".to_string(),
            details(),
        ));
    }
    if min_successful_discovery_receipts > 0
        && successful_discovery_receipt_count < min_successful_discovery_receipts
    {
        return Some((
            "runtime_lane_required_native_discovery_receipt_missing".to_string(),
            details(),
        ));
    }
    if min_successful_tool_receipts > 0 && successful_tool_receipt_count < min_successful_tool_receipts
    {
        return Some((
            "runtime_lane_required_native_tool_receipt_missing".to_string(),
            details(),
        ));
    }
    if requires_successful_mutation_receipt && successful_mutation_receipt_count == 0 {
        return Some((
            "runtime_lane_required_native_mutation_receipt_missing".to_string(),
            details(),
        ));
    }
    None
}

fn native_tool_receipt_summary(receipts: &[Value]) -> Vec<Value> {
    receipts
        .iter()
        .map(|receipt| {
            json!({
                "call_id": receipt.get("call_id").and_then(Value::as_str).unwrap_or(""),
                "tool_name": receipt.get("tool_name").and_then(Value::as_str).unwrap_or(""),
                "status": receipt.get("status").and_then(Value::as_str).unwrap_or(""),
                "error": receipt.get("error").cloned().unwrap_or(Value::Null),
                "path": receipt
                    .get("result")
                    .and_then(|result| result.get("path"))
                    .cloned()
                    .unwrap_or(Value::Null),
            })
        })
        .collect()
}

fn normalize_native_tool_name(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "list_files" | "workspace.list" | "workspace_list" => "file_list".to_string(),
        "stat_file" | "file_exists" | "workspace.stat" | "workspace_stat" => {
            "file_stat".to_string()
        }
        "write_file" | "workspace.write" | "workspace_write" => "file_write".to_string(),
        "patch_file" | "apply_patch" | "workspace.patch" | "workspace_patch" => {
            "file_patch".to_string()
        }
        "command_run" | "run_command" | "command.run" | "shell.run" | "shell_run" => {
            "command_run".to_string()
        }
        "read_file" | "workspace.read" | "workspace_read" => "file_read".to_string(),
        "read_many_files" | "workspace.read_many" | "workspace_read_many" => {
            "file_read_many".to_string()
        }
        other => other.to_string(),
    }
}

fn public_reasoning_contract_violation(
    metadata: &Value,
    run_receipt: &Value,
    output: &str,
) -> Option<(String, Value)> {
    let contract = metadata.get("public_reasoning_trace_contract")?;
    if !contract.is_object() {
        return None;
    }
    let agent_status = run_receipt
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("ok");
    if agent_status != "ok" {
        return None;
    }

    let emitted = contract
        .get("emits")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let requires_trace = emitted
        .iter()
        .any(|item| item == "public_reasoning_trace_v1")
        || contract
            .get("local_trace_required_fields")
            .and_then(Value::as_array)
            .is_some();
    let requires_rollup = emitted
        .iter()
        .any(|item| item == "public_reasoning_rollup_v1");
    let has_trace =
        output.contains("public_reasoning_trace") && output.contains("public_reasoning_trace_v1");
    let has_rollup =
        output.contains("reasoning_rollup") && output.contains("public_reasoning_rollup_v1");
    let still_requests_tools = output.contains("\"tool_calls\"") || output.contains("{\"tool_calls\"");
    let redaction_policy = contract
        .get("redaction_policy")
        .and_then(Value::as_str)
        .unwrap_or("no_hidden_chain_of_thought");
    let mentions_redaction = output.contains(redaction_policy)
        || output.contains("hidden chain-of-thought")
        || output.contains("hidden chain of thought")
        || output.contains("redaction");

    if still_requests_tools
        || (requires_trace && !has_trace)
        || (requires_rollup && !has_rollup)
        || !mentions_redaction
    {
        return Some((
            "runtime_lane_public_reasoning_trace_missing".to_string(),
            json!({
                "criteria": {
                    "requires_public_reasoning_trace": requires_trace,
                    "requires_reasoning_rollup": requires_rollup,
                    "requires_redaction_policy_ack": true,
                    "redaction_policy": redaction_policy,
                },
                "observed": {
                    "has_public_reasoning_trace": has_trace,
                    "has_reasoning_rollup": has_rollup,
                    "mentions_redaction_policy": mentions_redaction,
                    "still_requests_tools": still_requests_tools,
                },
                "agent_status": agent_status,
                "agent_output_preview": output.chars().take(1200).collect::<String>(),
                "workflow": metadata.get("workflow").cloned().unwrap_or(Value::Null),
                "enforcement_mode": "strict_fail_closed",
            }),
        ));
    }
    None
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
    tools.iter().any(|tool| {
        matches!(
            tool.as_str(),
            "web.search" | "web.fetch" | "network.request"
        )
    })
}
