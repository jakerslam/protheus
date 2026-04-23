// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{Mutability, OperationKind, ResourceKind, TargetDescriptor, UserConstraint};
use serde_json::{Map, Value};

pub fn tokenize(intent: &str) -> Vec<String> {
    intent
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

pub fn operation_candidates(tokens: &[String], payload: &Value) -> Vec<OperationKind> {
    let mut candidates = Vec::new();
    let has_any = |needles: &[&str]| {
        needles
            .iter()
            .any(|needle| tokens.iter().any(|token| token == needle))
    };
    let payload_has = |key: &str| payload.get(key).is_some();
    let has_workspace_signal = has_any(&[
        "file",
        "files",
        "workspace",
        "path",
        "paths",
        "repo",
        "repository",
        "directory",
        "directories",
        "folder",
        "folders",
        "filesystem",
        "local",
        "cwd",
        "pwd",
    ]) || payload_has("path")
        || payload_has("paths")
        || payload_has("file")
        || payload_has("files")
        || payload_has("workspace_path")
        || payload_has("repo_path")
        || payload_has("repository_path")
        || payload_has("cwd_path")
        || payload_has("directory")
        || payload_has("directories")
        || payload_has("folder")
        || payload_has("folders");

    if has_any(&["search", "query", "lookup", "find"]) || payload_has("query") {
        candidates.push(OperationKind::Search);
    }
    if has_workspace_signal
        && has_any(&["rg", "grep", "glob", "pattern", "match", "matches"])
        && !candidates.contains(&OperationKind::Search)
    {
        candidates.push(OperationKind::Search);
    }
    if has_any(&["fetch", "download", "retrieve", "crawl"]) || payload_has("url") {
        candidates.push(OperationKind::Fetch);
    }
    if has_any(&["compare", "vs", "versus"]) {
        candidates.push(OperationKind::Compare);
    }
    if has_any(&["tool", "tools", "route", "runtime", "bridge", "command"]) || payload_has("tool") {
        candidates.push(OperationKind::InspectTooling);
    }
    if has_any(&["assimilate", "assimilation", "ingest", "import"]) {
        candidates.push(OperationKind::Assimilate);
    }
    if has_any(&["task", "tasks", "plan", "backlog", "proposal"]) {
        candidates.push(OperationKind::Plan);
    }
    if has_workspace_signal
        && has_any(&[
            "list",
            "ls",
            "dir",
            "tree",
            "directory",
            "directories",
            "folder",
            "folders",
            "open",
            "browse",
            "view",
            "look",
            "looking",
            "read",
            "cat",
            "head",
            "tail",
        ])
        && !candidates.contains(&OperationKind::Read)
    {
        candidates.push(OperationKind::Read);
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
        candidates.push(OperationKind::Mutate);
    }
    if candidates.is_empty()
        && (has_any(&["read", "status", "show", "inspect", "does", "why"]) || payload_has("target"))
    {
        candidates.push(OperationKind::Read);
    }
    candidates
}

pub fn resource_candidates(
    tokens: &[String],
    payload: &Value,
    target_descriptors: &[TargetDescriptor],
) -> Vec<ResourceKind> {
    let mut candidates = Vec::new();
    let has_any = |needles: &[&str]| {
        needles
            .iter()
            .any(|needle| tokens.iter().any(|token| token == needle))
    };
    let payload_has_any = |keys: &[&str]| keys.iter().any(|key| payload.get(key).is_some());

    if has_any(&["web", "url", "http", "https", "site"]) || payload_has_any(&["url", "urls"]) {
        candidates.push(ResourceKind::Web);
    }
    if has_any(&[
        "file",
        "files",
        "workspace",
        "path",
        "paths",
        "repo",
        "directory",
        "directories",
        "folder",
        "folders",
        "filesystem",
        "disk",
        "project",
        "local",
        "cwd",
        "pwd",
    ]) || payload_has_any(&[
        "path",
        "paths",
        "workspace_path",
        "repo_path",
        "repository_path",
        "cwd_path",
        "directory",
        "directories",
        "folder",
        "folders",
    ])
    {
        candidates.push(ResourceKind::Workspace);
    }
    if has_any(&["tool", "tools", "runtime", "bridge", "command"])
        || payload_has_any(&["tool", "tool_name"])
    {
        candidates.push(ResourceKind::Tooling);
    }
    if has_any(&["task", "tasks", "workflow", "backlog"]) || payload_has_any(&["target", "targets"])
    {
        candidates.push(ResourceKind::TaskGraph);
    }
    if has_any(&["memory", "context", "history", "status"]) {
        candidates.push(ResourceKind::Memory);
    }
    match infer_resource_from_descriptors(target_descriptors) {
        Some(ResourceKind::Mixed) => candidates.push(ResourceKind::Mixed),
        Some(kind) if !candidates.contains(&kind) => candidates.push(kind),
        _ => {}
    }
    candidates
}

pub fn extract_target_descriptors(payload: &Value) -> Vec<TargetDescriptor> {
    let mut descriptors = Vec::new();
    if let Some(obj) = payload.as_object() {
        descriptors.extend(extract_target_descriptors_from_object(obj));
        for nested in ["cli", "gateway", "sdk", "dashboard"] {
            if let Some(nested_obj) = payload.get(nested).and_then(Value::as_object) {
                descriptors.extend(extract_target_descriptors_from_object(nested_obj));
            }
        }
    }
    dedupe_target_descriptors(descriptors)
}

pub fn extract_target_refs(target_descriptors: &[TargetDescriptor]) -> Vec<String> {
    let mut refs = target_descriptors
        .iter()
        .map(target_descriptor_ref)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    refs.sort();
    refs.dedup();
    refs
}

pub fn extract_tool_hints(
    payload: &Value,
    operation_kind: &OperationKind,
    resource_kind: &ResourceKind,
) -> Vec<String> {
    let mut hints = Vec::new();
    for key in ["tool", "tool_name"] {
        if let Some(value) = payload.get(key).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                hints.push(trimmed.to_string());
            }
        }
    }
    match (resource_kind, operation_kind) {
        (ResourceKind::Web, OperationKind::Search | OperationKind::Compare) => {
            hints.push("web_search".to_string());
        }
        (ResourceKind::Web, OperationKind::Fetch) => {
            hints.push("web_fetch".to_string());
        }
        (ResourceKind::Workspace, OperationKind::Search | OperationKind::Fetch) => {
            hints.push("workspace_search".to_string());
        }
        (ResourceKind::Workspace, OperationKind::Read) => {
            hints.push("workspace_read".to_string());
        }
        (ResourceKind::Workspace, OperationKind::Plan) => {
            hints.push("workspace_search".to_string());
        }
        (ResourceKind::Workspace, OperationKind::Unknown) => {
            hints.push("workspace_read".to_string());
        }
        (ResourceKind::Tooling, _) | (_, OperationKind::InspectTooling) => {
            hints.push("tooling_route".to_string());
        }
        _ => {}
    }
    hints.sort();
    hints.dedup();
    hints
}

