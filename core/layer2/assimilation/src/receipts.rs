// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/assimilation (authoritative).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformationReceipt {
    pub receipt_id: String,
    pub parent_receipt_ids: Vec<String>,
    pub artifact_hash: String,
    pub plane: String,
    pub stage: String,
    pub action: String,
    pub policy_version: String,
    pub toolchain_fingerprint: String,
    pub assumption_set_hash: String,
    pub equivalence_scope: String,
    pub proof_type: String,
    pub confidence: f64,
    pub coverage: f64,
    pub uncertainty_vector: Vec<f64>,
    pub capability_gaps: Vec<String>,
    pub degraded: bool,
    pub event_id: String,
}

impl TransformationReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.receipt_id.trim().is_empty()
            || self.artifact_hash.trim().is_empty()
            || self.plane.trim().is_empty()
            || self.stage.trim().is_empty()
            || self.action.trim().is_empty()
            || self.policy_version.trim().is_empty()
            || self.toolchain_fingerprint.trim().is_empty()
            || self.assumption_set_hash.trim().is_empty()
            || self.equivalence_scope.trim().is_empty()
            || self.proof_type.trim().is_empty()
            || self.event_id.trim().is_empty()
        {
            return Err("receipt_missing_required_field".to_string());
        }
        if !(0.0..=1.0).contains(&self.confidence) || !(0.0..=1.0).contains(&self.coverage) {
            return Err("receipt_score_out_of_range".to_string());
        }
        if self
            .uncertainty_vector
            .iter()
            .any(|value| !(0.0..=1.0).contains(value))
        {
            return Err("receipt_uncertainty_out_of_range".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyGraph {
    pub edges: BTreeMap<String, BTreeSet<String>>,
}

impl DependencyGraph {
    pub fn add_dependency(&mut self, parent_receipt_id: &str, child_receipt_id: &str) {
        self.edges
            .entry(parent_receipt_id.to_string())
            .or_default()
            .insert(child_receipt_id.to_string());
    }

    pub fn dependents(&self, receipt_id: &str) -> Vec<String> {
        self.edges
            .get(receipt_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResumeIndex {
    pub latest_by_artifact: BTreeMap<String, String>,
}

impl ResumeIndex {
    pub fn update(&mut self, artifact_hash: &str, receipt_id: &str) {
        self.latest_by_artifact
            .insert(artifact_hash.to_string(), receipt_id.to_string());
    }

    pub fn latest_receipt_id(&self, artifact_hash: &str) -> Option<&str> {
        self.latest_by_artifact
            .get(artifact_hash)
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvalidationPlanner;

impl InvalidationPlanner {
    pub fn invalidate_receipt_lineage(
        graph: &DependencyGraph,
        root_receipt_id: &str,
    ) -> Vec<String> {
        let mut invalidated = Vec::new();
        let mut queue = vec![root_receipt_id.to_string()];
        while let Some(current) = queue.pop() {
            if invalidated.contains(&current) {
                continue;
            }
            invalidated.push(current.clone());
            queue.extend(graph.dependents(&current));
        }
        invalidated
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UncertaintyEngine;

impl UncertaintyEngine {
    pub fn propagate(
        parent_receipts: &[TransformationReceipt],
        local_uncertainty: &[f64],
    ) -> Vec<f64> {
        if parent_receipts.is_empty() {
            return local_uncertainty.to_vec();
        }
        let mut aggregate = vec![0.0f64; local_uncertainty.len()];
        for parent in parent_receipts {
            for (idx, value) in parent.uncertainty_vector.iter().enumerate() {
                if idx < aggregate.len() {
                    aggregate[idx] += *value;
                }
            }
        }
        for value in &mut aggregate {
            *value /= parent_receipts.len() as f64;
        }
        for (idx, local) in local_uncertainty.iter().enumerate() {
            aggregate[idx] = ((aggregate[idx] + *local) / 2.0).clamp(0.0, 1.0);
        }
        aggregate
    }
}

pub fn receipt_id(seed: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    format!("rcpt:{:x}", hasher.finalize())
}
