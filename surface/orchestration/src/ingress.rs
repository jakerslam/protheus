// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
mod classifier;
mod parser;

use crate::contracts::{
    Capability, CapabilityProbeSnapshot, CoreProbeEnvelope, Mutability, OperationKind,
    OrchestrationRequest, ParseResult, RequestKind, RequestSurface, ResourceKind, TargetDescriptor,
    TypedOrchestrationRequest,
};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq)]
struct SurfaceAdapterOutput {
    request_kind: Option<RequestKind>,
    operation_kind: Option<OperationKind>,
    resource_kind: Option<ResourceKind>,
    mutability: Option<Mutability>,
    target_descriptors: Vec<TargetDescriptor>,
    tool_hints: Vec<String>,
    reasons: Vec<String>,
}

pub fn normalize_request(input: OrchestrationRequest) -> ParseResult {
    let session_id = input.session_id.trim().to_string();
    let legacy_intent = input.intent.trim().to_lowercase();
    let request_surface = input.surface;
    let payload = match input.payload {
        Value::Null => Value::Object(Default::default()),
        other => other,
    };
    let surface = resolve_request_surface(request_surface, &payload);
    let tokens = parser::tokenize(&legacy_intent);
    let target_descriptors = parser::extract_target_descriptors(&payload);
    let legacy_operation_candidates = parser::operation_candidates(&tokens, &payload);
    let legacy_resource_candidates =
        parser::resource_candidates(&tokens, &payload, &target_descriptors);
    let adapted = adapt_surface_request(surface, &legacy_intent, &payload);
    let operation_candidates = adapted
        .as_ref()
        .and_then(|row| row.operation_kind.clone().map(|value| vec![value]))
        .unwrap_or_else(|| legacy_operation_candidates.clone());
    let resource_candidates = adapted
        .as_ref()
        .and_then(|row| row.resource_kind.clone().map(|value| vec![value]))
        .unwrap_or_else(|| legacy_resource_candidates.clone());
    let operation_kind = adapted
        .as_ref()
        .and_then(|row| row.operation_kind.clone())
        .unwrap_or_else(|| classifier::select_operation_kind(&legacy_operation_candidates));
    let target_descriptors = adapted
        .as_ref()
        .map(|row| row.target_descriptors.clone())
        .unwrap_or(target_descriptors);
    let resource_kind = adapted
        .as_ref()
        .and_then(|row| row.resource_kind.clone())
        .unwrap_or_else(|| classifier::select_resource_kind(&resource_candidates));
    let mutability = adapted
        .as_ref()
        .and_then(|row| row.mutability.clone())
        .unwrap_or_else(|| parser::infer_mutability(&operation_kind));
    let request_kind = adapted
        .as_ref()
        .and_then(|row| row.request_kind.clone())
        .unwrap_or_else(|| classifier::infer_request_kind(&operation_candidates, &operation_kind));
    let target_refs = parser::extract_target_refs(&target_descriptors);
    let tool_hints = adapted
        .as_ref()
        .map(|row| row.tool_hints.clone())
        .unwrap_or_else(|| parser::extract_tool_hints(&payload, &operation_kind));
    let policy_scope = classifier::infer_policy_scope(&resource_kind, &mutability);
    let user_constraints = parser::extract_user_constraints(&payload);
    let adapter_reasons = adapted
        .as_ref()
        .map(|row| row.reasons.clone())
        .unwrap_or_default();
    let core_probe_envelope = extract_core_probe_envelope(&payload, surface);

    classifier::parse_diagnostics(
        TypedOrchestrationRequest {
            session_id,
            surface,
            legacy_intent,
            adapted: adapted.is_some(),
            payload,
            request_kind,
            operation_kind,
            resource_kind,
            mutability,
            target_descriptors,
            target_refs,
            tool_hints,
            policy_scope,
            user_constraints,
            core_probe_envelope,
        },
        &operation_candidates,
        &resource_candidates,
        &adapter_reasons,
    )
}

fn resolve_request_surface(request_surface: RequestSurface, payload: &Value) -> RequestSurface {
    match request_surface {
        RequestSurface::Legacy => {
            if payload.get("sdk").is_some() {
                RequestSurface::Sdk
            } else if payload.get("gateway").is_some() {
                RequestSurface::Gateway
            } else if payload.get("dashboard").is_some() {
                RequestSurface::Dashboard
            } else if payload.get("cli").is_some() {
                RequestSurface::Cli
            } else {
                RequestSurface::Legacy
            }
        }
        explicit => explicit,
    }
}

