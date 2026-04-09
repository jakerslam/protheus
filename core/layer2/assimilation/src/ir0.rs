// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/assimilation (authoritative).

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvenanceAnchor {
    pub artifact_hash: String,
    pub offset: u64,
    pub length: u64,
    pub source_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecodeCandidate {
    pub candidate_id: String,
    pub decoder: String,
    pub hypothesis: String,
    pub window_start: u64,
    pub window_end: u64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Ir0EdgeKind {
    OverlapWindow,
    JumpTable,
    EmbeddedBlob,
    Alias,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ir0Edge {
    pub from_region_id: String,
    pub to_region_id: String,
    pub kind: Ir0EdgeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ir0Region {
    pub region_id: String,
    pub offset: u64,
    pub length: u64,
    pub bytes_sha256: String,
    pub interleaved_code_data: bool,
    pub packed_region: bool,
    pub embedded_blob: bool,
    pub partial_self_modifying: bool,
    pub decode_candidates: Vec<DecodeCandidate>,
    pub provenance_anchor: ProvenanceAnchor,
}

impl Ir0Region {
    pub fn end_offset(&self) -> u64 {
        self.offset.saturating_add(self.length)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ir0ArtifactGraph {
    pub artifact_id: String,
    pub artifact_hash: String,
    pub regions: Vec<Ir0Region>,
    pub edges: Vec<Ir0Edge>,
}

impl Ir0ArtifactGraph {
    pub fn validate(&self) -> Result<(), String> {
        if self.artifact_hash.trim().is_empty() {
            return Err("ir0_missing_artifact_hash".to_string());
        }
        if self.regions.is_empty() {
            return Err("ir0_empty_regions".to_string());
        }
        let mut ids = BTreeSet::new();
        for region in &self.regions {
            if !ids.insert(region.region_id.clone()) {
                return Err(format!("ir0_duplicate_region_id:{}", region.region_id));
            }
            if region.length == 0 {
                return Err(format!("ir0_zero_length_region:{}", region.region_id));
            }
            if region.provenance_anchor.artifact_hash != self.artifact_hash {
                return Err(format!("ir0_anchor_artifact_mismatch:{}", region.region_id));
            }
            if region.provenance_anchor.offset != region.offset
                || region.provenance_anchor.length != region.length
            {
                return Err(format!("ir0_anchor_offset_mismatch:{}", region.region_id));
            }
            for candidate in &region.decode_candidates {
                if candidate.confidence < 0.0 || candidate.confidence > 1.0 {
                    return Err(format!(
                        "ir0_decode_confidence_out_of_range:{}:{}",
                        region.region_id, candidate.candidate_id
                    ));
                }
                if candidate.window_start < region.offset
                    || candidate.window_end > region.end_offset()
                    || candidate.window_start >= candidate.window_end
                {
                    return Err(format!(
                        "ir0_decode_window_out_of_bounds:{}:{}",
                        region.region_id, candidate.candidate_id
                    ));
                }
            }
        }
        let known = ids;
        for edge in &self.edges {
            if !known.contains(&edge.from_region_id) || !known.contains(&edge.to_region_id) {
                return Err(format!(
                    "ir0_edge_unknown_region:{}->{}",
                    edge.from_region_id, edge.to_region_id
                ));
            }
        }
        Ok(())
    }

    pub fn region_for_offset(&self, offset: u64) -> Option<&Ir0Region> {
        self.regions
            .iter()
            .find(|region| offset >= region.offset && offset < region.end_offset())
    }
}
