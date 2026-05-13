// Layer ownership: orchestration (non-canonical workflow-CD composition checks only).
use super::workflow_contracts::{registered_workflow_registry, WorkflowRegistryEntry};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const COMPOSITION_CONTRACT_VERSION: &str = "workflow_composition_contract_v1";
const TERMINAL_ARTIFACT_SCHEMA: &str = "workflow_terminal_artifact_v1";
const RETURN_POLICY: &str = "return_single_terminal_artifact_to_parent";
const TRACE_VISIBILITY: &str = "refs_only";

const ALLOWED_CD_KINDS: &[&str] = &["primitive", "composite"];
const ALLOWED_TERMINAL_ARTIFACTS: &[&str] = &[
    "completed_final_answer",
    "clarification_request",
    "structured_failure",
    "delegated_workflow_result",
];

pub fn workflow_composition_contract_report() -> Value {
    let entries = registered_workflow_registry();
    let registry = workflow_registry_value();
    let default_workflow_id = registry
        .as_ref()
        .and_then(|value| value.get("default_workflow_id"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let known_ids: HashSet<String> = entries
        .iter()
        .map(|entry| entry.workflow_id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    let workflows = entries
        .iter()
        .map(|entry| validate_entry(entry, &known_ids, &default_workflow_id))
        .collect::<Vec<_>>();
    let ok = !workflows.is_empty()
        && !default_workflow_id.is_empty()
        && workflows
            .iter()
            .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    json!({
        "type": "workflow_composition_contract_report",
        "schema_version": 1,
        "ok": ok,
        "default_workflow_id": default_workflow_id,
        "checks": {
            "all_workflow_cds_declare_composition_contract": workflows.iter().all(|row| {
                !row.pointer("/composition_contract/version")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .is_empty()
            }),
            "terminal_artifacts_are_single_typed_returns": workflows.iter().all(|row| {
                row.pointer("/composition_contract/returns_exactly_one_terminal_artifact")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            }),
            "default_workflow_delegates_online_research": workflows.iter().any(|row| {
                row.get("workflow_id").and_then(Value::as_str) == Some(default_workflow_id.as_str())
                    && row.get("online_research_delegation")
                        .and_then(Value::as_str)
                        == Some("research_synthesize_verify")
            }),
        },
        "workflows": workflows,
    })
}

pub fn workflow_composition_contract_ok() -> bool {
    workflow_composition_contract_report()
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn validate_entry(
    entry: &WorkflowRegistryEntry,
    known_ids: &HashSet<String>,
    default_workflow_id: &str,
) -> Value {
    let mut errors = Vec::new();
    let source = read_workflow_source(&entry.source_path);
    let value = source
        .as_ref()
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok());
    if source.is_err() || value.is_none() {
        errors.push("workflow_source_unreadable_or_invalid_json".to_string());
    }
    let empty = json!({});
    let value = value.unwrap_or(empty);
    let contract = value.get("workflow_composition_contract");
    if contract.is_none() {
        errors.push("missing_workflow_composition_contract".to_string());
    }
    let contract = contract.unwrap_or(&Value::Null);
    validate_composition_contract(
        &entry.workflow_id,
        contract,
        &value,
        known_ids,
        default_workflow_id,
        &mut errors,
    );
    let online_research_delegation = child_workflow_calls(contract)
        .into_iter()
        .find(|call| json_str(call, "capability") == "online_research")
        .map(|call| json_str(call, "workflow_id").to_string())
        .unwrap_or_default();
    json!({
        "workflow_id": entry.workflow_id,
        "source_path": entry.source_path,
        "ok": errors.is_empty(),
        "errors": errors,
        "online_research_delegation": online_research_delegation,
        "composition_contract": contract,
    })
}

fn validate_composition_contract(
    workflow_id: &str,
    contract: &Value,
    workflow: &Value,
    known_ids: &HashSet<String>,
    default_workflow_id: &str,
    errors: &mut Vec<String>,
) {
    if json_str(contract, "version") != COMPOSITION_CONTRACT_VERSION {
        errors.push("invalid_workflow_composition_contract_version".to_string());
    }
    let cd_kind = json_str(contract, "cd_kind");
    if !ALLOWED_CD_KINDS.contains(&cd_kind) {
        errors.push("invalid_cd_kind".to_string());
    }
    if !json_bool(contract, "returns_exactly_one_terminal_artifact") {
        errors.push("composition_must_return_exactly_one_terminal_artifact".to_string());
    }
    validate_terminal_artifact_contract(contract, cd_kind, errors);
    let child_calls = child_workflow_calls(contract);
    if cd_kind == "primitive" && !child_calls.is_empty() {
        errors.push("primitive_workflow_must_not_call_child_workflows".to_string());
    }
    validate_child_calls(workflow_id, &child_calls, known_ids, errors);
    if workflow_id == default_workflow_id
        && !child_calls.iter().any(|call| {
            json_str(call, "capability") == "online_research"
                && json_str(call, "workflow_id") == "research_synthesize_verify"
        })
    {
        errors.push("default_workflow_missing_online_research_delegation".to_string());
    }
    if workflow.get("research_evidence_quality_contract").is_some() && cd_kind == "primitive" {
        errors.push("research_capability_workflow_must_not_be_declared_primitive".to_string());
    }
}

fn validate_terminal_artifact_contract(contract: &Value, cd_kind: &str, errors: &mut Vec<String>) {
    let terminal = contract
        .get("terminal_artifact_contract")
        .unwrap_or(&Value::Null);
    if json_str(terminal, "artifact_schema") != TERMINAL_ARTIFACT_SCHEMA {
        errors.push("invalid_terminal_artifact_schema".to_string());
    }
    if json_str(terminal, "parent_receives") != "terminal_artifact_only_refs_for_internals" {
        errors.push("parent_must_receive_terminal_artifact_refs_only".to_string());
    }
    let allowed = json_string_array(terminal, "allowed_kinds");
    let allowed_set: HashSet<&str> = allowed.iter().map(String::as_str).collect();
    if !allowed_set.contains("completed_final_answer")
        || !allowed_set.contains("structured_failure")
        || allowed.iter().any(|kind| {
            !ALLOWED_TERMINAL_ARTIFACTS
                .iter()
                .any(|allowed_kind| allowed_kind == kind)
        })
    {
        errors.push("invalid_terminal_artifact_kinds".to_string());
    }
    if cd_kind == "composite" && !allowed_set.contains("delegated_workflow_result") {
        errors.push("composite_workflow_missing_delegated_result_artifact".to_string());
    }
}

fn validate_child_calls(
    workflow_id: &str,
    child_calls: &[&Value],
    known_ids: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    let mut seen = HashSet::new();
    for (idx, call) in child_calls.iter().enumerate() {
        let capability = json_str(call, "capability");
        let child_id = json_str(call, "workflow_id");
        if capability.is_empty()
            || child_id.is_empty()
            || json_str(call, "input_contract").is_empty()
            || json_str(call, "output_artifact").is_empty()
        {
            errors.push(format!("child_workflow_call_missing_typed_fields:{idx}"));
        }
        if child_id == workflow_id {
            errors.push(format!("child_workflow_call_self_reference:{idx}"));
        }
        if !child_id.is_empty() && !known_ids.contains(child_id) {
            errors.push(format!(
                "child_workflow_call_unknown_workflow:{idx}:{child_id}"
            ));
        }
        if json_str(call, "return_policy") != RETURN_POLICY {
            errors.push(format!("child_workflow_call_invalid_return_policy:{idx}"));
        }
        if json_str(call, "internal_trace_visibility") != TRACE_VISIBILITY {
            errors.push(format!("child_workflow_call_must_return_refs_only:{idx}"));
        }
        if !seen.insert((capability.to_string(), child_id.to_string())) {
            errors.push(format!(
                "duplicate_child_workflow_call:{idx}:{capability}:{child_id}"
            ));
        }
    }
}

fn child_workflow_calls(contract: &Value) -> Vec<&Value> {
    contract
        .get("child_workflow_calls")
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn json_str<'a>(value: &'a Value, key: &str) -> &'a str {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
}

fn json_bool(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn json_string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|row| !row.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn workflow_registry_value() -> Option<Value> {
    let raw = fs::read_to_string(workflow_registry_path()).ok()?;
    serde_json::from_str(&raw).ok()
}

fn read_workflow_source(source_path: &str) -> Result<String, std::io::Error> {
    fs::read_to_string(repo_root().join(source_path))
}

fn workflow_registry_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/control_plane/workflows/workflow_registry.json")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_workflow_cds_have_composition_contracts() {
        let report = workflow_composition_contract_report();
        assert_eq!(report.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn primitive_workflow_cannot_call_child_workflow() {
        let contract = json!({
            "version": COMPOSITION_CONTRACT_VERSION,
            "cd_kind": "primitive",
            "returns_exactly_one_terminal_artifact": true,
            "terminal_artifact_contract": {
                "artifact_schema": TERMINAL_ARTIFACT_SCHEMA,
                "allowed_kinds": ["completed_final_answer", "structured_failure"],
                "parent_receives": "terminal_artifact_only_refs_for_internals"
            },
            "child_workflow_calls": [{
                "capability": "online_research",
                "workflow_id": "research_synthesize_verify",
                "input_contract": "workflow_input_envelope_v1",
                "output_artifact": "research_result_artifact_v1",
                "return_policy": RETURN_POLICY,
                "internal_trace_visibility": TRACE_VISIBILITY
            }]
        });
        let workflow = json!({});
        let known_ids = HashSet::from(["research_synthesize_verify".to_string()]);
        let mut errors = Vec::new();
        validate_composition_contract(
            "simple_parent",
            &contract,
            &workflow,
            &known_ids,
            "other_default",
            &mut errors,
        );
        assert!(errors
            .iter()
            .any(|error| error == "primitive_workflow_must_not_call_child_workflows"));
    }
}