fn adapt_surface_request(
    surface: RequestSurface,
    _legacy_intent: &str,
    payload: &Value,
) -> Option<SurfaceAdapterOutput> {
    if matches!(surface, RequestSurface::Legacy) {
        return None;
    }
    let obj = adapter_object(surface, payload)?;
    let descriptor_targets = parser::extract_target_descriptors(&Value::Object(obj.clone()));
    let mut tool_hints = extract_tool_hints_from_object(obj);
    for descriptor in &descriptor_targets {
        if let TargetDescriptor::ToolName { value } = descriptor {
            tool_hints.push(value.clone());
        }
    }
    tool_hints.sort();
    tool_hints.dedup();

    let explicit_operation = read_string(
        obj,
        &[
            "operation_kind",
            "operation",
            "action",
            "call",
            "command",
            "route_kind",
        ],
    )
    .and_then(parse_operation_kind);
    let explicit_resource = read_string(
        obj,
        &["resource_kind", "resource", "domain", "target_domain"],
    )
    .and_then(parse_resource_kind);
    let explicit_request_kind =
        read_string(obj, &["request_kind", "request_mode"]).and_then(parse_request_kind);
    let explicit_mutability =
        read_string(obj, &["mutability", "write_mode"]).and_then(parse_mutability);

    let mut tokens = Vec::new();
    for value in adapter_token_strings(surface, obj) {
        tokens.extend(parser::tokenize(&value));
    }
    let adapter_payload = Value::Object(obj.clone());
    let operation_candidates = parser::operation_candidates(&tokens, &adapter_payload);
    let resource_candidates =
        parser::resource_candidates(&tokens, &adapter_payload, descriptor_targets.as_slice());
    let operation_kind = explicit_operation.or_else(|| operation_candidates.first().cloned());
    let resource_kind = explicit_resource
        .or_else(|| parser::infer_resource_from_descriptors(&descriptor_targets))
        .or_else(|| resource_candidates.first().cloned());
    let mutability =
        explicit_mutability.or_else(|| operation_kind.as_ref().map(parser::infer_mutability));

    let adapted = operation_kind.is_some()
        || resource_kind.is_some()
        || !descriptor_targets.is_empty()
        || !tool_hints.is_empty();
    if !adapted {
        return None;
    }

    Some(SurfaceAdapterOutput {
        request_kind: explicit_request_kind,
        operation_kind,
        resource_kind,
        mutability,
        target_descriptors: descriptor_targets,
        tool_hints,
        reasons: vec![format!("surface_adapter:{surface:?}").to_lowercase()],
    })
}

fn adapter_object(surface: RequestSurface, payload: &Value) -> Option<&Map<String, Value>> {
    match surface {
        RequestSurface::Cli => payload
            .get("cli")
            .and_then(Value::as_object)
            .or_else(|| payload.as_object()),
        RequestSurface::Gateway => payload
            .get("gateway")
            .and_then(Value::as_object)
            .or_else(|| payload.as_object()),
        RequestSurface::Sdk => payload
            .get("sdk")
            .and_then(Value::as_object)
            .or_else(|| payload.as_object()),
        RequestSurface::Dashboard => payload
            .get("dashboard")
            .and_then(Value::as_object)
            .or_else(|| payload.as_object()),
        RequestSurface::Legacy => None,
    }
}

fn read_string<'a>(obj: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| obj.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn adapter_token_strings(surface: RequestSurface, obj: &Map<String, Value>) -> Vec<String> {
    let mut out = Vec::new();
    match surface {
        RequestSurface::Cli => {
            for key in ["command", "subcommand", "verb"] {
                if let Some(value) = read_string(obj, &[key]) {
                    out.push(value.to_string());
                }
            }
        }
        RequestSurface::Gateway => {
            for key in ["route", "method", "action"] {
                if let Some(value) = read_string(obj, &[key]) {
                    out.push(value.to_string());
                }
            }
        }
        RequestSurface::Sdk => {
            for key in ["call", "method", "operation_kind", "resource_kind"] {
                if let Some(value) = read_string(obj, &[key]) {
                    out.push(value.to_string());
                }
            }
        }
        RequestSurface::Dashboard => {
            for key in ["page", "action", "selection_mode"] {
                if let Some(value) = read_string(obj, &[key]) {
                    out.push(value.to_string());
                }
            }
        }
        RequestSurface::Legacy => {}
    }
    out
}

fn parse_operation_kind(value: &str) -> Option<OperationKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "read" | "status" | "inspect" => Some(OperationKind::Read),
        "search" | "query" | "lookup" => Some(OperationKind::Search),
        "fetch" | "download" | "retrieve" => Some(OperationKind::Fetch),
        "compare" => Some(OperationKind::Compare),
        "inspect_tooling" | "tool" | "tool_call" | "runtime_bridge" => {
            Some(OperationKind::InspectTooling)
        }
        "assimilate" | "ingest" => Some(OperationKind::Assimilate),
        "plan" | "propose" => Some(OperationKind::Plan),
        "mutate" | "update" | "write" | "edit" => Some(OperationKind::Mutate),
        _ => None,
    }
}

