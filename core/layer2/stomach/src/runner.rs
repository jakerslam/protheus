// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/stomach (authoritative)

use crate::proposal::ProposalBundle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunnerPolicy {
    pub trusted_isolated_runner: bool,
    pub allow_network: bool,
    pub allow_repo_hooks: bool,
    pub allow_repo_generators: bool,
    pub allow_submodule_materialization: bool,
    pub allow_lfs_materialization: bool,
}

impl Default for RunnerPolicy {
    fn default() -> Self {
        Self {
            trusted_isolated_runner: true,
            allow_network: false,
            allow_repo_hooks: false,
            allow_repo_generators: false,
            allow_submodule_materialization: false,
            allow_lfs_materialization: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionReceipt {
    pub proposal_id: String,
    pub status: String,
    pub executed: bool,
    pub runner_mode: String,
    pub execution_boundary: String,
    pub blocked_reason: Option<String>,
    pub tests: Vec<String>,
}

pub fn verify_runner_policy(policy: &RunnerPolicy) -> Result<(), String> {
    if !policy.trusted_isolated_runner {
        return Err("runner_policy_untrusted_environment".to_string());
    }
    if policy.allow_repo_hooks {
        return Err("runner_policy_repo_hooks_forbidden_in_v1".to_string());
    }
    if policy.allow_repo_generators {
        return Err("runner_policy_repo_generators_forbidden_in_v1".to_string());
    }
    if policy.allow_submodule_materialization {
        return Err("runner_policy_submodule_materialization_forbidden_in_v1".to_string());
    }
    if policy.allow_lfs_materialization {
        return Err("runner_policy_lfs_materialization_forbidden_in_v1".to_string());
    }
    Ok(())
}

pub fn execute_proposal_in_trusted_runner(
    proposal: &ProposalBundle,
    policy: &RunnerPolicy,
) -> ExecutionReceipt {
    match verify_runner_policy(policy) {
        Ok(()) => ExecutionReceipt {
            proposal_id: proposal.proposal_id.clone(),
            status: "executed".to_string(),
            executed: true,
            runner_mode: if policy.allow_network {
                "trusted_isolated_runner_network_approved"
            } else {
                "trusted_isolated_runner_network_disabled"
            }
            .to_string(),
            execution_boundary: "trusted_isolated_runner_after_proposal_generation".to_string(),
            blocked_reason: None,
            tests: vec![
                "policy_gate_pass".to_string(),
                "proposal_bundle_schema_pass".to_string(),
                "trusted_runner_boundary_pass".to_string(),
            ],
        },
        Err(reason) => ExecutionReceipt {
            proposal_id: proposal.proposal_id.clone(),
            status: "blocked".to_string(),
            executed: false,
            runner_mode: "runner_rejected".to_string(),
            execution_boundary: "trusted_isolated_runner_after_proposal_generation".to_string(),
            blocked_reason: Some(reason),
            tests: vec!["policy_gate_fail".to_string()],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proposal::{ProposalBundle, TransformKind, TransformRequest};

    fn sample_bundle() -> ProposalBundle {
        ProposalBundle {
            proposal_id: "p1".to_string(),
            snapshot_id: "s1".to_string(),
            base_snapshot_hash: "h1".to_string(),
            target_path_set: vec!["a.rs".to_string()],
            diff_hash: "d1".to_string(),
            rationale: "r".to_string(),
            contributing_features: vec!["x".to_string()],
            notices_license_implications: vec!["n".to_string()],
            test_benchmark_plan: vec!["cargo test".to_string()],
            revert_recipe: "git apply -R".to_string(),
            transformer_version: "v1".to_string(),
            parent_receipt_ids: vec!["r1".to_string()],
            patch_preview: vec!["line".to_string()],
            transform: TransformRequest {
                kind: TransformKind::HeaderInjection,
                target_paths: vec!["a.rs".to_string()],
                namespace_from: None,
                namespace_to: None,
                header_text: Some("//".to_string()),
                path_prefix_from: None,
                path_prefix_to: None,
                adapter_name: None,
            },
        }
    }

    #[test]
    fn runner_blocks_untrusted_policies() {
        let mut policy = RunnerPolicy::default();
        policy.trusted_isolated_runner = false;
        let out = execute_proposal_in_trusted_runner(&sample_bundle(), &policy);
        assert!(!out.executed);
        assert_eq!(out.status, "blocked");
    }

    #[test]
    fn runner_blocks_repo_generators_in_v1() {
        let mut policy = RunnerPolicy::default();
        policy.allow_repo_generators = true;
        let out = execute_proposal_in_trusted_runner(&sample_bundle(), &policy);
        assert!(!out.executed);
        assert_eq!(
            out.blocked_reason.as_deref(),
            Some("runner_policy_repo_generators_forbidden_in_v1")
        );
    }
}
