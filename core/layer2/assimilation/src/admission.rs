// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/assimilation (authoritative).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlaneRole {
    Safety,
    Cognition,
    Assimilation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MutationClass {
    ReadOnly,
    CanonicalStateMutation,
    ProofStatusUpgrade,
    EquivalenceFinalize,
    CacheResumeUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Proposal {
    pub proposal_id: String,
    pub target_id: String,
    pub requested_by: PlaneRole,
    pub mutation_class: MutationClass,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdmissionCheck {
    pub admitted: bool,
    pub reason: String,
    pub requires_sync_verity: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateTransition {
    pub transition_id: String,
    pub proposal_id: String,
    pub executed_by: PlaneRole,
    pub new_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdmissionProtocol {
    pub policy_version: String,
}

impl Default for AdmissionProtocol {
    fn default() -> Self {
        Self {
            policy_version: "assimilation_admission_v1".to_string(),
        }
    }
}

impl AdmissionProtocol {
    pub fn evaluate(&self, proposal: &Proposal) -> AdmissionCheck {
        let high_risk = matches!(
            proposal.mutation_class,
            MutationClass::CanonicalStateMutation
                | MutationClass::ProofStatusUpgrade
                | MutationClass::EquivalenceFinalize
        );
        if proposal.requested_by == PlaneRole::Cognition
            && proposal.mutation_class != MutationClass::ReadOnly
        {
            return AdmissionCheck {
                admitted: false,
                reason: "cognition_cannot_mutate_canonical_assimilation_state".to_string(),
                requires_sync_verity: true,
            };
        }
        AdmissionCheck {
            admitted: true,
            reason: if high_risk {
                "admitted_with_sync_verity".to_string()
            } else {
                "admitted_policy_allowed".to_string()
            },
            requires_sync_verity: high_risk,
        }
    }

    pub fn execute_transition(
        &self,
        proposal: &Proposal,
        check: &AdmissionCheck,
        executed_by: PlaneRole,
        new_state: &str,
    ) -> Result<StateTransition, String> {
        if !check.admitted {
            return Err(format!("admission_denied:{}", check.reason));
        }
        if executed_by == PlaneRole::Cognition && proposal.mutation_class != MutationClass::ReadOnly
        {
            return Err("cognition_execution_forbidden".to_string());
        }
        if new_state.trim().is_empty() {
            return Err("transition_missing_state".to_string());
        }
        Ok(StateTransition {
            transition_id: format!("tx:{}", proposal.proposal_id),
            proposal_id: proposal.proposal_id.clone(),
            executed_by,
            new_state: new_state.to_string(),
        })
    }
}
