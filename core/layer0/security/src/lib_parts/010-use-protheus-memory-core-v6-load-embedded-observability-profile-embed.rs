// SPDX-License-Identifier: Apache-2.0
use protheus_nexus_core_v1::memory_core_v6::{
    load_embedded_observability_profile, EmbeddedObservabilityProfile,
};
use crate::bridges::protheus_vault_core_v1_bridge::{
    evaluate_vault_policy, evaluate_vault_policy_json, load_embedded_vault_policy,
    load_embedded_vault_policy_json, VaultDecision, VaultOperationRequest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::{CStr, CString};
use std::fmt::{Display, Formatter};
use std::os::raw::c_char;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
pub(crate) fn contains_forbidden_runtime_context_marker(raw: &str) -> bool {
    const FORBIDDEN: [&str; 6] = [
        "You are an expert Python programmer.",
        "[PATCH v2",
        "List Leaves (25",
        "BEGIN_OPENCLAW_INTERNAL_CONTEXT",
        "END_OPENCLAW_INTERNAL_CONTEXT",
        "UNTRUSTED_CHILD_RESULT_DELIMITER",
    ];
    FORBIDDEN.iter().any(|marker| raw.contains(marker))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityOperationRequest {
    pub operation_id: String,
    pub subsystem: String,
    pub action: String,
    pub actor: String,
    pub risk_class: String,
    #[serde(default)]
    pub payload_digest: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub covenant_violation: bool,
    #[serde(default)]
    pub tamper_signal: bool,
    #[serde(default = "default_key_age_hours")]
    pub key_age_hours: u32,
    #[serde(default = "default_operator_quorum")]
    pub operator_quorum: u8,
    #[serde(default)]
    pub audit_receipt_nonce: Option<String>,
    #[serde(default)]
    pub zk_proof: Option<String>,
    #[serde(default)]
    pub ciphertext_digest: Option<String>,
}

fn default_key_age_hours() -> u32 {
    1
}

fn default_operator_quorum() -> u8 {
    2
}

impl Default for SecurityOperationRequest {
    fn default() -> Self {
        Self {
            operation_id: "op_default".to_string(),
            subsystem: "system".to_string(),
            action: "execute".to_string(),
            actor: "operator".to_string(),
            risk_class: "normal".to_string(),
            payload_digest: None,
            tags: Vec::new(),
            covenant_violation: false,
            tamper_signal: false,
            key_age_hours: default_key_age_hours(),
            operator_quorum: default_operator_quorum(),
            audit_receipt_nonce: Some("nonce-default".to_string()),
            zk_proof: Some("zk-proof-default".to_string()),
            ciphertext_digest: Some("sha256:cipher-default".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityDecision {
    pub ok: bool,
    pub fail_closed: bool,
    pub shutdown_required: bool,
    pub human_alert_required: bool,
    pub sovereignty_score_pct: f64,
    pub sovereignty_threshold_pct: u8,
    pub decision_digest: String,
    pub reasons: Vec<String>,
    pub vault_decision: VaultDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityAlert {
    pub ts: String,
    pub operation_id: String,
    pub subsystem: String,
    pub action: String,
    pub actor: String,
    pub severity: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum SecurityError {
    RequestDecodeFailed(String),
    VaultPolicyLoadFailed(String),
    ObservabilityProfileLoadFailed(String),
    EncodeFailed(String),
    IoFailed(String),
    ValidationFailed(String),
}

impl Display for SecurityError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::RequestDecodeFailed(msg) => write!(f, "request_decode_failed:{msg}"),
            SecurityError::VaultPolicyLoadFailed(msg) => {
                write!(f, "vault_policy_load_failed:{msg}")
            }
            SecurityError::ObservabilityProfileLoadFailed(msg) => {
                write!(f, "observability_profile_load_failed:{msg}")
            }
            SecurityError::EncodeFailed(msg) => write!(f, "encode_failed:{msg}"),
            SecurityError::IoFailed(msg) => write!(f, "io_failed:{msg}"),
            SecurityError::ValidationFailed(msg) => write!(f, "validation_failed:{msg}"),
        }
    }
}

impl std::error::Error for SecurityError {}

fn now_iso() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    chrono_like_iso(ts)
}

fn chrono_like_iso(epoch_secs: u64) -> String {
    // Avoid adding a heavy chrono dependency for this crate.
    let dt = std::time::UNIX_EPOCH + std::time::Duration::from_secs(epoch_secs);
    let datetime: chrono_stub::DateTime = dt.into();
    datetime.to_rfc3339()
}

fn normalize_token(raw: &str, max_len: usize) -> String {
    raw.trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/') {
                ch
            } else {
                '_'
            }
        })
        .take(max_len)
        .collect::<String>()
        .trim_matches('_')
        .to_ascii_lowercase()
}

fn has_tag(tags: &[String], target: &str) -> bool {
    let needle = normalize_token(target, 64);
    tags.iter().any(|tag| normalize_token(tag, 64) == needle)
}

fn digest_for_decision(req: &SecurityOperationRequest, reasons: &[String], score: f64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(req.operation_id.as_bytes());
    hasher.update(req.subsystem.as_bytes());
    hasher.update(req.action.as_bytes());
    hasher.update(req.actor.as_bytes());
    hasher.update(format!("{score:.3}").as_bytes());
    for reason in reasons {
        hasher.update(reason.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn to_vault_request(req: &SecurityOperationRequest) -> VaultOperationRequest {
    VaultOperationRequest {
        operation_id: req.operation_id.clone(),
        key_id: format!("{}:{}", req.subsystem, req.action),
        action: req.action.clone(),
        zk_proof: req.zk_proof.clone(),
        ciphertext_digest: req
            .ciphertext_digest
            .clone()
            .or_else(|| req.payload_digest.clone()),
        fhe_noise_budget: if has_tag(&req.tags, "aggressive") {
            12
        } else {
            24
        },
        key_age_hours: req.key_age_hours,
        tamper_signal: req.tamper_signal,
        operator_quorum: req.operator_quorum,
        audit_receipt_nonce: req.audit_receipt_nonce.clone(),
    }
}

fn compute_sovereignty_score(
    profile: &EmbeddedObservabilityProfile,
    req: &SecurityOperationRequest,
    vault_decision: &VaultDecision,
) -> (f64, u8) {
    let weights = &profile.sovereignty_scorer;
    let integrity = if vault_decision.allowed { 100.0 } else { 0.0 };

    let continuity = if req.covenant_violation {
        0.0
    } else if req.risk_class.eq_ignore_ascii_case("critical") {
        65.0
    } else if req.risk_class.eq_ignore_ascii_case("high") {
        80.0
    } else {
        95.0
    };

    let reliability = if req.tamper_signal {
        0.0
    } else if has_tag(&req.tags, "drift") {
        70.0
    } else {
        95.0
    };

    let weighted = (integrity * f64::from(weights.integrity_weight_pct)
        + continuity * f64::from(weights.continuity_weight_pct)
        + reliability * f64::from(weights.reliability_weight_pct))
        / 100.0;

    let mut chaos_penalty = 0.0;
    if req.covenant_violation || req.tamper_signal || has_tag(&req.tags, "drift") {
        chaos_penalty = f64::from(weights.chaos_penalty_pct);
    }

    let score = (weighted - chaos_penalty).clamp(0.0, 100.0);
    (score, weights.fail_closed_threshold_pct)
}
