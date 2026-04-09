// SPDX-License-Identifier: Apache-2.0
// Plane ownership: Safety, Cognition, Substrate, Assimilation stack.

use super::{HardSelector, HardSelectorMode, OutputContract, ProtocolStepReceipt, TransferClass};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn matches_hard_selector(selector: &HardSelector, source: &str) -> bool {
    let matched = source.contains(&selector.value) || source.contains(&selector.key);
    match selector.mode {
        HardSelectorMode::Exclude => !matched,
        HardSelectorMode::Constrain | HardSelectorMode::Require => matched,
    }
}

pub fn parse_transfer_class(raw: &str) -> Option<TransferClass> {
    match raw {
        "analysis_only" => Some(TransferClass::AnalysisOnly),
        "clean_room_spec" => Some(TransferClass::CleanRoomSpec),
        "behavioral_clone" => Some(TransferClass::BehavioralClone),
        "emitter_retarget" => Some(TransferClass::EmitterRetarget),
        "direct_lift" => Some(TransferClass::DirectLift),
        _ => None,
    }
}

pub fn parse_output_contract(raw: &str) -> Option<OutputContract> {
    match raw {
        "observation_bundle" => Some(OutputContract::ObservationBundle),
        "capability_spec" => Some(OutputContract::CapabilitySpec),
        "behavior_model" => Some(OutputContract::BehaviorModel),
        "ir_capsule" => Some(OutputContract::IrCapsule),
        "test_harness" => Some(OutputContract::TestHarness),
        "patchset" => Some(OutputContract::Patchset),
        "emitter_package" => Some(OutputContract::EmitterPackage),
        _ => None,
    }
}

pub fn parse_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub fn step_receipt(
    step_kind: &str,
    artifact_hash: &str,
    policy_version: &str,
    parents: &[ProtocolStepReceipt],
    uncertainty_delta: f64,
) -> ProtocolStepReceipt {
    let parent_receipt_ids = parents
        .iter()
        .map(|row| row.receipt_id.clone())
        .collect::<Vec<_>>();
    let lineage_chain = parents
        .iter()
        .flat_map(|row| row.lineage_chain.clone())
        .collect::<Vec<_>>();
    let seed = format!("{}:{}:{}:{}", step_kind, artifact_hash, policy_version, now_iso());
    ProtocolStepReceipt {
        receipt_id: format!("rcpt:{}", short_hash(&seed)),
        parent_receipt_ids,
        step_kind: step_kind.to_string(),
        artifact_hash: artifact_hash.to_string(),
        policy_version: policy_version.to_string(),
        lineage_chain: if lineage_chain.is_empty() {
            vec![step_kind.to_string()]
        } else {
            lineage_chain
        },
        uncertainty_delta: uncertainty_delta.clamp(0.0, 1.0),
        emitted_at: now_iso(),
    }
}

pub fn hash_json<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|err| format!("json_encode_failed:{err}"))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn short_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    hash[..16].to_string()
}

pub fn now_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}

