// Layer ownership: Core Layer 2 (Scheduling + Execution) - agent runtime surface coordination.
use crate::capability_pack::CapabilityPackCatalog;
use crate::native_evidence::{
    native_tool_artifact_contract_enabled, native_tool_artifact_repair_reasons,
    native_tool_changed_paths, native_tool_context_only_turn,
    native_tool_failed_validation_receipt_details, native_tool_has_successful_mutation,
    native_tool_has_successful_validation_command, native_tool_needs_artifact_finalization,
    native_tool_needs_public_report_finalization, native_tool_prompt_evidence_gaps,
    native_tool_prompt_has_multiple_requirements, native_tool_prompt_project_root,
    native_tool_prompt_required_changed_paths, native_tool_prompt_requires_product_mutation,
    native_tool_prompt_requires_validation_command, native_tool_should_synthesize_micro_final,
    native_tool_unique_code_path_mentions,
};
use crate::native_synthetic_artifact::{
    native_tool_synthetic_completion_evidence_response,
    native_tool_synthetic_micro_final_response,
};
use crate::native_workflow_artifact::{
    native_tool_auto_workflow_artifact_receipts,
};
use crate::native_prompt_policy::{
    native_tool_completion_repair_action_brief,
    native_tool_completion_evidence_repair_prompt, native_tool_context_to_mutation_retry_prompt,
    native_tool_empty_retry_prompt, native_tool_initial_prompt,
    native_tool_orchestration_prompt_text, native_tool_public_reasoning_finalization_prompt,
    native_tool_public_reasoning_metadata, native_tool_recovery_prompt,
};
use crate::native_tools::{
    native_tool_observation_prompt, parse_native_tool_calls, NativeToolCall, NativeToolDispatcher,
    NativeToolReceipt,
};
use crate::provider::{
    ProviderClientRegistry, ProviderError, ProviderErrorCode, ProviderRequest, ProviderResponse,
};
use crate::scheduler::SchedulePlan;
use crate::telemetry::{ReceiptEvent, ReceiptSpan};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentContract {
    pub name: String,
    pub preamble: String,
    pub initial_prompt: String,
    pub lifespan_seconds: u64,
    pub provider: String,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub capability_packs: Vec<String>,
    pub schedule: Option<SchedulePlan>,
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentRunResult {
    pub response: ProviderResponse,
    pub receipt: Value,
    pub trace: ReceiptSpan,
}

#[derive(Clone)]
pub struct AgentExecutionContext<'a> {
    pub provider_registry: &'a ProviderClientRegistry,
    pub capability_catalog: Option<&'a CapabilityPackCatalog>,
}

impl<'a> AgentExecutionContext<'a> {
    pub fn new(
        provider_registry: &'a ProviderClientRegistry,
        capability_catalog: Option<&'a CapabilityPackCatalog>,
    ) -> Self {
        Self {
            provider_registry,
            capability_catalog,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentBuildError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for AgentBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.code, self.message)
    }
}

impl std::error::Error for AgentBuildError {}

pub struct AgentBuilder {
    contract: AgentContract,
}

impl AgentBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            contract: AgentContract {
                name: sanitize_token(&name.into(), 80),
                preamble: String::new(),
                initial_prompt: String::new(),
                lifespan_seconds: 3600,
                provider: "local-echo".to_string(),
                model: None,
                tools: Vec::new(),
                capability_packs: Vec::new(),
                schedule: None,
                metadata: Value::Object(Map::new()),
            },
        }
    }

    pub fn preamble(mut self, preamble: impl Into<String>) -> Self {
        self.contract.preamble = sanitize_token(&preamble.into(), 2000);
        self
    }

    pub fn initial_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.contract.initial_prompt = sanitize_token(&prompt.into(), 64_000);
        self
    }

    pub fn lifespan_seconds(mut self, lifespan_seconds: u64) -> Self {
        self.contract.lifespan_seconds = lifespan_seconds;
        self
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.contract.provider = sanitize_token(&provider.into(), 120);
        self
    }

    pub fn provider_from_env(
        mut self,
        env: &std::collections::HashMap<String, String>,
        key: &str,
    ) -> Self {
        if let Some(provider) = env
            .get(key)
            .or_else(|| env.get("INFRING_PROVIDER"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            self.contract.provider = sanitize_token(&provider, 120);
        }
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.contract.model = Some(sanitize_token(&model.into(), 120));
        self
    }

    pub fn tool(mut self, tool: impl Into<String>) -> Self {
        let token = sanitize_token(&tool.into(), 120);
        if !token.is_empty() && !self.contract.tools.iter().any(|entry| entry == &token) {
            self.contract.tools.push(token);
        }
        self
    }

    pub fn capability_pack(mut self, pack: impl Into<String>) -> Self {
        let token = sanitize_token(&pack.into(), 120);
        if !token.is_empty()
            && !self
                .contract
                .capability_packs
                .iter()
                .any(|entry| entry == &token)
        {
            self.contract.capability_packs.push(token);
        }
        self
    }

    pub fn schedule(mut self, plan: SchedulePlan) -> Self {
        self.contract.schedule = Some(plan);
        self
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.contract.metadata = metadata;
        self
    }

    pub fn build(self) -> Result<AgentContract, AgentBuildError> {
        if self.contract.name.trim().is_empty() {
            return Err(AgentBuildError {
                code: "agent_name_required".to_string(),
                message: "Agent name is required.".to_string(),
            });
        }
        if self.contract.initial_prompt.trim().is_empty() {
            return Err(AgentBuildError {
                code: "agent_initial_prompt_required".to_string(),
                message: "Agent contract needs an initial prompt.".to_string(),
            });
        }
        if self.contract.lifespan_seconds == 0 || self.contract.lifespan_seconds > 31_536_000 {
            return Err(AgentBuildError {
                code: "agent_lifespan_invalid".to_string(),
                message: "Lifespan must be between 1 second and 1 year.".to_string(),
            });
        }
        Ok(self.contract)
    }
}

impl AgentContract {
    pub fn resolved_tools(&self, catalog: Option<&CapabilityPackCatalog>) -> Vec<String> {
        if let Some(catalog) = catalog {
            return catalog.expand_tools(&self.capability_packs, &self.tools);
        }
        self.tools.clone()
    }

    pub fn with_default_schedule_from_packs(mut self, catalog: &CapabilityPackCatalog) -> Self {
        if self.schedule.is_some() {
            return self;
        }
        let mut chosen = None;
        for pack_id in &self.capability_packs {
            if let Some(interval) = catalog.default_interval_for_pack(pack_id) {
                chosen = Some(
                    chosen
                        .map(|current: u64| current.min(interval))
                        .unwrap_or(interval),
                );
            }
        }
        if let Some(interval) = chosen {
            let max_runs = catalog.default_max_runs_for_packs(&self.capability_packs);
            self.schedule = Some(SchedulePlan {
                interval_seconds: interval,
                jitter_seconds: 15,
                max_runs,
            });
        }
        self
    }

