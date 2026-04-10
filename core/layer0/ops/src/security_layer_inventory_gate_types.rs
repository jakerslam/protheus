// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct InventoryConfig {
    #[serde(default)]
    pub layers: Vec<LayerConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LayerConfig {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub implementation_paths: Vec<String>,
    #[serde(default)]
    pub policy_paths: Vec<String>,
    #[serde(default)]
    pub test_paths: Vec<String>,
    #[serde(default)]
    pub guard_check_ids: Vec<String>,
    #[serde(default)]
    pub runtime_checks: Vec<RuntimeCheckSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RuntimeCheckSpec {
    #[serde(default)]
    pub plane: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GuardRegistry {
    #[serde(default)]
    pub merge_guard: GuardMerge,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GuardMerge {
    #[serde(default)]
    pub checks: Vec<GuardCheck>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GuardCheck {
    #[serde(default)]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct MissingPath {
    pub path: String,
}

impl MissingPath {
    pub fn from_path(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct MissingGuardCheck {
    pub id: String,
}

impl MissingGuardCheck {
    pub fn from_id(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RuntimeCheckResult {
    pub plane: String,
    pub command: String,
    pub args: Vec<String>,
    pub ok: bool,
    pub reachable: bool,
    pub policy_fail_closed: bool,
    pub status: i32,
    pub stderr: Option<String>,
    pub output_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LayerResult {
    pub id: String,
    pub title: String,
    pub implementation_paths: Vec<String>,
    pub policy_paths: Vec<String>,
    pub test_paths: Vec<String>,
    pub guard_check_ids: Vec<String>,
    pub runtime_checks: Vec<RuntimeCheckResult>,
    pub missing_paths: Vec<MissingPath>,
    pub missing_guard_checks: Vec<MissingGuardCheck>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct InventorySummary {
    pub layers_checked: usize,
    pub missing_paths: usize,
    pub missing_guard_checks: usize,
    pub runtime_check_failures: usize,
}

impl InventorySummary {
    pub fn record_layer(&mut self, layer: &LayerResult) {
        self.layers_checked = self.layers_checked.saturating_add(1);
        self.missing_paths = self.missing_paths.saturating_add(layer.missing_paths.len());
        self.missing_guard_checks = self
            .missing_guard_checks
            .saturating_add(layer.missing_guard_checks.len());
        self.runtime_check_failures = self
            .runtime_check_failures
            .saturating_add(layer.runtime_checks.iter().filter(|row| !row.ok).count());
    }
}
