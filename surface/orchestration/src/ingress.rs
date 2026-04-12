use crate::contracts::{
    Mutability, OperationKind, OrchestrationRequest, PolicyScope, RequestKind, ResourceKind,
    TypedOrchestrationRequest, UserConstraint,
};
use serde_json::Value;

pub fn normalize_request(input: OrchestrationRequest) -> TypedOrchestrationRequest {
    let session_id = input.session_id.trim().to_string();
    let legacy_intent = input.intent.trim().to_lowercase();
    let payload = match input.payload {
        Value::Null => Value::Object(Default::default()),
        other => other,
    };
    let tokens = tokenize(&legacy_intent);
    let operation_candidates = operation_candidates(&tokens, &payload);
    let resource_candidates = resource_candidates(&tokens, &payload);
    let operation_kind = select_operation_kind(&operation_candidates);
    let resource_kind = select_resource_kind(&resource_candidates);
    let mutability = infer_mutability(&operation_kind);
    let request_kind = infer_request_kind(&operation_candidates, &operation_kind);
    let target_refs = extract_target_refs(&payload);
    let tool_hints = extract_tool_hints(&payload, &operation_kind);
    let policy_scope = infer_policy_scope(&resource_kind, &mutability);
    let user_constraints = extract_user_constraints(&payload);
    let (parse_confidence, parse_reasons) = parse_diagnostics(
        &operation_candidates,
        &resource_candidates,
        &operation_kind,
        &resource_kind,
        &request_kind,
        &target_refs,
    );

    TypedOrchestrationRequest {
        session_id,
        legacy_intent,
        payload,
        request_kind,
        operation_kind,
        resource_kind,
        mutability,
        target_refs,
        tool_hints,
        policy_scope,
        user_constraints,
        parse_confidence,
        parse_reasons,
    }
}

fn tokenize(intent: &str) -> Vec<String> {
    intent
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn operation_candidates(tokens: &[String], payload: &Value) -> Vec<OperationKind> {
    let mut out = Vec::new();
    let has_any = |needles: &[&str]| {
        needles
            .iter()
            .any(|needle| tokens.iter().any(|token| token == needle))
    };
    let payload_has = |key: &str| payload.get(key).is_some();

    if has_any(&["search", "query", "lookup", "find"]) || payload_has("query") {
        out.push(OperationKind::Search);
    }
    if has_any(&["fetch", "download", "retrieve", "crawl"]) || payload_has("url") {
        out.push(OperationKind::Fetch);
    }
    if has_any(&["compare", "vs", "versus"]) {
        out.push(OperationKind::Compare);
    }
    if has_any(&["tool", "tools", "route", "runtime", "bridge", "command"]) || payload_has("tool") {
        out.push(OperationKind::InspectTooling);
    }
    if has_any(&["assimilate", "assimilation", "ingest", "import"]) {
        out.push(OperationKind::Assimilate);
    }
    if has_any(&["task", "tasks", "plan", "backlog", "proposal"]) {
        out.push(OperationKind::Plan);
    }
    if has_any(&[
        "update",
        "write",
        "apply",
        "edit",
        "change",
        "implement",
        "mutate",
    ]) {
        out.push(OperationKind::Mutate);
    }
    if out.is_empty()
        && (has_any(&["read", "status", "show", "inspect", "does", "why"]) || payload_has("target"))
    {
        out.push(OperationKind::Read);
    }
    out
}

fn select_operation_kind(candidates: &[OperationKind]) -> OperationKind {
    match candidates {
        [] => OperationKind::Unknown,
        [only] => only.clone(),
        many if many.iter().any(|row| row == &OperationKind::Mutate) => OperationKind::Mutate,
        many if many.iter().any(|row| row == &OperationKind::Search) => OperationKind::Search,
        many if many.iter().any(|row| row == &OperationKind::Fetch) => OperationKind::Fetch,
        many if many.iter().any(|row| row == &OperationKind::Plan) => OperationKind::Plan,
        [first, ..] => first.clone(),
    }
}

fn resource_candidates(tokens: &[String], payload: &Value) -> Vec<ResourceKind> {
    let mut out = Vec::new();
    let has_any = |needles: &[&str]| {
        needles
            .iter()
            .any(|needle| tokens.iter().any(|token| token == needle))
    };
    let payload_has_any = |keys: &[&str]| keys.iter().any(|key| payload.get(key).is_some());

    if has_any(&["web", "url", "http", "https", "site"]) || payload_has_any(&["url", "urls"]) {
        out.push(ResourceKind::Web);
    }
    if has_any(&["file", "files", "workspace", "path", "paths", "repo"])
        || payload_has_any(&["path", "paths"])
    {
        out.push(ResourceKind::Workspace);
    }
    if has_any(&["tool", "tools", "runtime", "bridge", "command"])
        || payload_has_any(&["tool", "tool_name"])
    {
        out.push(ResourceKind::Tooling);
    }
    if has_any(&["task", "tasks", "workflow", "backlog"]) || payload_has_any(&["target", "targets"])
    {
        out.push(ResourceKind::TaskGraph);
    }
    if has_any(&["memory", "context", "history", "status"]) {
        out.push(ResourceKind::Memory);
    }
    out
}

fn select_resource_kind(candidates: &[ResourceKind]) -> ResourceKind {
    match candidates {
        [] => ResourceKind::Unspecified,
        [only] => only.clone(),
        _ => ResourceKind::Mixed,
    }
}

fn infer_mutability(operation_kind: &OperationKind) -> Mutability {
    match operation_kind {
        OperationKind::Mutate => Mutability::Mutation,
        OperationKind::Assimilate | OperationKind::Plan => Mutability::Proposal,
        _ => Mutability::ReadOnly,
    }
}

fn infer_request_kind(
    operation_candidates: &[OperationKind],
    operation_kind: &OperationKind,
) -> RequestKind {
    if operation_candidates.len() > 1 {
        return RequestKind::Ambiguous;
    }
    match operation_kind {
        OperationKind::Compare => RequestKind::Comparative,
        OperationKind::Assimilate | OperationKind::Plan => RequestKind::Workflow,
        OperationKind::Unknown => RequestKind::Ambiguous,
        _ => RequestKind::Direct,
    }
}

fn extract_target_refs(payload: &Value) -> Vec<String> {
    let mut refs = Vec::new();
    for key in ["target", "path", "url", "ref"] {
        if let Some(value) = payload.get(key).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                refs.push(trimmed.to_string());
            }
        }
    }
    for key in ["targets", "paths", "urls", "refs"] {
        if let Some(values) = payload.get(key).and_then(Value::as_array) {
            for value in values.iter().filter_map(Value::as_str) {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    refs.push(trimmed.to_string());
                }
            }
        }
    }
    refs.sort();
    refs.dedup();
    refs
}

