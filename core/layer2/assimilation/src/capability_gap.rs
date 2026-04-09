// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/assimilation (authoritative).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityClass {
    TimingSensitive,
    MmioHeavy,
    FixedPoint,
    GeneralCompute,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FallbackPolicy {
    Block,
    Emulate,
    ApproximateWithReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    pub class: CapabilityClass,
    pub source_surface: String,
    pub target_surface: String,
    pub equivalent_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityGapReport {
    pub capability: CapabilityDescriptor,
    pub gaps: Vec<String>,
    pub uncertainty_delta: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DegradationReceipt {
    pub degraded: bool,
    pub fallback_policy: FallbackPolicy,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityMappingOutput {
    pub descriptor: CapabilityDescriptor,
    pub gap_report: CapabilityGapReport,
    pub degradation: DegradationReceipt,
}

pub fn evaluate_capability_mapping(
    class: CapabilityClass,
    source_surface: &str,
    target_surface: &str,
    equivalent_supported: bool,
    fallback_policy: FallbackPolicy,
) -> CapabilityMappingOutput {
    let descriptor = CapabilityDescriptor {
        class: class.clone(),
        source_surface: source_surface.to_string(),
        target_surface: target_surface.to_string(),
        equivalent_supported,
    };
    let high_risk = requires_explicit_degradation(&class);
    let mut gaps = Vec::new();
    if !equivalent_supported {
        gaps.push("equivalence_not_proven".to_string());
        if high_risk {
            gaps.push("high_risk_capability_class".to_string());
        }
    }
    let reason = if equivalent_supported {
        "full_equivalence".to_string()
    } else if high_risk {
        "explicit_degradation_required".to_string()
    } else {
        "degraded_with_fallback".to_string()
    };
    let degraded = !equivalent_supported;
    let uncertainty_delta = if equivalent_supported {
        vec![0.0, 0.0, 0.0]
    } else if high_risk {
        vec![0.6, 0.7, 0.8]
    } else {
        vec![0.2, 0.3, 0.4]
    };
    CapabilityMappingOutput {
        gap_report: CapabilityGapReport {
            capability: descriptor.clone(),
            gaps,
            uncertainty_delta,
        },
        degradation: DegradationReceipt {
            degraded,
            fallback_policy,
            reason,
        },
        descriptor,
    }
}

pub fn requires_explicit_degradation(class: &CapabilityClass) -> bool {
    matches!(
        class,
        CapabilityClass::TimingSensitive | CapabilityClass::MmioHeavy | CapabilityClass::FixedPoint
    )
}