fn parse_resource_kind(value: &str) -> Option<ResourceKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "web" | "url" => Some(ResourceKind::Web),
        "workspace" | "file" | "repo" => Some(ResourceKind::Workspace),
        "tooling" | "tool" | "runtime" => Some(ResourceKind::Tooling),
        "task" | "task_graph" | "workflow" => Some(ResourceKind::TaskGraph),
        "memory" | "history" => Some(ResourceKind::Memory),
        "mixed" => Some(ResourceKind::Mixed),
        _ => None,
    }
}

fn parse_request_kind(value: &str) -> Option<RequestKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "direct" => Some(RequestKind::Direct),
        "comparative" | "compare" => Some(RequestKind::Comparative),
        "workflow" => Some(RequestKind::Workflow),
        "ambiguous" => Some(RequestKind::Ambiguous),
        _ => None,
    }
}

fn parse_mutability(value: &str) -> Option<Mutability> {
    match value.trim().to_ascii_lowercase().as_str() {
        "read_only" | "readonly" | "read" => Some(Mutability::ReadOnly),
        "proposal" | "plan" => Some(Mutability::Proposal),
        "mutation" | "write" | "mutate" => Some(Mutability::Mutation),
        _ => None,
    }
}

fn extract_tool_hints_from_object(obj: &Map<String, Value>) -> Vec<String> {
    let mut hints = Vec::new();
    for key in ["tool", "tool_name"] {
        if let Some(value) = read_string(obj, &[key]) {
            hints.push(value.to_string());
        }
    }
    if let Some(values) = obj.get("tool_hints").and_then(Value::as_array) {
        for value in values.iter().filter_map(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                hints.push(trimmed.to_string());
            }
        }
    }
    hints.sort();
    hints.dedup();
    hints
}

fn extract_core_probe_envelope(
    payload: &Value,
    surface: RequestSurface,
) -> Option<CoreProbeEnvelope> {
    let mut probes = if let Some(explicit) = payload.get("core_probe_envelope") {
        parse_capability_probe_rows(explicit)
    } else {
        Vec::new()
    };
    if probes.is_empty() && matches!(surface, RequestSurface::Legacy) {
        if let Some(compat) = payload
            .get("capability_probes")
            .or_else(|| payload.get("probes"))
        {
            probes = parse_capability_probe_rows(compat);
        }
    }
    if probes.is_empty() {
        None
    } else {
        Some(CoreProbeEnvelope { probes })
    }
}

fn parse_capability_probe_rows(value: &Value) -> Vec<CapabilityProbeSnapshot> {
    if let Some(rows) = value.get("probes").and_then(Value::as_array) {
        let mut out = rows
            .iter()
            .filter_map(parse_capability_probe_snapshot)
            .collect::<Vec<_>>();
        out.sort_by_key(|row| format!("{:?}", row.capability));
        return out;
    }
    let Some(map) = value.as_object() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (raw_capability, raw_probe) in map {
        let Some(capability) = parse_capability_name(raw_capability) else {
            continue;
        };
        let Some(probe) = raw_probe.as_object() else {
            continue;
        };
        out.push(CapabilityProbeSnapshot {
            capability,
            tool_available: probe.get("tool_available").and_then(Value::as_bool),
            target_supplied: probe.get("target_supplied").and_then(Value::as_bool),
            target_syntactically_valid: probe
                .get("target_syntactically_valid")
                .and_then(Value::as_bool),
            target_exists: probe.get("target_exists").and_then(Value::as_bool),
            authorization_valid: probe.get("authorization_valid").and_then(Value::as_bool),
            policy_allows: probe.get("policy_allows").and_then(Value::as_bool),
            transport_available: probe.get("transport_available").and_then(Value::as_bool),
        });
    }
    out.sort_by_key(|row| format!("{:?}", row.capability));
    out
}

fn parse_capability_probe_snapshot(value: &Value) -> Option<CapabilityProbeSnapshot> {
    let row = value.as_object()?;
    let capability = row
        .get("capability")
        .and_then(Value::as_str)
        .and_then(parse_capability_name)?;
    Some(CapabilityProbeSnapshot {
        capability,
        tool_available: row.get("tool_available").and_then(Value::as_bool),
        target_supplied: row.get("target_supplied").and_then(Value::as_bool),
        target_syntactically_valid: row
            .get("target_syntactically_valid")
            .and_then(Value::as_bool),
        target_exists: row.get("target_exists").and_then(Value::as_bool),
        authorization_valid: row.get("authorization_valid").and_then(Value::as_bool),
        policy_allows: row.get("policy_allows").and_then(Value::as_bool),
        transport_available: row.get("transport_available").and_then(Value::as_bool),
    })
}

