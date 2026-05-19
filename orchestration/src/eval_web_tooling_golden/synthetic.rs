use serde_json::{json, Value};

use super::super::eval_research_golden_utils::{normalize_for_compare, str_at, u64_at};
use super::direct_tool::direct_tool_status;

pub(super) fn synthesize_tooling_eval_payload(
    tool_name: &str,
    request: &Value,
    direct_payload: &Value,
) -> Value {
    let tool_status = direct_tool_status(tool_name, direct_payload);
    let mut tool_row = json!({
        "name": tool_name,
        "tool": tool_name,
        "status": tool_status,
        "input": request,
    });
    if let Some(tool_map) = tool_row.as_object_mut() {
        if let Some(direct_map) = direct_payload.as_object() {
            for (key, value) in direct_map {
                if matches!(key.as_str(), "response" | "text" | "message") {
                    continue;
                }
                tool_map.insert(key.clone(), value.clone());
            }
        }
    }
    let evidence_count = u64_at(direct_payload, &["evidence_refs", "length"], 0);
    let checkpoint_status =
        if evidence_count > 0 || direct_tool_status(tool_name, direct_payload) == "ok" {
            "pass"
        } else {
            "fail"
        };
    let mut payload = json!({
        "ok": direct_payload.get("ok").and_then(Value::as_bool).unwrap_or(true),
        "response": "Tooling-only eval: direct web retrieval executed without final synthesis.",
        "text": "Tooling-only eval: direct web retrieval executed without final synthesis.",
        "message": "Tooling-only eval: direct web retrieval executed without final synthesis.",
        "pending_tool_request": {
            "status": "executed",
            "source": "web_tooling_golden_direct_route",
            "tool_name": tool_name,
            "tool_key": tool_name,
            "selected_tool_key": tool_name,
            "selected_tool_family": "web_research",
            "input": request
        },
        "tools": [tool_row.clone()],
        "response_workflow": {
            "final_llm_response": {
                "status": "tooling_only_eval"
            }
        },
        "response_finalization": {
            "outcome": "tooling_only_eval",
            "tool_completion": {
                "tool_attempts": [tool_row],
                "checkpoints": [
                    {
                        "checkpoint": "5e_agent_received_evidence_context",
                        "status": checkpoint_status
                    }
                ]
            }
        }
    });
    if let Some(payload_map) = payload.as_object_mut() {
        if let Some(direct_map) = direct_payload.as_object() {
            for (key, value) in direct_map {
                if matches!(key.as_str(), "response" | "text" | "message") {
                    continue;
                }
                payload_map.insert(key.clone(), value.clone());
            }
        }
    }
    payload
}

