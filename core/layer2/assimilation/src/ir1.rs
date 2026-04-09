// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/assimilation (authoritative).

use crate::ir0::Ir0ArtifactGraph;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryModel {
    pub little_endian: bool,
    pub pointer_width_bits: u16,
    pub supports_mmio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionBlock {
    pub block_id: String,
    pub source_region_id: String,
    pub start_offset: u64,
    pub end_offset: u64,
    pub successors: Vec<String>,
    pub register_reads: Vec<String>,
    pub register_writes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrontierNode {
    pub path_id: String,
    pub block_id: String,
    pub symbolic_constraints: Vec<String>,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SymbolicFrontier {
    pub open_paths: Vec<FrontierNode>,
    pub exhausted_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ir1ExecutionStructure {
    pub ir1_id: String,
    pub artifact_hash: String,
    pub blocks: Vec<ExecutionBlock>,
    pub register_model: Vec<String>,
    pub memory_model: MemoryModel,
    pub frontier: SymbolicFrontier,
    pub reversible_region_map: BTreeMap<String, String>,
}

impl Ir1ExecutionStructure {
    pub fn commit_from_ir0(ir0: &Ir0ArtifactGraph) -> Self {
        let mut blocks = Vec::new();
        let mut reversible_region_map = BTreeMap::new();
        let mut open_paths = Vec::new();
        for (idx, region) in ir0.regions.iter().enumerate() {
            let block_id = format!("blk-{idx:04}");
            blocks.push(ExecutionBlock {
                block_id: block_id.clone(),
                source_region_id: region.region_id.clone(),
                start_offset: region.offset,
                end_offset: region.end_offset(),
                successors: Vec::new(),
                register_reads: vec!["pc".to_string()],
                register_writes: vec!["pc".to_string()],
            });
            reversible_region_map.insert(block_id.clone(), region.region_id.clone());
            open_paths.push(FrontierNode {
                path_id: format!("path-{idx:04}"),
                block_id,
                symbolic_constraints: vec!["entry".to_string()],
                depth: 0,
            });
        }
        Self {
            ir1_id: format!("ir1:{}", ir0.artifact_id),
            artifact_hash: ir0.artifact_hash.clone(),
            blocks,
            register_model: vec![
                "pc".to_string(),
                "sp".to_string(),
                "flags".to_string(),
                "acc".to_string(),
            ],
            memory_model: MemoryModel {
                little_endian: true,
                pointer_width_bits: 64,
                supports_mmio: true,
            },
            frontier: SymbolicFrontier {
                open_paths,
                exhausted_paths: Vec::new(),
            },
            reversible_region_map,
        }
    }

    pub fn validate_against_ir0(&self, ir0: &Ir0ArtifactGraph) -> Result<(), String> {
        if self.artifact_hash != ir0.artifact_hash {
            return Err("ir1_ir0_hash_mismatch".to_string());
        }
        if self.blocks.is_empty() {
            return Err("ir1_empty_blocks".to_string());
        }
        for block in &self.blocks {
            if block.start_offset >= block.end_offset {
                return Err(format!("ir1_invalid_block_span:{}", block.block_id));
            }
            let region = ir0
                .regions
                .iter()
                .find(|region| region.region_id == block.source_region_id)
                .ok_or_else(|| format!("ir1_unknown_source_region:{}", block.block_id))?;
            if block.start_offset < region.offset || block.end_offset > region.end_offset() {
                return Err(format!("ir1_block_outside_region:{}", block.block_id));
            }
            if self.reversible_region_map.get(&block.block_id) != Some(&region.region_id) {
                return Err(format!(
                    "ir1_missing_reversible_region_link:{}",
                    block.block_id
                ));
            }
        }
        Ok(())
    }
}