fn parse_capability_name(value: &str) -> Option<Capability> {
    match value.trim().to_ascii_lowercase().as_str() {
        "read_memory" => Some(Capability::ReadMemory),
        "mutate_task" => Some(Capability::MutateTask),
        "execute_tool" => Some(Capability::ExecuteTool),
        "plan_assimilation" => Some(Capability::PlanAssimilation),
        "verify_claim" => Some(Capability::VerifyClaim),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::TargetDescriptor;
    use serde_json::json;

    #[test]
    fn target_descriptors_resolve_domain_specific_keys() {
        let descriptors = parser::extract_target_descriptors(&json!({
            "path": "surface/orchestration/src/lib.rs",
            "url": "https://example.com/docs",
            "task_id": "TF-9",
            "memory_scope": "session",
            "memory_object_id": "abc123",
            "tool_name": "web_search"
        }));
        assert!(descriptors
            .iter()
            .any(|row| matches!(row, TargetDescriptor::WorkspacePath { .. })));
        assert!(descriptors
            .iter()
            .any(|row| matches!(row, TargetDescriptor::Url { .. })));
        assert!(descriptors
            .iter()
            .any(|row| matches!(row, TargetDescriptor::TaskId { .. })));
        assert!(descriptors
            .iter()
            .any(|row| matches!(row, TargetDescriptor::MemoryRef { .. })));
        assert!(descriptors
            .iter()
            .any(|row| matches!(row, TargetDescriptor::ToolName { .. })));
    }

    #[test]
    fn sdk_surface_adapter_prefers_typed_fields_over_legacy_intent() {
        let adapted = adapt_surface_request(
            RequestSurface::Sdk,
            "maybe do something",
            &json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "tool_hints": ["web_search"],
                    "targets": [{ "kind": "url", "value": "https://example.com" }]
                }
            }),
        )
        .expect("adapter should produce typed request");
        assert_eq!(adapted.operation_kind, Some(OperationKind::Search));
        assert_eq!(adapted.resource_kind, Some(ResourceKind::Web));
        assert_eq!(adapted.request_kind, Some(RequestKind::Direct));
        assert!(adapted
            .reasons
            .iter()
            .any(|row| row == "surface_adapter:sdk"));
    }

    #[test]
    fn request_payload_execution_observation_is_ignored_for_legacy_surface() {
        let parsed = normalize_request(OrchestrationRequest {
            session_id: "legacy-observation".to_string(),
            intent: "read status".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({
                "core_execution": {
                    "status": "completed",
                    "receipt_ids": ["r1"]
                },
                "core_execution_observation": {
                    "status": "running",
                    "receipt_ids": ["r2"]
                }
            }),
        });
        assert_eq!(parsed.typed_request.surface, RequestSurface::Legacy);
        assert_eq!(parsed.typed_request.operation_kind, OperationKind::Read);
    }

    #[test]
    fn request_payload_execution_observation_is_ignored_for_typed_surface() {
        let parsed = normalize_request(OrchestrationRequest {
            session_id: "sdk-observation".to_string(),
            intent: "search".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct"
                },
                "core_execution_observation": {
                    "status": "running",
                    "receipt_ids": ["r1"]
                }
            }),
        });
        assert_eq!(parsed.typed_request.surface, RequestSurface::Sdk);
        assert_eq!(parsed.typed_request.operation_kind, OperationKind::Search);
    }

    #[test]
    fn non_legacy_surface_rejects_capability_probe_compat_shortcut() {
        let envelope = extract_core_probe_envelope(
            &json!({
                "capability_probes": {
                    "execute_tool": {
                        "transport_available": true
                    }
                }
            }),
            RequestSurface::Sdk,
        );
        assert!(envelope.is_none());
    }

    #[test]
    fn legacy_surface_accepts_capability_probe_compat_shortcut() {
        let envelope = extract_core_probe_envelope(
            &json!({
                "capability_probes": {
                    "execute_tool": {
                        "transport_available": true
                    }
                }
            }),
            RequestSurface::Legacy,
        )
        .expect("legacy probe compatibility path should remain available");
        assert_eq!(envelope.probes.len(), 1);
        assert_eq!(envelope.probes[0].capability, Capability::ExecuteTool);
        assert_eq!(envelope.probes[0].transport_available, Some(true));
    }
}
