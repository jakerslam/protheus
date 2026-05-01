// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub const KERNEL_SENTINEL_BIG_PICTURE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelSentinelBigPictureMode {
    LocalTicketing,
    StructuralDiagnosis,
    RebuildRealignment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelBigPictureInput {
    #[serde(default)]
    pub symptom_ids: Vec<String>,
    #[serde(default)]
    pub affected_layers: Vec<String>,
    #[serde(default)]
    pub repeated_local_fixes: u32,
    #[serde(default)]
    pub command_runtime_contradiction: bool,
    #[serde(default)]
    pub authority_shape_ghost: bool,
    #[serde(default)]
    pub policy_syntax_removed_but_behavior_remains: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSentinelBigPictureAssessment {
    pub schema_version: u32,
    pub mode: KernelSentinelBigPictureMode,
    pub symptom_count: usize,
    pub affected_layer_count: usize,
    pub stop_local_ticketing: bool,
    pub recommended_action: String,
    pub reasons: Vec<String>,
}

fn clean(value: &str, max: usize) -> String {
    value.trim().chars().take(max).collect()
}

fn unique_nonempty(values: Vec<String>, max_len: usize) -> BTreeSet<String> {
    values
        .into_iter()
        .map(|value| clean(&value, max_len))
        .filter(|value| !value.is_empty())
        .collect()
}

pub fn assess_kernel_sentinel_big_picture_regression(
    input: KernelSentinelBigPictureInput,
) -> KernelSentinelBigPictureAssessment {
    let symptoms = unique_nonempty(input.symptom_ids, 160);
    let layers = unique_nonempty(input.affected_layers, 80);
    let mut reasons = Vec::new();
    if symptoms.len() >= 5 {
        reasons.push("many_symptoms_co_occur".to_string());
    }
    if layers.len() >= 3 {
        reasons.push("symptoms_span_three_or_more_layers".to_string());
    }
    if input.repeated_local_fixes >= 2 {
        reasons.push("local_fixes_repeated_without_closure".to_string());
    }
    if input.command_runtime_contradiction {
        reasons.push("command_output_contradicts_runtime_observation".to_string());
    }
    if input.authority_shape_ghost {
        reasons.push("authority_shape_reemerged_after_syntax_removal".to_string());
    }
    if input.policy_syntax_removed_but_behavior_remains {
        reasons.push("policy_behavior_survived_syntax_removal".to_string());
    }

    let architecture_failure = layers.len() >= 3
        && (input.authority_shape_ghost || input.policy_syntax_removed_but_behavior_remains);
    let rebuild_required = architecture_failure
        && (input.repeated_local_fixes >= 2 || input.command_runtime_contradiction);
    let repeated_cross_layer_failure =
        symptoms.len() >= 5 && layers.len() >= 2 && input.repeated_local_fixes >= 2;
    let structural_required = reasons.len() >= 3 || architecture_failure || repeated_cross_layer_failure;
    let mode = if rebuild_required {
        KernelSentinelBigPictureMode::RebuildRealignment
    } else if structural_required {
        KernelSentinelBigPictureMode::StructuralDiagnosis
    } else {
        KernelSentinelBigPictureMode::LocalTicketing
    };
    let recommended_action = match mode {
        KernelSentinelBigPictureMode::LocalTicketing => "continue_local_ticketing",
        KernelSentinelBigPictureMode::StructuralDiagnosis => "pause_local_tickets_emit_structural_diagnosis",
        KernelSentinelBigPictureMode::RebuildRealignment => "stop_patching_rebuild_or_realign_authority_model",
    };

    KernelSentinelBigPictureAssessment {
        schema_version: KERNEL_SENTINEL_BIG_PICTURE_SCHEMA_VERSION,
        mode,
        symptom_count: symptoms.len(),
        affected_layer_count: layers.len(),
        stop_local_ticketing: mode != KernelSentinelBigPictureMode::LocalTicketing,
        recommended_action: recommended_action.to_string(),
        reasons,
    }
}

pub fn kernel_sentinel_big_picture_regression_model() -> Value {
    json!({
        "schema_version": KERNEL_SENTINEL_BIG_PICTURE_SCHEMA_VERSION,
        "purpose": "Pause local ticketing when many subsystem symptoms indicate a structural architecture or authority-model failure.",
        "modes": [
            "local_ticketing",
            "structural_diagnosis",
            "rebuild_realignment"
        ],
        "rebuild_reasons": [
            "symptoms_span_three_or_more_layers",
            "local_fixes_repeated_without_closure",
            "command_output_contradicts_runtime_observation",
            "authority_shape_reemerged_after_syntax_removal",
            "policy_behavior_survived_syntax_removal"
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> KernelSentinelBigPictureInput {
        KernelSentinelBigPictureInput {
            symptom_ids: vec![
                "tool_boxes_outside_chat".into(),
                "metadata_outside_bubble".into(),
                "offline_taskbar".into(),
                "gateway_restart_loop".into(),
                "shell_state_mirror".into(),
            ],
            affected_layers: vec!["shell".into(), "gateway".into(), "kernel".into()],
            repeated_local_fixes: 2,
            command_runtime_contradiction: true,
            authority_shape_ghost: true,
            policy_syntax_removed_but_behavior_remains: true,
        }
    }

    #[test]
    fn multi_layer_authority_ghost_stops_local_ticketing() {
        let assessment = assess_kernel_sentinel_big_picture_regression(input());
        assert_eq!(assessment.mode, KernelSentinelBigPictureMode::RebuildRealignment);
        assert!(assessment.stop_local_ticketing);
        assert_eq!(
            assessment.recommended_action,
            "stop_patching_rebuild_or_realign_authority_model"
        );
        assert!(assessment
            .reasons
            .contains(&"authority_shape_reemerged_after_syntax_removal".to_string()));
    }

    #[test]
    fn sparse_single_layer_symptoms_remain_local_tickets() {
        let assessment = assess_kernel_sentinel_big_picture_regression(
            KernelSentinelBigPictureInput {
                symptom_ids: vec!["one_broken_widget".into()],
                affected_layers: vec!["shell".into()],
                repeated_local_fixes: 0,
                command_runtime_contradiction: false,
                authority_shape_ghost: false,
                policy_syntax_removed_but_behavior_remains: false,
            },
        );
        assert_eq!(assessment.mode, KernelSentinelBigPictureMode::LocalTicketing);
        assert!(!assessment.stop_local_ticketing);
    }

    #[test]
    fn repeated_cross_layer_symptoms_trigger_structural_diagnosis() {
        let assessment = assess_kernel_sentinel_big_picture_regression(
            KernelSentinelBigPictureInput {
                symptom_ids: vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()],
                affected_layers: vec!["gateway".into(), "observability".into()],
                repeated_local_fixes: 2,
                command_runtime_contradiction: false,
                authority_shape_ghost: false,
                policy_syntax_removed_but_behavior_remains: false,
            },
        );
        assert_eq!(
            assessment.mode,
            KernelSentinelBigPictureMode::StructuralDiagnosis
        );
        assert!(assessment.stop_local_ticketing);
    }
}
