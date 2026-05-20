use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::super::eval_research_golden_utils::{
    clean_text, normalize_for_compare, read_json, str_at, string_array_at,
};

pub(super) fn request_pack_for_case(
    case: &Value,
    report_request: Option<&Value>,
    default_tool: &str,
) -> Value {
    if let Some(request) = report_request.filter(report_request_usable) {
        return json!({
            "request_pack_source": "research_report_pending_tool_request",
            "tool_name": str_at(request, &["tool_name"], default_tool),
            "input": request.get("input").cloned().unwrap_or_else(|| json!({}))
        });
    }
    if let Some(request) = case.get("tooling_request").and_then(Value::as_object) {
        return json!({
            "request_pack_source": "case_tooling_request",
            "tool_name": request
                .get("tool_name")
                .and_then(Value::as_str)
                .unwrap_or(default_tool),
            "input": Value::Object(request.clone())
        });
    }
    let prompt = str_at(case, &["prompt"], "");
    let required_entities = string_array_at(case, &["required_entities"]);
    let (coverage_entities, coverage_facets) =
        partition_required_coverage_terms(&prompt, &required_entities);
    let keywords = derived_keywords(&prompt, &required_entities);
    let queries = derived_queries(&prompt, &coverage_facets);
    json!({
        "request_pack_source": "derived_minimal_prompt_request",
        "tool_name": default_tool,
        "input": {
            "source": "web",
            "query": prompt,
            "queries": queries,
            "keywords": keywords,
            "required_coverage": {
                "entities": coverage_entities,
                "facets": coverage_facets
            },
            "aperture": "medium",
            "query_metadata_policy": {
                "classification": "derived_prompt_request"
            }
        }
    })
}

fn derived_keywords(prompt: &str, required_entities: &[String]) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for entity in required_entities {
        let cleaned = clean_text(entity, 160);
        if !cleaned.is_empty() && !out.iter().any(|current| current == &cleaned) {
            out.push(cleaned);
        }
    }
    let normalized = normalize_for_compare(prompt);
    for token in normalized.split_whitespace() {
        let cleaned = derived_keyword_token(token);
        if cleaned.len() < 4 {
            continue;
        }
        if matches!(
            cleaned.as_str(),
            "with"
                | "that"
                | "from"
                | "into"
                | "give"
                | "what"
                | "when"
                | "where"
                | "which"
                | "would"
                | "about"
                | "using"
                | "research"
                | "compare"
                | "practical"
                | "tradeoffs"
        ) {
            continue;
        }
        if !out
            .iter()
            .any(|current| normalize_for_compare(current) == cleaned)
        {
            out.push(cleaned);
        }
        if out.len() >= 12 {
            break;
        }
    }
    out
}

fn partition_required_coverage_terms(
    prompt: &str,
    required_entities: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut entities = Vec::<String>::new();
    let mut facets = Vec::<String>::new();
    for term in required_entities {
        let cleaned = clean_text(term, 160);
        if cleaned.is_empty() {
            continue;
        }
        if looks_like_named_subject(&cleaned, prompt) {
            if !entities.iter().any(|current| current == &cleaned) {
                entities.push(cleaned);
            }
        } else if !facets.iter().any(|current| current == &cleaned) {
            facets.push(cleaned);
        }
    }
    (entities, facets)
}

fn looks_like_named_subject(term: &str, prompt: &str) -> bool {
    if term.chars().any(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit()) {
        return true;
    }
    if term.contains('/') || term.contains('.') || term.contains('-') {
        return true;
    }
    prompt.contains(term)
        && term
            .split_whitespace()
            .any(|token| token.chars().next().map(|ch| ch.is_ascii_uppercase()).unwrap_or(false))
}

fn derived_queries(prompt: &str, coverage_facets: &[String]) -> Vec<String> {
    let mut queries = vec![clean_text(prompt, 600)];
    for facet in coverage_facets.iter().take(2) {
        let facet = clean_text(facet, 160);
        if facet.is_empty() {
            continue;
        }
        let followup = format!("{facet} source-backed evidence");
        if !queries.iter().any(|current| current == &followup) {
            queries.push(followup);
        }
    }
    queries
}

fn derived_keyword_token(raw: &str) -> String {
    let normalized = normalize_for_compare(raw);
    let trimmed = normalized
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
        .to_string();
    clean_text(&trimmed, 64)
}

pub(super) fn load_request_pack_index(path: &str) -> BTreeMap<String, Value> {
    let report = read_json(path);
    report
        .get("cases")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let case_id = str_at(row, &["case_id"], "");
                    if case_id.is_empty() {
                        return None;
                    }
                    let request = row
                        .pointer("/response_diagnostics/pending_tool_request")
                        .or_else(|| {
                            row.pointer(
                                "/turn_sequence/initial_response_diagnostics/pending_tool_request",
                            )
                        })
                        .filter(report_request_usable)
                        .cloned()?;
                    Some((case_id, request))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default()
}

fn report_request_usable(request: &&Value) -> bool {
    let Some(request_obj) = request.as_object() else {
        return false;
    };
    if let Some(input_obj) = request.get("input").and_then(Value::as_object) {
        if !input_obj.is_empty() {
            return true;
        }
    }
    request_obj.contains_key("query")
        || request_obj.contains_key("queries")
        || request_obj.contains_key("url")
        || request_obj.contains_key("locator")
}
