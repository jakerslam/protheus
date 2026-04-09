// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use protheus_memory_core_v1::{
    CapabilityAction, CapabilityToken, Classification, DefaultVerityMemoryPolicy, MemoryObject,
    MemoryScope, NexusRouteContext, TrustState, UnifiedMemoryHeap,
};
use protheus_tooling_core_v1::{
    BrokerCaller, EvidenceExtractor, EvidenceStore, StructuredVerifier, ToolBroker,
    ToolCallRequest, WorkerBudgetUsed, WorkerOutput, WorkerTaskStatus,
};
use serde_json::{json, Map, Value};

use crate::deterministic_receipt_hash;

#[derive(Debug, Clone)]
pub struct GovernedWorkflowExecution {
    pub workflow_id: String,
    pub payload: Value,
}

pub fn execute_governed_workflow(
    framework: &str,
    payload: &Map<String, Value>,
) -> Result<GovernedWorkflowExecution, String> {
    let framework_id = clean_token(framework, 80);
    let task_id = payload
        .get("task_id")
        .and_then(Value::as_str)
        .map(|v| clean_token(v, 160))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            format!(
                "task_{}",
                short_hash(&json!({
                    "framework": framework_id,
                    "seed": payload
                }))
            )
        });
    let trace_id = payload
        .get("trace_id")
        .and_then(Value::as_str)
        .map(|v| clean_token(v, 160))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            format!(
                "trace_{}",
                short_hash(&json!({
                    "task_id": task_id,
                    "framework": framework_id
                }))
            )
        });
    let tool_name = payload
        .get("tool_name")
        .and_then(Value::as_str)
        .map(|v| clean_token(v, 120).to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "batch_query".to_string());
    let tool_args = normalized_tool_args(payload, tool_name.as_str());

    let mut broker = ToolBroker::default();
    let _ = broker.recover_from_ledger();
    let extractor = EvidenceExtractor;
    let mut store = EvidenceStore::default();
    let _ = store.recover_from_ledger();
    let verifier = StructuredVerifier;
    let request = ToolCallRequest {
        trace_id: trace_id.clone(),
        task_id: task_id.clone(),
        tool_name: tool_name.clone(),
        args: tool_args.clone(),
        lineage: vec![
            "framework_adapter_contract".to_string(),
            format!("framework:{framework_id}"),
        ],
        caller: BrokerCaller::Worker,
        policy_revision: Some("policy.tooling.framework_adapter.v1".to_string()),
        tool_version: Some(format!("{tool_name}.v1")),
        freshness_window_ms: None,
        force_no_dedupe: false,
    };
    let raw_seed = payload
        .get("raw_result")
        .cloned()
        .or_else(|| payload.get("tool_result").cloned());
    let execution = broker
        .execute_and_normalize(request, |normalized_args| {
            if let Some(raw) = raw_seed.clone() {
                return Ok(raw);
            }
            Ok(default_raw_payload(
                framework_id.as_str(),
                tool_name.as_str(),
                normalized_args,
            ))
        })
        .map_err(|err| err.as_message())?;
    let cards = extractor.extract(&execution.normalized_result, &execution.raw_payload);
    let evidence_ids = store.append_evidence(&cards);
    let bundle = verifier.derive_claim_bundle(task_id.as_str(), &cards);
    let claim_ref_validation = verifier.validate_claim_evidence_refs(&bundle, &cards).err();
    let supported_claims = verifier
        .supported_claims_for_synthesis(&bundle)
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    let status = if evidence_ids.is_empty() || claim_ref_validation.is_some() {
        WorkerTaskStatus::Blocked
    } else {
        WorkerTaskStatus::Completed
    };
    let mut blockers = execution.normalized_result.errors.clone();
    if let Some(validation_error) = claim_ref_validation {
        blockers.push(validation_error);
    }
    let worker_output = WorkerOutput {
        task_id: task_id.clone(),
        status,
        produced_evidence_ids: evidence_ids.clone(),
        open_questions: if evidence_ids.is_empty() {
            vec!["No evidence cards were extracted from the normalized tool result.".to_string()]
        } else {
            Vec::new()
        },
        recommended_next_actions: if evidence_ids.is_empty() {
            vec!["Retry with narrower arguments and rerun through the Tool Broker.".to_string()]
        } else {
            Vec::new()
        },
        blockers,
        budget_used: WorkerBudgetUsed {
            tool_calls: 1,
            input_tokens: estimate_tokens(&tool_args),
            output_tokens: estimate_tokens(&execution.raw_payload),
        },
    };

    let memory = persist_to_unified_memory(
        framework_id.as_str(),
        task_id.as_str(),
        trace_id.as_str(),
        &execution.normalized_result,
        &evidence_ids,
        &bundle,
    )?;

    let workflow_id = format!(
        "gwf_{}",
        short_hash(&json!({
            "framework": framework_id,
            "task_id": task_id,
            "trace_id": trace_id,
            "result_id": execution.normalized_result.result_id
        }))
    );
    Ok(GovernedWorkflowExecution {
        workflow_id: workflow_id.clone(),
        payload: json!({
            "ok": true,
            "workflow_id": workflow_id,
            "framework": framework_id,
            "trace_id": trace_id,
            "task_id": task_id,
            "schema_contract": protheus_tooling_core_v1::published_schema_contract_v1(),
            "normalized_result": execution.normalized_result,
            "raw_payload": execution.raw_payload,
            "evidence_cards": cards,
            "evidence_store_records": store.records(),
            "worker_output": worker_output,
            "claim_bundle": bundle,
            "synthesis_input": {
                "claims": supported_claims
            },
            "memory": memory
        }),
    })
}

