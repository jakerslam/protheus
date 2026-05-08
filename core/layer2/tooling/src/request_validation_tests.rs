use crate::request_validation::repair_and_validate_args;
use serde_json::{json, Value};

#[test]
fn validation_accepts_governed_tooling_surface_contracts() {
    let folder =
        repair_and_validate_args("folder_export", &json!({"path":"notes"})).expect("folder_export");
    let batch = repair_and_validate_args(
        "batch_query",
        &json!({"queries":["langgraph observability", "langsmith tracing"]}),
    )
    .expect("batch_query");
    let terminal =
        repair_and_validate_args("terminal_exec", &json!({"cmd":"ls -la"})).expect("terminal_exec");
    let workspace = repair_and_validate_args(
        "workspace_analyze",
        &json!({"query":"inspect the workspace tree"}),
    )
    .expect("workspace_analyze");
    let spawn =
        repair_and_validate_args("spawn_subagents", &json!({"task":"parallelize tool audit"}))
            .expect("spawn_subagents");
    let manage = repair_and_validate_args(
        "manage_agent",
        &json!({"action":"message","session_id":"agent-7"}),
    )
    .expect("manage_agent");

    assert_eq!(folder.get("path").and_then(Value::as_str), Some("notes"));
    assert_eq!(
        batch.get("query").and_then(Value::as_str),
        Some("langgraph observability")
    );
    assert_eq!(
        batch.get("aperture").and_then(Value::as_str),
        Some("medium")
    );
    assert_eq!(
        terminal.get("command").and_then(Value::as_str),
        Some("ls -la")
    );
    assert_eq!(
        workspace.get("query").and_then(Value::as_str),
        Some("inspect the workspace tree")
    );
    assert_eq!(
        spawn.get("objective").and_then(Value::as_str),
        Some("parallelize tool audit")
    );
    assert_eq!(
        manage.get("agent_id").and_then(Value::as_str),
        Some("agent-7")
    );
}

#[test]
fn file_read_many_normalizes_single_path_into_paths_array() {
    let normalized = repair_and_validate_args("file_read_many", &json!({"path":"docs/plan.md"}))
        .expect("file_read_many");
    assert_eq!(normalized.get("path"), None);
    assert_eq!(
        normalized
            .get("paths")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(Value::as_str),
        Some("docs/plan.md")
    );
}

#[test]
fn workspace_analyze_synthesizes_query_from_workspace_pattern_aliases() {
    let normalized = repair_and_validate_args(
        "workspace_analyze",
        &json!({"workspace_path":"core/layer2/tooling", "pattern":"tool_route"}),
    )
    .expect("workspace_analyze");
    assert_eq!(
        normalized.get("query").and_then(Value::as_str),
        Some("search `tool_route` in `core/layer2/tooling`")
    );
}

#[test]
fn workspace_analyze_synthesizes_query_from_context_mentions() {
    let normalized = repair_and_validate_args(
        "workspace_analyze",
        &json!({"mentions":["tool_broker", "request_validation"]}),
    )
    .expect("workspace_analyze");
    assert_eq!(
        normalized.get("query").and_then(Value::as_str),
        Some("synthesize context from tool_broker, request_validation")
    );
    assert_eq!(
        normalized
            .get("context_mentions")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(2)
    );
    assert_eq!(
        normalized.get("route_hint").and_then(Value::as_str),
        Some("workspace_general")
    );
}

#[test]
fn file_read_many_parses_delimited_paths_string() {
    let normalized = repair_and_validate_args(
        "file_read_many",
        &json!({"paths":"docs/a.md, docs/b.md\ndocs/c.md"}),
    )
    .expect("file_read_many");
    assert_eq!(
        normalized
            .get("paths")
            .and_then(Value::as_array)
            .map(|rows| rows.len()),
        Some(3)
    );
}

#[test]
fn workspace_analyze_uses_command_alias_when_query_missing() {
    let normalized = repair_and_validate_args(
        "workspace_analyze",
        &json!({"powershell":"Get-ChildItem -Force"}),
    )
    .expect("workspace_analyze");
    assert_eq!(
        normalized.get("query").and_then(Value::as_str),
        Some("analyze command behavior `Get-ChildItem -Force`")
    );
    assert_eq!(
        normalized.get("route_hint").and_then(Value::as_str),
        Some("terminal_exec")
    );
}
