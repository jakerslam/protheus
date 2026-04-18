use crate::graph_runtime::{
    run_graph_workflow, GraphCheckpointStore, GraphEdge, GraphExecutionInput, GraphNode,
    GraphNodeKind, GraphWorkflowDefinition, HitlDecision, InMemoryCheckpointStore,
};
use std::collections::BTreeMap;

fn demo_workflow() -> GraphWorkflowDefinition {
    GraphWorkflowDefinition {
        workflow_id: "wf.demo".to_string(),
        start_node: "start".to_string(),
        max_steps: 20,
        nodes: vec![
            GraphNode {
                id: "start".to_string(),
                kind: GraphNodeKind::Action,
                action: "collect".to_string(),
                command: String::new(),
                checkpoint: true,
                params: BTreeMap::new(),
            },
            GraphNode {
                id: "approve".to_string(),
                kind: GraphNodeKind::HitlGate,
                action: String::new(),
                command: String::new(),
                checkpoint: true,
                params: BTreeMap::new(),
            },
            GraphNode {
                id: "done".to_string(),
                kind: GraphNodeKind::End,
                action: String::new(),
                command: String::new(),
                checkpoint: true,
                params: BTreeMap::new(),
            },
        ],
        edges: vec![
            GraphEdge {
                from: "start".to_string(),
                to: "approve".to_string(),
                condition: String::new(),
            },
            GraphEdge {
                from: "approve".to_string(),
                to: "done".to_string(),
                condition: String::new(),
            },
        ],
        metadata: BTreeMap::new(),
    }
}

#[test]
fn graph_pauses_without_hitl_decision() {
    let input = GraphExecutionInput {
        workflow: demo_workflow(),
        resume_from: None,
        context: BTreeMap::new(),
        hitl_decisions: Vec::new(),
    };
    let mut store = InMemoryCheckpointStore::default();
    let receipt = run_graph_workflow(&input, &mut store).expect("graph");
    assert_eq!(receipt.status, "paused");
    assert!(receipt.hitl_pending);
    assert_eq!(store.count("wf.demo"), 2);
}

#[test]
fn graph_completes_with_hitl_approval() {
    let input = GraphExecutionInput {
        workflow: demo_workflow(),
        resume_from: None,
        context: BTreeMap::new(),
        hitl_decisions: vec![HitlDecision {
            node_id: "approve".to_string(),
            approved: true,
            note: "approved".to_string(),
        }],
    };
    let mut store = InMemoryCheckpointStore::default();
    let receipt = run_graph_workflow(&input, &mut store).expect("graph");
    assert_eq!(receipt.status, "completed");
    assert!(!receipt.hitl_pending);
    assert_eq!(receipt.current_node.as_deref(), Some("done"));
}

#[test]
fn graph_loop_condition_routes_deterministically() {
    let workflow = GraphWorkflowDefinition {
        workflow_id: "wf.loop".to_string(),
        start_node: "check".to_string(),
        max_steps: 10,
        nodes: vec![
            GraphNode {
                id: "check".to_string(),
                kind: GraphNodeKind::LoopCheck,
                action: String::new(),
                command: String::new(),
                checkpoint: false,
                params: BTreeMap::from([("max_iterations".to_string(), "2".to_string())]),
            },
            GraphNode {
                id: "body".to_string(),
                kind: GraphNodeKind::Action,
                action: "work".to_string(),
                command: String::new(),
                checkpoint: false,
                params: BTreeMap::new(),
            },
            GraphNode {
                id: "done".to_string(),
                kind: GraphNodeKind::End,
                action: String::new(),
                command: String::new(),
                checkpoint: false,
                params: BTreeMap::new(),
            },
        ],
        edges: vec![
            GraphEdge {
                from: "check".to_string(),
                to: "body".to_string(),
                condition: "loop".to_string(),
            },
            GraphEdge {
                from: "check".to_string(),
                to: "done".to_string(),
                condition: "exit".to_string(),
            },
            GraphEdge {
                from: "body".to_string(),
                to: "check".to_string(),
                condition: String::new(),
            },
        ],
        metadata: BTreeMap::new(),
    };
    let input = GraphExecutionInput {
        workflow,
        resume_from: None,
        context: BTreeMap::new(),
        hitl_decisions: Vec::new(),
    };
    let mut store = InMemoryCheckpointStore::default();
    let receipt = run_graph_workflow(&input, &mut store).expect("graph");
    assert_eq!(receipt.status, "completed");
    assert_eq!(receipt.current_node.as_deref(), Some("done"));
}
