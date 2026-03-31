#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyRankScoreOutput {
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExpectedValueSignalInput {
    #[serde(default)]
    pub explicit_score: Option<f64>,
    #[serde(default)]
    pub expected_value_usd: Option<f64>,
    #[serde(default)]
    pub oracle_priority_score: Option<f64>,
    pub impact_weight: f64,
    #[serde(default)]
    pub selected_currency: Option<String>,
    pub currency_multiplier: f64,
    pub matched_first_sentence_contains_selected: bool,
    pub currency_ranking_enabled: bool,
    pub oracle_applies: bool,
    pub oracle_pass: bool,
    pub rank_blend: f64,
    pub bonus_cap: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExpectedValueSignalOutput {
    pub score: f64,
    pub base_score: f64,
    pub source: String,
    pub value_oracle_priority: Option<f64>,
    pub currency_adjusted_score: Option<f64>,
    pub currency_delta: f64,
    pub oracle_applies: bool,
    pub oracle_pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValueSignalScoreInput {
    pub expected_value: f64,
    pub time_to_value: f64,
    pub actionability: f64,
    pub directive_fit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValueSignalScoreOutput {
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyRankAdjustedInput {
    pub base: f64,
    pub pulse_score: f64,
    pub pulse_weight: f64,
    pub objective_allocation_score: f64,
    pub base_objective_weight: f64,
    pub canary_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyRankAdjustedBonus {
    pub pulse_weight: f64,
    pub pulse_score: f64,
    pub objective_weight: f64,
    pub objective_allocation_score: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyRankAdjustedOutput {
    pub adjusted: f64,
    pub bonus: StrategyRankAdjustedBonus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TritShadowRankScoreInput {
    pub score: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TritShadowRankScoreOutput {
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyCircuitCooldownInput {
    #[serde(default)]
    pub last_error_code: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    pub http_429_cooldown_hours: f64,
    pub http_5xx_cooldown_hours: f64,
    pub dns_error_cooldown_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyCircuitCooldownOutput {
    pub cooldown_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyTritShadowAdjustedInput {
    pub base_score: f64,
    pub bonus_raw: f64,
    pub bonus_blend: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyTritShadowAdjustedOutput {
    pub adjusted_score: f64,
    pub bonus_applied: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NonYieldPenaltyScoreInput {
    pub policy_hold_rate: f64,
    pub no_progress_rate: f64,
    pub stop_rate: f64,
    pub shipped_rate: f64,
    pub policy_hold_weight: f64,
    pub no_progress_weight: f64,
    pub stop_weight: f64,
    pub shipped_relief_weight: f64,
    pub max_penalty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NonYieldPenaltyScoreOutput {
    pub penalty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectiveShadowAdjustmentsInput {
    pub penalty_raw: f64,
    pub bonus_raw: f64,
    pub max_penalty: f64,
    pub max_bonus: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectiveShadowAdjustmentsOutput {
    pub penalty: f64,
    pub bonus: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyTritShadowRankRowInput {
    pub index: u32,
    pub proposal_id: String,
    pub legacy_rank: f64,
    pub trit_rank: f64,
    pub trit_label: String,
    pub trit_confidence: f64,
    #[serde(default)]
    pub trit_top_sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyTritShadowRankingSummaryInput {
    #[serde(default)]
    pub rows: Vec<StrategyTritShadowRankRowInput>,
    #[serde(default)]
    pub selected_proposal_id: Option<String>,
    #[serde(default)]
    pub selection_mode: Option<String>,
    pub top_k: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyTritShadowRankingSummaryOutput {
    pub considered: u32,
    #[serde(default)]
    pub selection_mode: Option<String>,
    #[serde(default)]
    pub selected_proposal_id: Option<String>,
    #[serde(default)]
    pub legacy_top_proposal_id: Option<String>,
    #[serde(default)]
    pub trit_top_proposal_id: Option<String>,
    pub diverged_from_legacy_top: bool,
    pub diverged_from_selected: bool,
    #[serde(default)]
    pub top: Vec<StrategyTritShadowRankRowInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShadowScopeMatchesInput {
    #[serde(default)]
    pub scope_type: Option<String>,
    #[serde(default)]
    pub scope_value: Option<String>,
    #[serde(default)]
    pub risk_levels: Vec<String>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub capability_key: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShadowScopeMatchesOutput {
    pub matched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectiveShadowAggregateEntryInput {
    #[serde(default)]
    pub kind: Option<String>,
    pub confidence: f64,
    pub score_impact: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectiveShadowAggregateInput {
    #[serde(default)]
    pub entries: Vec<CollectiveShadowAggregateEntryInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectiveShadowAggregateOutput {
    pub matches: u32,
    pub confidence_avg: f64,
    pub penalty_raw: f64,
    pub bonus_raw: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompositeEligibilityScoreInput {
    pub quality_score: f64,
    pub directive_fit_score: f64,
    pub actionability_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompositeEligibilityScoreOutput {
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeToValueScoreInput {
    #[serde(default)]
    pub time_to_cash_hours: Option<f64>,
    #[serde(default)]
    pub expected_impact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeToValueScoreOutput {
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValueDensityScoreInput {
    pub expected_value: f64,
    pub est_tokens: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValueDensityScoreOutput {
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveTierWeightInput {
    #[serde(default)]
    pub tier: Option<f64>,
    #[serde(default)]
    pub fallback: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveTierWeightOutput {
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeDirectiveTierInput {
    #[serde(default)]
    pub raw_tier: Option<f64>,
    #[serde(default)]
    pub fallback: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeDirectiveTierOutput {
    pub tier: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveTierMinShareInput {
    #[serde(default)]
    pub tier: Option<f64>,
    #[serde(default)]
    pub fallback: Option<f64>,
    #[serde(default)]
    pub t1_min_share: f64,
    #[serde(default)]
    pub t2_min_share: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveTierMinShareOutput {
    pub min_share: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveTierCoverageBonusInput {
    #[serde(default)]
    pub tier: Option<f64>,
    #[serde(default)]
    pub fallback: Option<f64>,
    #[serde(default)]
    pub attempts_today: f64,
    #[serde(default)]
    pub current_for_tier: f64,
    #[serde(default)]
    pub t1_min_share: f64,
    #[serde(default)]
    pub t2_min_share: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveTierCoverageBonusOutput {
    pub bonus: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveTierReservationNeedInput {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub attempts_today: f64,
    #[serde(default)]
    pub tier1_attempts: f64,
    #[serde(default)]
    pub tier2_attempts: f64,
    #[serde(default)]
    pub tier1_min_share: f64,
    #[serde(default)]
    pub tier2_min_share: f64,
    #[serde(default)]
    pub candidate_tiers: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveTierReservationNeedOutput {
    pub reserve: bool,
    #[serde(default)]
    pub tier: Option<u32>,
    #[serde(default)]
    pub min_share: Option<f64>,
    pub attempts_today: f64,
    #[serde(default)]
    pub current_tier_attempts: Option<f64>,
    #[serde(default)]
    pub required_after_next: Option<f64>,
    #[serde(default)]
    pub candidate_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PulseObjectiveCooldownActiveInput {
    #[serde(default)]
    pub no_progress_streak: f64,
    #[serde(default)]
    pub no_progress_limit: f64,
    #[serde(default)]
    pub last_attempt_ts: Option<String>,
    #[serde(default)]
    pub cooldown_hours: f64,
    #[serde(default)]
    pub now_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PulseObjectiveCooldownActiveOutput {
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveTokenHitsInput {
    #[serde(default)]
    pub text_tokens: Vec<String>,
    #[serde(default)]
    pub text_stems: Vec<String>,
    #[serde(default)]
    pub directive_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveTokenHitsOutput {
    #[serde(default)]
    pub hits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToStemInput {
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToStemOutput {
    pub stem: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeDirectiveTextInput {
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeDirectiveTextOutput {
    pub normalized: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenizeDirectiveTextInput {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub stopwords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenizeDirectiveTextOutput {
    #[serde(default)]
    pub tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeSpacesInput {
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeSpacesOutput {
    pub normalized: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseLowerListInput {
    #[serde(default)]
    pub list: Vec<String>,
    #[serde(default)]
    pub csv: Option<String>,
}
