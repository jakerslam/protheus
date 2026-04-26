// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use super::EvidenceSourceConfig;
use crate::kernel_sentinel::{
    KernelSentinelEvidenceSource, KernelSentinelFindingCategory, KernelSentinelSeverity,
};

fn source(
    source: KernelSentinelEvidenceSource,
    file_name: &'static str,
    collector_family: &'static str,
    default_category: KernelSentinelFindingCategory,
    default_severity: KernelSentinelSeverity,
    missing_required_severity: KernelSentinelSeverity,
) -> EvidenceSourceConfig {
    EvidenceSourceConfig {
        source,
        file_name,
        collector_family,
        default_category,
        default_severity,
        missing_required_severity,
    }
}

pub(super) fn source_configs() -> Vec<EvidenceSourceConfig> {
    vec![
        source(
            KernelSentinelEvidenceSource::KernelReceipt,
            "kernel_receipts.jsonl",
            "kernel_receipt",
            KernelSentinelFindingCategory::ReceiptIntegrity,
            KernelSentinelSeverity::Critical,
            KernelSentinelSeverity::Critical,
        ),
        source(
            KernelSentinelEvidenceSource::RuntimeObservation,
            "runtime_observations.jsonl",
            "runtime_observation",
            KernelSentinelFindingCategory::RuntimeCorrectness,
            KernelSentinelSeverity::High,
            KernelSentinelSeverity::Critical,
        ),
        source(
            KernelSentinelEvidenceSource::RuntimeObservation,
            "state_mutations.jsonl",
            "state_mutation",
            KernelSentinelFindingCategory::StateTransition,
            KernelSentinelSeverity::High,
            KernelSentinelSeverity::High,
        ),
        source(
            KernelSentinelEvidenceSource::RuntimeObservation,
            "scheduler_admission.jsonl",
            "scheduler_admission",
            KernelSentinelFindingCategory::CapabilityEnforcement,
            KernelSentinelSeverity::High,
            KernelSentinelSeverity::High,
        ),
        source(
            KernelSentinelEvidenceSource::RuntimeObservation,
            "live_recovery.jsonl",
            "live_recovery",
            KernelSentinelFindingCategory::RuntimeCorrectness,
            KernelSentinelSeverity::High,
            KernelSentinelSeverity::High,
        ),
        source(
            KernelSentinelEvidenceSource::RuntimeObservation,
            "boundedness_observations.jsonl",
            "boundedness_observation",
            KernelSentinelFindingCategory::Boundedness,
            KernelSentinelSeverity::High,
            KernelSentinelSeverity::High,
        ),
        source(
            KernelSentinelEvidenceSource::ReleaseProofPack,
            "release_proof_packs.jsonl",
            "release_proof_pack",
            KernelSentinelFindingCategory::ReleaseEvidence,
            KernelSentinelSeverity::Critical,
            KernelSentinelSeverity::Critical,
        ),
        source(
            KernelSentinelEvidenceSource::ReleaseProofPack,
            "release_repairs.jsonl",
            "release_repair",
            KernelSentinelFindingCategory::ReleaseEvidence,
            KernelSentinelSeverity::High,
            KernelSentinelSeverity::High,
        ),
        source(
            KernelSentinelEvidenceSource::GatewayHealth,
            "gateway_health.jsonl",
            "gateway_health",
            KernelSentinelFindingCategory::GatewayIsolation,
            KernelSentinelSeverity::High,
            KernelSentinelSeverity::High,
        ),
        source(
            KernelSentinelEvidenceSource::GatewayHealth,
            "gateway_quarantine.jsonl",
            "gateway_quarantine",
            KernelSentinelFindingCategory::GatewayIsolation,
            KernelSentinelSeverity::High,
            KernelSentinelSeverity::High,
        ),
        source(
            KernelSentinelEvidenceSource::GatewayHealth,
            "gateway_recovery.jsonl",
            "gateway_recovery",
            KernelSentinelFindingCategory::GatewayIsolation,
            KernelSentinelSeverity::High,
            KernelSentinelSeverity::High,
        ),
        source(
            KernelSentinelEvidenceSource::GatewayHealth,
            "gateway_isolation.jsonl",
            "gateway_isolation",
            KernelSentinelFindingCategory::GatewayIsolation,
            KernelSentinelSeverity::High,
            KernelSentinelSeverity::High,
        ),
        source(
            KernelSentinelEvidenceSource::QueueBackpressure,
            "queue_backpressure.jsonl",
            "queue_backpressure",
            KernelSentinelFindingCategory::QueueBackpressure,
            KernelSentinelSeverity::High,
            KernelSentinelSeverity::High,
        ),
        source(
            KernelSentinelEvidenceSource::ControlPlaneEval,
            "control_plane_eval.jsonl",
            "control_plane_eval_advisory",
            KernelSentinelFindingCategory::RuntimeCorrectness,
            KernelSentinelSeverity::Medium,
            KernelSentinelSeverity::Low,
        ),
    ]
}
