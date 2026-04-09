// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/assimilation (authoritative).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UncertaintyMetadata {
    pub confidence: f64,
    pub uncertainty_vector: Vec<f64>,
    pub unresolved_assumptions: Vec<String>,
    pub loss_class: String,
}

impl UncertaintyMetadata {
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err("abstraction_confidence_out_of_range".to_string());
        }
        let invalid = self
            .uncertainty_vector
            .iter()
            .any(|value| !(0.0..=1.0).contains(value));
        if invalid {
            return Err("abstraction_uncertainty_value_out_of_range".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AbstractionStep {
    pub step_id: String,
    pub operator: String,
    pub source_ids: Vec<String>,
    pub target_id: String,
    pub uncertainty: UncertaintyMetadata,
    pub back_references: Vec<String>,
    pub reversible: bool,
    pub requires_reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefinementHook {
    pub hook_id: String,
    pub stage: String,
    pub validator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HierarchicalAbstraction {
    pub steps: Vec<AbstractionStep>,
    pub refinement_hooks: Vec<RefinementHook>,
}

impl HierarchicalAbstraction {
    pub fn add_step(&mut self, step: AbstractionStep) -> Result<(), String> {
        step.uncertainty.validate()?;
        if step.source_ids.is_empty() {
            return Err(format!("abstraction_empty_sources:{}", step.step_id));
        }
        if step.requires_reversible && !step.reversible {
            return Err(format!("abstraction_requires_reversible:{}", step.step_id));
        }
        if step.reversible && step.back_references.is_empty() {
            return Err(format!("abstraction_missing_backrefs:{}", step.step_id));
        }
        self.steps.push(step);
        Ok(())
    }

    pub fn add_refinement_hook(&mut self, hook: RefinementHook) {
        self.refinement_hooks.push(hook);
    }
}
