use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphNodeKind {
    Action,
    Branch,
    LoopCheck,
    ParallelFanout,
    HitlGate,
    End,
}

impl Default for GraphNodeKind {
    fn default() -> Self {
        Self::Action
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphNode {
    pub id: String,
    #[serde(default)]
    pub kind: GraphNodeKind,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub checkpoint: bool,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphWorkflowDefinition {
    pub workflow_id: String,
    pub start_node: String,
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
    #[serde(default = "default_max_steps")]
    pub max_steps: u32,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HitlDecision {
    pub node_id: String,
    pub approved: bool,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphExecutionInput {
    pub workflow: GraphWorkflowDefinition,
    #[serde(default)]
    pub resume_from: Option<GraphCheckpoint>,
    #[serde(default)]
    pub context: BTreeMap<String, String>,
    #[serde(default)]
    pub hitl_decisions: Vec<HitlDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphCheckpoint {
    pub checkpoint_id: String,
    pub workflow_id: String,
    pub node_id: String,
    pub cursor: u32,
    pub context: BTreeMap<String, String>,
    pub event_digest: String,
    pub created_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphRuntimeEvent {
    pub index: u32,
    pub node_id: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphExecutionReceipt {
    pub workflow_id: String,
    pub status: String,
    pub cursor: u32,
    pub current_node: Option<String>,
    pub pause_reason: Option<String>,
    pub hitl_pending: bool,
    pub checkpoint: Option<GraphCheckpoint>,
    pub event_digest: String,
    pub events: Vec<GraphRuntimeEvent>,
    pub metadata: BTreeMap<String, String>,
}

pub trait GraphCheckpointStore {
    fn save(&mut self, checkpoint: GraphCheckpoint) -> Result<(), String>;
    fn latest(&self, workflow_id: &str) -> Option<GraphCheckpoint>;
    fn count(&self, workflow_id: &str) -> usize;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InMemoryCheckpointStore {
    pub by_workflow: BTreeMap<String, Vec<GraphCheckpoint>>,
}

impl GraphCheckpointStore for InMemoryCheckpointStore {
    fn save(&mut self, checkpoint: GraphCheckpoint) -> Result<(), String> {
        self.by_workflow
            .entry(checkpoint.workflow_id.clone())
            .or_default()
            .push(checkpoint);
        Ok(())
    }

    fn latest(&self, workflow_id: &str) -> Option<GraphCheckpoint> {
        self.by_workflow
            .get(workflow_id)
            .and_then(|items| items.last())
            .cloned()
    }

    fn count(&self, workflow_id: &str) -> usize {
        self.by_workflow
            .get(workflow_id)
            .map(|items| items.len())
            .unwrap_or(0)
    }
}

pub fn run_graph_workflow_json(input_json: &str) -> Result<String, String> {
    let input: GraphExecutionInput =
        serde_json::from_str(input_json).map_err(|error| format!("graph_input_parse_failed:{error}"))?;
    let mut store = InMemoryCheckpointStore::default();
    let receipt = run_graph_workflow(&input, &mut store)?;
    serde_json::to_string(&receipt).map_err(|error| format!("graph_receipt_encode_failed:{error}"))
}

pub fn run_graph_workflow(
    input: &GraphExecutionInput,
    store: &mut dyn GraphCheckpointStore,
) -> Result<GraphExecutionReceipt, String> {
    validate_graph(&input.workflow)?;

    let workflow_id = normalized_token(&input.workflow.workflow_id, "workflow_id")?;
    let max_steps = input.workflow.max_steps.max(1);
    let mut context = input.context.clone();
    let mut events = Vec::<GraphRuntimeEvent>::new();
    let mut cursor = input.resume_from.as_ref().map(|cp| cp.cursor).unwrap_or(0);
    let mut current_node = input
        .resume_from
        .as_ref()
        .map(|cp| cp.node_id.clone())
        .unwrap_or_else(|| input.workflow.start_node.clone());

    if let Some(resume) = &input.resume_from {
        if resume.workflow_id != workflow_id {
            return Err("graph_resume_workflow_mismatch".to_string());
        }
        for (key, value) in &resume.context {
            context.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }

    let mut hitl_map = BTreeMap::<String, HitlDecision>::new();
    for decision in &input.hitl_decisions {
        hitl_map.insert(decision.node_id.clone(), decision.clone());
    }

    loop {
        if cursor >= max_steps {
            return Ok(build_receipt(
                &workflow_id,
                "failed",
                cursor,
                Some(current_node.clone()),
                Some("graph_max_steps_exceeded".to_string()),
                false,
                store.latest(&workflow_id),
                &events,
                &input.workflow.metadata,
            ));
        }

        let node = find_node(&input.workflow, &current_node)?;
        let event_index = events.len() as u32;
        events.push(GraphRuntimeEvent {
            index: event_index,
            node_id: node.id.clone(),
            status: "entered".to_string(),
            detail: node.kind.as_label().to_string(),
        });

        if matches!(node.kind, GraphNodeKind::HitlGate) {
            if let Some(decision) = hitl_map.get(&node.id) {
                if !decision.approved {
                    events.push(GraphRuntimeEvent {
                        index: events.len() as u32,
                        node_id: node.id.clone(),
                        status: "blocked".to_string(),
                        detail: format!("hitl_rejected:{}", sanitize_detail(&decision.note)),
                    });
                    let checkpoint = save_checkpoint(
                        &workflow_id,
                        &node.id,
                        cursor,
                        &context,
                        &events,
                        store,
                    )?;
                    return Ok(build_receipt(
                        &workflow_id,
                        "failed",
                        cursor,
                        Some(node.id.clone()),
                        Some("hitl_rejected".to_string()),
                        false,
                        Some(checkpoint),
                        &events,
                        &input.workflow.metadata,
                    ));
                }
                events.push(GraphRuntimeEvent {
                    index: events.len() as u32,
                    node_id: node.id.clone(),
                    status: "approved".to_string(),
                    detail: sanitize_detail(&decision.note),
                });
            } else {
                let checkpoint =
                    save_checkpoint(&workflow_id, &node.id, cursor, &context, &events, store)?;
                return Ok(build_receipt(
                    &workflow_id,
                    "paused",
                    cursor,
                    Some(node.id.clone()),
                    Some(format!("hitl_approval_required:{}", node.id)),
                    true,
                    Some(checkpoint),
                    &events,
                    &input.workflow.metadata,
                ));
            }
        }

        if node.checkpoint || matches!(node.kind, GraphNodeKind::HitlGate | GraphNodeKind::End) {
            let _ = save_checkpoint(&workflow_id, &node.id, cursor, &context, &events, store)?;
        }

        if matches!(node.kind, GraphNodeKind::End) {
            return Ok(build_receipt(
                &workflow_id,
                "completed",
                cursor,
                Some(node.id.clone()),
                None,
                false,
                store.latest(&workflow_id),
                &events,
                &input.workflow.metadata,
            ));
        }

        let outgoing = outgoing_edges(&input.workflow, &node.id);
        if outgoing.is_empty() {
            return Ok(build_receipt(
                &workflow_id,
                "completed",
                cursor,
                Some(node.id.clone()),
                Some("graph_terminal_without_edges".to_string()),
                false,
                store.latest(&workflow_id),
                &events,
                &input.workflow.metadata,
            ));
        }

        let condition = edge_condition_for_node(node, &mut context);
        let selected = select_edge(&outgoing, condition.as_deref())?;
        current_node = selected.to.clone();
        cursor = cursor.saturating_add(1);
    }
}

fn save_checkpoint(
    workflow_id: &str,
    node_id: &str,
    cursor: u32,
    context: &BTreeMap<String, String>,
    events: &[GraphRuntimeEvent],
    store: &mut dyn GraphCheckpointStore,
) -> Result<GraphCheckpoint, String> {
    let created_unix_ms = now_unix_ms();
    let event_digest = digest_events(events);
    let checkpoint_id = format!("cp_{}", stable_hash(&[
        workflow_id.to_string(),
        node_id.to_string(),
        cursor.to_string(),
        event_digest.clone(),
        created_unix_ms.to_string(),
    ]));
    let checkpoint = GraphCheckpoint {
        checkpoint_id,
        workflow_id: workflow_id.to_string(),
        node_id: node_id.to_string(),
        cursor,
        context: context.clone(),
        event_digest,
        created_unix_ms,
    };
    store.save(checkpoint.clone())?;
    Ok(checkpoint)
}

fn build_receipt(
    workflow_id: &str,
    status: &str,
    cursor: u32,
    current_node: Option<String>,
    pause_reason: Option<String>,
    hitl_pending: bool,
    checkpoint: Option<GraphCheckpoint>,
    events: &[GraphRuntimeEvent],
    metadata: &BTreeMap<String, String>,
) -> GraphExecutionReceipt {
    GraphExecutionReceipt {
        workflow_id: workflow_id.to_string(),
        status: status.to_string(),
        cursor,
        current_node,
        pause_reason,
        hitl_pending,
        checkpoint,
        event_digest: digest_events(events),
        events: events.to_vec(),
        metadata: metadata.clone(),
    }
}

fn validate_graph(workflow: &GraphWorkflowDefinition) -> Result<(), String> {
    let _workflow_id = normalized_token(&workflow.workflow_id, "workflow_id")?;
    let start = normalized_token(&workflow.start_node, "start_node")?;
    if workflow.nodes.is_empty() {
        return Err("graph_nodes_required".to_string());
    }
    let mut nodes = BTreeSet::<String>::new();
    for node in &workflow.nodes {
        let node_id = normalized_token(&node.id, "node_id")?;
        if !nodes.insert(node_id.clone()) {
            return Err(format!("graph_duplicate_node:{node_id}"));
        }
    }
    if !nodes.contains(&start) {
        return Err("graph_start_node_missing".to_string());
    }
    for edge in &workflow.edges {
        if !nodes.contains(&edge.from) || !nodes.contains(&edge.to) {
            return Err(format!("graph_edge_missing_node:{}->{}", edge.from, edge.to));
        }
    }
    Ok(())
}

fn edge_condition_for_node(node: &GraphNode, context: &mut BTreeMap<String, String>) -> Option<String> {
    match node.kind {
        GraphNodeKind::Branch => {
            let key = node.params.get("predicate_key")?;
            let expected = node.params.get("expected_value").cloned().unwrap_or_default();
            let actual = context.get(key).cloned().unwrap_or_default();
            if actual == expected {
                Some("true".to_string())
            } else {
                Some("false".to_string())
            }
        }
        GraphNodeKind::LoopCheck => {
            let key = node
                .params
                .get("counter_key")
                .cloned()
                .unwrap_or_else(|| format!("{}_loop_count", node.id));
            let max_iterations = node
                .params
                .get("max_iterations")
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(1);
            let count = context
                .get(&key)
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(0)
                .saturating_add(1);
            context.insert(key, count.to_string());
            if count <= max_iterations {
                Some("loop".to_string())
            } else {
                Some("exit".to_string())
            }
        }
        GraphNodeKind::ParallelFanout => Some("parallel".to_string()),
        _ => None,
    }
}

fn find_node<'a>(workflow: &'a GraphWorkflowDefinition, node_id: &str) -> Result<&'a GraphNode, String> {
    workflow
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| format!("graph_node_not_found:{node_id}"))
}

fn outgoing_edges<'a>(workflow: &'a GraphWorkflowDefinition, node_id: &str) -> Vec<&'a GraphEdge> {
    let mut edges = workflow
        .edges
        .iter()
        .filter(|edge| edge.from == node_id)
        .collect::<Vec<_>>();
    edges.sort_by(|left, right| left.to.cmp(&right.to).then_with(|| left.condition.cmp(&right.condition)));
    edges
}

fn select_edge<'a>(edges: &[&'a GraphEdge], condition: Option<&str>) -> Result<&'a GraphEdge, String> {
    if let Some(condition) = condition {
        if let Some(edge) = edges.iter().copied().find(|edge| edge.condition == condition) {
            return Ok(edge);
        }
    }
    edges
        .first()
        .copied()
        .ok_or_else(|| "graph_edge_missing".to_string())
}

fn normalized_token(raw: &str, field: &str) -> Result<String, String> {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return Err(format!("{field}_required"));
    }
    if cleaned.len() > 180 {
        return Err(format!("{field}_too_long"));
    }
    Ok(cleaned.to_string())
}

fn sanitize_detail(raw: &str) -> String {
    raw.chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
        .take(200)
        .collect::<String>()
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn digest_events(events: &[GraphRuntimeEvent]) -> String {
    let lines = events
        .iter()
        .map(|event| format!("{}:{}:{}:{}", event.index, event.node_id, event.status, event.detail))
        .collect::<Vec<_>>();
    stable_hash(&lines)
}

fn stable_hash(lines: &[String]) -> String {
    let mut hasher = Sha256::new();
    for (index, line) in lines.iter().enumerate() {
        hasher.update(format!("{index}:{line}|").as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn default_max_steps() -> u32 {
    128
}

impl GraphNodeKind {
    fn as_label(&self) -> &'static str {
        match self {
            Self::Action => "action",
            Self::Branch => "branch",
            Self::LoopCheck => "loop_check",
            Self::ParallelFanout => "parallel_fanout",
            Self::HitlGate => "hitl_gate",
            Self::End => "end",
        }
    }
}
