// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/stomach (authoritative)
//
// V6-ORGAN-001 — Stomach Adaptive Import & Digestion Organ (v1)

pub mod analyzer;
pub mod burn;
pub mod proposal;
pub mod provenance;
pub mod quarantine;
pub mod runner;
pub mod state;

use proposal::{validate_proposal_bundle, ProposalBundle, TransformRequest};
use quarantine::{
    create_quarantine_snapshot, record_fetch_phase, FetchMetadata, IngestPolicy, SnapshotMetadata,
};
use runner::{execute_proposal_in_trusted_runner, ExecutionReceipt, RunnerPolicy};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use state::{transition, DigestState, DigestStatus};
use std::path::Path;

pub const V6_ORGAN_001_CONTRACT_ID: &str = "V6-ORGAN-001";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StomachConfig {
    pub policy_version: String,
    pub analyzer_version: String,
    pub transformer_version: String,
    pub ingest_policy: IngestPolicy,
    pub runner_policy: RunnerPolicy,
    pub autonomy_ladder: AutonomyLadder,
}

impl Default for StomachConfig {
    fn default() -> Self {
        Self {
            policy_version: "stomach_policy_v1".to_string(),
            analyzer_version: "stomach_analyzer_v1".to_string(),
            transformer_version: "stomach_transformer_v1".to_string(),
            ingest_policy: IngestPolicy::default(),
            runner_policy: RunnerPolicy::default(),
            autonomy_ladder: AutonomyLadder::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutonomyLadder {
    pub enabled: bool,
    pub shadow_min_reviews: u32,
    pub staged_acceptance_floor: f64,
    pub bounded_auto_merge_acceptance_floor: f64,
    pub bounded_auto_merge_min_reviews: u32,
    pub broader_autonomy_min_reviews: u32,
    pub broader_autonomy_rollback_success_floor: f64,
    pub broader_autonomy_benchmark_coverage_floor: f64,
    pub require_fixed_policy_analyzer_versions: bool,
}

impl Default for AutonomyLadder {
    fn default() -> Self {
        Self {
            enabled: false,
            shadow_min_reviews: 20,
            staged_acceptance_floor: 0.70,
            bounded_auto_merge_acceptance_floor: 0.85,
            bounded_auto_merge_min_reviews: 50,
            broader_autonomy_min_reviews: 80,
            broader_autonomy_rollback_success_floor: 0.99,
            broader_autonomy_benchmark_coverage_floor: 0.95,
            require_fixed_policy_analyzer_versions: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewWindowMetrics {
    pub reviewed_proposals: u32,
    pub accepted_proposals: u32,
    pub regressions: u32,
    pub policy_escapes: u32,
    pub rollback_success_rate: f64,
    pub benchmark_coverage_rate: f64,
    pub repos_observed: u32,
    pub policy_version: String,
    pub analyzer_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyStage {
    Shadow,
    Staged,
    BoundedAutoMerge,
    BroaderAutonomy,
}

fn acceptance_rate(metrics: &ReviewWindowMetrics) -> f64 {
    metrics.accepted_proposals as f64 / metrics.reviewed_proposals.max(1) as f64
}

fn is_stable_review_window(metrics: &ReviewWindowMetrics) -> bool {
    metrics.regressions == 0 && metrics.policy_escapes == 0
}

pub fn evaluate_autonomy_stage(
    config: &AutonomyLadder,
    metrics: &ReviewWindowMetrics,
    expected_policy_version: &str,
    expected_analyzer_version: &str,
) -> AutonomyStage {
    if !config.enabled {
        return AutonomyStage::Shadow;
    }
    let versions_match = metrics.policy_version == expected_policy_version
        && metrics.analyzer_version == expected_analyzer_version;
    if config.require_fixed_policy_analyzer_versions && !versions_match {
        return AutonomyStage::Shadow;
    }
    let acceptance = acceptance_rate(metrics);
    let stable_window = is_stable_review_window(metrics);
    if stable_window
        && metrics.reviewed_proposals >= config.broader_autonomy_min_reviews
        && metrics.rollback_success_rate >= config.broader_autonomy_rollback_success_floor
        && metrics.benchmark_coverage_rate >= config.broader_autonomy_benchmark_coverage_floor
        && metrics.repos_observed > 1
    {
        return AutonomyStage::BroaderAutonomy;
    }
    if stable_window
        && metrics.reviewed_proposals >= config.bounded_auto_merge_min_reviews
        && acceptance >= config.bounded_auto_merge_acceptance_floor
    {
        return AutonomyStage::BoundedAutoMerge;
    }
    if stable_window
        && metrics.reviewed_proposals >= config.shadow_min_reviews
        && acceptance >= config.staged_acceptance_floor
    {
        return AutonomyStage::Staged;
    }
    AutonomyStage::Shadow
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StomachDaemonOutput {
    pub fetch: FetchMetadata,
    pub snapshot: SnapshotMetadata,
    pub provenance: provenance::ProvenanceRecord,
    pub analysis: analyzer::AnalysisReport,
    pub proposal: ProposalBundle,
    pub execution: ExecutionReceipt,
    pub state: DigestState,
}

pub fn stable_hash(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

pub fn run_stomach_cycle(
    state_root: &Path,
    digest_id: &str,
    source_root: &Path,
    origin_url: &str,
    commit_hash: &str,
    fetched_refs: &[String],
    spdx: Option<&str>,
    transform: &TransformRequest,
    config: &StomachConfig,
) -> Result<StomachDaemonOutput, String> {
    let fetch = record_fetch_phase(
        digest_id,
        source_root,
        origin_url,
        commit_hash,
        fetched_refs,
        &config.ingest_policy,
        &config.policy_version,
        &format!("receipt:{digest_id}:fetch"),
    )?;
    let snapshot = create_quarantine_snapshot(
        state_root,
        digest_id,
        source_root,
        origin_url,
        &config.ingest_policy,
    )?;
    let provenance = provenance::build_provenance(
        &snapshot,
        &fetch,
        spdx,
        &config.analyzer_version,
        &format!("receipt:{digest_id}:ingest"),
    );
    provenance::gate_provenance(&provenance)?;

    let analysis =
        analyzer::deterministic_analyze(&snapshot, Path::new(&snapshot.quarantine_root), None)?;
    let proposal = proposal::generate_proposal(
        &snapshot,
        Path::new(&snapshot.quarantine_root),
        transform,
        &analysis
            .cravings
            .iter()
            .map(|row| row.signal.clone())
            .collect::<Vec<_>>(),
        &[format!("receipt:{digest_id}:analysis")],
        &config.transformer_version,
    )?;
    validate_proposal_bundle(&proposal)?;

    let execution = execute_proposal_in_trusted_runner(&proposal, &config.runner_policy);
    let mut digest_state = DigestState::new(digest_id);
    transition(
        &mut digest_state,
        DigestStatus::Analyzed,
        format!("receipt:{digest_id}:analysis"),
        "analysis_complete",
    )?;
    transition(
        &mut digest_state,
        DigestStatus::Proposed,
        format!("receipt:{digest_id}:proposal"),
        "proposal_generated",
    )?;
    if execution.executed {
        transition(
            &mut digest_state,
            DigestStatus::Verified,
            format!("receipt:{digest_id}:verified"),
            "trusted_runner_execution_passed",
        )?;
    } else {
        transition(
            &mut digest_state,
            DigestStatus::Rejected,
            format!("receipt:{digest_id}:rejected"),
            execution
                .blocked_reason
                .as_deref()
                .unwrap_or("trusted_runner_blocked"),
        )?;
    }

    Ok(StomachDaemonOutput {
        fetch,
        snapshot,
        provenance,
        analysis,
        proposal,
        execution,
        state: digest_state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn autonomy_stage_defaults_to_shadow_when_disabled() {
        let cfg = AutonomyLadder::default();
        let metrics = ReviewWindowMetrics {
            reviewed_proposals: 200,
            accepted_proposals: 200,
            regressions: 0,
            policy_escapes: 0,
            rollback_success_rate: 1.0,
            benchmark_coverage_rate: 1.0,
            repos_observed: 3,
            policy_version: "stomach_policy_v1".to_string(),
            analyzer_version: "stomach_analyzer_v1".to_string(),
        };
        assert_eq!(
            evaluate_autonomy_stage(&cfg, &metrics, "stomach_policy_v1", "stomach_analyzer_v1"),
            AutonomyStage::Shadow
        );
    }

    #[test]
    fn stomach_cycle_produces_verified_state_in_trusted_runner() {
        let root = tempdir().expect("tmp");
        let source = root.path().join("source");
        fs::create_dir_all(&source).expect("mkdir");
        fs::write(
            source.join("Cargo.toml"),
            "[package]\nname=\"demo\"\nversion=\"0.1.0\"\n",
        )
        .expect("write");
        fs::write(source.join("LICENSE"), "MIT").expect("license");

        let transform = TransformRequest::header_injection(
            vec!["Cargo.toml".to_string()],
            "// staged by stomach".to_string(),
        );
        let mut cfg = StomachConfig::default();
        cfg.runner_policy.trusted_isolated_runner = true;

        let out = run_stomach_cycle(
            root.path(),
            "digest-demo",
            &source,
            "https://github.com/example/demo",
            "abc123",
            &["refs/heads/main".to_string()],
            Some("MIT"),
            &transform,
            &cfg,
        )
        .expect("run");
        assert_eq!(out.state.status, DigestStatus::Verified);
        assert!(out.execution.executed);
    }
}
