// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{PlanVariant, WorkflowTemplate};

pub(super) fn variant_priority(variant: &PlanVariant) -> usize {
    match variant {
        PlanVariant::Safest => 0,
        PlanVariant::Fastest => 1,
        PlanVariant::DegradedFallback => 2,
        PlanVariant::ClarificationFirst => 3,
    }
}

pub(super) fn template_variant_bias(
    template_hint: Option<&WorkflowTemplate>,
    variant: &PlanVariant,
) -> f32 {
    let Some(template) = template_hint else {
        return 0.0;
    };
    match template {
        WorkflowTemplate::ClarifyThenCoordinate => match variant {
            PlanVariant::ClarificationFirst => 0.08,
            PlanVariant::Safest => 0.04,
            PlanVariant::Fastest => -0.02,
            PlanVariant::DegradedFallback => -0.04,
        },
        WorkflowTemplate::ResearchSynthesizeVerify => match variant {
            PlanVariant::Safest => 0.06,
            PlanVariant::Fastest => 0.02,
            PlanVariant::ClarificationFirst => 0.00,
            PlanVariant::DegradedFallback => -0.04,
        },
        WorkflowTemplate::PlanExecuteReview => match variant {
            PlanVariant::Fastest => 0.06,
            PlanVariant::Safest => 0.02,
            PlanVariant::DegradedFallback => 0.00,
            PlanVariant::ClarificationFirst => -0.04,
        },
        WorkflowTemplate::DiagnoseRetryEscalate => match variant {
            PlanVariant::DegradedFallback => 0.08,
            PlanVariant::ClarificationFirst => 0.02,
            PlanVariant::Safest => 0.00,
            PlanVariant::Fastest => -0.02,
        },
        WorkflowTemplate::CodexToolingSynthesis => match variant {
            PlanVariant::Safest => 0.07,
            PlanVariant::Fastest => 0.04,
            PlanVariant::DegradedFallback => -0.01,
            PlanVariant::ClarificationFirst => -0.03,
        },
        WorkflowTemplate::ForgeCodeAgentComposition => match variant {
            PlanVariant::Safest => 0.08,
            PlanVariant::Fastest => 0.05,
            PlanVariant::ClarificationFirst => 0.03,
            PlanVariant::DegradedFallback => -0.02,
        },
        WorkflowTemplate::ForgeCodeRawCapabilityAssimilation => match variant {
            PlanVariant::Safest => 0.09,
            PlanVariant::Fastest => 0.06,
            PlanVariant::ClarificationFirst => 0.00,
            PlanVariant::DegradedFallback => -0.03,
        },
    }
}