    pub fn run_once(
        &self,
        context: &AgentExecutionContext<'_>,
    ) -> Result<AgentRunResult, ProviderError> {
        let provider = context
            .provider_registry
            .from_provider_id(self.provider.as_str())?;
        let tools = self.resolved_tools(context.capability_catalog);
        let started_ms = Utc::now().timestamp_millis();
        let (response, tool_receipts, provider_call_count, terminal_status) =
            self.run_with_optional_native_tools(provider, &tools)?;
        let finished_ms = Utc::now().timestamp_millis();
        let duration_ms = (finished_ms - started_ms).max(0) as u64;
        let mut events = vec![ReceiptEvent {
            event_id: "provider.complete".to_string(),
            status: terminal_status.clone(),
            duration_ms,
            error_code: None,
            timestamp_ms: finished_ms,
            attributes: BTreeMap::from([
                ("provider".to_string(), response.provider.clone()),
                ("model".to_string(), response.model.clone()),
                (
                    "provider_call_count".to_string(),
                    provider_call_count.to_string(),
                ),
            ]),
        }];
        for receipt in &tool_receipts {
            events.push(ReceiptEvent {
                event_id: format!("native_tool.{}", receipt.tool_name),
                status: receipt.status.clone(),
                duration_ms: receipt.duration_ms,
                error_code: receipt.error.clone(),
                timestamp_ms: finished_ms,
                attributes: BTreeMap::from([
                    ("tool_name".to_string(), receipt.tool_name.clone()),
                    ("call_id".to_string(), receipt.call_id.clone()),
                ]),
            });
        }
        let trace = ReceiptSpan {
            trace_id: format!("trace-{}-{}", self.name, finished_ms),
            agent_name: self.name.clone(),
            started_at_ms: started_ms,
            events,
            attributes: BTreeMap::from([("tools".to_string(), tools.join(","))]),
        };
        let receipt = json!({
            "type": "agent_run_receipt",
            "agent": self.name,
            "provider": response.provider,
            "model": response.model,
            "status": terminal_status,
            "tool_count": tools.len(),
            "native_tool_call_count": tool_receipts.len(),
            "lifespan_seconds": self.lifespan_seconds,
            "duration_ms": duration_ms,
            "trace_id": trace.trace_id,
            "workflow": self
                .metadata
                .get("workflow")
                .cloned()
                .unwrap_or(Value::Null),
            "native_tool_receipts": tool_receipts,
        });
        Ok(AgentRunResult {
            response,
            receipt,
            trace,
        })
    }

    fn run_with_optional_native_tools(
        &self,
        provider: Arc<dyn crate::provider::ProviderClient>,
        tools: &[String],
    ) -> Result<(ProviderResponse, Vec<NativeToolReceipt>, u64, String), ProviderError> {
        let dispatcher = NativeToolDispatcher::new(tools);
        if !dispatcher.has_native_tools() {
            let request = ProviderRequest {
                prompt: self.initial_prompt.clone(),
                system: Some(self.preamble.clone()),
                tools: tools.to_vec(),
                model: self.model.clone(),
                metadata: self.metadata.clone(),
            };
            return provider
                .complete(&request)
                .map(|response| (response, Vec::new(), 1, "ok".to_string()));
        }

        let max_turns = self
            .metadata
            .get("native_tool_max_turns")
            .and_then(Value::as_u64)
            .unwrap_or(8)
            .clamp(1, 16);
        let mut prompt = native_tool_initial_prompt(&self.initial_prompt, &self.metadata);
        let system = if self.preamble.trim().is_empty() {
            dispatcher.tool_protocol_prompt()
        } else {
            format!("{}\n\n{}", self.preamble, dispatcher.tool_protocol_prompt())
        };
        let mut all_receipts = Vec::<NativeToolReceipt>::new();
        let mut last_response = None;
        let mut provider_call_count = 0u64;
        let empty_tool_retry_limit = native_tool_empty_retry_limit(&self.metadata);
        let mut empty_tool_retry_count = 0u64;
        let mut context_only_turn_count = 0u64;
        let loop_started = Instant::now();
        let wall_timeout = native_tool_wall_timeout(&self.metadata);
        if native_tool_bootstrap_context_before_first_provider(&self.metadata)
            && native_tool_requires_successful_mutation(&self.metadata)
            && native_tool_prompt_has_multiple_requirements(&self.initial_prompt)
        {
            let bootstrap_receipts =
                native_tool_bootstrap_context_receipts(&dispatcher, &self.initial_prompt);
            if !bootstrap_receipts.is_empty() {
                let observation = native_tool_observation_prompt(&bootstrap_receipts);
                all_receipts.extend(bootstrap_receipts);
                let bootstrap_rule = native_tool_orchestration_prompt_text(
                    &self.metadata,
                    "bootstrap_context_continuation_rule",
                    "Runtime bootstrap context was collected before the first model call. Continue from this already-read context and return only JSON tool calls next.",
                );
                prompt = format!(
                    "{}\n\n{}\n\nNative tool observations:\n{}",
                    self.initial_prompt,
                    bootstrap_rule,
                    observation
                );
            }
        }

        for turn_idx in 0..max_turns {
            if let Some(timeout) = wall_timeout {
                if loop_started.elapsed() >= timeout {
                    if native_tool_has_successful_mutation(&all_receipts)
                        && native_tool_partial_progress_on_timeout(&self.metadata)
                    {
                        return native_tool_recovery_or_partial_progress(
                            &provider,
                            &dispatcher,
                            tools,
                            self.model.clone(),
                            &self.metadata,
                            &self.initial_prompt,
                            &system,
                            "native_tool_loop_wall_timeout",
                            provider_call_count,
                            all_receipts,
                        );
                    }
                    return Err(ProviderError::new(
                        ProviderErrorCode::Timeout,
                        format!(
                            "native_tool_loop_wall_timeout:timeout_seconds={}",
                            timeout.as_secs()
                        ),
                    ));
                }
            }
            provider_call_count += 1;
            let request = ProviderRequest {
                prompt: prompt.clone(),
                system: Some(system.clone()),
                tools: tools.to_vec(),
                model: self.model.clone(),
                metadata: self.metadata.clone(),
            };
            let response = match provider.complete(&request) {
                Ok(response) => response,
                Err(error)
                    if native_tool_provider_error_is_timeout(&error)
                        && native_tool_has_successful_mutation(&all_receipts)
                        && native_tool_partial_progress_on_timeout(&self.metadata) =>
                {
                    return native_tool_recovery_or_partial_progress(
                        &provider,
                        &dispatcher,
                        tools,
                        self.model.clone(),
                        &self.metadata,
                        &self.initial_prompt,
                        &system,
                        error.message.as_str(),
                        provider_call_count,
                        all_receipts,
                    );
                }
                Err(error)
                    if native_tool_provider_error_is_timeout(&error)
                        && all_receipts.is_empty()
                        && native_tool_requires_successful_mutation(&self.metadata)
                        && native_tool_prompt_has_multiple_requirements(&self.initial_prompt) =>
                {
                    if let Some(bootstrap_receipt) =
                        native_tool_bootstrap_discovery_receipt(&dispatcher, &self.initial_prompt)
                    {
                        let observation =
                            native_tool_observation_prompt(&[bootstrap_receipt.clone()]);
                        all_receipts.push(bootstrap_receipt);
                        let bootstrap_rule = native_tool_orchestration_prompt_text(
                            &self.metadata,
                            "bootstrap_timeout_discovery_rule",
                            "Runtime bootstrap discovery was performed after an initial timeout. Continue from these observations and return only JSON tool calls next.",
                        );
                        prompt = format!(
                            "{}\n\n{}\n\nNative tool observations:\n{}",
                            self.initial_prompt,
                            bootstrap_rule,
                            observation
                        );
                        continue;
                    }
                    return Err(error);
                }
                Err(error) => return Err(error),
            };
            let calls = parse_native_tool_calls(&response.output);
            if calls.is_empty() {
                if all_receipts.is_empty() && empty_tool_retry_count < empty_tool_retry_limit {
                    empty_tool_retry_count += 1;
                    prompt = native_tool_empty_retry_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        &response.output,
                        empty_tool_retry_count,
                    );
                    last_response = Some(response);
                    continue;
                }
                if all_receipts.is_empty()
                    && native_tool_requires_successful_mutation(&self.metadata)
                    && native_tool_prompt_has_multiple_requirements(&self.initial_prompt)
                {
                    if let Some(bootstrap_receipt) =
                        native_tool_bootstrap_discovery_receipt(&dispatcher, &self.initial_prompt)
                    {
                        let observation =
                            native_tool_observation_prompt(&[bootstrap_receipt.clone()]);
                        all_receipts.push(bootstrap_receipt);
                        let bootstrap_rule = native_tool_orchestration_prompt_text(
                            &self.metadata,
                            "bootstrap_no_tool_discovery_rule",
                            "Runtime bootstrap discovery was performed because previous responses did not call native tools. Continue from these observations and return only JSON tool calls next.",
                        );
                        prompt = format!(
                            "{}\n\n{}\n\nNative tool observations:\n{}",
                            self.initial_prompt,
                            bootstrap_rule,
                            observation
                        );
                        last_response = Some(response);
                        continue;
                    }
                }
                if native_tool_requires_successful_mutation(&self.metadata)
                    && !native_tool_has_successful_mutation(&all_receipts)
                    && !all_receipts.is_empty()
                    && empty_tool_retry_count < empty_tool_retry_limit
                {
                    empty_tool_retry_count += 1;
                    let observation = native_tool_observation_prompt(&all_receipts);
                    prompt = native_tool_context_to_mutation_retry_prompt(
                        &self.metadata,
                        &self.initial_prompt,
                        &response.output,
                        &observation,
                        empty_tool_retry_count,
                    );
                    last_response = Some(response);
                    continue;
                }
                last_response = Some(response);
                break;
            }
            let mut turn_receipts = Vec::new();
            for call in calls
                .into_iter()
                .take(native_tool_max_calls_per_turn(&self.metadata))
            {
                let call = native_tool_call_with_prompt_defaults(call, &self.initial_prompt);
                let context_blocked = native_tool_requires_successful_mutation(&self.metadata)
                    && !native_tool_has_successful_mutation(&all_receipts)
                    && context_only_turn_count
                        >= native_tool_max_context_only_turns(&self.metadata)
                    && native_tool_has_successful_context_receipt(&all_receipts)
                    && native_tool_call_is_context_only(&call);
                let receipt = if context_blocked {
                    native_tool_product_repair_context_blocked_receipt(call)
                } else {
                    dispatcher.dispatch(call)
                };
                turn_receipts.push(receipt.clone());
                all_receipts.push(receipt);
            }
            if native_tool_should_synthesize_micro_final(
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
            ) {
                let mut response =
                    native_tool_synthetic_micro_final_response(
                    &response,
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                );
                response.raw = json!({
                    "provider_raw": response.raw,
                    "native_tool_loop": {
                        "enabled": true,
                        "provider_call_count": provider_call_count,
                        "tool_call_count": all_receipts.len(),
                        "empty_tool_retry_count": empty_tool_retry_count,
                        "tool_receipts": all_receipts.clone(),
                        "terminal_status": "ok",
                        "synthetic_micro_final": true,
                    }
                });
                return Ok((response, all_receipts, provider_call_count, "ok".to_string()));
            }
            if native_tool_requires_successful_mutation(&self.metadata)
                && !native_tool_has_successful_mutation(&all_receipts)
                && native_tool_context_only_turn(&turn_receipts)
            {
                context_only_turn_count += 1;
            } else if native_tool_has_successful_mutation(&turn_receipts) {
                context_only_turn_count = 0;
            }
            let observation = native_tool_observation_prompt(&turn_receipts);
            if native_tool_requires_successful_mutation(&self.metadata)
                && !native_tool_has_successful_mutation(&all_receipts)
                && context_only_turn_count >= native_tool_max_context_only_turns(&self.metadata)
            {
                prompt = native_tool_context_to_mutation_retry_prompt(
                    &self.metadata,
                    &self.initial_prompt,
                    &response.output,
                    &observation,
                    context_only_turn_count,
                );
                last_response = Some(response);
                continue;
            }
            prompt = format!(
                "{}\n\nAssistant tool request turn {}:\n{}\n\nNative tool observations:\n{}\n\n{}",
                self.initial_prompt,
                turn_idx + 1,
                response.output,
                observation,
                "Continue."
            );
            last_response = Some(response);
        }

