// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use std::collections::{HashMap, HashSet};

use super::{SubdomainBoundary, SubdomainContract};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowNodeKind {
    Start,
    End,
    Agent,
    Parallel,
    Condition,
    Loop,
    Collect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowGraphNode {
    pub id: String,
    pub label: String,
    pub kind: WorkflowNodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowGraphConnection {
    pub from: String,
    pub from_port: usize,
    pub to: String,
    pub to_port: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowGraphDraft {
    pub name: String,
    pub nodes: Vec<WorkflowGraphNode>,
    pub connections: Vec<WorkflowGraphConnection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledStepMode {
    Sequential,
    FanOut,
    Conditional,
    Loop,
    Collect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledWorkflowStep {
    pub id: String,
    pub name: String,
    pub mode: CompiledStepMode,
    pub next: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowGraphValidationError {
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowGraphCompilation {
    pub steps: Vec<CompiledWorkflowStep>,
    pub validation_errors: Vec<WorkflowGraphValidationError>,
}

pub struct WorkflowGraphCompilationContract;

impl SubdomainContract for WorkflowGraphCompilationContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "workflow_graph_compilation",
        legacy_module_bindings: &[
            "workflow-builder",
            "workflow_builder_persist_trace_helpers",
            "workflow_builder_canvas_helpers",
            "workflows",
        ],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "execution_observation_snapshot",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "workflow_graph_validation_projection",
            "compiled_workflow_preview_projection",
            "workflow_sequence_projection",
        ],
        message_boundaries: &[
            "workflow_graph_to_shell_preview_boundary",
            "workflow_graph_to_runtime_persist_boundary",
            "workflow_graph_to_sequence_boundary",
        ],
    }
}

pub fn compile_workflow_graph(draft: &WorkflowGraphDraft) -> WorkflowGraphCompilation {
    let validation_errors = validate_graph(draft);
    if !validation_errors.is_empty() {
        return WorkflowGraphCompilation {
            steps: Vec::new(),
            validation_errors,
        };
    }

    let node_by_id = draft
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<HashMap<_, _>>();
    let step_name_by_id = draft
        .nodes
        .iter()
        .filter(|node| !matches!(node.kind, WorkflowNodeKind::Start | WorkflowNodeKind::End))
        .map(|node| (node.id.as_str(), normalized_step_name(node)))
        .collect::<HashMap<_, _>>();

    let mut steps = Vec::new();
    for node in draft
        .nodes
        .iter()
        .filter(|node| !matches!(node.kind, WorkflowNodeKind::Start | WorkflowNodeKind::End))
    {
        let mut outgoing = draft
            .connections
            .iter()
            .filter(|edge| edge.from == node.id)
            .collect::<Vec<_>>();
        outgoing.sort_by_key(|edge| edge.from_port);
        let next = outgoing
            .into_iter()
            .filter_map(|edge| {
                node_by_id.get(edge.to.as_str()).and_then(|target| {
                    if target.kind == WorkflowNodeKind::End {
                        None
                    } else {
                        step_name_by_id.get(target.id.as_str()).cloned()
                    }
                })
            })
            .collect::<Vec<_>>();
        steps.push(CompiledWorkflowStep {
            id: node.id.clone(),
            name: normalized_step_name(node),
            mode: step_mode(&node.kind),
            next,
        });
    }

    WorkflowGraphCompilation {
        steps,
        validation_errors: Vec::new(),
    }
}

fn validate_graph(draft: &WorkflowGraphDraft) -> Vec<WorkflowGraphValidationError> {
    let mut errors = Vec::new();
    if draft.name.trim().is_empty() {
        errors.push(error("missing_name", "workflow graph requires a name"));
    }
    if !draft
        .nodes
        .iter()
        .any(|node| node.kind == WorkflowNodeKind::Start)
    {
        errors.push(error(
            "missing_start",
            "workflow graph requires a start node",
        ));
    }
    if !draft
        .nodes
        .iter()
        .any(|node| node.kind == WorkflowNodeKind::End)
    {
        errors.push(error("missing_end", "workflow graph requires an end node"));
    }

    let mut ids = HashSet::new();
    for node in &draft.nodes {
        if node.id.trim().is_empty() {
            errors.push(error("missing_node_id", "workflow graph node id is empty"));
        } else if !ids.insert(node.id.as_str()) {
            errors.push(error(
                "duplicate_node_id",
                "workflow graph node id is duplicated",
            ));
        }
    }
    for edge in &draft.connections {
        if !ids.contains(edge.from.as_str()) || !ids.contains(edge.to.as_str()) {
            errors.push(error(
                "dangling_connection",
                "workflow graph connection references a missing node",
            ));
        }
    }
    errors
}

fn normalized_step_name(node: &WorkflowGraphNode) -> String {
    let label = node.label.trim();
    if label.is_empty() {
        node.id.clone()
    } else {
        label.to_string()
    }
}

fn step_mode(kind: &WorkflowNodeKind) -> CompiledStepMode {
    match kind {
        WorkflowNodeKind::Parallel => CompiledStepMode::FanOut,
        WorkflowNodeKind::Condition => CompiledStepMode::Conditional,
        WorkflowNodeKind::Loop => CompiledStepMode::Loop,
        WorkflowNodeKind::Collect => CompiledStepMode::Collect,
        WorkflowNodeKind::Start | WorkflowNodeKind::End | WorkflowNodeKind::Agent => {
            CompiledStepMode::Sequential
        }
    }
}

fn error(code: &str, detail: &str) -> WorkflowGraphValidationError {
    WorkflowGraphValidationError {
        code: code.to_string(),
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, label: &str, kind: WorkflowNodeKind) -> WorkflowGraphNode {
        WorkflowGraphNode {
            id: id.to_string(),
            label: label.to_string(),
            kind,
        }
    }

    fn edge(from: &str, port: usize, to: &str) -> WorkflowGraphConnection {
        WorkflowGraphConnection {
            from: from.to_string(),
            from_port: port,
            to: to.to_string(),
            to_port: 0,
        }
    }

    #[test]
    fn compiles_sequential_graph() {
        let draft = WorkflowGraphDraft {
            name: "sequential".to_string(),
            nodes: vec![
                node("start", "Start", WorkflowNodeKind::Start),
                node("a", "Draft", WorkflowNodeKind::Agent),
                node("b", "Review", WorkflowNodeKind::Agent),
                node("end", "End", WorkflowNodeKind::End),
            ],
            connections: vec![
                edge("start", 0, "a"),
                edge("a", 0, "b"),
                edge("b", 0, "end"),
            ],
        };

        let compiled = compile_workflow_graph(&draft);

        assert!(compiled.validation_errors.is_empty());
        assert_eq!(compiled.steps[0].mode, CompiledStepMode::Sequential);
        assert_eq!(compiled.steps[0].next, vec!["Review".to_string()]);
    }

    #[test]
    fn compiles_conditional_branches_by_port_order() {
        let draft = WorkflowGraphDraft {
            name: "conditional".to_string(),
            nodes: vec![
                node("start", "Start", WorkflowNodeKind::Start),
                node("cond", "Check", WorkflowNodeKind::Condition),
                node("yes", "Yes", WorkflowNodeKind::Agent),
                node("no", "No", WorkflowNodeKind::Agent),
                node("end", "End", WorkflowNodeKind::End),
            ],
            connections: vec![
                edge("start", 0, "cond"),
                edge("cond", 1, "no"),
                edge("cond", 0, "yes"),
                edge("yes", 0, "end"),
                edge("no", 0, "end"),
            ],
        };

        let compiled = compile_workflow_graph(&draft);
        let condition = compiled
            .steps
            .iter()
            .find(|step| step.id == "cond")
            .expect("condition should compile");

        assert_eq!(condition.mode, CompiledStepMode::Conditional);
        assert_eq!(condition.next, vec!["Yes".to_string(), "No".to_string()]);
    }

    #[test]
    fn compiles_fan_out_targets() {
        let draft = WorkflowGraphDraft {
            name: "fanout".to_string(),
            nodes: vec![
                node("start", "Start", WorkflowNodeKind::Start),
                node("fan", "Fan", WorkflowNodeKind::Parallel),
                node("a", "A", WorkflowNodeKind::Agent),
                node("b", "B", WorkflowNodeKind::Agent),
                node("end", "End", WorkflowNodeKind::End),
            ],
            connections: vec![
                edge("start", 0, "fan"),
                edge("fan", 0, "a"),
                edge("fan", 1, "b"),
                edge("a", 0, "end"),
                edge("b", 0, "end"),
            ],
        };

        let fan = compile_workflow_graph(&draft)
            .steps
            .into_iter()
            .find(|step| step.id == "fan")
            .expect("fan-out should compile");

        assert_eq!(fan.mode, CompiledStepMode::FanOut);
        assert_eq!(fan.next, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn compiles_loop_and_collect_nodes() {
        let draft = WorkflowGraphDraft {
            name: "loop_collect".to_string(),
            nodes: vec![
                node("start", "Start", WorkflowNodeKind::Start),
                node("loop", "Loop", WorkflowNodeKind::Loop),
                node("collect", "Collect", WorkflowNodeKind::Collect),
                node("end", "End", WorkflowNodeKind::End),
            ],
            connections: vec![
                edge("start", 0, "loop"),
                edge("loop", 0, "collect"),
                edge("collect", 0, "end"),
            ],
        };

        let compiled = compile_workflow_graph(&draft);

        assert_eq!(compiled.steps[0].mode, CompiledStepMode::Loop);
        assert_eq!(compiled.steps[1].mode, CompiledStepMode::Collect);
    }

    #[test]
    fn invalid_graph_returns_structured_validation_errors() {
        let draft = WorkflowGraphDraft {
            name: String::new(),
            nodes: vec![node("a", "A", WorkflowNodeKind::Agent)],
            connections: vec![edge("a", 0, "missing")],
        };

        let compiled = compile_workflow_graph(&draft);
        let codes = compiled
            .validation_errors
            .iter()
            .map(|row| row.code.as_str())
            .collect::<Vec<_>>();

        assert!(compiled.steps.is_empty());
        assert!(codes.contains(&"missing_name"));
        assert!(codes.contains(&"missing_start"));
        assert!(codes.contains(&"missing_end"));
        assert!(codes.contains(&"dangling_connection"));
    }
}
