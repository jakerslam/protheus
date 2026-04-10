use crate::{deterministic_hash, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const TASK_FABRIC_NAMESPACE: &str = "task_fabric";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphNode {
    pub node_id: String,
    pub namespace: String,
    pub payload: Value,
    pub cas_version: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphEdge {
    pub edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub edge_type: String,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskFabricLease {
    pub lease_id: String,
    pub node_id: String,
    pub holder_principal: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub cas_snapshot: u64,
}

#[derive(Debug, Clone, Default)]
pub struct GraphSubsystem {
    nodes: BTreeMap<String, GraphNode>,
    edges: Vec<GraphEdge>,
    leases: BTreeMap<String, TaskFabricLease>,
}

fn lease_id(node_id: &str, holder_principal: &str, now: u64, ttl_ms: u64) -> String {
    format!(
        "lease_{}",
        &deterministic_hash(&(
            node_id.to_string(),
            holder_principal.to_string(),
            now,
            ttl_ms
        ))[..24]
    )
}

fn edge_id(source_node_id: &str, target_node_id: &str, edge_type: &str, now: u64) -> String {
    format!(
        "edge_{}",
        &deterministic_hash(&(
            source_node_id.to_string(),
            target_node_id.to_string(),
            edge_type.to_string(),
            now
        ))[..24]
    )
}

impl GraphSubsystem {
    pub fn create_task_node(&mut self, node_id: impl Into<String>, payload: Value) -> GraphNode {
        let now = now_ms();
        let node = GraphNode {
            node_id: node_id.into(),
            namespace: TASK_FABRIC_NAMESPACE.to_string(),
            payload,
            cas_version: 0,
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.nodes.insert(node.node_id.clone(), node.clone());
        node
    }

    pub fn get_node(&self, node_id: &str) -> Option<&GraphNode> {
        self.nodes.get(node_id)
    }

    pub fn issue_lease(
        &mut self,
        node_id: &str,
        holder_principal: &str,
        ttl_ms: u64,
    ) -> Result<TaskFabricLease, String> {
        let node = self
            .nodes
            .get(node_id)
            .ok_or_else(|| "task_node_not_found".to_string())?;
        let now = now_ms();
        let lease = TaskFabricLease {
            lease_id: lease_id(node_id, holder_principal, now, ttl_ms),
            node_id: node_id.to_string(),
            holder_principal: holder_principal.to_string(),
            issued_at_ms: now,
            expires_at_ms: now.saturating_add(ttl_ms.max(1)),
            cas_snapshot: node.cas_version,
        };
        self.leases.insert(lease.lease_id.clone(), lease.clone());
        Ok(lease)
    }

    pub fn mutate_task_node(
        &mut self,
        node_id: &str,
        lease_id: &str,
        holder_principal: &str,
        expected_cas: u64,
        payload: Value,
    ) -> Result<GraphNode, String> {
        self.validate_lease(node_id, lease_id, holder_principal)?;
        let now = now_ms();
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| "task_node_not_found".to_string())?;
        if node.cas_version != expected_cas {
            return Err("task_cas_mismatch".to_string());
        }
        node.payload = payload;
        node.cas_version = node.cas_version.saturating_add(1);
        node.updated_at_ms = now;
        Ok(node.clone())
    }

    pub fn add_edge(
        &mut self,
        source_node_id: &str,
        target_node_id: &str,
        lease_id: &str,
        holder_principal: &str,
        expected_source_cas: u64,
        edge_type: &str,
    ) -> Result<GraphEdge, String> {
        self.validate_lease(source_node_id, lease_id, holder_principal)?;
        let source = self
            .nodes
            .get(source_node_id)
            .ok_or_else(|| "source_task_node_not_found".to_string())?;
        if source.cas_version != expected_source_cas {
            return Err("task_cas_mismatch".to_string());
        }
        if !self.nodes.contains_key(target_node_id) {
            return Err("target_task_node_not_found".to_string());
        }
        let now = now_ms();
        let edge = GraphEdge {
            edge_id: edge_id(source_node_id, target_node_id, edge_type, now),
            source_node_id: source_node_id.to_string(),
            target_node_id: target_node_id.to_string(),
            edge_type: edge_type.to_string(),
            created_at_ms: now,
        };
        self.edges.push(edge.clone());
        Ok(edge)
    }

    pub fn edges(&self) -> &[GraphEdge] {
        self.edges.as_slice()
    }

    fn validate_lease(
        &self,
        node_id: &str,
        lease_id: &str,
        holder_principal: &str,
    ) -> Result<(), String> {
        let lease = self
            .leases
            .get(lease_id)
            .ok_or_else(|| "task_lease_not_found".to_string())?;
        if lease.node_id != node_id {
            return Err("task_lease_node_mismatch".to_string());
        }
        if lease.holder_principal != holder_principal {
            return Err("task_lease_holder_mismatch".to_string());
        }
        if now_ms() > lease.expires_at_ms {
            return Err("task_lease_expired".to_string());
        }
        Ok(())
    }
}