        let mut response = last_response.ok_or_else(|| {
            ProviderError::new(
                crate::provider::ProviderErrorCode::Unavailable,
                "native_tool_loop_no_provider_response",
            )
        })?;
        let pending_terminal_calls = parse_native_tool_calls(&response.output);
        if !pending_terminal_calls.is_empty()
            && native_tool_requires_successful_mutation(&self.metadata)
            && !native_tool_has_successful_mutation(&all_receipts)
        {
            let existing_call_ids = all_receipts
                .iter()
                .map(|receipt| receipt.call_id.clone())
                .collect::<std::collections::BTreeSet<_>>();
            let mut terminal_receipts = Vec::new();
            for call in pending_terminal_calls
                .into_iter()
                .take(native_tool_max_calls_per_turn(&self.metadata))
            {
                if existing_call_ids.contains(&call.id) {
                    continue;
                }
                let receipt = dispatcher.dispatch(native_tool_call_with_prompt_defaults(
                    call,
                    &self.initial_prompt,
                ));
                terminal_receipts.push(receipt.clone());
                all_receipts.push(receipt);
            }
            if native_tool_should_synthesize_micro_final(
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
            ) {
                let mut response = native_tool_synthetic_micro_final_response(
                    &response,
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                );
                response.raw = json!({
                    "provider_raw": response.raw,
                    "native_tool_loop": {
                        "enabled": true,
                        "provider_call_count": provider_call_count,
                        "tool_call_count": all_receipts.len(),
                        "empty_tool_retry_count": empty_tool_retry_count,
                        "tool_receipts": all_receipts.clone(),
                        "terminal_status": "ok",
                        "synthetic_micro_final": true,
                        "executed_pending_terminal_tool_calls": terminal_receipts.len(),
                    }
                });
                return Ok((response, all_receipts, provider_call_count, "ok".to_string()));
            }
        }
        if let Some(validation_receipt) =
            native_tool_auto_validation_receipt(&dispatcher, &self.initial_prompt, &all_receipts)
        {
            all_receipts.push(validation_receipt);
        }
        let auto_handoff_receipts = native_tool_auto_workflow_artifact_receipts(
            &dispatcher,
            &self.metadata,
            &self.initial_prompt,
            &all_receipts,
        );
        if !auto_handoff_receipts.is_empty() {
            all_receipts.extend(auto_handoff_receipts);
        }
        let initial_repair_reasons = native_tool_artifact_repair_reasons(
            &self.metadata,
            &self.initial_prompt,
            &response.output,
            &all_receipts,
        );
        if !initial_repair_reasons.is_empty()
            && native_tool_completion_evidence_repair_enabled(&self.metadata)
        {
            let repaired = native_tool_completion_evidence_repair_loop(
                &provider,
                &dispatcher,
                tools,
                self.model.clone(),
                &self.metadata,
                &self.initial_prompt,
                &system,
                response,
                all_receipts,
                provider_call_count,
                initial_repair_reasons,
            )?;
            response = repaired.0;
            all_receipts = repaired.1;
            provider_call_count = repaired.2;
        }
        let terminal_output_has_tool_calls = !parse_native_tool_calls(&response.output).is_empty();
        let completion_evidence_finalization = native_tool_needs_artifact_finalization(
            &self.metadata,
            &self.initial_prompt,
            &response.output,
            &all_receipts,
        );
        if (terminal_output_has_tool_calls
            || native_tool_needs_public_report_finalization(&self.metadata, &response.output)
            || completion_evidence_finalization)
            && native_tool_synthesize_final_after_successful_validation(&self.metadata)
            && native_tool_has_successful_mutation(&all_receipts)
            && native_tool_has_successful_validation_command(&all_receipts)
            && native_tool_prompt_evidence_gaps(&self.initial_prompt, &all_receipts).is_empty()
        {
            response = native_tool_synthetic_completion_evidence_response(
                &response,
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
                "successful_validation_receipt_runtime_synthesized_final",
            );
        } else if terminal_output_has_tool_calls
            || native_tool_needs_public_report_finalization(&self.metadata, &response.output)
            || completion_evidence_finalization
        {
            provider_call_count += 1;
                let mut finalization_prompt = native_tool_public_reasoning_finalization_prompt(
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                    &response.output,
            );
            if completion_evidence_finalization {
                finalization_prompt.push_str("\n\n");
                finalization_prompt.push_str(
                    &native_tool_orchestration_prompt_text(
                        &self.metadata,
                        "coverage_finalization_guard_rule",
                        "Add workflow-required coverage status for the original task. Mark uncovered or blocked requirements accurately instead of reporting success without receipt-backed evidence.",
                    ),
                );
            }
            if terminal_output_has_tool_calls {
                finalization_prompt.push_str("\n\n");
                finalization_prompt.push_str(
                    &native_tool_orchestration_prompt_text(
                        &self.metadata,
                        "terminal_tool_call_finalization_guard_rule",
                        "The previous assistant response still contained native tool calls, so tools are disabled for this pass. Return only a final receipt-backed user response and do not output tool calls.",
                    ),
                );
            }
            let request = ProviderRequest {
                prompt: finalization_prompt,
                system: Some(system.clone()),
                tools: Vec::new(),
                model: self.model.clone(),
                metadata: native_tool_public_reasoning_metadata(&self.metadata),
            };
            response = match provider.complete(&request) {
                Ok(response) => response,
                Err(error)
                    if error.code == ProviderErrorCode::Timeout
                        && native_tool_has_successful_mutation(&all_receipts)
                        && native_tool_completion_evidence_timeout_synthesis_enabled(
                            &self.metadata,
                        ) =>
                {
                    native_tool_synthetic_completion_evidence_response(
                        &response,
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                        error.message.as_str(),
                    )
                }
                Err(error) => return Err(error),
            };
            if !parse_native_tool_calls(&response.output).is_empty() {
                return Err(ProviderError::new(
                    ProviderErrorCode::InvalidRequest,
                    "native_tool_terminal_tool_calls_after_finalization",
                ));
            }
            if native_tool_needs_artifact_finalization(
                &self.metadata,
                &self.initial_prompt,
                &response.output,
                &all_receipts,
            ) && native_tool_has_successful_mutation(&all_receipts)
            {
                response = native_tool_synthetic_completion_evidence_response(
                    &response,
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                    "missing_task_requirement_checklist_after_finalization",
                );
            }
        }
        let final_repair_reasons = native_tool_artifact_repair_reasons(
            &self.metadata,
            &self.initial_prompt,
            &response.output,
            &all_receipts,
        );
        if !final_repair_reasons.is_empty()
            && native_tool_completion_evidence_repair_enabled(&self.metadata)
        {
            let repaired = native_tool_completion_evidence_repair_loop(
                &provider,
                &dispatcher,
                tools,
                self.model.clone(),
                &self.metadata,
                &self.initial_prompt,
                &system,
                response,
                all_receipts,
                provider_call_count,
                final_repair_reasons,
            )?;
            response = repaired.0;
            all_receipts = repaired.1;
            provider_call_count = repaired.2;
        }
        if let Some(validation_receipt) =
            native_tool_auto_validation_receipt(&dispatcher, &self.initial_prompt, &all_receipts)
        {
            all_receipts.push(validation_receipt);
        }
        if !parse_native_tool_calls(&response.output).is_empty()
            && native_tool_has_successful_mutation(&all_receipts)
            && native_tool_completion_evidence_timeout_synthesis_enabled(&self.metadata)
        {
            response = native_tool_synthetic_completion_evidence_response(
                &response,
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
                "terminal_native_requests_after_evidence_repair",
            );
        }
        let auto_handoff_receipts =
            native_tool_auto_workflow_artifact_receipts(&dispatcher, &self.metadata, &self.initial_prompt, &all_receipts);
        if !auto_handoff_receipts.is_empty() {
            all_receipts.extend(auto_handoff_receipts);
            if native_tool_prompt_evidence_gaps(&self.initial_prompt, &all_receipts).is_empty() {
                response = native_tool_synthetic_completion_evidence_response(
                    &response,
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                    "runtime_synthesized_handoff_artifacts",
                );
            }
        }
        let mut unresolved_final_reasons = native_tool_artifact_repair_reasons(
            &self.metadata,
            &self.initial_prompt,
            &response.output,
            &all_receipts,
        );
        if unresolved_final_reasons
            .iter()
            .any(|reason| reason == "missing_product_mutation_receipt")
            && native_tool_artifact_contract_enabled(&self.metadata)
            && native_tool_requires_successful_mutation(&self.metadata)
            && !native_tool_has_successful_mutation(&all_receipts)
            && empty_tool_retry_count < empty_tool_retry_limit
        {
            provider_call_count += 1;
            let observation = native_tool_observation_prompt(&all_receipts);
            let forced_mutation_prompt = native_tool_context_to_mutation_retry_prompt(
                &self.metadata,
                &self.initial_prompt,
                &response.output,
                &observation,
                empty_tool_retry_count + 1,
            );
            let request = ProviderRequest {
                prompt: forced_mutation_prompt,
                system: Some(system.clone()),
                tools: tools.to_vec(),
                model: self.model.clone(),
                metadata: native_tool_recovery_timeout_metadata(&self.metadata),
            };
            let forced_response = provider.complete(&request)?;
            let forced_calls = parse_native_tool_calls(&forced_response.output);
            response = forced_response;
            if !forced_calls.is_empty() {
                let mut forced_receipts = Vec::new();
                for call in forced_calls
                    .into_iter()
                    .take(native_tool_max_calls_per_turn(&self.metadata))
                {
                    let call = native_tool_call_with_prompt_defaults(call, &self.initial_prompt);
                    let context_blocked =
                        native_tool_has_successful_context_receipt(&all_receipts)
                            && !native_tool_has_successful_mutation(&all_receipts)
                            && native_tool_call_is_context_only(&call);
                    let receipt = if context_blocked {
                        native_tool_product_repair_context_blocked_receipt(call)
                    } else {
                        dispatcher.dispatch(call)
                    };
                    forced_receipts.push(receipt.clone());
                    all_receipts.push(receipt);
                }
                if let Some(validation_receipt) = native_tool_auto_validation_receipt(
                    &dispatcher,
                    &self.initial_prompt,
                    &all_receipts,
                ) {
                    all_receipts.push(validation_receipt);
                }
                let auto_handoff_receipts = native_tool_auto_workflow_artifact_receipts(
                    &dispatcher,
                    &self.metadata,
                    &self.initial_prompt,
                    &all_receipts,
                );
                if !auto_handoff_receipts.is_empty() {
                    all_receipts.extend(auto_handoff_receipts);
                }
                unresolved_final_reasons = native_tool_artifact_repair_reasons(
                    &self.metadata,
                    &self.initial_prompt,
                    &response.output,
                    &all_receipts,
                );
                if !unresolved_final_reasons.is_empty()
                    && native_tool_completion_evidence_repair_enabled(&self.metadata)
                    && native_tool_has_successful_mutation(&all_receipts)
                {
                    let repaired = native_tool_completion_evidence_repair_loop(
                        &provider,
                        &dispatcher,
                        tools,
                        self.model.clone(),
                        &self.metadata,
                        &self.initial_prompt,
                        &system,
                        response,
                        all_receipts,
                        provider_call_count,
                        unresolved_final_reasons,
                    )?;
                    response = repaired.0;
                    all_receipts = repaired.1;
                    provider_call_count = repaired.2;
                    if let Some(validation_receipt) = native_tool_auto_validation_receipt(
                        &dispatcher,
                        &self.initial_prompt,
                        &all_receipts,
                    ) {
                        all_receipts.push(validation_receipt);
                    }
                    let auto_handoff_receipts = native_tool_auto_workflow_artifact_receipts(
                        &dispatcher,
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                    );
                    if !auto_handoff_receipts.is_empty() {
                        all_receipts.extend(auto_handoff_receipts);
                    }
                    unresolved_final_reasons = native_tool_artifact_repair_reasons(
                        &self.metadata,
                        &self.initial_prompt,
                        &response.output,
                        &all_receipts,
                    );
                }
                if unresolved_final_reasons.is_empty() {
                    response = native_tool_synthetic_completion_evidence_response(
                        &response,
                        &self.metadata,
                        &self.initial_prompt,
                        &all_receipts,
                        "forced_product_mutation_retry_completed",
                    );
                }
            }
        }
        if !unresolved_final_reasons.is_empty()
            && native_tool_artifact_contract_enabled(&self.metadata)
        {
            return Err(ProviderError::new(
                ProviderErrorCode::InvalidRequest,
                format!(
                    "native_tool_unresolved_completion_evidence:{};receipt_summary={}",
                    unresolved_final_reasons.join(","),
                    native_tool_receipt_error_summary(&all_receipts)
                ),
            ));
        }
        response.raw = json!({
            "provider_raw": response.raw,
            "native_tool_loop": {
                "enabled": true,
                "provider_call_count": provider_call_count,
                "tool_call_count": all_receipts.len(),
                "empty_tool_retry_count": empty_tool_retry_count,
                "tool_receipts": all_receipts.clone(),
                "terminal_status": "ok",
            }
        });
        Ok((response, all_receipts, provider_call_count, "ok".to_string()))
    }
}

