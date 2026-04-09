// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/assimilation (authoritative).

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentAssumptionClass {
    NativeOs,
    EmulatedOs,
    TimingSensitive,
    MmioHeavy,
    FixedPoint,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrontierPath {
    pub path_id: String,
    pub symbolic_constraints: Vec<String>,
    pub information_gain_score: f64,
    pub compute_cost_score: f64,
    pub assumption_class: EnvironmentAssumptionClass,
    pub pruned: bool,
    pub prune_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrontierManager {
    pub max_paths: usize,
    pub bailout_min_info_gain: f64,
}

impl Default for FrontierManager {
    fn default() -> Self {
        Self {
            max_paths: 256,
            bailout_min_info_gain: 0.05,
        }
    }
}

impl FrontierManager {
    pub fn prioritize(&self, paths: &mut [FrontierPath]) {
        paths.sort_by(|a, b| {
            let left = normalized_gain(a);
            let right = normalized_gain(b);
            right
                .partial_cmp(&left)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.path_id.cmp(&b.path_id))
        });
    }

    pub fn prune(&self, mut paths: Vec<FrontierPath>) -> (Vec<FrontierPath>, Vec<FrontierPath>) {
        self.prioritize(&mut paths);
        let mut kept = Vec::new();
        let mut pruned = Vec::new();
        for (idx, mut path) in paths.into_iter().enumerate() {
            if normalized_gain(&path) < self.bailout_min_info_gain {
                path.pruned = true;
                path.prune_reason = Some("bailout_low_information_gain".to_string());
                pruned.push(path);
                continue;
            }
            if idx >= self.max_paths {
                path.pruned = true;
                path.prune_reason = Some("prune_max_paths".to_string());
                pruned.push(path);
                continue;
            }
            kept.push(path);
        }
        (kept, pruned)
    }
}

fn normalized_gain(path: &FrontierPath) -> f64 {
    let cost = if path.compute_cost_score <= 0.0 {
        1.0
    } else {
        path.compute_cost_score
    };
    (path.information_gain_score / cost).max(0.0)
}