fn persist_to_unified_memory(
    framework_id: &str,
    task_id: &str,
    trace_id: &str,
    normalized_result: &protheus_tooling_core_v1::NormalizedToolResult,
    evidence_ids: &[String],
    bundle: &protheus_tooling_core_v1::ClaimBundle,
) -> Result<Value, String> {
    let mut heap = UnifiedMemoryHeap::new(DefaultVerityMemoryPolicy);
    let principal_id = format!("core:framework_adapter:{framework_id}");
    let capability = CapabilityToken {
        token_id: format!("cap_{}", short_hash(&json!({ "task_id": task_id }))),
        principal_id: principal_id.clone(),
        scopes: vec![MemoryScope::Core],
        allowed_actions: vec![
            CapabilityAction::Read,
            CapabilityAction::Write,
            CapabilityAction::MaterializeContext,
        ],
        expires_at_ms: u64::MAX,
        verity_class: "standard".to_string(),
        receipt_id: format!(
            "cap_receipt_{}",
            short_hash(&json!({ "trace_id": trace_id }))
        ),
    };
    let route = NexusRouteContext {
        issuer: "framework_adapter_contract".to_string(),
        source: format!("framework:{framework_id}"),
        target: "memory_heap".to_string(),
        schema_id: "framework.adapter.governed_workflow".to_string(),
        lease_id: format!("lease_{}", short_hash(&json!({ "task_id": task_id }))),
        template_version_id: Some("v1".to_string()),
        ttl_ms: Some(300_000),
    };
    let object_id = format!(
        "wfmem_{}",
        short_hash(&json!({
            "framework": framework_id,
            "task_id": task_id,
            "trace_id": trace_id
        }))
    );
    let object = MemoryObject {
        object_id: object_id.clone(),
        scope: MemoryScope::Core,
        classification: Classification::Internal,
        namespace: "framework.adapter.workflow".to_string(),
        key: clean_token(task_id, 160),
        payload: json!({
            "framework": framework_id,
            "trace_id": trace_id,
            "task_id": task_id,
            "tool_result_id": normalized_result.result_id,
            "tool_name": normalized_result.tool_name,
            "evidence_ids": evidence_ids,
            "claim_bundle_id": bundle.claim_bundle_id,
            "coverage_score": bundle.coverage_score
        }),
        metadata: json!({
            "source": "framework_adapter_contract",
            "trace_id": trace_id,
            "task_id": task_id
        }),
        created_at_ms: 0,
        updated_at_ms: 0,
    };
    let lineage_refs = vec![
        format!("framework:{framework_id}"),
        format!("trace:{trace_id}"),
        format!("task:{task_id}"),
        format!("tool_result:{}", normalized_result.result_id),
        format!("claim_bundle:{}", bundle.claim_bundle_id),
    ];
    let version = heap.write_memory_object(
        &route,
        principal_id.as_str(),
        &capability,
        object,
        TrustState::Corroborated,
        lineage_refs.clone(),
    )?;
    let context = heap.materialize_context_stack(
        &route,
        principal_id.as_str(),
        &capability,
        vec![MemoryScope::Core],
        lineage_refs,
    )?;
    let canonical = heap
        .canonical_head_record(principal_id.as_str(), &capability, object_id.as_str())?
        .map(|row| serde_json::to_value(row).unwrap_or_else(|_| json!(null)))
        .unwrap_or_else(|| json!(null));
    Ok(json!({
        "object_id": object_id,
        "version_id": version.version_id,
        "receipt_id": version.receipt_id,
        "canonical_head_record": canonical,
        "context_manifest": context.manifest,
        "context_entries": context.entries,
        "memory_receipts": heap.receipts(),
        "replay_rows": heap.replay_mutation_rows()
    }))
}