fn native_tool_empty_retry_limit(metadata: &Value) -> u64 {
    let criteria = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"));
    let default = criteria
        .and_then(|value| value.get("requires_native_tool_use"))
        .and_then(Value::as_bool)
        .unwrap_or(false) as u64;
    criteria
        .and_then(|value| value.get("empty_tool_retry_limit"))
        .and_then(Value::as_u64)
        .unwrap_or(default)
        .clamp(0, 3)
}

fn native_tool_provider_error_is_timeout(error: &ProviderError) -> bool {
    error.code == ProviderErrorCode::Timeout || error.message.contains("ollama_run_timeout")
}

fn native_tool_requires_successful_mutation(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("requires_successful_mutation_receipt"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_max_context_only_turns(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("max_context_only_turns"))
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 6)
}

fn native_tool_max_calls_per_turn(metadata: &Value) -> usize {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("max_tool_calls_per_turn"))
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(1, 16) as usize
}
















fn native_tool_wall_timeout(metadata: &Value) -> Option<Duration> {
    let seconds = metadata
        .get("native_tool_wall_timeout_seconds")
        .and_then(Value::as_u64)
        .or_else(|| {
            metadata
                .pointer("/native_success_criteria/native_wall_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/native_wall_timeout_seconds")
                .and_then(Value::as_u64)
        })?;
    if seconds == 0 {
        None
    } else {
        Some(Duration::from_secs(seconds.clamp(1, 7200)))
    }
}