pub(super) fn synthetic_transition_diagnostics(
    payload: &Value,
    retrieval_quality: &Value,
) -> Value {
    let packaged = retrieval_quality
        .get("evidence_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0;
    json!({
        "checkpoints": [
            {
                "checkpoint": "5e_agent_received_evidence_context",
                "status": if packaged || payload.get("tools").and_then(Value::as_array).map(|rows| !rows.is_empty()).unwrap_or(false) { "pass" } else { "fail" }
            }
        ],
        "first_failed_checkpoint": if packaged || payload.get("tools").and_then(Value::as_array).map(|rows| !rows.is_empty()).unwrap_or(false) {
            Value::Null
        } else {
            Value::String("5e_agent_received_evidence_context".to_string())
        }
    })
}

pub(super) fn query_metadata_diagnostics(payload: &Value) -> Value {
    let request = research_pending_request(payload);
    let Some(request) = request else {
        return json!({
            "eligible_batch_query_request": false,
            "eligible_web_retrieval_request": false,
            "metadata_present": false,
            "rich_query_pack_or_narrow_marker": false,
            "query_lane_count": 0,
            "followup_query_count": 0,
            "multi_query_present": false,
            "keyword_count": 0,
            "alias_count": 0,
            "negative_term_count": 0,
            "required_coverage_entities_count": 0,
            "required_coverage_facets_count": 0,
            "fields_present": [],
            "source": "none"
        });
    };
    let mut tool = str_at(request, &["selected_tool_key"], "");
    if tool.is_empty() {
        tool = str_at(request, &["tool_key"], "");
    }
    if tool.is_empty() {
        tool = str_at(request, &["tool_name"], "");
    }
    let input = request.get("input").unwrap_or(&Value::Null);
    let normalized_tool = normalize_for_compare(&tool);
    let eligible_batch_query = normalized_tool == "batch_query";
    let eligible_web_retrieval = matches!(normalized_tool.as_str(), "batch_query" | "web_search");
    let query_lane_count = array_len(input.get("queries"));
    let followup_query_count = query_lane_count.saturating_sub(1);
    let keyword_count = array_len(input.get("keywords"));
    let alias_count = array_len(input.get("aliases"));
    let negative_term_count = array_len(input.get("negative_terms"));
    let required_coverage_entities_count =
        required_coverage_count(input.get("required_coverage"), "entities");
    let required_coverage_facets_count =
        required_coverage_count(input.get("required_coverage"), "facets");
    let fields_present = input
        .as_object()
        .map(|map| {
            [
                "queries",
                "keywords",
                "required_coverage",
                "aliases",
                "negative_terms",
                "query_metadata_policy",
            ]
            .iter()
            .filter(|field| map.contains_key(**field))
            .map(|field| (*field).to_string())
            .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let metadata_present = fields_present.iter().any(|field| {
        matches!(
            field.as_str(),
            "keywords"
                | "required_coverage"
                | "aliases"
                | "negative_terms"
                | "query_metadata_policy"
        )
    });
    let rich_query_pack = !json_array_empty(input.get("queries"))
        && (!json_array_empty(input.get("keywords"))
            || required_coverage_nonempty(input.get("required_coverage")));
    let narrow_or_expanded_marker = input
        .pointer("/query_metadata_policy/classification")
        .and_then(Value::as_str)
        .map(|raw| {
            matches!(
                raw,
                "expanded_query_pack"
                    | "narrow_lookup_or_initial_discovery"
                    | "derived_prompt_request"
            )
        })
        .unwrap_or(false);
    json!({
        "eligible_batch_query_request": eligible_batch_query,
        "eligible_web_retrieval_request": eligible_web_retrieval,
        "metadata_present": eligible_web_retrieval && metadata_present,
        "rich_query_pack_or_narrow_marker": eligible_web_retrieval && (rich_query_pack || narrow_or_expanded_marker),
        "query_lane_count": query_lane_count,
        "followup_query_count": followup_query_count,
        "multi_query_present": query_lane_count > 1,
        "keyword_count": keyword_count,
        "alias_count": alias_count,
        "negative_term_count": negative_term_count,
        "required_coverage_entities_count": required_coverage_entities_count,
        "required_coverage_facets_count": required_coverage_facets_count,
        "fields_present": fields_present,
        "tool": normalized_tool,
        "source": str_at(request, &["source"], "unknown"),
        "classification": input
            .pointer("/query_metadata_policy/classification")
            .and_then(Value::as_str)
            .unwrap_or("")
    })
}

fn research_pending_request(payload: &Value) -> Option<&Value> {
    payload
        .get("pending_tool_request")
        .or_else(|| payload.pointer("/response_workflow/pending_tool_request"))
        .or_else(|| payload.pointer("/response_workflow/manual_toolbox_pending_tool_request"))
        .or_else(|| payload.pointer("/response_finalization/pending_tool_request"))
}

fn json_array_empty(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .map(|rows| rows.is_empty())
        .unwrap_or(true)
}

fn array_len(value: Option<&Value>) -> u64 {
    value
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0)
}

fn required_coverage_nonempty(value: Option<&Value>) -> bool {
    let Some(map) = value.and_then(Value::as_object) else {
        return false;
    };
    !json_array_empty(map.get("entities")) || !json_array_empty(map.get("facets"))
}

fn required_coverage_count(value: Option<&Value>, field: &str) -> u64 {
    value
        .and_then(Value::as_object)
        .and_then(|map| map.get(field))
        .and_then(Value::as_array)
        .map(|rows| rows.len() as u64)
        .unwrap_or(0)
}
