// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::{deterministic_receipt_hash, now_iso};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const VERITY_PLANE_SCHEMA_ID: &str = "infring_verity_plane_policy";
const VERITY_PLANE_SCHEMA_VERSION: u32 = 1;
const VERITY_PLANE_POLICY_VERSION: u32 = 1;
const VERITY_MODE_PRODUCTION: &str = "production";
const VERITY_MODE_SIMULATION: &str = "simulation";
const VERITY_FIDELITY_WARNING_DEFAULT: f64 = 0.95;
const VERITY_FIDELITY_LOCK_DEFAULT: f64 = 0.85;
const VERITY_VECTOR_WARNING_DEFAULT: f64 = 0.90;

const VERITY_DRIFT_CONFIG_SCHEMA_ID: &str = "infring_verity_drift_policy";
const VERITY_DRIFT_CONFIG_SCHEMA_VERSION: u32 = 1;
const VERITY_DRIFT_CONFIG_POLICY_VERSION: u32 = 1;
const VERITY_DRIFT_PRODUCTION_DEFAULT_MS: i64 = 500;
const VERITY_DRIFT_SIMULATION_DEFAULT_MS: i64 = 30_000;

const ULTIMATE_VECTOR_ID: &str = "ULTIMATE_VECTOR";
const ULTIMATE_VECTOR_DESCRIPTION: &str =
    "A computational substrate so deeply integrated with reality that it can edit its own governing rules, with truth as the only invariant.";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerityPlaneSignedConfig {
    schema_id: String,
    schema_version: u32,
    policy_version: u32,
    mode: String,
    fidelity_warning_threshold: f64,
    fidelity_lock_threshold: f64,
    vector_warning_threshold: f64,
    signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerityDriftSignedConfig {
    schema_id: String,
    schema_version: u32,
    policy_version: u32,
    mode: String,
    production_tolerance_ms: i64,
    simulation_tolerance_ms: i64,
    signature: String,
}