fn native_tool_partial_progress_on_timeout(metadata: &Value) -> bool {
    metadata
        .get("partial_progress_on_timeout")
        .and_then(Value::as_bool)
        .or_else(|| {
            metadata
                .pointer("/native_success_criteria/partial_progress_on_timeout")
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/partial_progress_on_timeout")
                .and_then(Value::as_bool)
        })
        .unwrap_or(false)
}

fn native_tool_partial_progress_response(
    provider_id: &str,
    model: Option<&str>,
    reason: &str,
    provider_call_count: u64,
    receipts: &[NativeToolReceipt],
) -> ProviderResponse {
    let successful_mutations = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .filter(|receipt| receipt.tool_name == "file_write" || receipt.tool_name == "file_patch")
        .count();
    let output = format!(
        "Native coding run stopped with partial progress: {reason}. Successful native mutation receipts: {successful_mutations}. Returning a structured partial-progress terminal result so the parent workflow can report the timeout instead of hanging."
    );
    ProviderResponse {
        provider: provider_id.to_string(),
        model: model.unwrap_or("unknown").to_string(),
        usage_tokens: output.split_whitespace().count() as u64,
        output,
        raw: json!({
            "ok": false,
            "provider": provider_id,
            "terminal_status": "partial_timeout",
            "reason": reason,
            "provider_call_count": provider_call_count,
            "native_tool_call_count": receipts.len(),
            "successful_mutation_receipt_count": successful_mutations,
            "native_tool_receipt_summary": receipts.iter().map(|receipt| {
                json!({
                    "call_id": receipt.call_id.clone(),
                    "tool_name": receipt.tool_name.clone(),
                    "status": receipt.status.clone(),
                    "error": receipt.error.clone(),
                    "path": receipt.result.get("path").cloned().unwrap_or(Value::Null),
                })
            }).collect::<Vec<_>>(),
        }),
    }
}

fn native_tool_recovery_or_partial_progress(
    provider: &Arc<dyn crate::provider::ProviderClient>,
    dispatcher: &NativeToolDispatcher,
    tools: &[String],
    model: Option<String>,
    metadata: &Value,
    original_prompt: &str,
    system: &str,
    reason: &str,
    mut provider_call_count: u64,
    mut receipts: Vec<NativeToolReceipt>,
) -> Result<(ProviderResponse, Vec<NativeToolReceipt>, u64, String), ProviderError> {
    let changed_paths = native_tool_changed_paths(&receipts);
    if changed_paths.is_empty() {
        return Ok((
            native_tool_partial_progress_response(
                provider.provider_id(),
                model.as_deref(),
                reason,
                provider_call_count,
                &receipts,
            ),
            receipts,
            provider_call_count,
            "partial_timeout".to_string(),
        ));
    }

    let max_turns = native_tool_recovery_max_turns(metadata);
    let mut recovery_metadata = metadata.clone();
    if let Some(object) = recovery_metadata.as_object_mut() {
        object.insert(
            "provider_timeout_seconds".to_string(),
            json!(native_tool_recovery_provider_timeout_seconds(metadata)),
        );
        object.insert("native_recovery_pass".to_string(), json!(true));
    }
    let mut prompt =
        native_tool_recovery_prompt(metadata, original_prompt, reason, &changed_paths, &receipts);

    for turn_idx in 0..max_turns {
        provider_call_count += 1;
        let request = ProviderRequest {
            prompt: prompt.clone(),
            system: Some(system.to_string()),
            tools: tools.to_vec(),
            model: model.clone(),
            metadata: recovery_metadata.clone(),
        };
        let response = match provider.complete(&request) {
            Ok(response) => response,
            Err(error) if error.code == ProviderErrorCode::Timeout => {
                return Ok((
                    native_tool_partial_progress_response(
                        provider.provider_id(),
                        model.as_deref(),
                        error.message.as_str(),
                        provider_call_count,
                        &receipts,
                    ),
                    receipts,
                    provider_call_count,
                    "partial_timeout".to_string(),
                ));
            }
            Err(error) => return Err(error),
        };
        let calls = parse_native_tool_calls(&response.output);
        if calls.is_empty() {
            if native_tool_needs_public_report_finalization(metadata, &response.output) {
                provider_call_count += 1;
                let request = ProviderRequest {
                    prompt: native_tool_public_reasoning_finalization_prompt(
                        metadata,
                        original_prompt,
                        &receipts,
                        &response.output,
                    ),
                    system: Some(system.to_string()),
                    tools: Vec::new(),
                    model: model.clone(),
                    metadata: native_tool_public_reasoning_metadata(metadata),
                };
                let finalized = match provider.complete(&request) {
                    Ok(finalized) => finalized,
                    Err(error) if error.code == ProviderErrorCode::Timeout => {
                        return Ok((
                            native_tool_partial_progress_response(
                                provider.provider_id(),
                                model.as_deref(),
                                error.message.as_str(),
                                provider_call_count,
                                &receipts,
                            ),
                            receipts,
                            provider_call_count,
                            "partial_timeout".to_string(),
                        ));
                    }
                    Err(error) => return Err(error),
                };
                let mut finalized = finalized;
                finalized.raw = json!({
                    "provider_raw": finalized.raw,
                    "native_tool_recovery": {
                        "enabled": true,
                        "reason": reason,
                        "provider_call_count": provider_call_count,
                        "recovery_turns_used": turn_idx + 1,
                        "changed_paths": changed_paths,
                        "tool_call_count": receipts.len(),
                        "terminal_status": "ok",
                        "public_reasoning_finalization": true
                    }
                });
                return Ok((finalized, receipts, provider_call_count, "ok".to_string()));
            }
            let mut response = response;
            response.raw = json!({
                "provider_raw": response.raw,
                "native_tool_recovery": {
                    "enabled": true,
                    "reason": reason,
                    "provider_call_count": provider_call_count,
                    "recovery_turns_used": turn_idx + 1,
                    "changed_paths": changed_paths,
                    "tool_call_count": receipts.len(),
                    "terminal_status": "ok"
                }
            });
            return Ok((response, receipts, provider_call_count, "ok".to_string()));
        }
        let mut turn_receipts = Vec::new();
        for call in calls
            .into_iter()
            .take(native_tool_max_calls_per_turn(metadata))
        {
            let receipt = dispatcher.dispatch(native_tool_call_with_prompt_defaults(
                call,
                original_prompt,
            ));
            turn_receipts.push(receipt.clone());
            receipts.push(receipt);
        }
        let observation = native_tool_observation_prompt(&turn_receipts);
        let recovery_turn_rule = native_tool_orchestration_prompt_text(
            metadata,
            "partial_progress_recovery_turn_rule",
            "Continue the bounded recovery pass. If the changed files are repaired or no safe repair remains, provide the final user response.",
        );
        prompt = format!(
            "{}\n\nRecovery tool request turn {}:\n{}\n\nNative tool observations:\n{}\n\n{}",
            native_tool_recovery_prompt(metadata, original_prompt, reason, &changed_paths, &receipts),
            turn_idx + 1,
            response.output,
            observation,
            recovery_turn_rule
        );
    }

    if let Some(validation_receipt) =
        native_tool_auto_validation_receipt(dispatcher, original_prompt, &receipts)
    {
        receipts.push(validation_receipt);
    }
    let auto_handoff_receipts =
        native_tool_auto_workflow_artifact_receipts(dispatcher, metadata, original_prompt, &receipts);
    if !auto_handoff_receipts.is_empty() {
        receipts.extend(auto_handoff_receipts);
    }
    let mut response = native_tool_partial_progress_response(
        provider.provider_id(),
        model.as_deref(),
        "native_tool_recovery_pass_exhausted",
        provider_call_count,
        &receipts,
    );
    let completed_after_recovery = native_tool_has_successful_mutation(&receipts)
        && native_tool_has_successful_validation_command(&receipts)
        && native_tool_prompt_evidence_gaps(original_prompt, &receipts).is_empty();
    if completed_after_recovery {
        response = native_tool_synthetic_completion_evidence_response(
            &response,
            metadata,
            original_prompt,
            &receipts,
            "runtime_synthesized_timeout_recovery_closure",
        );
    }
    let terminal_status = if completed_after_recovery {
        "ok".to_string()
    } else {
        "partial_timeout".to_string()
    };
    Ok((response, receipts, provider_call_count, terminal_status))
}