pub fn infer_resource_from_descriptors(
    target_descriptors: &[TargetDescriptor],
) -> Option<ResourceKind> {
    let mut kinds = target_descriptors
        .iter()
        .filter_map(|descriptor| match descriptor {
            TargetDescriptor::WorkspacePath { .. } => Some(ResourceKind::Workspace),
            TargetDescriptor::Url { .. } => Some(ResourceKind::Web),
            TargetDescriptor::TaskId { .. } => Some(ResourceKind::TaskGraph),
            TargetDescriptor::MemoryRef { .. } => Some(ResourceKind::Memory),
            TargetDescriptor::ToolName { .. } => Some(ResourceKind::Tooling),
            TargetDescriptor::Unknown { .. } => None,
        })
        .collect::<Vec<_>>();
    kinds.sort_by_key(|value| format!("{value:?}"));
    kinds.dedup();
    match kinds.as_slice() {
        [] => None,
        [only] => Some(only.clone()),
        _ => Some(ResourceKind::Mixed),
    }
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

fn read_string<'a>(obj: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| obj.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn extract_target_descriptors_from_object(obj: &Map<String, Value>) -> Vec<TargetDescriptor> {
    let mut out = Vec::new();
    collect_string_targets(
        obj,
        &[
            "workspace_path",
            "repo_path",
            "repository_path",
            "path",
            "file",
            "directory",
            "folder",
        ],
        &[
            "workspace_paths",
            "repo_paths",
            "repository_paths",
            "paths",
            "files",
            "directories",
            "folders",
        ],
        |value| TargetDescriptor::WorkspacePath {
            value: value.to_string(),
        },
        &mut out,
    );
    collect_string_targets(
        obj,
        &["url", "link"],
        &["urls", "links"],
        |value| TargetDescriptor::Url {
            value: value.to_string(),
        },
        &mut out,
    );
    collect_string_targets(
        obj,
        &["task_id", "task"],
        &["task_ids", "tasks"],
        |value| TargetDescriptor::TaskId {
            value: value.to_string(),
        },
        &mut out,
    );
    collect_string_targets(
        obj,
        &["tool", "tool_name"],
        &["tool_hints"],
        |value| TargetDescriptor::ToolName {
            value: value.to_string(),
        },
        &mut out,
    );
    collect_generic_targets(obj, "target", "targets", &mut out);
    collect_generic_targets(obj, "ref", "refs", &mut out);
    collect_memory_targets(obj, &mut out);
    if let Some(values) = obj.get("targets").and_then(Value::as_array) {
        for value in values {
            if let Some(descriptor) = parse_structured_target(value) {
                out.push(descriptor);
            }
        }
    }
    dedupe_target_descriptors(out)
}

fn collect_string_targets<F>(
    obj: &Map<String, Value>,
    singular_keys: &[&str],
    plural_keys: &[&str],
    map: F,
    out: &mut Vec<TargetDescriptor>,
) where
    F: Fn(&str) -> TargetDescriptor,
{
    for key in singular_keys {
        if let Some(value) = read_string(obj, &[*key]) {
            out.push(map(value));
        }
    }
    for key in plural_keys {
        if let Some(values) = obj.get(*key).and_then(Value::as_array) {
            for value in values.iter().filter_map(Value::as_str) {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    out.push(map(trimmed));
                }
            }
        }
    }
}

