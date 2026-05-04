// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::{
    kernel_sentinel_diagnostic_authorization_model, kernel_sentinel_diagnostic_executor_model,
    kernel_sentinel_diagnostic_failure_probe_policies, kernel_sentinel_diagnostic_request_model,
    kernel_sentinel_diagnostic_result_model, kernel_sentinel_failure_level_taxonomy,
    kernel_sentinel_incident_event_model, kernel_sentinel_targeted_regression_executor_model,
    kernel_system_understanding_dossier_model, KernelSentinelAuthorityClass,
    KernelSentinelAuthorityRule, KernelSentinelEvidenceSource, KERNEL_SENTINEL_CLI_DOMAIN,
    KERNEL_SENTINEL_CONTRACT_VERSION, KERNEL_SENTINEL_MODULE_ID, KERNEL_SENTINEL_NAME,
};
use serde_json::{json, Value};

pub fn authority_rule(source: KernelSentinelEvidenceSource) -> KernelSentinelAuthorityRule {
    match source {
        KernelSentinelEvidenceSource::KernelReceipt
        | KernelSentinelEvidenceSource::RuntimeObservation
        | KernelSentinelEvidenceSource::ReleaseProofPack
        | KernelSentinelEvidenceSource::GatewayHealth
        | KernelSentinelEvidenceSource::QueueBackpressure => KernelSentinelAuthorityRule {
            source,
            authority_class: KernelSentinelAuthorityClass::DeterministicKernelAuthority,
            may_open_finding: true,
            may_write_verdict: true,
            may_block_release: true,
            may_waive_finding: false,
            reason: "kernel_owned_evidence_can_drive_sentinel_verdicts_but_cannot_self_waive",
        },
        KernelSentinelEvidenceSource::ControlPlaneEval => KernelSentinelAuthorityRule {
            source,
            authority_class: KernelSentinelAuthorityClass::AdvisoryWorkflowQuality,
            may_open_finding: true,
            may_write_verdict: false,
            may_block_release: false,
            may_waive_finding: false,
            reason: "control_plane_eval_is_advisory_input_only_for_kernel_sentinel",
        },
        KernelSentinelEvidenceSource::ShellTelemetry => KernelSentinelAuthorityRule {
            source,
            authority_class: KernelSentinelAuthorityClass::PresentationTelemetryOnly,
            may_open_finding: false,
            may_write_verdict: false,
            may_block_release: false,
            may_waive_finding: false,
            reason: "shell_telemetry_cannot_authorize_kernel_runtime_truth",
        },
    }
}

pub fn kernel_sentinel_contract() -> Value {
    let authority_rules = [
        KernelSentinelEvidenceSource::KernelReceipt,
        KernelSentinelEvidenceSource::RuntimeObservation,
        KernelSentinelEvidenceSource::ReleaseProofPack,
        KernelSentinelEvidenceSource::GatewayHealth,
        KernelSentinelEvidenceSource::QueueBackpressure,
        KernelSentinelEvidenceSource::ControlPlaneEval,
        KernelSentinelEvidenceSource::ShellTelemetry,
    ]
    .into_iter()
    .map(|source| serde_json::to_value(authority_rule(source)).unwrap_or(Value::Null))
    .collect::<Vec<_>>();

    let mut payload = json!({
        "ok": true,
        "type": KERNEL_SENTINEL_MODULE_ID,
        "contract_version": KERNEL_SENTINEL_CONTRACT_VERSION,
        "canonical_name": KERNEL_SENTINEL_NAME,
        "module_id": KERNEL_SENTINEL_MODULE_ID,
        "cli_domain": KERNEL_SENTINEL_CLI_DOMAIN,
        "not_names": ["kernel_eval_agent", "eval_agent", "control_plane_eval"],
        "mission": "kernel_resident_failure_intelligence_and_runtime_law_verification",
        "priority_order": [
            "failures",
            "security_correctness",
            "reliability_hardening",
            "optimization",
            "automation"
        ],
        "control_plane_eval_relationship": {
            "role": "advisory_workflow_quality_input",
            "may_write_sentinel_verdict": false,
            "may_waive_sentinel_finding": false,
            "promotion_requires_deterministic_kernel_rule": true
        },
        "system_understanding_role": {
            "role": "kernel_owned_shared_schema_for_sentinel_rsi_and_external_assimilation",
            "policy_ref": "docs/workspace/system_understanding_dossier_policy.md",
            "implementation_without_dossier_is_disallowed_when_confidence_thresholds_are_unmet": true
        },
        "diagnostic_execution_role": {
            "role": "kernel_owned_bounded_diagnostic_probe_authority",
            "policy_ref": "docs/workspace/kernel_sentinel_diagnostic_execution_policy.md",
            "diagnostic_authorization_model_type": "kernel_sentinel_diagnostic_authorization_model",
            "diagnostic_executor_model_type": "kernel_sentinel_diagnostic_executor_model",
            "diagnostic_failure_signature_policy_map_required": true,
            "diagnostic_request_model_type": "kernel_sentinel_diagnostic_request_model",
            "diagnostic_result_model_type": "kernel_sentinel_diagnostic_result_model",
            "explicit_stop_conditions_required": true,
            "open_ended_test_execution_allowed": false
        },
        "external_assimilation_role": {
            "role": "capability_first_transfer_plan_from_shared_external_dossier",
            "output_type": "kernel_sentinel_external_assimilation_transfer_plan",
            "strategy": "capability_first_not_file_burn_down",
            "comparison_and_gap_analysis_follow_in_dossier_comparison_step": true,
            "comparison_output_type": "kernel_sentinel_external_assimilation_dossier_comparison",
            "comparison_requires": [
                "capability_gap_analysis",
                "soul_fit",
                "authority_fit",
                "proof_burden"
            ]
        },
        "authority_rules": authority_rules,
        "failure_level_taxonomy": kernel_sentinel_failure_level_taxonomy(),
        "incident_event_model": kernel_sentinel_incident_event_model(),
        "diagnostic_authorization_model": kernel_sentinel_diagnostic_authorization_model(),
        "diagnostic_executor_model": kernel_sentinel_diagnostic_executor_model(),
        "diagnostic_targeted_regression_executor_model":
            kernel_sentinel_targeted_regression_executor_model(),
        "diagnostic_failure_signature_probe_policies": kernel_sentinel_diagnostic_failure_probe_policies(),
        "diagnostic_request_model": kernel_sentinel_diagnostic_request_model(),
        "diagnostic_result_model": kernel_sentinel_diagnostic_result_model(),
        "system_understanding_dossier_model": kernel_system_understanding_dossier_model()
    });
    let receipt_hash = crate::deterministic_receipt_hash(&payload);
    payload["receipt_hash"] = Value::String(receipt_hash);
    payload
}
