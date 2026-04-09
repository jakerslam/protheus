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

#[derive(Debug, Clone, Serialize, Default)]
pub struct MissingGuardCheck {
    pub id: String,
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