fn native_tool_completion_evidence_repair_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("repair_uncovered_requirements_before_final"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_completion_evidence_repair_max_turns(metadata: &Value) -> u64 {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("completion_evidence_repair_max_turns"))
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 5)
}

fn native_tool_repair_reasons_include_product_mutation(repair_reasons: &[String]) -> bool {
    repair_reasons
        .iter()
        .any(|reason| reason == "missing_product_mutation_receipt")
}

fn native_tool_repair_reasons_include_product_slice(repair_reasons: &[String]) -> bool {
    repair_reasons.iter().any(|reason| {
        reason.starts_with("incomplete_product_slice")
            || reason.starts_with("missing_product_source_evidence:")
    })
}

fn native_tool_repair_reasons_require_product_work(repair_reasons: &[String]) -> bool {
    repair_reasons.iter().any(|reason| {
        reason == "missing_product_mutation_receipt"
            || reason == "missing_test_change_receipt"
            || reason.starts_with("incomplete_product_slice")
            || reason.starts_with("missing_product_source_evidence:")
            || reason.starts_with("missing_changed_path:")
    })
}

fn native_tool_has_successful_context_receipt(receipts: &[NativeToolReceipt]) -> bool {
    receipts.iter().any(|receipt| {
        receipt.status == "ok"
            && matches!(
                receipt.tool_name.as_str(),
                "file_read" | "file_read_many" | "file_list" | "file_stat"
            )
    })
}

fn native_tool_has_successful_test_mutation(receipts: &[NativeToolReceipt]) -> bool {
    receipts.iter().any(|receipt| {
        if receipt.status != "ok"
            || !(receipt.tool_name == "file_write" || receipt.tool_name == "file_patch")
        {
            return false;
        }
        let Some(path) = receipt.result.get("path").and_then(Value::as_str) else {
            return false;
        };
        let lower = path.to_ascii_lowercase();
        lower.contains("/test")
            || lower.contains("\\test")
            || lower.contains("tests/")
            || lower.contains("test_")
    })
}

fn native_tool_has_unmutated_required_repair_path(
    repair_reasons: &[String],
    receipts: &[NativeToolReceipt],
) -> bool {
    repair_reasons
        .iter()
        .filter_map(|reason| reason.strip_prefix("missing_changed_path:"))
        .filter(|path| !path.contains("checkpoint_") && !path.contains("handoff"))
        .any(|path| !native_tool_has_successful_mutation_for_suffix(receipts, path))
}

fn native_tool_has_successful_mutation_for_suffix(
    receipts: &[NativeToolReceipt],
    expected: &str,
) -> bool {
    let expected = expected
        .trim()
        .trim_start_matches("./")
        .replace('\\', "/")
        .to_ascii_lowercase();
    receipts.iter().any(|receipt| {
        if receipt.status != "ok"
            || !(receipt.tool_name == "file_write" || receipt.tool_name == "file_patch")
        {
            return false;
        }
        let Some(path) = receipt.result.get("path").and_then(Value::as_str) else {
            return false;
        };
        let normalized = path.replace('\\', "/").to_ascii_lowercase();
        normalized.ends_with(&expected) || normalized.contains(&format!("/{expected}"))
    })
}

fn native_tool_call_is_context_only(call: &NativeToolCall) -> bool {
    matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_list"
            | "list_files"
            | "workspace.list"
            | "workspace_list"
            | "file_stat"
            | "stat_file"
            | "file_exists"
            | "workspace.stat"
            | "workspace_stat"
            | "file_read"
            | "read_file"
            | "workspace.read"
            | "workspace_read"
            | "file_read_many"
            | "read_many_files"
            | "workspace.read_many"
            | "workspace_read_many"
    )
}

fn native_tool_product_repair_context_blocked_receipt(call: NativeToolCall) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: Value::Null,
        error: Some(
            "product_repair_requires_file_write_or_file_patch_before_more_context_reads"
                .to_string(),
        ),
    }
}

fn native_tool_call_with_prompt_defaults(
    mut call: NativeToolCall,
    original_prompt: &str,
) -> NativeToolCall {
    let project_root = native_tool_prompt_project_root(original_prompt);
    if let Some(project_root) = project_root.as_deref() {
        native_tool_apply_project_relative_file_paths(&mut call, project_root);
    }
    if native_tool_call_is_command_run(&call) && !native_tool_command_args_have_cwd(&call.args) {
        if let Some(project_root) = project_root {
            if !call.args.is_object() {
                call.args = json!({});
            }
            if let Some(args) = call.args.as_object_mut() {
                args.insert("cwd".to_string(), json!(project_root));
            }
        }
    }
    call
}

fn native_tool_apply_project_relative_file_paths(call: &mut NativeToolCall, project_root: &str) {
    if !native_tool_call_is_file_path_tool(call) || !call.args.is_object() {
        return;
    }
    let Some(args) = call.args.as_object_mut() else {
        return;
    };
    let path_keys = [
        "path",
        "file_path",
        "filepath",
        "target_path",
        "target",
        "file",
        "absolute_path",
        "full_path",
        "output_path",
        "destination",
        "dest",
        "filename",
    ];
    let mut normalized_path = None;
    for key in path_keys {
        let Some(value) = args.get(key).and_then(Value::as_str) else {
            continue;
        };
        let normalized = native_tool_project_relative_path(value, project_root);
        args.insert(key.to_string(), json!(normalized.clone()));
        normalized_path.get_or_insert(normalized);
    }
    if !args.contains_key("path") {
        if let Some(path) = normalized_path {
            args.insert("path".to_string(), json!(path));
        }
    }
    if let Some(paths) = args.get_mut("paths").and_then(Value::as_array_mut) {
        for path in paths {
            let Some(value) = path.as_str() else {
                continue;
            };
            *path = json!(native_tool_project_relative_path(value, project_root));
        }
    }
}

fn native_tool_project_relative_path(path: &str, project_root: &str) -> String {
    let path = path.trim();
    if path.is_empty() || std::path::Path::new(path).is_absolute() {
        return path.to_string();
    }
    let path = path.trim_start_matches("./");
    format!("{}/{}", project_root.trim_end_matches('/'), path)
}