fn extract_tool_hints(payload: &Value, operation_kind: &OperationKind) -> Vec<String> {
    let mut hints = Vec::new();
    for key in ["tool", "tool_name"] {
        if let Some(value) = payload.get(key).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                hints.push(trimmed.to_string());
            }
        }
    }
    match operation_kind {
        OperationKind::Search => hints.push("web_search".to_string()),
        OperationKind::Fetch => hints.push("web_fetch".to_string()),
        OperationKind::InspectTooling => hints.push("tooling_route".to_string()),
        _ => {}
    }
    hints.sort();
    hints.dedup();
    hints
}

fn infer_policy_scope(resource_kind: &ResourceKind, mutability: &Mutability) -> PolicyScope {
    match (resource_kind, mutability) {
        (ResourceKind::Web, _) => PolicyScope::WebOnly,
        (ResourceKind::Workspace, _) => PolicyScope::WorkspaceOnly,
        (_, Mutability::Proposal | Mutability::Mutation) => PolicyScope::CoreProposal,
        (ResourceKind::Mixed, _) => PolicyScope::CrossBoundary,
        _ => PolicyScope::Default,
    }
}

fn extract_user_constraints(payload: &Value) -> Vec<UserConstraint> {
    let mut constraints = Vec::new();
    if let Some(obj) = payload.as_object() {
        for (key, value) in obj {
            if matches!(
                key.as_str(),
                "target"
                    | "targets"
                    | "path"
                    | "paths"
                    | "url"
                    | "urls"
                    | "tool"
                    | "tool_name"
                    | "ref"
                    | "refs"
            ) {
                continue;
            }
            let rendered = match value {
                Value::String(inner) => inner.clone(),
                Value::Bool(inner) => inner.to_string(),
                Value::Number(inner) => inner.to_string(),
                _ => continue,
            };
            constraints.push(UserConstraint {
                key: key.clone(),
                value: rendered,
            });
        }
    }
    constraints.sort_by(|left, right| left.key.cmp(&right.key));
    constraints
}

fn parse_diagnostics(
    operation_candidates: &[OperationKind],
    resource_candidates: &[ResourceKind],
    operation_kind: &OperationKind,
    resource_kind: &ResourceKind,
    request_kind: &RequestKind,
    target_refs: &[String],
) -> (f32, Vec<String>) {
    let mut confidence: f32 = 0.25;
    let mut reasons = vec!["legacy_intent_compatibility_shim".to_string()];

    if operation_kind != &OperationKind::Unknown {
        confidence += 0.30;
        reasons.push(format!("operation_kind:{operation_kind:?}").to_lowercase());
    } else {
        reasons.push("operation_kind:unknown".to_string());
    }
    if resource_kind != &ResourceKind::Unspecified {
        confidence += 0.20;
        reasons.push(format!("resource_kind:{resource_kind:?}").to_lowercase());
    }
    if request_kind != &RequestKind::Ambiguous {
        confidence += 0.10;
    } else {
        reasons.push("request_kind:ambiguous".to_string());
    }
    if operation_candidates.len() > 1 {
        reasons.push(format!(
            "operation_candidates:{}",
            operation_candidates.len()
        ));
        confidence -= 0.20;
    }
    if resource_candidates.len() > 1 {
        reasons.push(format!("resource_candidates:{}", resource_candidates.len()));
        confidence -= 0.10;
    }
    if !target_refs.is_empty() {
        confidence += 0.10;
        reasons.push("targets:present".to_string());
    }

    (confidence.clamp(0.0, 0.99), reasons)
}
