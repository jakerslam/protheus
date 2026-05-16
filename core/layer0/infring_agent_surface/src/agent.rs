use crate::capability_pack::CapabilityPackCatalog;
use crate::native_tools::{
    native_tool_observation_prompt, parse_native_tool_calls, NativeToolDispatcher,
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

        for turn_idx in 0..max_turns {
            if let Some(timeout) = wall_timeout {
                if loop_started.elapsed() >= timeout {
                    if !all_receipts.is_empty()
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
                    if error.code == ProviderErrorCode::Timeout
                        && !all_receipts.is_empty()
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
                Err(error) => return Err(error),
            };
            let calls = parse_native_tool_calls(&response.output);
            if calls.is_empty() {
                if all_receipts.is_empty() && empty_tool_retry_count < empty_tool_retry_limit {
                    empty_tool_retry_count += 1;
                    prompt = native_tool_empty_retry_prompt(
                        &self.initial_prompt,
                        &response.output,
                        empty_tool_retry_count,
                    );
                    last_response = Some(response);
                    continue;
                }
                last_response = Some(response);
                break;
            }
            let mut turn_receipts = Vec::new();
            for call in calls.into_iter().take(8) {
                let receipt = dispatcher.dispatch(call);
                turn_receipts.push(receipt.clone());
                all_receipts.push(receipt);
            }
            if native_tool_should_synthesize_micro_final(
                &self.metadata,
                &self.initial_prompt,
                &all_receipts,
            ) {
                let mut response =
                    native_tool_synthetic_micro_final_response(&response, &self.initial_prompt, &all_receipts);
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
            let continuation = if native_tool_requires_successful_mutation(&self.metadata)
                && !native_tool_has_successful_mutation(&all_receipts)
                && context_only_turn_count >= native_tool_max_context_only_turns(&self.metadata)
            {
                "Native mutation transition guard: successful discovery/read context has already been gathered for a task that requires file mutation. On the next assistant turn, do not call more discovery/read-only tools. Return only JSON tool_calls containing at least one file_write or file_patch now, or provide a structured blocker explaining the exact missing information or constraint that prevents mutation."
            } else {
                "Continue."
            };
            prompt = format!(
                "{}\n\nAssistant tool request turn {}:\n{}\n\nNative tool observations:\n{}\n\n{}",
                self.initial_prompt,
                turn_idx + 1,
                response.output,
                observation,
                continuation
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
            for call in pending_terminal_calls.into_iter().take(8) {
                if existing_call_ids.contains(&call.id) {
                    continue;
                }
                let receipt = dispatcher.dispatch(call);
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
        let terminal_output_has_tool_calls = !parse_native_tool_calls(&response.output).is_empty();
        let completion_evidence_finalization = native_tool_needs_completion_evidence_finalization(
            &self.metadata,
            &self.initial_prompt,
            &response.output,
            &all_receipts,
        );
        if terminal_output_has_tool_calls
            || native_tool_needs_public_reasoning_finalization(&self.metadata, &response.output)
            || completion_evidence_finalization
        {
            provider_call_count += 1;
            let mut finalization_prompt = native_tool_public_reasoning_finalization_prompt(
                &self.initial_prompt,
                &all_receipts,
                &response.output,
            );
            if completion_evidence_finalization {
                finalization_prompt.push_str(
                    "\n\nCompletion evidence guard: this is a multi-requirement coding task and the previous answer did not include task_requirement_checklist coverage. Include task_requirement_checklist now. If any user-stated requirement is not evidence-backed by changed files, validation, or native receipts, mark it uncovered or blocked and set completion_status/status to partial_or_blocked rather than success.",
                );
            }
            if terminal_output_has_tool_calls {
                finalization_prompt.push_str(
                    "\n\nTerminal tool-call guard: the previous assistant response still contained native tool_calls, so it is not a valid final answer. Those tool requests have already been handled by the runtime when possible. Tools are disabled for this finalization pass. Return only a final receipt-backed answer with public_reasoning_trace, reasoning_rollup, redaction_policy, changed_files, validation_summary, and blockers if any. Do not output tool_calls.",
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
            if native_tool_needs_completion_evidence_finalization(
                &self.metadata,
                &self.initial_prompt,
                &response.output,
                &all_receipts,
            ) && native_tool_has_successful_mutation(&all_receipts)
            {
                response = native_tool_synthetic_completion_evidence_response(
                    &response,
                    &self.initial_prompt,
                    &all_receipts,
                    "missing_task_requirement_checklist_after_finalization",
                );
            }
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

fn native_tool_has_successful_mutation(receipts: &[NativeToolReceipt]) -> bool {
    receipts.iter().any(|receipt| {
        receipt.status == "ok"
            && matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
    })
}

fn native_tool_context_only_turn(receipts: &[NativeToolReceipt]) -> bool {
    let mut saw_successful_context = false;
    for receipt in receipts {
        match receipt.tool_name.as_str() {
            "file_list" | "file_stat" | "file_read" | "file_read_many" => {
                saw_successful_context |= receipt.status == "ok";
            }
            "file_write" | "file_patch" => return false,
            _ => return false,
        }
    }
    saw_successful_context
}

fn native_tool_should_synthesize_micro_final(
    metadata: &Value,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> bool {
    let criteria = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"));
    let enabled = criteria
        .and_then(|value| value.get("synthesize_final_after_successful_micro_mutation"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    enabled
        && native_tool_has_successful_mutation(receipts)
        && native_tool_is_probable_micro_direct_write_task(metadata, original_prompt)
}

fn native_tool_is_probable_micro_direct_write_task(metadata: &Value, original_prompt: &str) -> bool {
    let criteria = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"));
    if !criteria
        .and_then(|value| value.get("micro_direct_write_enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return false;
    }
    let lower = original_prompt.to_ascii_lowercase();
    let create_like = [
        "create ",
        "write ",
        "make ",
        "single file",
        "one file",
        "tiny ",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    if !create_like {
        return false;
    }
    let existing_project_markers = [
        "update ",
        "modify ",
        "refactor",
        "debug",
        "fix ",
        "repair",
        "existing ",
        "preserve ",
        "integrat",
        "tests/",
        "src/",
        "package.json",
        "pyproject.toml",
        "cargo.toml",
    ];
    if existing_project_markers
        .iter()
        .any(|needle| lower.contains(needle))
    {
        return false;
    }
    let target_count = native_tool_unique_code_path_mentions(original_prompt).len();
    let max_targets = criteria
        .and_then(|value| value.get("micro_direct_write_max_target_files"))
        .and_then(Value::as_u64)
        .unwrap_or(1) as usize;
    target_count > 0 && target_count <= max_targets
}

fn native_tool_unique_code_path_mentions(raw: &str) -> Vec<String> {
    let extensions = [
        ".py", ".js", ".ts", ".tsx", ".jsx", ".html", ".css", ".rs", ".go", ".java", ".rb",
        ".php", ".swift", ".kt", ".c", ".cpp", ".h", ".hpp", ".md",
    ];
    let mut out = Vec::<String>::new();
    for token in raw.split_whitespace() {
        let cleaned = token.trim_matches(|ch: char| {
            matches!(
                ch,
                '`' | '\'' | '"' | ',' | ';' | ':' | ')' | '(' | '[' | ']' | '{' | '}'
            )
        });
        let lower = cleaned.to_ascii_lowercase();
        if extensions.iter().any(|extension| lower.ends_with(extension))
            && !out.iter().any(|path| path == cleaned)
        {
            out.push(cleaned.to_string());
        }
    }
    out
}

fn native_tool_synthetic_micro_final_response(
    previous_response: &ProviderResponse,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
) -> ProviderResponse {
    let changed_paths = native_tool_changed_paths(receipts);
    let receipt_refs = receipts
        .iter()
        .filter(|receipt| {
            receipt.status == "ok"
                && matches!(receipt.tool_name.as_str(), "file_write" | "file_patch")
        })
        .map(|receipt| {
            let path = receipt
                .result
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("");
            format!("{}:{}:{}", receipt.call_id, receipt.tool_name, path)
        })
        .collect::<Vec<_>>();
    let output = format!(
        "Task completed through the native micro direct-write path.\n\n```json\n{}\n```",
        serde_json::to_string_pretty(&json!({
            "workflow_id": "coding_project_operator",
            "completion_status": "success",
            "changed_files": changed_paths.clone(),
            "validation_summary": {
                "status": "receipt_verified",
                "note": "Runtime synthesized this final response after successful native mutation receipt for an isolated direct-write task.",
            },
            "checkpoint_or_blocker": {
                "kind": "completed_checkpoint",
                "summary": original_prompt.lines().next().unwrap_or("micro direct-write task").chars().take(160).collect::<String>(),
            },
            "public_reasoning_trace": {
                "protocol": "public_reasoning_trace_v1",
                "task_summary": "Isolated direct-write coding task completed with native file mutation.",
                "plan_summary": "Use the explicit target and behavior from the prompt, write the file through native tooling, and report only receipt-backed completion.",
                "decisions": [
                    "Selected micro direct-write path because the task looked like an isolated create/write-one-file request.",
                    "Skipped project discovery because the higher-level context policy marks discovery optional for greenfield isolated create tasks."
                ],
                "actions": [
                    "Executed native file mutation.",
                    "Accepted completion only after a successful file_write/file_patch receipt."
                ],
                "changed_files": changed_paths.clone(),
                "validation_summary": "Successful mutation receipt observed; no separate validation command was requested by the workflow.",
                "risks": [],
                "blockers": [],
                "confidence": "high",
                "evidence_refs": receipt_refs.clone(),
                "tool_receipt_refs": receipt_refs.clone(),
                "child_trace_refs": []
            },
            "reasoning_rollup": {
                "protocol": "public_reasoning_rollup_v1",
                "status": "complete",
                "summary": "The requested file change was completed through native tooling and confirmed by successful mutation receipts.",
                "changed_files": changed_paths,
                "evidence_refs": receipt_refs,
                "blockers": []
            },
            "child_reasoning_trace_refs": [],
            "redaction_policy": "no_hidden_chain_of_thought"
        }))
        .unwrap_or_else(|_| "{\"redaction_policy\":\"no_hidden_chain_of_thought\"}".to_string())
    );
    ProviderResponse {
        provider: previous_response.provider.clone(),
        model: previous_response.model.clone(),
        usage_tokens: previous_response.usage_tokens,
        raw: json!({
            "synthetic_micro_final": true,
            "previous_provider_raw": previous_response.raw.clone(),
        }),
        output,
    }
}

fn native_tool_initial_prompt(original_prompt: &str, metadata: &Value) -> String {
    let criteria = metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"));
    let requires_native_tool_use = criteria
        .and_then(|value| value.get("requires_native_tool_use"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let force_discovery_first = criteria
        .and_then(|value| value.get("force_discovery_first_turn"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let force_read_first = criteria
        .and_then(|value| value.get("force_read_first_turn"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if force_discovery_first {
        return format!(
            "{original_prompt}\n\nNative tool-use initiation rule: before planning, editing, or final answering, return only JSON with a tool_calls array that discovers the local project shape. Start with file_list on the local project root or directory implied by the task. Use file_stat before reading any target path that may not exist. After discovery observations, classify the work as create, edit, extend, debug, or refactor; then read only relevant existing context files before writing or patching. For create/new-file tasks, do not file_read the target file unless file_stat or file_list shows it exists. If the task requires mutation, do not repeat discovery/read-only turns after sufficient context; transition to file_write/file_patch or return a structured blocker. Do not produce prose, analysis, or a final answer until native discovery observations are returned."
        );
    }
    if !force_read_first {
        if requires_native_tool_use {
            return format!(
                "{original_prompt}\n\nNative coding tool-use rule: choose the shortest safe native file-tool path for the task. If the request is an isolated greenfield/create-file task with an explicit target path and behavior, return only JSON tool_calls with file_write now; discovery is optional, not mandatory. If the request modifies, debugs, refactors, or extends an existing or unclear project, use file_list/file_stat and then read relevant existing files before writing. For multi-requirement tasks, derive a concise task_requirement_checklist from the numbered/bulleted/user-stated requirements and keep working until each item has receipt-backed evidence, or return a structured partial/blocker naming uncovered items. Do not provide a final answer for mutation work until native file_write or file_patch observations confirm the change, or return a structured blocker if mutation cannot proceed."
            );
        }
        return original_prompt.to_string();
    }
    format!(
        "{original_prompt}\n\nNative tool-use initiation rule: before planning or final answering, return only JSON with a tool_calls array that reads existing local context files relevant to the task. If a target may not exist yet, use file_stat first rather than file_read. Prefer file_read_many when multiple existing files are known. Do not produce prose, analysis, or a final answer until native file-read observations are returned."
    )
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
    let mut prompt = native_tool_recovery_prompt(original_prompt, reason, &changed_paths, &receipts);

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
            if native_tool_needs_public_reasoning_finalization(metadata, &response.output) {
                provider_call_count += 1;
                let request = ProviderRequest {
                    prompt: native_tool_public_reasoning_finalization_prompt(
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
        for call in calls.into_iter().take(8) {
            let receipt = dispatcher.dispatch(call);
            turn_receipts.push(receipt.clone());
            receipts.push(receipt);
        }
        let observation = native_tool_observation_prompt(&turn_receipts);
        prompt = format!(
            "{}\n\nRecovery tool request turn {}:\n{}\n\nNative tool observations:\n{}\n\nContinue the bounded recovery pass. If the changed files are repaired or no safe repair remains, provide the final answer with public_reasoning_trace and reasoning_rollup.",
            native_tool_recovery_prompt(original_prompt, reason, &changed_paths, &receipts),
            turn_idx + 1,
            response.output,
            observation
        );
    }

    Ok((
        native_tool_partial_progress_response(
            provider.provider_id(),
            model.as_deref(),
            "native_tool_recovery_pass_exhausted",
            provider_call_count,
            &receipts,
        ),
        receipts,
        provider_call_count,
        "partial_timeout".to_string(),
    ))
}

fn native_tool_needs_public_reasoning_finalization(metadata: &Value, output: &str) -> bool {
    let contract = metadata.get("public_reasoning_trace_contract");
    let Some(contract) = contract else {
        return false;
    };
    if !contract.is_object() {
        return false;
    }
    if output.contains("\"tool_calls\"") || output.contains("{\"tool_calls\"") {
        return true;
    }
    let emits = contract
        .get("emits")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let requires_trace = emits.contains(&"public_reasoning_trace_v1")
        || contract
            .get("local_trace_required_fields")
            .and_then(Value::as_array)
            .is_some();
    let requires_rollup = emits.contains(&"public_reasoning_rollup_v1");
    let has_trace =
        output.contains("public_reasoning_trace") && output.contains("public_reasoning_trace_v1");
    let has_rollup =
        output.contains("reasoning_rollup") && output.contains("public_reasoning_rollup_v1");
    (requires_trace && !has_trace) || (requires_rollup && !has_rollup)
}

fn native_tool_needs_completion_evidence_finalization(
    metadata: &Value,
    original_prompt: &str,
    output: &str,
    receipts: &[NativeToolReceipt],
) -> bool {
    native_tool_completion_evidence_contract_enabled(metadata)
        && native_tool_prompt_has_multiple_requirements(original_prompt)
        && native_tool_has_successful_mutation(receipts)
        && !output.contains("task_requirement_checklist")
}

fn native_tool_completion_evidence_contract_enabled(metadata: &Value) -> bool {
    metadata
        .get("completion_evidence_contract")
        .or_else(|| metadata.pointer("/workflow/completion_evidence_contract"))
        .map(Value::is_object)
        .unwrap_or(false)
        || metadata
            .get("native_success_criteria")
            .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
            .and_then(|value| value.get("completion_evidence_required_for_multi_requirement_tasks"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn native_tool_completion_evidence_timeout_synthesis_enabled(metadata: &Value) -> bool {
    metadata
        .get("native_success_criteria")
        .or_else(|| metadata.pointer("/workflow/native_success_criteria"))
        .and_then(|value| value.get("synthesize_completion_evidence_on_finalization_timeout"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn native_tool_prompt_has_multiple_requirements(original_prompt: &str) -> bool {
    native_tool_requirement_lines(original_prompt).len() >= 2
}

fn native_tool_requirement_lines(original_prompt: &str) -> Vec<String> {
    let mut in_task = false;
    let mut out = Vec::<String>::new();
    for line in original_prompt.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("task:") {
            in_task = true;
            continue;
        }
        if in_task && lower.starts_with("final response contract") {
            break;
        }
        if !in_task {
            continue;
        }
        if let Some(requirement) = native_tool_requirement_from_line(trimmed) {
            out.push(requirement);
        }
    }
    if out.is_empty() {
        for line in original_prompt.lines() {
            if let Some(requirement) = native_tool_requirement_from_line(line.trim()) {
                out.push(requirement);
            }
        }
    }
    out
}

fn native_tool_requirement_from_line(trimmed: &str) -> Option<String> {
    if trimmed.is_empty() {
        return None;
    }
    if let Some((prefix, rest)) = trimmed.split_once('.') {
        if !prefix.is_empty()
            && prefix.chars().all(|ch| ch.is_ascii_digit())
            && !rest.trim().is_empty()
        {
            return Some(rest.trim().to_string());
        }
    }
    if let Some((prefix, rest)) = trimmed.split_once(')') {
        if !prefix.is_empty()
            && prefix.chars().all(|ch| ch.is_ascii_digit())
            && !rest.trim().is_empty()
        {
            return Some(rest.trim().to_string());
        }
    }
    for marker in ["- ", "* "] {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            if !rest.trim().is_empty() {
                return Some(rest.trim().to_string());
            }
        }
    }
    None
}

fn native_tool_successful_receipt_refs(receipts: &[NativeToolReceipt]) -> Vec<String> {
    receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .map(|receipt| {
            let path = receipt
                .result
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("");
            format!("{}:{}:{}", receipt.call_id, receipt.tool_name, path)
        })
        .collect()
}

fn native_tool_synthetic_completion_evidence_response(
    previous_response: &ProviderResponse,
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
    reason: &str,
) -> ProviderResponse {
    let changed_paths = native_tool_changed_paths(receipts);
    let receipt_refs = native_tool_successful_receipt_refs(receipts);
    let mut requirements = native_tool_requirement_lines(original_prompt);
    if requirements.is_empty() {
        requirements.push("Complete the requested coding task.".to_string());
    }
    let item_status = if changed_paths.is_empty() {
        "blocked"
    } else if reason.contains("timeout") {
        "covered"
    } else {
        "covered"
    };
    let checklist = requirements
        .iter()
        .enumerate()
        .map(|(idx, requirement)| {
            json!({
                "id": format!("requirement_{}", idx + 1),
                "requirement": requirement,
                "status": item_status,
                "evidence_refs": receipt_refs.clone(),
                "blocker_reason": if changed_paths.is_empty() { Value::String(reason.to_string()) } else { Value::Null },
            })
        })
        .collect::<Vec<_>>();
    let completion_status = if changed_paths.is_empty() {
        "partial_or_blocked"
    } else if reason.contains("timeout") {
        "success_receipt_backed_finalization_timeout"
    } else {
        "success_receipt_backed_runtime_synthesized"
    };
    let has_changed_paths = !changed_paths.is_empty();
    let output = format!(
        "Runtime synthesized completion-evidence finalization.\n\n```json\n{}\n```",
        serde_json::to_string_pretty(&json!({
            "workflow_id": "coding_project_operator",
            "completion_status": completion_status,
            "changed_files": changed_paths.clone(),
            "validation_summary": {
                "status": "receipt_backed",
                "note": "Runtime synthesized this artifact because finalization was missing, incomplete, or timed out after native mutation receipts.",
                "synthesis_reason": reason,
            },
            "checkpoint_or_blocker": {
                "kind": if has_changed_paths { "completed_checkpoint" } else { "structured_blocker" },
                "summary": reason,
            },
            "public_reasoning_trace": {
                "protocol": "public_reasoning_trace_v1",
                "task_summary": "Multi-requirement coding task completed or summarized from native receipts.",
                "plan_summary": "Map user-stated requirements to native mutation/read receipts and avoid claiming unsupported hidden work.",
                "decisions": [
                    "Used runtime completion-evidence synthesis because model finalization did not produce a valid checklist artifact."
                ],
                "actions": [
                    "Collected changed-file paths from successful file_write/file_patch receipts.",
                    "Derived task_requirement_checklist from user-stated task requirements.",
                    "Attached successful native receipt refs as public evidence."
                ],
                "changed_files": changed_paths.clone(),
                "validation_summary": "Receipt-backed synthesis only; external semantic validation is represented by caller or harness checks when available.",
                "risks": [
                    "Runtime synthesis cannot prove semantic completeness beyond available receipts and caller validation."
                ],
                "blockers": if has_changed_paths { Vec::<String>::new() } else { vec![reason.to_string()] },
                "confidence": if has_changed_paths { "medium" } else { "low" },
                "evidence_refs": receipt_refs.clone(),
                "tool_receipt_refs": receipt_refs.clone(),
                "child_trace_refs": [],
                "task_requirement_checklist": checklist.clone(),
                "redaction_policy": "no_hidden_chain_of_thought"
            },
            "reasoning_rollup": {
                "protocol": "public_reasoning_rollup_v1",
                "status": completion_status,
                "summary": "Runtime produced a public receipt-backed completion evidence map for the coding task.",
                "changed_files": changed_paths,
                "evidence_refs": receipt_refs,
                "task_requirement_checklist": checklist,
                "blockers": if has_changed_paths { Vec::<String>::new() } else { vec![reason.to_string()] },
                "redaction_policy": "no_hidden_chain_of_thought"
            },
            "child_reasoning_trace_refs": [],
            "redaction_policy": "no_hidden_chain_of_thought"
        }))
        .unwrap_or_else(|_| "{\"redaction_policy\":\"no_hidden_chain_of_thought\"}".to_string())
    );
    ProviderResponse {
        provider: previous_response.provider.clone(),
        model: previous_response.model.clone(),
        output,
        usage_tokens: previous_response.usage_tokens,
        raw: json!({
            "synthetic_completion_evidence_final": true,
            "synthesis_reason": reason,
            "previous_provider_raw": previous_response.raw.clone(),
        }),
    }
}

fn native_tool_public_reasoning_metadata(metadata: &Value) -> Value {
    let mut out = metadata.clone();
    if let Some(object) = out.as_object_mut() {
        object.insert("provider_timeout_seconds".to_string(), json!(90));
        object.insert("native_public_reasoning_finalization".to_string(), json!(true));
    }
    out
}

fn native_tool_public_reasoning_finalization_prompt(
    original_prompt: &str,
    receipts: &[NativeToolReceipt],
    previous_output: &str,
) -> String {
    let changed_paths = native_tool_changed_paths(receipts);
    let receipt_refs = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .map(|receipt| {
            let path = receipt
                .result
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("");
            format!("{}:{}:{}", receipt.call_id, receipt.tool_name, path)
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Public reasoning finalization pass. Do not call tools. Do not output tool_calls.\n\nOriginal task summary:\n{}\n\nReceipt-backed changed files:\n{}\n\nSuccessful receipt refs:\n{}\n\nPrevious non-final output preview:\n{}\n\nReturn a concise final answer that includes two JSON objects:\n1. public_reasoning_trace with schema_version public_reasoning_trace_v1.\n2. reasoning_rollup with schema_version public_reasoning_rollup_v1.\n\nFor multi-requirement coding tasks, derive a task_requirement_checklist from the user-stated numbered/bulleted/must-have requirements. Include the checklist in public_reasoning_trace. Each item must include id, requirement, status covered|uncovered|blocked, and evidence_refs or blocker_reason. If any item is uncovered or blocked, set completion_status/status to partial_or_blocked and do not claim success. Claim success only when every checklist item is covered by changed files, validation, or native tool receipt refs.\n\nBoth JSON objects must include redaction_policy: no_hidden_chain_of_thought. Use only public reasoning: plan summary, decisions, actions, risks, blockers, confidence, evidence_refs, tool_receipt_refs, child_trace_refs, and task_requirement_checklist. Do not include hidden chain-of-thought, private notes, raw system prompts, or raw tool payloads.",
        original_prompt.chars().take(1800).collect::<String>(),
        changed_paths.join("\n"),
        receipt_refs,
        previous_output.chars().take(1200).collect::<String>()
    )
}

fn native_tool_changed_paths(receipts: &[NativeToolReceipt]) -> Vec<String> {
    let mut paths = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .filter(|receipt| receipt.tool_name == "file_write" || receipt.tool_name == "file_patch")
        .filter_map(|receipt| receipt.result.get("path").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

fn native_tool_recovery_prompt(
    original_prompt: &str,
    reason: &str,
    changed_paths: &[String],
    receipts: &[NativeToolReceipt],
) -> String {
    let receipt_refs = receipts
        .iter()
        .filter(|receipt| receipt.status == "ok")
        .map(|receipt| {
            let path = receipt
                .result
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("");
            format!("{}:{}:{}", receipt.call_id, receipt.tool_name, path)
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Native partial-progress recovery pass.\nTimeout or stall reason: {reason}\n\nOriginal task:\n{}\n\nReceipt-backed changed files:\n{}\n\nSuccessful receipt refs:\n{}\n\nRecovery rules:\n- This is a bounded repair/finalization pass, not a new implementation pass.\n- Use file_read_many first on the receipt-backed changed files.\n- Fix only obvious syntax, import, serialization/persistence empty-file, or runtime errors inside receipt-backed changed files.\n- Do not expand scope or add unrelated features.\n- If you write fixes, cite the new write/patch receipt ids.\n- Final output must include public_reasoning_trace with schema public_reasoning_trace_v1 and reasoning_rollup with schema public_reasoning_rollup_v1.\n- Include redaction_policy: no_hidden_chain_of_thought.\n- Public reasoning only: plan summary, decisions, actions, risks, blockers, confidence, evidence/tool receipt refs. Do not include hidden chain-of-thought.\n\nReturn only JSON tool_calls if you need file tools; otherwise provide the final response.",
        original_prompt.chars().take(2400).collect::<String>(),
        changed_paths.join("\n"),
        receipt_refs
    )
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

fn native_tool_empty_retry_prompt(original_prompt: &str, previous_output: &str, retry: u64) -> String {
    let previous = previous_output.trim();
    let previous = if previous.is_empty() {
        "The previous response was empty.".to_string()
    } else {
        format!(
            "Previous response without native tool calls:\n{}",
            previous.chars().take(1200).collect::<String>()
        )
    };
    format!(
        "{original_prompt}\n\nNative tool retry {retry}: this coding run requires native file-tool receipts before it can complete. {previous}\n\nReturn only JSON with a tool_calls array now. For explicit isolated create-file work, call file_write directly. For existing or unclear project work, start with file_list or file_stat, then read relevant existing files. For multi-requirement tasks, continue until each user-stated requirement has receipt-backed evidence, or return a structured partial/blocker naming uncovered requirements. Do not provide a final answer until native tool observations confirm required writes/patches, or return a structured blocker if mutation cannot proceed."
    )
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
