use crate::contracts::{Mutability, OperationKind, ResourceKind, UserConstraint};
use serde_json::Value;

pub fn tokenize(intent: &str) -> Vec<String> {
    intent
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

pub fn operation_candidates(tokens: &[String], payload: &Value) -> Vec<OperationKind> {
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
    if has_any(&["tool", "tools", "route", "runtime", "bridge", "command"]) || payload_has("tool")
    {
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

pub fn resource_candidates(tokens: &[String], payload: &Value) -> Vec<ResourceKind> {
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

pub fn extract_target_refs(payload: &Value) -> Vec<String> {
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

pub fn extract_tool_hints(payload: &Value, operation_kind: &OperationKind) -> Vec<String> {
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

pub fn extract_user_constraints(payload: &Value) -> Vec<UserConstraint> {
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

pub fn infer_mutability(operation_kind: &OperationKind) -> Mutability {
    match operation_kind {
        OperationKind::Mutate => Mutability::Mutation,
        OperationKind::Assimilate | OperationKind::Plan => Mutability::Proposal,
        _ => Mutability::ReadOnly,
    }
}