fn normalized_tool_args(payload: &Map<String, Value>, tool_name: &str) -> Value {
    let mut args = payload
        .get("tool_args")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if !args.contains_key("query") {
        if let Some(query) = payload.get("query").and_then(Value::as_str) {
            args.insert("query".to_string(), json!(clean_text(query, 1200)));
        } else if matches!(tool_name, "web_search" | "batch_query") {
            args.insert("query".to_string(), json!("adapter workflow synthesis"));
        }
    }
    if !args.contains_key("url") {
        if let Some(url) = payload.get("url").and_then(Value::as_str) {
            args.insert("url".to_string(), json!(clean_text(url, 2000)));
        }
    }
    if !args.contains_key("path") {
        if let Some(path) = payload.get("path").and_then(Value::as_str) {
            args.insert("path".to_string(), json!(clean_text(path, 2000)));
        }
    }
    if !args.contains_key("paths") {
        if let Some(paths) = payload.get("paths").and_then(Value::as_array) {
            args.insert("paths".to_string(), Value::Array(paths.clone()));
        }
    }
    Value::Object(args)
}

fn default_raw_payload(framework_id: &str, tool_name: &str, args: &Value) -> Value {
    match tool_name {
        "web_search" | "batch_query" => json!({
            "results": [
                {
                    "source": format!("{framework_id}_adapter"),
                    "title": "governed workflow synthetic result",
                    "summary": format!(
                        "Adapter {} executed {} through the canonical Tool Broker path.",
                        framework_id, tool_name
                    ),
                    "excerpt": format!("args={}", clean_text(&args.to_string(), 240))
                }
            ]
        }),
        "web_fetch" => json!({
            "url": args.get("url").cloned().unwrap_or_else(|| json!("")),
            "title": "governed workflow synthetic fetch",
            "excerpt": format!("framework={} tool={}", framework_id, tool_name)
        }),
        "file_read" => json!({
            "path": args.get("path").cloned().unwrap_or_else(|| json!("")),
            "summary": "synthetic file read result from governed adapter path",
            "excerpt": format!("framework={} tool={}", framework_id, tool_name)
        }),
        "file_read_many" => json!({
            "paths": args.get("paths").cloned().unwrap_or_else(|| json!([])),
            "summary": "synthetic multi-file read result from governed adapter path"
        }),
        _ => json!({
            "message": "synthetic governed workflow result",
            "framework": framework_id,
            "tool_name": tool_name,
            "args": args
        }),
    }
}

fn estimate_tokens(value: &Value) -> usize {
    let raw = clean_text(&value.to_string(), 12_000);
    (raw.len() / 4).max(1)
}

fn short_hash(value: &Value) -> String {
    deterministic_receipt_hash(value).chars().take(24).collect()
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.trim().chars().take(max_len).collect::<String>()
}

fn clean_token(raw: &str, max_len: usize) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_' || *ch == ':')
        .take(max_len)
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governed_workflow_emits_claims_evidence_and_memory_lineage() {
        let payload = json!({
            "task_id": "task_adapter_contract",
            "trace_id": "trace_adapter_contract",
            "tool_name": "web_search",
            "tool_args": {
                "query": "infring framework adapters"
            }
        });
        let out = execute_governed_workflow("langgraph", payload.as_object().expect("obj"))
            .expect("governed workflow");
        assert!(out.workflow_id.starts_with("gwf_"));
        assert_eq!(out.payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.payload
                .pointer("/normalized_result/tool_name")
                .and_then(Value::as_str),
            Some("web_search")
        );
        assert!(out
            .payload
            .pointer("/worker_output/produced_evidence_ids")
            .and_then(Value::as_array)
            .is_some_and(|rows| !rows.is_empty()));
        assert!(out
            .payload
            .pointer("/memory/memory_receipts")
            .and_then(Value::as_array)
            .is_some_and(|rows| !rows.is_empty()));
    }

    #[test]
    fn governed_workflow_rejects_unauthorized_tool_name() {
        let payload = json!({
            "task_id": "task_blocked",
            "trace_id": "trace_blocked",
            "tool_name": "spawn_subagents",
            "tool_args": {}
        });
        let err = execute_governed_workflow("openai_agents", payload.as_object().expect("obj"))
            .expect_err("unauthorized");
        assert!(err.contains("unauthorized_tool_request"));
    }
}
