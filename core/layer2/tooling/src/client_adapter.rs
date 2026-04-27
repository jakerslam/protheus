use crate::evidence_extractor::EvidenceExtractor;
use crate::evidence_store::EvidenceStore;
use crate::schemas::{ClaimBundle, WorkerBudgetUsed, WorkerOutput, WorkerTaskStatus};
use crate::tool_broker::{BrokerCaller, BrokerError, ToolBroker, ToolCallRequest};
use crate::verifier::StructuredVerifier;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientAdapterRequest {
    pub trace_id: String,
    pub task_id: String,
    pub user_input: String,
    pub web_query: Option<String>,
    pub file_path: Option<String>,
    pub file_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientDelegationResult {
    pub task_id: String,
    pub worker_output: WorkerOutput,
    pub claim_bundle: ClaimBundle,
    pub evidence_refs: Vec<String>,
    pub grounded_result: String,
}

pub trait ToolBridge {
    fn web_search(&self, args: &Value) -> Result<Value, String>;
    fn file_read(&self, args: &Value) -> Result<Value, String>;
    fn file_read_many(&self, args: &Value) -> Result<Value, String>;
    fn workspace_analyze(&self, args: &Value) -> Result<Value, String>;
}

pub struct ThinClientDelegator {
    pub broker: ToolBroker,
    pub extractor: EvidenceExtractor,
    pub evidence_store: EvidenceStore,
    pub verifier: StructuredVerifier,
}

impl Default for ThinClientDelegator {
    fn default() -> Self {
        let mut broker = ToolBroker::default();
        let _ = broker.recover_from_ledger();
        let mut evidence_store = EvidenceStore::default();
        let _ = evidence_store.recover_from_ledger();
        Self {
            broker,
            extractor: EvidenceExtractor,
            evidence_store,
            verifier: StructuredVerifier,
        }
    }
}

impl ThinClientDelegator {
    pub fn run(
        &mut self,
        request: &ClientAdapterRequest,
        bridge: &dyn ToolBridge,
    ) -> Result<ClientDelegationResult, BrokerError> {
        let mut produced_evidence_ids = Vec::<String>::new();
        let mut open_questions = Vec::<String>::new();
        let mut recommended_next_actions = Vec::<String>::new();
        let mut blockers = Vec::<String>::new();
        let mut tool_calls = 0usize;
        for (tool_name, args) in plan_tools(request) {
            let probe = self
                .broker
                .capability_probe(BrokerCaller::Client, tool_name);
            if !probe.available {
                blockers.push(format!(
                    "tool_capability_unavailable:{}:{}",
                    probe.tool_name, probe.reason
                ));
                continue;
            }
            tool_calls += 1;
            let execution = self.broker.execute_and_normalize(
                ToolCallRequest {
                    trace_id: request.trace_id.clone(),
                    task_id: request.task_id.clone(),
                    tool_name: tool_name.to_string(),
                    args,
                    lineage: vec!["thin_client_delegator".to_string()],
                    caller: BrokerCaller::Client,
                    policy_revision: Some("policy.tooling.v1".to_string()),
                    tool_version: Some(format!("{tool_name}.v1")),
                    freshness_window_ms: None,
                    force_no_dedupe: false,
                },
                |normalized_args| match tool_name {
                    "web_search" => bridge.web_search(normalized_args),
                    "file_read" => bridge.file_read(normalized_args),
                    "file_read_many" => bridge.file_read_many(normalized_args),
                    "workspace_analyze" => bridge.workspace_analyze(normalized_args),
                    _ => Err("unsupported_tool".to_string()),
                },
            )?;
            if !execution.normalized_result.errors.is_empty() {
                blockers.extend(execution.normalized_result.errors.clone());
                continue;
            }
            let cards = self
                .extractor
                .extract(&execution.normalized_result, &execution.raw_payload);
            let ids = self.evidence_store.append_evidence(&cards);
            produced_evidence_ids.extend(ids);
        }
        if produced_evidence_ids.is_empty() {
            if user_input_requests_local_workspace_only(request.user_input.as_str()) {
                open_questions.push(
                    "No workspace target was supplied for this local tooling turn.".to_string(),
                );
                recommended_next_actions.push(
                    "Provide one exact workspace path (file or folder) so I can stay local and continue."
                        .to_string(),
                );
            } else {
                open_questions.push(
                    "No usable evidence was produced. Retry with narrower web query or specific file path."
                        .to_string(),
                );
                recommended_next_actions.push(
                    "Retry with one explicit source URL or one exact workspace file path."
                        .to_string(),
                );
            }
        }
        append_synthesis_followups(
            request,
            &mut recommended_next_actions,
            &mut open_questions,
            &blockers,
            tool_calls,
        );
        recommended_next_actions.sort();
        recommended_next_actions.dedup();
        open_questions.sort();
        open_questions.dedup();
        let active_cards = produced_evidence_ids
            .iter()
            .filter_map(|id| self.evidence_store.evidence_by_id(id).cloned())
            .collect::<Vec<_>>();
        let claim_bundle = self
            .verifier
            .derive_claim_bundle(request.task_id.as_str(), &active_cards);
        if let Err(err) = self
            .verifier
            .validate_claim_evidence_refs(&claim_bundle, &active_cards)
        {
            blockers.push(err);
        }
        let supported_claims = self.verifier.supported_claims_for_synthesis(&claim_bundle);
        if supported_claims.is_empty() && blockers.is_empty() {
            blockers.push("no_supported_claims".to_string());
        }
        let status = if !blockers.is_empty() {
            WorkerTaskStatus::Blocked
        } else if produced_evidence_ids.is_empty() {
            WorkerTaskStatus::Failed
        } else {
            WorkerTaskStatus::Completed
        };
        let worker_output = WorkerOutput {
            task_id: request.task_id.clone(),
            status,
            produced_evidence_ids: produced_evidence_ids.clone(),
            open_questions,
            recommended_next_actions,
            blockers,
            budget_used: WorkerBudgetUsed {
                tool_calls,
                input_tokens: estimate_tokens(request.user_input.as_str()),
                output_tokens: supported_claims
                    .iter()
                    .map(|claim| estimate_tokens(claim.text.as_str()))
                    .sum(),
            },
        };
        let grounded_result = render_grounded_result(&claim_bundle);
        Ok(ClientDelegationResult {
            task_id: request.task_id.clone(),
            worker_output,
            claim_bundle,
            evidence_refs: produced_evidence_ids,
            grounded_result,
        })
    }

    pub fn worker_direct_tool_call_attempt(&self) -> Result<(), BrokerError> {
        self.broker.direct_tool_bypass_attempt(BrokerCaller::Worker)
    }

    pub fn client_direct_tool_call_attempt(&self) -> Result<(), BrokerError> {
        self.broker.direct_tool_bypass_attempt(BrokerCaller::Client)
    }
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn estimate_tokens(raw: &str) -> usize {
    (clean_text(raw, 8000).len() / 4).max(1)
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn user_input_requests_local_workspace_only(user_input: &str) -> bool {
    let lowered = user_input.to_ascii_lowercase();
    let local_tokens = [
        "workspace",
        "local",
        "directory",
        "folder",
        "file tooling",
        "file tool",
        "repo",
        "path",
    ];
    let web_tokens = [
        "http://", "https://", "web", "internet", "online", "browser",
    ];
    contains_any(lowered.as_str(), &local_tokens) && !contains_any(lowered.as_str(), &web_tokens)
}

fn user_input_requests_tooling_surface(user_input: &str) -> bool {
    let lowered = user_input.to_ascii_lowercase();
    contains_any(
        lowered.as_str(),
        &[
            "mcp",
            "tool route",
            "tooling",
            "gateway",
            "/mcp",
            "/tool",
            "/tools",
            "feature flag",
            "remote config",
            "telemetry",
        ],
    )
}

fn user_input_requests_worker_queue_context(user_input: &str) -> bool {
    let lowered = user_input.to_ascii_lowercase();
    contains_any(
        lowered.as_str(),
        &["worker", "queue", "backfill", "sync", "retry", "backoff"],
    )
}

fn append_synthesis_followups(
    request: &ClientAdapterRequest,
    recommended_next_actions: &mut Vec<String>,
    open_questions: &mut Vec<String>,
    blockers: &[String],
    tool_calls: usize,
) {
    if user_input_requests_tooling_surface(request.user_input.as_str()) {
        recommended_next_actions.push(
            "Include an explicit tool route scope (mcp, workspace, or web) to keep routing deterministic."
                .to_string(),
        );
    }
    if user_input_requests_worker_queue_context(request.user_input.as_str()) {
        recommended_next_actions.push(
            "Attach queue context (lane, retry budget, and backoff window) so worker recommendations stay reproducible."
                .to_string(),
        );
    }
    if tool_calls == 0 {
        open_questions.push(
            "No governed tool routes executed. Should this turn force a local workspace route, web route, or mixed route?"
                .to_string(),
        );
    }
    if blockers
        .iter()
        .any(|row| row.contains("tool_capability_unavailable"))
    {
        recommended_next_actions.push(
            "Inspect capability probe status and re-run with a route that is marked available."
                .to_string(),
        );
    }
}

fn plan_tools(request: &ClientAdapterRequest) -> Vec<(&'static str, Value)> {
    let mut out = Vec::<(&'static str, Value)>::new();
    let user_input = request.user_input.to_ascii_lowercase();
    let web_query = request
        .web_query
        .as_deref()
        .map(|v| clean_text(v, 1000))
        .unwrap_or_else(|| clean_text(&request.user_input, 1000));
    let file_path = request
        .file_path
        .as_deref()
        .map(|v| clean_text(v, 2000))
        .unwrap_or_default();
    let file_paths = request
        .file_paths
        .iter()
        .map(|v| clean_text(v, 2000))
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    let local_workspace_only_turn = user_input_requests_local_workspace_only(user_input.as_str());
    let tooling_surface_turn = user_input_requests_tooling_surface(user_input.as_str());
    let explicit_web_query = !web_query.is_empty();
    let wants_web = explicit_web_query
        || ((user_input.contains("search") || user_input.contains("web"))
            && !local_workspace_only_turn);
    let wants_file = user_input.contains("file")
        || user_input.contains("read")
        || user_input.contains("workspace")
        || user_input.contains("directory")
        || user_input.contains("folder")
        || !file_path.is_empty()
        || !file_paths.is_empty();
    if tooling_surface_turn {
        out.push((
            "workspace_analyze",
            json!({"query": format!("tooling route synthesis: {}", clean_text(request.user_input.as_str(), 800))}),
        ));
    }
    if wants_web {
        out.push(("web_search", json!({"query": web_query})));
    }
    if !file_paths.is_empty() {
        out.push(("file_read_many", json!({"paths": file_paths})));
    } else if wants_file && !file_path.is_empty() {
        out.push(("file_read", json!({"path": file_path})));
    }
    if out.is_empty() && local_workspace_only_turn {
        out.push((
            "workspace_analyze",
            json!({"query": format!("workspace-only request: {}", clean_text(request.user_input.as_str(), 800))}),
        ));
    }
    if out.is_empty() && !local_workspace_only_turn {
        out.push(("web_search", json!({"query": web_query})));
    }
    out
}

fn render_grounded_result(bundle: &ClaimBundle) -> String {
    let mut lines = Vec::<String>::new();
    for claim in &bundle.claims {
        if !matches!(
            claim.status,
            crate::schemas::ClaimStatus::Supported | crate::schemas::ClaimStatus::Partial
        ) {
            continue;
        }
        let refs = if claim.evidence_ids.is_empty() {
            "[]".to_string()
        } else {
            format!("[{}]", claim.evidence_ids.join(", "))
        };
        lines.push(format!("- {} {}", claim.text, refs));
    }
    if lines.is_empty() {
        return "No supported claims available yet.".to_string();
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBridge;

    impl ToolBridge for MockBridge {
        fn web_search(&self, args: &Value) -> Result<Value, String> {
            let query = args.get("query").and_then(Value::as_str).unwrap_or("");
            Ok(json!({
                "results": [
                    {
                        "url": "https://example.com/search",
                        "summary": format!("web result for {query}"),
                        "excerpt": "measured throughput improved by 25%"
                    }
                ]
            }))
        }

        fn file_read(&self, args: &Value) -> Result<Value, String> {
            let path = args.get("path").and_then(Value::as_str).unwrap_or("");
            Ok(json!({
                "path": path,
                "summary": format!("file summary for {path}"),
                "excerpt": "config sets retry_backoff_ms=400"
            }))
        }

        fn file_read_many(&self, args: &Value) -> Result<Value, String> {
            let rows = args
                .get("paths")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            Ok(json!({
                "results": rows.iter().enumerate().map(|(idx, row)| {
                    json!({
                        "source_ref": row.as_str().unwrap_or(""),
                        "source_location": format!("paths[{idx}]"),
                        "summary": format!("summary for {}", row.as_str().unwrap_or("")),
                        "excerpt": "batched file evidence"
                    })
                }).collect::<Vec<_>>()
            }))
        }

        fn workspace_analyze(&self, args: &Value) -> Result<Value, String> {
            let query = args.get("query").and_then(Value::as_str).unwrap_or("");
            Ok(json!({
                "results": [
                    {
                        "source_ref": "workspace://analysis",
                        "summary": format!("workspace analyze result for {query}"),
                        "excerpt": "tooling route available; queue pressure nominal"
                    }
                ]
            }))
        }
    }

    fn request(
        user_input: &str,
        web_query: Option<&str>,
        file_path: Option<&str>,
    ) -> ClientAdapterRequest {
        ClientAdapterRequest {
            trace_id: "trace-1".to_string(),
            task_id: "task-1".to_string(),
            user_input: user_input.to_string(),
            web_query: web_query.map(|v| v.to_string()),
            file_path: file_path.map(|v| v.to_string()),
            file_paths: Vec::new(),
        }
    }

    #[test]
    fn typed_worker_output_does_not_include_prose_or_raw_dump_fields() {
        let mut delegator = ThinClientDelegator::default();
        let result = delegator
            .run(
                &request("search web benchmarks", Some("benchmarks"), None),
                &MockBridge,
            )
            .expect("run");
        let as_json = serde_json::to_value(&result.worker_output).expect("serialize");
        let mut keys = as_json
            .as_object()
            .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        keys.sort();
        let mut expected = vec![
            "task_id",
            "status",
            "produced_evidence_ids",
            "open_questions",
            "recommended_next_actions",
            "blockers",
            "budget_used",
        ];
        expected.sort();
        assert_eq!(keys, expected);
    }

    #[test]
    fn end_to_end_web_only_task_produces_claims() {
        let mut delegator = ThinClientDelegator::default();
        let out = delegator
            .run(
                &request("search web for results", Some("agent benchmarks"), None),
                &MockBridge,
            )
            .expect("run");
        assert!(!out.worker_output.produced_evidence_ids.is_empty());
        assert!(!out.claim_bundle.claims.is_empty());
    }

    #[test]
    fn end_to_end_file_only_task_produces_claims() {
        let mut delegator = ThinClientDelegator::default();
        let out = delegator
            .run(
                &request("read file for retry policy", None, Some("config/app.yml")),
                &MockBridge,
            )
            .expect("run");
        assert!(!out.worker_output.produced_evidence_ids.is_empty());
        assert!(out.grounded_result.contains("["));
    }

    #[test]
    fn end_to_end_mixed_sources_task_produces_claims() {
        let mut delegator = ThinClientDelegator::default();
        let out = delegator
            .run(
                &request(
                    "search web and read file for comparison",
                    Some("framework comparison"),
                    Some("README.md"),
                ),
                &MockBridge,
            )
            .expect("run");
        assert!(!out.worker_output.produced_evidence_ids.is_empty());
        assert!(out.claim_bundle.coverage_score > 0.0);
    }

    #[test]
    fn tooling_surface_requests_route_through_workspace_analyze() {
        let mut delegator = ThinClientDelegator::default();
        let out = delegator
            .run(
                &request("check mcp tooling route and gateway status", None, None),
                &MockBridge,
            )
            .expect("run");
        assert!(!out.worker_output.produced_evidence_ids.is_empty());
        assert!(out
            .worker_output
            .recommended_next_actions
            .iter()
            .any(|row| row.contains("tool route scope")));
    }

    #[test]
    fn workers_and_clients_cannot_bypass_tool_broker() {
        let delegator = ThinClientDelegator::default();
        assert!(matches!(
            delegator.worker_direct_tool_call_attempt(),
            Err(BrokerError::DirectToolBypassDenied(_))
        ));
        assert!(matches!(
            delegator.client_direct_tool_call_attempt(),
            Err(BrokerError::DirectToolBypassDenied(_))
        ));
    }

    #[test]
    fn claim_bundle_claims_always_reference_existing_evidence() {
        let mut delegator = ThinClientDelegator::default();
        let out = delegator
            .run(
                &request("search web for results", Some("agent benchmarks"), None),
                &MockBridge,
            )
            .expect("run");
        let evidence_set = out
            .worker_output
            .produced_evidence_ids
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        assert!(
            !evidence_set.is_empty(),
            "expected evidence to be produced for claim validation"
        );
        for claim in &out.claim_bundle.claims {
            assert!(
                !claim.evidence_ids.is_empty(),
                "claim {} missing evidence refs",
                claim.claim_id
            );
            for evidence_id in &claim.evidence_ids {
                assert!(
                    evidence_set.contains(evidence_id),
                    "claim {} points to unknown evidence {}",
                    claim.claim_id,
                    evidence_id
                );
            }
        }
    }
}
