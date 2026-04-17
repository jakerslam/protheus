// SPDX-License-Identifier: Apache-2.0
use chrono::{NaiveDate, SecondsFormat, TimeZone, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeImpactInput {
    #[serde(default, alias = "impact")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeImpactOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeModeInput {
    #[serde(default, alias = "mode")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeModeOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeTargetInput {
    #[serde(default, alias = "target")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeTargetOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeResultInput {
    #[serde(default, alias = "result")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeResultOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ObjectiveIdValidInput {
    #[serde(default, alias = "objective_id", alias = "objectiveId")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ObjectiveIdValidOutput {
    pub valid: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TritVectorFromInputInput {
    #[serde(default, alias = "tritVector")]
    pub trit_vector: Option<Vec<Value>>,
    #[serde(default, alias = "tritVectorCsv")]
    pub trit_vector_csv: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TritVectorFromInputOutput {
    pub vector: Vec<i32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct JaccardSimilarityInput {
    #[serde(default)]
    pub left_tokens: Vec<String>,
    #[serde(default)]
    pub right_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct JaccardSimilarityOutput {
    pub similarity: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TritSimilarityInput {
    #[serde(default)]
    pub query_vector: Vec<Value>,
    #[serde(default)]
    pub entry_trit: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TritSimilarityOutput {
    pub similarity: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CertaintyThresholdInput {
    #[serde(default)]
    pub thresholds: Option<Value>,
    #[serde(default)]
    pub band: Option<String>,
    #[serde(default)]
    pub impact: Option<String>,
    #[serde(default)]
    pub allow_zero_for_legendary_critical: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CertaintyThresholdOutput {
    pub threshold: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct MaxTargetRankInput {
    #[serde(default)]
    pub maturity_max_target_rank_by_band: Option<Value>,
    #[serde(default)]
    pub impact_max_target_rank: Option<Value>,
    #[serde(default)]
    pub maturity_band: Option<String>,
    #[serde(default)]
    pub impact: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MaxTargetRankOutput {
    pub rank: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CreativePenaltyInput {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub preferred_creative_lane_ids: Vec<String>,
    #[serde(default)]
    pub non_creative_certainty_penalty: Option<f64>,
    #[serde(default)]
    pub selected_lane: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CreativePenaltyOutput {
    pub creative_lane_preferred: bool,
    pub selected_lane: Option<String>,
    pub preferred_lanes: Vec<String>,
    pub penalty: f64,
    pub applied: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtractBulletsInput {
    #[serde(default)]
    pub markdown: Option<String>,
    #[serde(default)]
    pub max_items: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExtractBulletsOutput {
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtractListItemsInput {
    #[serde(default)]
    pub markdown: Option<String>,
    #[serde(default)]
    pub max_items: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExtractListItemsOutput {
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParseSystemInternalPermissionInput {
    #[serde(default, alias = "text")]
    pub markdown: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ParseSystemInternalPermissionOutput {
    pub enabled: bool,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParseSoulTokenDataPassRulesInput {
    #[serde(default, alias = "text")]
    pub markdown: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ParseSoulTokenDataPassRulesOutput {
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EnsureSystemPassedSectionInput {
    #[serde(default, alias = "source_text", alias = "text")]
    pub feed_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EnsureSystemPassedSectionOutput {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SystemPassedPayloadHashInput {
    #[serde(default, alias = "source_id")]
    pub source: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, alias = "body")]
    pub payload: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SystemPassedPayloadHashOutput {
    pub hash: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BuildLensPositionInput {
    #[serde(default, alias = "objective_text")]
    pub objective: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub impact: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BuildLensPositionOutput {
    pub position: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BuildConclaveProposalSummaryInput {
    #[serde(default, alias = "objective_text")]
    pub objective: Option<String>,
    #[serde(default, alias = "objectiveId")]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub impact: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BuildConclaveProposalSummaryOutput {
    pub summary: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConclaveHighRiskFlagsInput {
    #[serde(default)]
    pub payload: Option<Value>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default, alias = "maxDivergence")]
    pub max_divergence: Option<f64>,
    #[serde(default, alias = "minConfidence")]
    pub min_confidence: Option<f64>,
    #[serde(default)]
    pub high_risk_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ConclaveHighRiskFlagsOutput {
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TokenizeTextInput {
    #[serde(default, alias = "text")]
    pub value: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TokenizeTextOutput {
    pub tokens: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeListInput {
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub max_len: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeListOutput {
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeTextListInput {
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub max_len: Option<i64>,
    #[serde(default)]
    pub max_items: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeTextListOutput {
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParseJsonFromStdoutInput {
    #[serde(default)]
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ParseJsonFromStdoutOutput {
    pub parsed: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParseArgsInput {
    #[serde(default)]
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ParseArgsOutput {
    pub args: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LibraryMatchScoreInput {
    #[serde(default)]
    pub query_signature_tokens: Vec<String>,
    #[serde(default)]
    pub query_trit_vector: Vec<Value>,
    #[serde(default, alias = "query_target_id")]
    pub query_target: Option<String>,
    #[serde(default)]
    pub row_signature_tokens: Vec<String>,
    #[serde(default)]
    pub row_outcome_trit: Option<i64>,
    #[serde(default, alias = "row_target_id")]
    pub row_target: Option<String>,
    #[serde(default)]
    pub token_weight: Option<f64>,
    #[serde(default)]
    pub trit_weight: Option<f64>,
    #[serde(default)]
    pub target_weight: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LibraryMatchScoreOutput {
    pub score: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct KnownFailurePressureInput {
    #[serde(default)]
    pub candidates: Vec<Value>,
    #[serde(default, alias = "similarity_block")]
    pub failed_repetition_similarity_block: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct KnownFailurePressureOutput {
    pub fail_count: i64,
    pub hard_block: bool,
    pub max_similarity: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HasSignalTermMatchInput {
    #[serde(default)]
    pub haystack: Option<String>,
    #[serde(default)]
    pub token_set: Vec<String>,
    #[serde(default)]
    pub term: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HasSignalTermMatchOutput {
    pub matched: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CountAxiomSignalGroupsInput {
    #[serde(default)]
    pub action_terms: Vec<String>,
    #[serde(default)]
    pub subject_terms: Vec<String>,
    #[serde(default)]
    pub object_terms: Vec<String>,
    #[serde(default)]
    pub min_signal_groups: Option<i64>,
    #[serde(default)]
    pub haystack: Option<String>,
    #[serde(default)]
    pub token_set: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CountAxiomSignalGroupsOutput {
    pub configured_groups: i64,
    pub matched_groups: i64,
    pub required_groups: i64,
    pub pass: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EffectiveFirstNHumanVetoUsesInput {
    #[serde(default, alias = "minimum_uses_by_target")]
    pub first_live_uses_require_human_veto: Option<Value>,
    #[serde(default)]
    pub minimum_first_live_uses_require_human_veto: Option<Value>,
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EffectiveFirstNHumanVetoUsesOutput {
    pub uses: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeBandMapInput {
    #[serde(default)]
    pub raw: Option<Value>,
    #[serde(default)]
    pub base: Option<Value>,
    #[serde(default)]
    pub lo: Option<f64>,
    #[serde(default)]
    pub hi: Option<f64>,
}
