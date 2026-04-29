// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const SYSTEM_UNDERSTANDING_DOSSIER_SCHEMA_VERSION: u32 = 1;
const SYSTEM_UNDERSTANDING_POLICY_REF: &str =
    "docs/workspace/system_understanding_dossier_policy.md";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemUnderstandingDossierTargetMode {
    ExternalAssimilation,
    InternalRsi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemUnderstandingDossierStatus {
    Draft,
    Usable,
    Stale,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemUnderstandingCapabilityKind {
    RawRuntime,
    Workflow,
    Policy,
    Ux,
    Architecture,
    Tooling,
    Evidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemUnderstandingCapabilityValue {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemUnderstandingTransferTarget {
    Kernel,
    Orchestration,
    Shell,
    Gateway,
    WorkflowJson,
    Docs,
    Tests,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemUnderstandingCapabilityRow {
    pub id: String,
    pub kind: SystemUnderstandingCapabilityKind,
    pub value: SystemUnderstandingCapabilityValue,
    pub evidence: Vec<String>,
    pub runtime_proof: Vec<String>,
    pub transfer_target: SystemUnderstandingTransferTarget,
    pub fit_rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemUnderstandingImplementationItem {
    pub id: String,
    pub summary: String,
    pub owner_layer: String,
    pub invariant: String,
    pub proof_requirement: String,
    pub rollback_plan: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemUnderstandingDossier {
    pub dossier_id: String,
    pub target_mode: SystemUnderstandingDossierTargetMode,
    pub target_system: String,
    pub target_version_or_revision: String,
    pub dossier_version: u32,
    pub created_at: String,
    pub updated_at: String,
    pub owners: Vec<String>,
    pub status: SystemUnderstandingDossierStatus,
    pub confidence_overall: f64,
    pub blocking_unknowns: Vec<String>,
    pub evidence_index: Vec<String>,
    pub soul_confidence: f64,
    pub soul_evidence: Vec<String>,
    pub soul_unknowns: Vec<String>,
    pub runtime_confidence: f64,
    pub runtime_evidence: Vec<String>,
    pub runtime_unknowns: Vec<String>,
    pub required_next_probes: Vec<String>,
    pub ecology_confidence: f64,
    pub ecology_evidence: Vec<String>,
    pub ecology_unknowns: Vec<String>,
    pub authority_confidence: f64,
    pub authority_evidence: Vec<String>,
    pub authority_unknowns: Vec<String>,
    pub authority_risks: Vec<String>,
    pub architecture_confidence: f64,
    pub architecture_evidence: Vec<String>,
    pub architecture_unknowns: Vec<String>,
    pub runtime_architecture_mismatches: Vec<String>,
    pub capability_confidence: f64,
    pub capabilities: Vec<SystemUnderstandingCapabilityRow>,
    pub rejected_capabilities: Vec<SystemUnderstandingCapabilityRow>,
    pub capability_unknowns: Vec<String>,
    pub failure_model_confidence: f64,
    pub known_failure_modes: Vec<String>,
    pub violated_invariants: Vec<String>,
    pub stop_patching_triggers: Vec<String>,
    pub transfer_confidence: f64,
    pub implementation_items: Vec<SystemUnderstandingImplementationItem>,
    pub proof_requirements: Vec<String>,
    pub rollback_plan: Vec<String>,
    pub implementation_confidence: f64,
    pub files_inspected: Vec<String>,
    pub implementation_unknowns: Vec<String>,
    pub syntax_confidence: f64,
    pub syntax_evidence: Vec<String>,
    pub syntax_unknowns: Vec<String>,
}

fn confidence_in_range(value: f64) -> bool {
    (0.0..=1.0).contains(&value)
}

fn require_nonempty(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("missing_{name}"));
    }
    Ok(())
}

fn require_confidence(name: &str, value: f64) -> Result<(), String> {
    if !confidence_in_range(value) {
        return Err(format!("invalid_{name}"));
    }
    Ok(())
}

fn validate_capability_row(
    row: &SystemUnderstandingCapabilityRow,
    field_name: &str,
) -> Result<(), String> {
    require_nonempty(&format!("{field_name}_id"), &row.id)?;
    require_nonempty(&format!("{field_name}_fit_rationale"), &row.fit_rationale)?;
    if row.evidence.is_empty() || row.evidence.iter().any(|item| item.trim().is_empty()) {
        return Err(format!("missing_{field_name}_evidence"));
    }
    if row.runtime_proof.iter().any(|item| item.trim().is_empty()) {
        return Err(format!("invalid_{field_name}_runtime_proof"));
    }
    Ok(())
}

pub fn validate_system_understanding_dossier(
    dossier: &SystemUnderstandingDossier,
) -> Result<(), String> {
    require_nonempty("dossier_id", &dossier.dossier_id)?;
    require_nonempty("target_system", &dossier.target_system)?;
    require_nonempty(
        "target_version_or_revision",
        &dossier.target_version_or_revision,
    )?;
    require_nonempty("created_at", &dossier.created_at)?;
    require_nonempty("updated_at", &dossier.updated_at)?;
    if dossier.dossier_version != SYSTEM_UNDERSTANDING_DOSSIER_SCHEMA_VERSION {
        return Err("invalid_dossier_version".to_string());
    }
    if dossier.owners.is_empty() || dossier.owners.iter().any(|item| item.trim().is_empty()) {
        return Err("missing_owners".to_string());
    }
    if dossier.evidence_index.iter().any(|item| item.trim().is_empty()) {
        return Err("invalid_evidence_index".to_string());
    }
    for (name, value) in [
        ("confidence_overall", dossier.confidence_overall),
        ("soul_confidence", dossier.soul_confidence),
        ("runtime_confidence", dossier.runtime_confidence),
        ("ecology_confidence", dossier.ecology_confidence),
        ("authority_confidence", dossier.authority_confidence),
        ("architecture_confidence", dossier.architecture_confidence),
        ("capability_confidence", dossier.capability_confidence),
        ("failure_model_confidence", dossier.failure_model_confidence),
        ("transfer_confidence", dossier.transfer_confidence),
        ("implementation_confidence", dossier.implementation_confidence),
        ("syntax_confidence", dossier.syntax_confidence),
    ] {
        require_confidence(name, value)?;
    }
    if dossier.runtime_evidence.iter().any(|item| item.trim().is_empty()) {
        return Err("invalid_runtime_evidence".to_string());
    }
    if dossier.required_next_probes.iter().any(|item| item.trim().is_empty()) {
        return Err("invalid_required_next_probes".to_string());
    }
    if dossier.authority_evidence.iter().any(|item| item.trim().is_empty()) {
        return Err("invalid_authority_evidence".to_string());
    }
    if dossier.runtime_architecture_mismatches.iter().any(|item| item.trim().is_empty()) {
        return Err("invalid_runtime_architecture_mismatches".to_string());
    }
    if dossier.stop_patching_triggers.iter().any(|item| item.trim().is_empty()) {
        return Err("invalid_stop_patching_triggers".to_string());
    }
    if dossier.files_inspected.iter().any(|item| item.trim().is_empty()) {
        return Err("invalid_files_inspected".to_string());
    }
    for row in &dossier.capabilities {
        validate_capability_row(row, "capability")?;
    }
    for row in &dossier.rejected_capabilities {
        validate_capability_row(row, "rejected_capability")?;
    }
    for item in &dossier.implementation_items {
        require_nonempty("implementation_item_id", &item.id)?;
        require_nonempty("implementation_item_summary", &item.summary)?;
        require_nonempty("implementation_item_owner_layer", &item.owner_layer)?;
        require_nonempty("implementation_item_invariant", &item.invariant)?;
        require_nonempty(
            "implementation_item_proof_requirement",
            &item.proof_requirement,
        )?;
        require_nonempty("implementation_item_rollback_plan", &item.rollback_plan)?;
    }
    Ok(())
}

pub fn kernel_system_understanding_dossier_model() -> Value {
    json!({
        "ok": true,
        "type": "kernel_system_understanding_dossier_model",
        "schema_version": SYSTEM_UNDERSTANDING_DOSSIER_SCHEMA_VERSION,
        "policy_ref": SYSTEM_UNDERSTANDING_POLICY_REF,
        "owners": ["kernel-sentinel", "assimilation", "rsi"],
        "shared_consumers": [
            "kernel_sentinel",
            "internal_rsi_planning",
            "external_assimilation"
        ],
        "target_modes": ["external_assimilation", "internal_rsi"],
        "statuses": ["draft", "usable", "stale", "superseded"],
        "capability_kinds": [
            "raw_runtime",
            "workflow",
            "policy",
            "ux",
            "architecture",
            "tooling",
            "evidence"
        ],
        "capability_values": ["low", "medium", "high", "critical"],
        "transfer_targets": [
            "kernel",
            "orchestration",
            "shell",
            "gateway",
            "workflow_json",
            "docs",
            "tests",
            "reject"
        ],
        "required_top_level_fields": [
            "dossier_id",
            "target_mode",
            "target_system",
            "target_version_or_revision",
            "dossier_version",
            "created_at",
            "updated_at",
            "owners",
            "status",
            "confidence_overall",
            "blocking_unknowns",
            "evidence_index"
        ],
        "sections": {
            "soul": {
                "required_fields": ["soul_confidence", "soul_evidence", "soul_unknowns"],
                "minimum_confidence": 0.60
            },
            "runtime": {
                "required_fields": [
                    "runtime_confidence",
                    "runtime_evidence",
                    "runtime_unknowns",
                    "required_next_probes"
                ],
                "minimum_confidence": 0.70
            },
            "ecology": {
                "required_fields": ["ecology_confidence", "ecology_evidence", "ecology_unknowns"]
            },
            "authority": {
                "required_fields": [
                    "authority_confidence",
                    "authority_evidence",
                    "authority_unknowns",
                    "authority_risks"
                ],
                "minimum_confidence": 0.80
            },
            "architecture": {
                "required_fields": [
                    "architecture_confidence",
                    "architecture_evidence",
                    "architecture_unknowns",
                    "runtime_architecture_mismatches"
                ],
                "minimum_confidence": 0.70
            },
            "capability": {
                "required_fields": [
                    "capability_confidence",
                    "capabilities",
                    "rejected_capabilities",
                    "capability_unknowns"
                ],
                "minimum_confidence": 0.70
            },
            "failure_model": {
                "required_fields": [
                    "failure_model_confidence",
                    "known_failure_modes",
                    "violated_invariants",
                    "stop_patching_triggers"
                ]
            },
            "transfer_or_improvement_plan": {
                "required_fields": [
                    "transfer_confidence",
                    "implementation_items",
                    "proof_requirements",
                    "rollback_plan"
                ],
                "minimum_confidence": 0.80
            },
            "implementation_structure": {
                "required_fields": [
                    "implementation_confidence",
                    "files_inspected",
                    "implementation_unknowns"
                ]
            },
            "syntax": {
                "required_fields": [
                    "syntax_confidence",
                    "syntax_evidence",
                    "syntax_unknowns"
                ]
            }
        },
        "capability_row_contract": {
            "required_fields": [
                "id",
                "kind",
                "value",
                "evidence",
                "runtime_proof",
                "transfer_target",
                "fit_rationale"
            ]
        },
        "implementation_item_contract": {
            "required_fields": [
                "id",
                "summary",
                "owner_layer",
                "invariant",
                "proof_requirement",
                "rollback_plan"
            ]
        },
        "stop_patching_rule": {
            "behavior": "if thresholds are not met or evidence contradicts the current model, gather evidence or update the dossier before implementation",
            "triggers_layered_reframing": true
        }
    })
}

#[cfg(test)]
#[path = "system_understanding_dossier_tests.rs"]
mod tests;
