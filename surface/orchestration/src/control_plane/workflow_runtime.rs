// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::workflow_contracts::{
    registered_workflow_graphs, NormalizedWorkflowGraph, REQUIRED_TELEMETRY_STREAMS,
    REQUIRED_TOOL_FAMILIES,
};
use super::workflow_runtime_fixtures::{
    workflow_replay_fixtures, WorkflowInput, WorkflowReplayFixture,
};
use super::workflow_runtime_types::{
    ToolFamilyDiagnostic, ToolRequestEnvelope, WorkflowBudgetSnapshot, WorkflowInspectorArtifact,
    WorkflowReplayReport, WorkflowRuntimeEvent,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedGateInput {
    NeedTools(bool),
    ToolFamily(String),
    ToolName(String),
    RequestPayload(String),
    Finish,
    AnotherTool,
}

pub fn run_registered_replay_fixtures() -> Vec<WorkflowReplayReport> {
    workflow_replay_fixtures()
        .iter()
        .map(run_workflow_replay)
        .collect()
}

pub fn workflow_runtime_contract_ok(reports: &[WorkflowReplayReport]) -> bool {
    let fixture_ids: HashSet<&str> = reports.iter().map(|row| row.fixture_id.as_str()).collect();
    [
        "direct_conversation",
        "workspace_search",
        "web_search",
        "failed_tool_retry",
        "user_aborted",
    ]
    .iter()
    .all(|id| fixture_ids.contains(id))
        && reports.iter().all(|row| row.ok)
}

pub fn run_workflow_replay(fixture: &WorkflowReplayFixture) -> WorkflowReplayReport {
    let (graph, selection_failure) = match select_runtime_workflow(fixture.workflow_id) {
        Some(graph) => (graph, None),
        None => {
            let fallback = registered_workflow_graphs()
                .into_iter()
                .next()
                .expect("registered workflow graph required");
            (
                fallback,
                Some(format!("unregistered_workflow_id:{}", fixture.workflow_id)),
            )
        }
    };
    let graph_hash = graph_hash(&graph);
    let mut state = RuntimeState::new(fixture, graph, graph_hash);
    state.event(
        "workflow_state",
        "user_input",
        "intake",
        json!({"text": fixture.user_input}),
        false,
    );
    state.event(
        "workflow_state",
        "workflow_selected",
        "intake",
        json!({
            "source": "orchestration_typed_graph_v1",
            "workflow_id": state.graph.workflow_id.clone(),
            "source_json_path": state.graph.source_json_path.clone(),
            "contract_schema_version": state.graph.contract_schema_version.clone(),
            "graph_hash": state.graph_hash.clone()
        }),
        false,
    );
    state.event(
        "workflow_state",
        "tool_family_menu_ready",
        "gate_2_tool_family_menu",
        json!({
            "families": REQUIRED_TOOL_FAMILIES,
            "llm_selects_tool_family": true,
            "automatic_tool_family_selection": false
        }),
        false,
    );
    if let Some(reason) = selection_failure {
        state.failures.push(reason.clone());
        state.event(
            "eval_trace",
            "workflow_selection_rejected",
            "intake",
            json!({
                "reason": reason,
                "requested_workflow_id": fixture.workflow_id,
                "registered_graph_required": true
            }),
            false,
        );
        state.terminal_state = Some("failed".to_string());
    }
    for input in &fixture.inputs {
        if state.terminal_state.is_some() {
            break;
        }
        state.apply_input(input);
    }
    if state.terminal_state.is_none() {
        state.terminal_state = Some("completed".to_string());
    }
    state.finish()
}

pub fn select_runtime_workflow(workflow_id: &str) -> Option<NormalizedWorkflowGraph> {
    registered_workflow_graphs()
        .into_iter()
        .find(|graph| graph.workflow_id == workflow_id)
}

pub fn graph_hash(graph: &NormalizedWorkflowGraph) -> String {
    let mut hasher = DefaultHasher::new();
    graph.workflow_id.hash(&mut hasher);
    graph.source_json_path.hash(&mut hasher);
    graph.contract_schema_version.hash(&mut hasher);
    graph.stages.hash(&mut hasher);
    graph.transitions.hash(&mut hasher);
    graph.visible_chat_policy.hash(&mut hasher);
    format!("typed_graph:{:016x}", hasher.finish())
}

pub fn adapt_tool_request(
    family: &str,
    tool_name: &str,
    request_payload: &str,
) -> Result<ToolRequestEnvelope, String> {
    if !REQUIRED_TOOL_FAMILIES.contains(&family) {
        return Err(format!("unknown_tool_family:{family}"));
    }
    if tool_name.trim().is_empty() {
        return Err("missing_tool_name".to_string());
    }
    if request_payload.trim().is_empty() {
        return Err("missing_request_payload".to_string());
    }
    Ok(ToolRequestEnvelope {
        family: family.to_string(),
        tool_name: tool_name.to_string(),
        request_payload: request_payload.to_string(),
        request_schema: format!("{family}_tool_request_v1"),
        receipt_binding_required: true,
    })
}

fn parse_gate_input(stage: &str, text: &str) -> Result<ParsedGateInput, String> {
    let normalized = text.trim().to_ascii_lowercase();
    match stage {
        "gate_1_need_tool_access_menu" => match normalized.as_str() {
            "yes" | "y" | "true" | "t" | "1" => Ok(ParsedGateInput::NeedTools(true)),
            "no" | "n" | "false" | "f" | "0" => Ok(ParsedGateInput::NeedTools(false)),
            _ => Err("gate_1_requires_yes_or_no".to_string()),
        },
        "gate_2_tool_family_menu" => {
            let family = match normalized.as_str() {
                "1" => "workspace",
                "2" => "web",
                "3" => "memory",
                "4" => "agent",
                "5" => "shell",
                "6" => "browser",
                other => other,
            };
            if REQUIRED_TOOL_FAMILIES.contains(&family) {
                Ok(ParsedGateInput::ToolFamily(family.to_string()))
            } else {
                Err(format!("unknown_tool_family:{family}"))
            }
        }
        "gate_3_tool_menu" => Ok(ParsedGateInput::ToolName(text.trim().to_string())),
        "gate_4_request_payload_input" => {
            Ok(ParsedGateInput::RequestPayload(text.trim().to_string()))
        }
        "gate_5_post_tool_menu" => match normalized.as_str() {
            "finish" | "done" | "submit" => Ok(ParsedGateInput::Finish),
            "another" | "another tool" | "use another tool" => Ok(ParsedGateInput::AnotherTool),
            _ => Err("gate_5_requires_finish_or_another_tool".to_string()),
        },
        _ => Err(format!("unknown_gate_stage:{stage}")),
    }
}

struct RuntimeState {
    fixture_id: String,
    graph: NormalizedWorkflowGraph,
    graph_hash: String,
    events: Vec<WorkflowRuntimeEvent>,
    tool_requests: Vec<ToolRequestEnvelope>,
    failures: Vec<String>,
    current_family: Option<String>,
    current_tool: Option<String>,
    terminal_state: Option<String>,
    model_turns_seen: u64,
    tool_calls_seen: u64,
    estimated_tokens_seen: u64,
    loop_signatures: BTreeMap<String, u64>,
}

impl RuntimeState {
    fn new(
        fixture: &WorkflowReplayFixture,
        graph: NormalizedWorkflowGraph,
        graph_hash: String,
    ) -> Self {
        Self {
            fixture_id: fixture.id.to_string(),
            graph,
            graph_hash,
            events: Vec::new(),
            tool_requests: Vec::new(),
            failures: Vec::new(),
            current_family: None,
            current_tool: None,
            terminal_state: None,
            model_turns_seen: 0,
            tool_calls_seen: 0,
            estimated_tokens_seen: token_estimate(fixture.user_input),
            loop_signatures: BTreeMap::new(),
        }
    }

    fn apply_input(&mut self, input: &WorkflowInput) {
        match input {
            WorkflowInput::GateText { stage, text } => self.apply_gate_text(stage, text),
            WorkflowInput::ToolObservation { ok, summary } => {
                self.apply_tool_observation(*ok, summary)
            }
            WorkflowInput::FinalAnswer(text) => {
                self.model_turns_seen += 1;
                self.estimated_tokens_seen += token_estimate(text);
                self.event(
                    "final_answer",
                    "llm_final_output",
                    "gate_6_llm_final_output",
                    json!({"text": text}),
                    true,
                );
                self.terminal_state = Some("completed".to_string());
            }
            WorkflowInput::Abort => {
                self.event(
                    "workflow_state",
                    "user_abort",
                    "aborted",
                    json!({"terminal_state": "aborted"}),
                    false,
                );
                self.terminal_state = Some("aborted".to_string());
            }
        }
        self.enforce_budget();
    }

    fn apply_gate_text(&mut self, stage: &str, text: &str) {
        self.model_turns_seen += 1;
        self.estimated_tokens_seen += token_estimate(text);
        self.event(
            "workflow_state",
            "gate_answered",
            stage,
            json!({"answer": text}),
            false,
        );
        match parse_gate_input(stage, text) {
            Ok(ParsedGateInput::NeedTools(false)) => {
                self.event(
                    "workflow_state",
                    "direct_conversation_selected",
                    stage,
                    json!({"needs_tools": false}),
                    false,
                );
            }
            Ok(ParsedGateInput::NeedTools(true)) => {
                self.event(
                    "workflow_state",
                    "tool_workflow_selected",
                    stage,
                    json!({"needs_tools": true}),
                    false,
                );
            }
            Ok(ParsedGateInput::ToolFamily(family)) => self.current_family = Some(family),
            Ok(ParsedGateInput::ToolName(tool)) => self.current_tool = Some(tool),
            Ok(ParsedGateInput::RequestPayload(payload)) => self.emit_tool_request(payload),
            Ok(ParsedGateInput::Finish) => self.terminal_state = Some("completed".to_string()),
            Ok(ParsedGateInput::AnotherTool) => self.event(
                "workflow_state",
                "loop_to_tool_menu",
                stage,
                json!({}),
                false,
            ),
            Err(reason) => {
                self.failures.push(reason.clone());
                self.event(
                    "eval_trace",
                    "gate_input_rejected",
                    stage,
                    json!({"reason": reason}),
                    false,
                );
                self.terminal_state = Some("needs_input".to_string());
            }
        }
    }

    fn emit_tool_request(&mut self, payload: String) {
        let family = self
            .current_family
            .clone()
            .unwrap_or_else(|| "workspace".to_string());
        let tool = self
            .current_tool
            .clone()
            .unwrap_or_else(|| format!("{family}_tool"));
        match adapt_tool_request(&family, &tool, &payload) {
            Ok(request) => {
                self.tool_calls_seen += 1;
                self.event(
                    "tool_trace",
                    "tool_request",
                    "gate_4_request_payload_input",
                    json!(&request),
                    false,
                );
                self.tool_requests.push(request);
            }
            Err(reason) => {
                self.failures.push(reason.clone());
                self.event(
                    "eval_trace",
                    "tool_request_rejected",
                    "gate_4_request_payload_input",
                    json!({"reason": reason}),
                    false,
                );
                self.terminal_state = Some("needs_input".to_string());
            }
        }
    }

    fn apply_tool_observation(&mut self, ok: bool, summary: &str) {
        let event_kind = if ok {
            "tool_observation"
        } else {
            "tool_failure_observation"
        };
        self.event(
            "tool_trace",
            event_kind,
            "tool_observation",
            json!({"ok": ok, "summary": summary}),
            false,
        );
        if !ok {
            self.event(
                "workflow_state",
                "recover_or_retry",
                "recovery_escalation",
                json!({"max_retries": 1}),
                false,
            );
        }
    }

    fn event(
        &mut self,
        stream: &str,
        event_kind: &str,
        stage: &str,
        payload: Value,
        visible: bool,
    ) {
        let signature = format!("{stage}:{event_kind}:{stream}");
        let count = self.loop_signatures.entry(signature).or_insert(0);
        *count += 1;
        self.events.push(WorkflowRuntimeEvent {
            seq: self.events.len() + 1,
            stage: stage.to_string(),
            event_kind: event_kind.to_string(),
            stream: stream.to_string(),
            payload,
            visible_chat_eligible: visible,
        });
    }

    fn enforce_budget(&mut self) {
        let budget = &self.graph.run_budgets;
        let stages_seen = self
            .events
            .iter()
            .map(|event| event.stage.as_str())
            .collect::<HashSet<_>>()
            .len() as u64;
        let budget_exceeded = stages_seen > budget.max_stages
            || self.model_turns_seen > budget.max_model_turns
            || self.tool_calls_seen > budget.max_tool_calls
            || self.estimated_tokens_seen > budget.token_budget;
        let loop_signature_repeated = self.loop_signatures.values().any(|count| *count > 2);
        if budget_exceeded || loop_signature_repeated {
            self.event(
                "eval_trace",
                "runtime_budget_terminated",
                "failed",
                json!({"budget_exceeded": budget_exceeded, "loop_signature_repeated": loop_signature_repeated}),
                false,
            );
            self.terminal_state = Some("failed".to_string());
        }
    }

    fn finish(self) -> WorkflowReplayReport {
        let budget = self.budget_snapshot();
        let inspector = inspector_artifact(&self.graph, &self.graph_hash, &self.events);
        let terminal_state = self
            .terminal_state
            .clone()
            .unwrap_or_else(|| "failed".to_string());
        let no_injection = self
            .events
            .iter()
            .all(|event| event.stream == "final_answer" || !event.visible_chat_eligible);
        let ok = self.failures.is_empty()
            && no_injection
            && self.graph.terminal_states.contains(&terminal_state)
            && !budget.budget_exceeded
            && !budget.loop_signature_repeated;
        WorkflowReplayReport {
            fixture_id: self.fixture_id,
            ok,
            terminal_state,
            workflow_id: self.graph.workflow_id,
            graph_hash: self.graph_hash,
            source_json_path: self.graph.source_json_path,
            contract_schema_version: self.graph.contract_schema_version,
            events: self.events,
            tool_requests: self.tool_requests,
            budget,
            inspector,
            failures: self.failures,
        }
    }

    fn budget_snapshot(&self) -> WorkflowBudgetSnapshot {
        let budget = &self.graph.run_budgets;
        let stages_seen = self
            .events
            .iter()
            .map(|event| event.stage.as_str())
            .collect::<HashSet<_>>()
            .len() as u64;
        let loop_signature_repeated = self.loop_signatures.values().any(|count| *count > 2);
        let budget_exceeded = stages_seen > budget.max_stages
            || self.model_turns_seen > budget.max_model_turns
            || self.tool_calls_seen > budget.max_tool_calls
            || self.estimated_tokens_seen > budget.token_budget;
        WorkflowBudgetSnapshot {
            max_stages: budget.max_stages,
            stages_seen,
            max_model_turns: budget.max_model_turns,
            model_turns_seen: self.model_turns_seen,
            max_tool_calls: budget.max_tool_calls,
            tool_calls_seen: self.tool_calls_seen,
            token_budget: budget.token_budget,
            estimated_tokens_seen: self.estimated_tokens_seen,
            loop_guard_active: !budget.loop_signature_detector.trim().is_empty(),
            budget_exceeded,
            loop_signature_repeated,
        }
    }
}

fn inspector_artifact(
    graph: &NormalizedWorkflowGraph,
    graph_hash: &str,
    events: &[WorkflowRuntimeEvent],
) -> WorkflowInspectorArtifact {
    let mut streams: BTreeMap<String, Vec<WorkflowRuntimeEvent>> = REQUIRED_TELEMETRY_STREAMS
        .iter()
        .map(|stream| ((*stream).to_string(), Vec::new()))
        .collect();
    for event in events {
        streams
            .entry(event.stream.clone())
            .or_default()
            .push(event.clone());
    }
    let stage_statuses = events
        .iter()
        .map(|event| json!({"seq": event.seq, "stage": event.stage, "event": event.event_kind}))
        .collect();
    WorkflowInspectorArtifact {
        workflow_id: graph.workflow_id.clone(),
        graph_hash: graph_hash.to_string(),
        source_json_path: graph.source_json_path.clone(),
        contract_schema_version: graph.contract_schema_version.clone(),
        selected_graph_source: "orchestration_typed_graph_v1".to_string(),
        stage_statuses,
        trace_streams: streams,
        tool_family_diagnostics: tool_family_diagnostics(events),
        visible_chat_source: "final_answer_stream_only".to_string(),
        system_chat_injection_allowed: false,
    }
}

fn tool_family_diagnostics(events: &[WorkflowRuntimeEvent]) -> Vec<ToolFamilyDiagnostic> {
    let selected = selected_tool_families(events);
    REQUIRED_TOOL_FAMILIES
        .iter()
        .map(|family| ToolFamilyDiagnostic {
            family: (*family).to_string(),
            status: "menu_available_probe_required_before_execution".to_string(),
            reason: "workflow_reader_exposes_family_without_autoselection".to_string(),
            selected_by_llm: selected.contains(*family),
        })
        .collect()
}

fn selected_tool_families(events: &[WorkflowRuntimeEvent]) -> HashSet<String> {
    events
        .iter()
        .filter(|event| event.stream == "tool_trace" && event.event_kind == "tool_request")
        .filter_map(|event| event.payload.get("family").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

fn token_estimate(text: &str) -> u64 {
    (text.len() as u64 / 4).max(1)
}