fn native_tool_call_is_file_path_tool(call: &NativeToolCall) -> bool {
    matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_list"
            | "list_files"
            | "workspace.list"
            | "workspace_list"
            | "file_stat"
            | "stat_file"
            | "file_exists"
            | "workspace.stat"
            | "workspace_stat"
            | "file_read"
            | "read_file"
            | "workspace.read"
            | "workspace_read"
            | "file_read_many"
            | "read_many_files"
            | "workspace.read_many"
            | "workspace_read_many"
            | "file_write"
            | "write_file"
            | "workspace.write"
            | "workspace_write"
            | "file_patch"
            | "patch_file"
            | "workspace.patch"
            | "workspace_patch"
    )
}

fn native_tool_call_is_command_run(call: &NativeToolCall) -> bool {
    matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "command_run" | "run_command" | "command.run" | "shell.run" | "shell_run"
    )
}

fn native_tool_command_args_have_cwd(args: &Value) -> bool {
    [
        "cwd",
        "path",
        "working_directory",
        "working_dir",
        "workdir",
        "directory",
        "dir",
        "project_root",
        "root",
    ]
    .iter()
    .any(|key| args.get(*key).and_then(Value::as_str).is_some())
}

fn native_tool_call_targets_handoff_artifact(call: &NativeToolCall) -> bool {
    if !matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_write" | "write_file" | "workspace.write" | "workspace_write" | "file_patch"
            | "patch_file" | "workspace.patch" | "workspace_patch"
    ) {
        return false;
    }
    let path = call
        .args
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    path.contains("/receipts/")
        || path.contains("\\receipts\\")
        || path.contains("checkpoint_")
        || path.contains("handoff")
}

fn native_tool_handoff_blocked_repair_receipt(call: NativeToolCall) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: Value::Null,
        error: Some(
            "checkpoint_or_handoff_write_blocked_until_product_source_and_test_evidence_is_complete"
                .to_string(),
        ),
    }
}