fn collect_generic_targets(
    obj: &Map<String, Value>,
    singular_key: &str,
    plural_key: &str,
    out: &mut Vec<TargetDescriptor>,
) {
    if let Some(value) = read_string(obj, &[singular_key]) {
        out.push(parse_generic_target(value));
    }
    if let Some(values) = obj.get(plural_key).and_then(Value::as_array) {
        for value in values.iter().filter_map(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                out.push(parse_generic_target(trimmed));
            }
        }
    }
}

fn collect_memory_targets(obj: &Map<String, Value>, out: &mut Vec<TargetDescriptor>) {
    if let Some(scope) = read_string(obj, &["memory_scope"]) {
        let object_id = read_string(obj, &["memory_object_id", "memory_ref"]).map(str::to_string);
        out.push(TargetDescriptor::MemoryRef {
            scope: scope.to_string(),
            object_id,
        });
    }
}

fn parse_structured_target(value: &Value) -> Option<TargetDescriptor> {
    let obj = value.as_object()?;
    let kind = read_string(obj, &["kind", "type"])?;
    match kind.to_ascii_lowercase().as_str() {
        "workspace_path" | "path" => {
            read_string(obj, &["value", "path"]).map(|value| TargetDescriptor::WorkspacePath {
                value: value.to_string(),
            })
        }
        "url" => read_string(obj, &["value", "url"]).map(|value| TargetDescriptor::Url {
            value: value.to_string(),
        }),
        "task_id" | "task" => {
            read_string(obj, &["value", "task_id"]).map(|value| TargetDescriptor::TaskId {
                value: value.to_string(),
            })
        }
        "tool_name" | "tool" => {
            read_string(obj, &["value", "tool"]).map(|value| TargetDescriptor::ToolName {
                value: value.to_string(),
            })
        }
        "memory_ref" | "memory" => {
            read_string(obj, &["scope"]).map(|scope| TargetDescriptor::MemoryRef {
                scope: scope.to_string(),
                object_id: read_string(obj, &["object_id", "value"]).map(str::to_string),
            })
        }
        _ => read_string(obj, &["value"]).map(|value| TargetDescriptor::Unknown {
            value: value.to_string(),
        }),
    }
}

fn parse_generic_target(value: &str) -> TargetDescriptor {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return TargetDescriptor::Url {
            value: trimmed.to_string(),
        };
    }
    if let Some(task) = trimmed.strip_prefix("task:") {
        return TargetDescriptor::TaskId {
            value: task.trim().to_string(),
        };
    }
    if let Some(tool) = trimmed.strip_prefix("tool:") {
        return TargetDescriptor::ToolName {
            value: tool.trim().to_string(),
        };
    }
    if let Some(memory) = trimmed.strip_prefix("memory:") {
        let parts = memory.splitn(2, '/').collect::<Vec<_>>();
        return TargetDescriptor::MemoryRef {
            scope: parts
                .first()
                .copied()
                .unwrap_or_default()
                .trim()
                .to_string(),
            object_id: parts
                .get(1)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        };
    }
    if !trimmed.is_empty()
        && !trimmed.contains(char::is_whitespace)
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':'))
    {
        return TargetDescriptor::TaskId {
            value: trimmed.to_string(),
        };
    }
    let looks_like_windows_drive_path = {
        let bytes = trimmed.as_bytes();
        bytes.len() > 2
            && bytes[1] == b':'
            && (bytes[2] == b'\\' || bytes[2] == b'/')
    };
    if trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains('.')
        || trimmed.starts_with("~/")
        || trimmed.starts_with("./")
        || trimmed.starts_with(".\\")
        || trimmed.starts_with("../")
        || trimmed.starts_with("..\\")
        || looks_like_windows_drive_path
    {
        return TargetDescriptor::WorkspacePath {
            value: trimmed.to_string(),
        };
    }
    TargetDescriptor::Unknown {
        value: trimmed.to_string(),
    }
}

fn dedupe_target_descriptors(targets: Vec<TargetDescriptor>) -> Vec<TargetDescriptor> {
    let mut seen = std::collections::BTreeSet::new();
    targets
        .into_iter()
        .filter(|target| seen.insert(target_descriptor_ref(target)))
        .collect()
}

fn target_descriptor_ref(target: &TargetDescriptor) -> String {
    match target {
        TargetDescriptor::WorkspacePath { value }
        | TargetDescriptor::Url { value }
        | TargetDescriptor::TaskId { value }
        | TargetDescriptor::ToolName { value }
        | TargetDescriptor::Unknown { value } => value.clone(),
        TargetDescriptor::MemoryRef { scope, object_id } => match object_id {
            Some(object_id) if !object_id.is_empty() => format!("{scope}/{object_id}"),
            _ => scope.clone(),
        },
    }
}
