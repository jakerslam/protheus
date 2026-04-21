// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// V6-ORGAN-001 — Stomach v1 kernel wrapper

use protheus_nexus_core_v1::{
    DefaultNexusPolicy, DeliveryAuthorizationInput, LeaseIssueRequest, MainNexusControlPlane,
    NexusFeatureFlags, TrustClass, VerityClass,
};
use protheus_nexus_core_v1::stomach_core::burn::{
    purge_artifact_path, transition_retention, RetentionEvent,
};
use protheus_nexus_core_v1::stomach_core::proposal::{TransformKind, TransformRequest};
use protheus_nexus_core_v1::stomach_core::state::{rollback_by_receipt, DigestState, DigestStatus};
use protheus_nexus_core_v1::stomach_core::{run_stomach_cycle, stable_hash, StomachConfig};
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{clean, deterministic_receipt_hash, now_iso};