fn native_tool_call_targets_unrelated_repair_path(
    call: &NativeToolCall,
    repair_reasons: &[String],
    original_prompt: &str,
) -> bool {
    if !matches!(
        call.name.trim().to_ascii_lowercase().as_str(),
        "file_write" | "write_file" | "workspace.write" | "workspace_write" | "file_patch"
            | "patch_file" | "workspace.patch" | "workspace_patch"
    ) || native_tool_call_targets_handoff_artifact(call)
    {
        return false;
    }
    let mut required = repair_reasons
        .iter()
        .filter_map(|reason| reason.strip_prefix("missing_changed_path:"))
        .filter(|path| !path.contains("checkpoint_") && !path.contains("handoff"))
        .map(|path| path.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if required.is_empty()
        && repair_reasons
            .iter()
            .any(|reason| reason == "missing_product_mutation_receipt")
    {
        required = native_tool_prompt_required_changed_paths(original_prompt)
            .into_iter()
            .filter(|path| !path.contains("checkpoint_") && !path.contains("handoff"))
            .map(|path| path.to_ascii_lowercase())
            .collect::<Vec<_>>();
    }
    if required.is_empty() {
        return false;
    }
    let Some(path) = native_tool_call_path_arg(call).map(|path| path.to_ascii_lowercase()) else {
        return false;
    };
    if repair_reasons
        .iter()
        .any(|reason| reason == "missing_test_change_receipt")
        && (path.contains("/test")
            || path.contains("\\test")
            || path.contains("tests/")
            || path.contains("test_"))
    {
        return false;
    }
    if repair_reasons
        .iter()
        .any(|reason| reason.starts_with("missing_product_source_evidence:"))
        && (path.contains("/src/")
            || path.contains("\\src\\")
            || path.ends_with(".py")
            || path.ends_with(".rs")
            || path.ends_with(".ts")
            || path.ends_with(".js"))
    {
        return false;
    }
    !required.iter().any(|required| path.ends_with(required))
}

fn native_tool_call_path_arg(call: &NativeToolCall) -> Option<&str> {
    [
        "path",
        "file_path",
        "filepath",
        "target_path",
        "target",
        "file",
        "absolute_path",
        "full_path",
        "output_path",
        "destination",
        "dest",
        "filename",
    ]
    .iter()
    .find_map(|key| call.args.get(*key).and_then(Value::as_str))
}

fn native_tool_unrelated_repair_path_receipt(call: NativeToolCall) -> NativeToolReceipt {
    NativeToolReceipt {
        call_id: call.id,
        tool_name: call.name.trim().to_ascii_lowercase(),
        status: "error".to_string(),
        duration_ms: 0,
        result: Value::Null,
        error: Some(
            "repair_write_blocked_until_missing_prompt_derived_paths_are_mutated".to_string(),
        ),
    }
}

fn native_tool_receipt_error_summary(receipts: &[NativeToolReceipt]) -> String {
    let summary = receipts
        .iter()
        .map(|receipt| {
            let result = &receipt.result;
            json!({
                "id": receipt.call_id,
                "tool": receipt.tool_name,
                "status": receipt.status,
                "path": result.get("path").or_else(|| result.get("cwd")).or_else(|| result.get("paths")).cloned().unwrap_or(Value::Null),
                "args_keys": result.get("args_keys").cloned().unwrap_or(Value::Null),
                "command": result.get("command").or_else(|| result.get("cmd")).cloned().unwrap_or(Value::Null),
                "error": receipt.error.clone().map(Value::String).unwrap_or(Value::Null),
                "success": result.get("success").cloned().unwrap_or(Value::Null),
                "exit_code": result.get("exit_code").cloned().unwrap_or(Value::Null),
                "stdout": result.get("stdout").and_then(Value::as_str).map(native_tool_compact_text).map(Value::String).unwrap_or(Value::Null),
                "stderr": result.get("stderr").and_then(Value::as_str).map(native_tool_compact_text).map(Value::String).unwrap_or(Value::Null)
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&summary).unwrap_or_else(|_| "[]".to_string())
}

fn native_tool_compact_text(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.len() <= 240 {
        compact
    } else {
        format!("{}...", compact.chars().take(240).collect::<String>())
    }
}




























fn native_tool_completion_evidence_repair_loop(
    provider: &Arc<dyn crate::provider::ProviderClient>,
    dispatcher: &NativeToolDispatcher,
    tools: &[String],
    model: Option<String>,
    metadata: &Value,
    original_prompt: &str,
    system: &str,
    mut response: ProviderResponse,
    mut receipts: Vec<NativeToolReceipt>,
    mut provider_call_count: u64,
    mut repair_reasons: Vec<String>,
) -> Result<(ProviderResponse, Vec<NativeToolReceipt>, u64), ProviderError> {
    let max_turns = native_tool_completion_evidence_repair_max_turns(metadata);
    let mut prompt =
        native_tool_completion_evidence_repair_prompt(
            metadata,
            original_prompt,
            &response.output,
            &receipts,
            &repair_reasons,
    );
    for turn_idx in 0..max_turns {
        provider_call_count += 1;
        let product_mutation_missing =
            native_tool_repair_reasons_include_product_mutation(&repair_reasons);
        let product_slice_incomplete =
            native_tool_repair_reasons_include_product_slice(&repair_reasons);
        let test_change_missing = repair_reasons
            .iter()
            .any(|reason| reason == "missing_test_change_receipt");
        let request = ProviderRequest {
            prompt: prompt.clone(),
            system: Some(system.to_string()),
            tools: tools.to_vec(),
            model: model.clone(),
            metadata: if product_mutation_missing || product_slice_incomplete {
                native_tool_recovery_timeout_metadata(metadata)
            } else {
                metadata.clone()
            },
        };
        let next_response = provider.complete(&request)?;
        let calls = parse_native_tool_calls(&next_response.output);
        if calls.is_empty() {
            response = next_response;
            repair_reasons = native_tool_artifact_repair_reasons(
                metadata,
                original_prompt,
                &response.output,
                &receipts,
            );
            if repair_reasons.is_empty() {
                break;
            }
            if product_mutation_missing || product_slice_incomplete {
                break;
            }
            prompt = native_tool_completion_evidence_repair_prompt(
                metadata,
                original_prompt,
                &response.output,
                &receipts,
                &repair_reasons,
            );
            continue;
        }
        let mut turn_receipts = Vec::new();
        let product_work_required =
            native_tool_repair_reasons_require_product_work(&repair_reasons);
        let has_successful_context = native_tool_has_successful_context_receipt(&receipts);
        let has_successful_mutation = native_tool_has_successful_mutation(&receipts);
        let has_successful_test_mutation = native_tool_has_successful_test_mutation(&receipts);
        let has_unmutated_required_repair_path =
            native_tool_has_unmutated_required_repair_path(&repair_reasons, &receipts);
        for call in calls
            .into_iter()
            .take(native_tool_max_calls_per_turn(metadata))
        {
            let call = native_tool_call_with_prompt_defaults(call, original_prompt);
            let receipt =
                if product_work_required
                    && has_successful_context
                    && (!has_successful_mutation
                        || (test_change_missing && !has_successful_test_mutation)
                        || has_unmutated_required_repair_path)
                    && native_tool_call_is_context_only(&call)
                {
                    native_tool_product_repair_context_blocked_receipt(call)
                } else if (product_mutation_missing || product_slice_incomplete || test_change_missing)
                    && native_tool_call_targets_handoff_artifact(&call)
                {
                    native_tool_handoff_blocked_repair_receipt(call)
                } else if product_work_required
                    && native_tool_call_targets_unrelated_repair_path(
                        &call,
                        &repair_reasons,
                        original_prompt,
                    )
                {
                    native_tool_unrelated_repair_path_receipt(call)
                } else {
                    dispatcher.dispatch(call)
                };
            turn_receipts.push(receipt.clone());
            receipts.push(receipt);
        }
        response = next_response;
        repair_reasons = native_tool_artifact_repair_reasons(
            metadata,
            original_prompt,
            &response.output,
            &receipts,
        );
        if product_mutation_missing && !native_tool_has_successful_mutation(&receipts) {
            break;
        }
        if repair_reasons.is_empty() {
            break;
        }
        let observation = native_tool_observation_prompt(&turn_receipts);
        let failed_validation_details = native_tool_failed_validation_receipt_details(&receipts);
        let repair_actions =
            native_tool_completion_repair_action_brief(metadata, original_prompt, &repair_reasons);
        let repair_turn_rule = native_tool_orchestration_prompt_text(
            metadata,
            "completion_evidence_repair_turn_rule",
            "Continue repairing only the remaining uncovered requirements from this native tool task. Return JSON tool calls, or return a structured blocker only when local completion is genuinely blocked.",
        );
        prompt = format!(
            "{}\n\nRepair turn {} produced observations:\n{}\n\nFailed validation receipt details:\n{}\n\nRemaining uncovered requirements:\n{}\n\nRequired repair actions:\n{}\n\n{}",
            original_prompt,
            turn_idx + 1,
            observation,
            failed_validation_details,
            repair_reasons.join("\n"),
            repair_actions,
            repair_turn_rule
        );
    }
    Ok((response, receipts, provider_call_count))
}





fn native_tool_completion_evidence_timeout_synthesis_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("synthesize_completion_evidence_on_finalization_timeout"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn native_tool_synthesize_final_after_successful_validation(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("synthesize_final_after_successful_validation"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn native_tool_recovery_timeout_metadata(metadata: &Value) -> Value {
    let mut out = metadata.clone();
    let timeout_seconds = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("recovery_provider_timeout_seconds"))
        .and_then(Value::as_u64)
        .unwrap_or(120);
    if let Some(object) = out.as_object_mut() {
        object.insert("provider_timeout_seconds".to_string(), json!(timeout_seconds));
    }
    out
}

fn native_tool_bootstrap_context_before_first_provider(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("bootstrap_context_before_first_provider"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}





fn native_tool_bootstrap_discovery_receipt(
    dispatcher: &NativeToolDispatcher,
    original_prompt: &str,
) -> Option<NativeToolReceipt> {
    let project_root = native_tool_prompt_project_root(original_prompt)?;
    let receipt = dispatcher.dispatch(crate::native_tools::NativeToolCall {
        id: "runtime_bootstrap_file_list".to_string(),
        name: "file_list".to_string(),
        args: json!({
            "path": project_root,
            "recursive": false,
            "max_entries": 200
        }),
    });
    if receipt.status == "ok" {
        Some(receipt)
    } else {
        None
    }
}

fn native_tool_bootstrap_context_receipts(
    dispatcher: &NativeToolDispatcher,
    original_prompt: &str,
) -> Vec<NativeToolReceipt> {
    let Some(project_root) = native_tool_prompt_project_root(original_prompt) else {
        return Vec::new();
    };
    let mut receipts = Vec::new();
    let list_receipt = dispatcher.dispatch(crate::native_tools::NativeToolCall {
        id: "runtime_bootstrap_file_list".to_string(),
        name: "file_list".to_string(),
        args: json!({
            "path": project_root,
            "recursive": true,
            "max_depth": 3,
            "max_entries": 200
        }),
    });
    receipts.push(list_receipt);
    let root = std::path::PathBuf::from(&project_root);
    let mut paths = native_tool_unique_code_path_mentions(original_prompt)
        .into_iter()
        .filter_map(|path| {
            let candidate = if path.starts_with('/') {
                std::path::PathBuf::from(path)
            } else {
                root.join(path.trim_start_matches("./"))
            };
            if candidate.is_file() {
                Some(candidate.display().to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    if !paths.is_empty() {
        receipts.push(dispatcher.dispatch(crate::native_tools::NativeToolCall {
            id: "runtime_bootstrap_file_read_many".to_string(),
            name: "file_read_many".to_string(),
            args: json!({ "paths": paths }),
        }));
    }
    receipts
        .into_iter()
        .filter(|receipt| receipt.status == "ok")
        .collect()
}

fn native_tool_auto_validation_receipt(
    dispatcher: &NativeToolDispatcher,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> Option<NativeToolReceipt> {
    let prompt_lower = original_prompt.to_ascii_lowercase();
    if !native_tool_prompt_requires_validation_command(&prompt_lower)
        || native_tool_has_successful_validation_command(receipts)
    {
        return None;
    }
    if native_tool_prompt_requires_product_mutation(&prompt_lower)
        && !native_tool_has_successful_mutation(receipts)
    {
        return None;
    }
    let project_root = native_tool_prompt_project_root(original_prompt)?;
    let project_root_path = std::path::PathBuf::from(&project_root);
    let cmd = if prompt_lower.contains("pytest")
        || (project_root_path.join("pyproject.toml").exists()
            && project_root_path.join("tests").is_dir())
    {
        vec!["python3", "-m", "pytest", "-q"]
    } else if prompt_lower.contains("unittest")
        || (project_root_path.join("src").is_dir() && project_root_path.join("tests").is_dir())
    {
        vec![
            "sh",
            "-c",
            "PYTHONPATH=src python3 -m unittest discover -s tests",
        ]
    } else {
        return None;
    };
    Some(dispatcher.dispatch(crate::native_tools::NativeToolCall {
        id: "runtime_auto_validation_command".to_string(),
        name: "command_run".to_string(),
        args: json!({
            "cwd": project_root,
            "cmd": cmd,
            "timeout_seconds": 120,
            "max_output_bytes": 12000
        }),
    }))
}

fn native_tool_recovery_max_turns(metadata: &Value) -> u64 {
    metadata
        .pointer("/native_success_criteria/partial_recovery_max_turns")
        .and_then(Value::as_u64)
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/partial_recovery_max_turns")
                .and_then(Value::as_u64)
        })
        .unwrap_or(3)
        .clamp(1, 6)
}

fn native_tool_recovery_provider_timeout_seconds(metadata: &Value) -> u64 {
    metadata
        .pointer("/native_success_criteria/recovery_provider_timeout_seconds")
        .and_then(Value::as_u64)
        .or_else(|| {
            metadata
                .pointer("/workflow/native_success_criteria/recovery_provider_timeout_seconds")
                .and_then(Value::as_u64)
        })
        .unwrap_or(120)
        .clamp(1, 600)
}



fn sanitize_token(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in raw.chars().take(max_len) {
        if ch.is_control() && ch != '\n' && ch != '\t' {
            continue;
        }
        out.push(ch);
    }
    out.trim().to_string()
}
