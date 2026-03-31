#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseSuccessCriteriaRowsOutput {
    #[serde(default)]
    pub rows: Vec<ParseSuccessCriteriaRowOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CollectOutcomeStatsBucketInput {
    #[serde(default)]
    pub shipped: f64,
    #[serde(default)]
    pub no_change: f64,
    #[serde(default)]
    pub reverted: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectOutcomeStatsInput {
    #[serde(default)]
    pub by_eye: std::collections::BTreeMap<String, CollectOutcomeStatsBucketInput>,
    #[serde(default)]
    pub by_topic: std::collections::BTreeMap<String, CollectOutcomeStatsBucketInput>,
    #[serde(default)]
    pub global: CollectOutcomeStatsBucketInput,
    #[serde(default)]
    pub eye_min_samples: f64,
    #[serde(default)]
    pub topic_min_samples: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectOutcomeStatsGlobalOutput {
    pub shipped: f64,
    pub no_change: f64,
    pub reverted: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectOutcomeStatsBiasOutput {
    pub shipped: f64,
    pub no_change: f64,
    pub reverted: f64,
    pub total: f64,
    pub bias: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectOutcomeStatsOutput {
    pub global: CollectOutcomeStatsGlobalOutput,
    #[serde(default)]
    pub eye_biases: std::collections::BTreeMap<String, CollectOutcomeStatsBiasOutput>,
    #[serde(default)]
    pub topic_biases: std::collections::BTreeMap<String, CollectOutcomeStatsBiasOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubdirectiveV2SignalsInput {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub has_concrete_target: bool,
    #[serde(default)]
    pub has_expected_delta: bool,
    #[serde(default)]
    pub has_verification_step: bool,
    #[serde(default)]
    pub target_count: f64,
    #[serde(default)]
    pub verify_count: f64,
    #[serde(default)]
    pub success_criteria_count: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubdirectiveV2SignalsOutput {
    pub required: bool,
    pub has_concrete_target: bool,
    pub has_expected_delta: bool,
    pub has_verification_step: bool,
    pub target_count: f64,
    pub verify_count: f64,
    pub success_criteria_count: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutoscaleRequest {
    pub mode: String,
    #[serde(default)]
    pub plan_input: Option<PlanInput>,
    #[serde(default)]
    pub batch_input: Option<BatchMaxInput>,
    #[serde(default)]
    pub dynamic_caps_input: Option<DynamicCapsInput>,
    #[serde(default)]
    pub token_usage_input: Option<TokenUsageInput>,
    #[serde(default)]
    pub normalize_queue_input: Option<NormalizeQueueInput>,
    #[serde(default)]
    pub criteria_gate_input: Option<CriteriaGateInput>,
    #[serde(default)]
    pub structural_preview_criteria_failure_input: Option<StructuralPreviewCriteriaFailureInput>,
    #[serde(default)]
    pub policy_hold_input: Option<PolicyHoldInput>,
    #[serde(default)]
    pub policy_hold_result_input: Option<PolicyHoldResultInput>,
    #[serde(default)]
    pub policy_hold_run_event_input: Option<PolicyHoldRunEventInput>,
    #[serde(default)]
    pub dod_evidence_diff_input: Option<DodEvidenceDiffInput>,
    #[serde(default)]
    pub score_only_result_input: Option<ScoreOnlyResultInput>,
    #[serde(default)]
    pub score_only_failure_like_input: Option<ScoreOnlyFailureLikeInput>,
    #[serde(default)]
    pub gate_exhausted_attempt_input: Option<GateExhaustedAttemptInput>,
    #[serde(default)]
    pub consecutive_gate_exhausted_attempts_input: Option<ConsecutiveGateExhaustedAttemptsInput>,
    #[serde(default)]
    pub runs_since_reset_index_input: Option<RunsSinceResetIndexInput>,
    #[serde(default)]
    pub attempt_event_indices_input: Option<AttemptEventIndicesInput>,
    #[serde(default)]
    pub capacity_counted_attempt_indices_input: Option<CapacityCountedAttemptIndicesInput>,
    #[serde(default)]
    pub consecutive_no_progress_runs_input: Option<ConsecutiveNoProgressRunsInput>,
    #[serde(default)]
    pub shipped_count_input: Option<ShippedCountInput>,
    #[serde(default)]
    pub executed_count_by_risk_input: Option<ExecutedCountByRiskInput>,
    #[serde(default)]
    pub run_result_tally_input: Option<RunResultTallyInput>,
    #[serde(default)]
    pub qos_lane_usage_input: Option<QosLaneUsageInput>,
    #[serde(default)]
    pub qos_lane_share_cap_exceeded_input: Option<QosLaneShareCapExceededInput>,
    #[serde(default)]
    pub qos_lane_from_candidate_input: Option<QosLaneFromCandidateInput>,
    #[serde(default)]
    pub eye_outcome_count_window_input: Option<EyeOutcomeWindowCountInput>,
    #[serde(default)]
    pub eye_outcome_count_last_hours_input: Option<EyeOutcomeLastHoursCountInput>,
    #[serde(default)]
    pub sorted_counts_input: Option<SortedCountsInput>,
    #[serde(default)]
    pub normalize_proposal_status_input: Option<NormalizeProposalStatusInput>,
    #[serde(default)]
    pub proposal_status_for_queue_pressure_input: Option<ProposalStatusForQueuePressureInput>,
    #[serde(default)]
    pub proposal_status_input: Option<ProposalStatusInput>,
    #[serde(default)]
    pub minutes_since_ts_input: Option<MinutesSinceTsInput>,
    #[serde(default)]
    pub date_window_input: Option<DateWindowInput>,
    #[serde(default)]
    pub in_window_input: Option<InWindowInput>,
    #[serde(default)]
    pub exec_window_match_input: Option<ExecWindowMatchInput>,
    #[serde(default)]
    pub start_of_next_utc_day_input: Option<StartOfNextUtcDayInput>,
    #[serde(default)]
    pub iso_after_minutes_input: Option<IsoAfterMinutesInput>,
    #[serde(default)]
    pub execute_confidence_history_match_input: Option<ExecuteConfidenceHistoryMatchInput>,
    #[serde(default)]
    pub execute_confidence_cooldown_key_input: Option<ExecuteConfidenceCooldownKeyInput>,
    #[serde(default)]
    pub qos_lane_weights_input: Option<QosLaneWeightsInput>,
    #[serde(default)]
    pub proposal_outcome_status_input: Option<ProposalOutcomeStatusInput>,
    #[serde(default)]
    pub queue_underflow_backfill_input: Option<QueueUnderflowBackfillInput>,
    #[serde(default)]
    pub proposal_risk_score_input: Option<ProposalRiskScoreInput>,
    #[serde(default)]
    pub proposal_score_input: Option<ProposalScoreInput>,
    #[serde(default)]
    pub proposal_admission_preview_input: Option<ProposalAdmissionPreviewInput>,
    #[serde(default)]
    pub impact_weight_input: Option<ImpactWeightInput>,
    #[serde(default)]
    pub risk_penalty_input: Option<RiskPenaltyInput>,
    #[serde(default)]
    pub estimate_tokens_input: Option<EstimateTokensInput>,
    #[serde(default)]
    pub proposal_remediation_depth_input: Option<ProposalRemediationDepthInput>,
    #[serde(default)]
    pub proposal_dedup_key_input: Option<ProposalDedupKeyInput>,
    #[serde(default)]
    pub proposal_semantic_fingerprint_input: Option<ProposalSemanticFingerprintInput>,
    #[serde(default)]
    pub semantic_token_similarity_input: Option<SemanticTokenSimilarityInput>,
    #[serde(default)]
    pub semantic_context_comparable_input: Option<SemanticContextComparableInput>,
    #[serde(default)]
    pub semantic_near_duplicate_match_input: Option<SemanticNearDuplicateMatchInput>,
    #[serde(default)]
    pub strategy_rank_score_input: Option<StrategyRankScoreInput>,
    #[serde(default)]
    pub strategy_rank_adjusted_input: Option<StrategyRankAdjustedInput>,
    #[serde(default)]
    pub trit_shadow_rank_score_input: Option<TritShadowRankScoreInput>,
    #[serde(default)]
    pub strategy_circuit_cooldown_input: Option<StrategyCircuitCooldownInput>,
    #[serde(default)]
    pub strategy_trit_shadow_adjusted_input: Option<StrategyTritShadowAdjustedInput>,
    #[serde(default)]
    pub non_yield_penalty_score_input: Option<NonYieldPenaltyScoreInput>,
    #[serde(default)]
    pub collective_shadow_adjustments_input: Option<CollectiveShadowAdjustmentsInput>,
    #[serde(default)]
    pub strategy_trit_shadow_ranking_summary_input: Option<StrategyTritShadowRankingSummaryInput>,
    #[serde(default)]
    pub shadow_scope_matches_input: Option<ShadowScopeMatchesInput>,
    #[serde(default)]
    pub collective_shadow_aggregate_input: Option<CollectiveShadowAggregateInput>,
    #[serde(default)]
    pub expected_value_signal_input: Option<ExpectedValueSignalInput>,
    #[serde(default)]
    pub value_signal_score_input: Option<ValueSignalScoreInput>,
    #[serde(default)]
    pub composite_eligibility_score_input: Option<CompositeEligibilityScoreInput>,
    #[serde(default)]
    pub time_to_value_score_input: Option<TimeToValueScoreInput>,
    #[serde(default)]
    pub value_density_score_input: Option<ValueDensityScoreInput>,
    #[serde(default)]
    pub normalize_directive_tier_input: Option<NormalizeDirectiveTierInput>,
    #[serde(default)]
    pub directive_tier_weight_input: Option<DirectiveTierWeightInput>,
    #[serde(default)]
    pub directive_tier_min_share_input: Option<DirectiveTierMinShareInput>,
    #[serde(default)]
    pub directive_tier_coverage_bonus_input: Option<DirectiveTierCoverageBonusInput>,
    #[serde(default)]
    pub directive_tier_reservation_need_input: Option<DirectiveTierReservationNeedInput>,
    #[serde(default)]
    pub pulse_objective_cooldown_active_input: Option<PulseObjectiveCooldownActiveInput>,
    #[serde(default)]
    pub directive_token_hits_input: Option<DirectiveTokenHitsInput>,
    #[serde(default)]
    pub to_stem_input: Option<ToStemInput>,
    #[serde(default)]
    pub normalize_directive_text_input: Option<NormalizeDirectiveTextInput>,
    #[serde(default)]
    pub tokenize_directive_text_input: Option<TokenizeDirectiveTextInput>,
    #[serde(default)]
    pub normalize_spaces_input: Option<NormalizeSpacesInput>,
    #[serde(default)]
    pub parse_lower_list_input: Option<ParseLowerListInput>,
    #[serde(default)]
    pub canary_failed_checks_allowed_input: Option<CanaryFailedChecksAllowedInput>,
    #[serde(default)]
    pub proposal_text_blob_input: Option<ProposalTextBlobInput>,
    #[serde(default)]
    pub percent_mentions_from_text_input: Option<PercentMentionsFromTextInput>,
    #[serde(default)]
    pub optimization_min_delta_percent_input: Option<OptimizationMinDeltaPercentInput>,
    #[serde(default)]
    pub source_eye_ref_input: Option<SourceEyeRefInput>,
    #[serde(default)]
    pub normalized_risk_input: Option<NormalizedRiskInput>,
    #[serde(default)]
    pub parse_iso_ts_input: Option<ParseIsoTsInput>,
    #[serde(default)]
    pub extract_objective_id_token_input: Option<ExtractObjectiveIdTokenInput>,
    #[serde(default)]
    pub normalize_value_currency_token_input: Option<NormalizeValueCurrencyTokenInput>,
    #[serde(default)]
    pub list_value_currencies_input: Option<ListValueCurrenciesInput>,
    #[serde(default)]
    pub infer_value_currencies_from_directive_bits_input:
        Option<InferValueCurrenciesFromDirectiveBitsInput>,
    #[serde(default)]
    pub has_linked_objective_entry_input: Option<HasLinkedObjectiveEntryInput>,
    #[serde(default)]
    pub verified_entry_outcome_input: Option<VerifiedEntryOutcomeInput>,
    #[serde(default)]
    pub verified_revenue_action_input: Option<VerifiedRevenueActionInput>,
    #[serde(default)]
    pub minutes_until_next_utc_day_input: Option<MinutesUntilNextUtcDayInput>,
    #[serde(default)]
    pub age_hours_input: Option<AgeHoursInput>,
    #[serde(default)]
    pub url_domain_input: Option<UrlDomainInput>,
    #[serde(default)]
    pub domain_allowed_input: Option<DomainAllowedInput>,
    #[serde(default)]
    pub is_execute_mode_input: Option<IsExecuteModeInput>,
    #[serde(default)]
    pub execution_allowed_by_feature_flag_input: Option<ExecutionAllowedByFeatureFlagInput>,
    #[serde(default)]
    pub is_tier1_objective_id_input: Option<IsTier1ObjectiveIdInput>,
    #[serde(default)]
    pub is_tier1_candidate_objective_input: Option<IsTier1CandidateObjectiveInput>,
    #[serde(default)]
    pub needs_execution_quota_input: Option<NeedsExecutionQuotaInput>,
    #[serde(default)]
    pub normalize_criteria_metric_input: Option<NormalizeCriteriaMetricInput>,
    #[serde(default)]
    pub escape_reg_exp_input: Option<EscapeRegExpInput>,
    #[serde(default)]
    pub tool_token_mentioned_input: Option<ToolTokenMentionedInput>,
    #[serde(default)]
    pub policy_hold_reason_from_event_input: Option<PolicyHoldReasonFromEventInput>,
    #[serde(default)]
    pub strategy_marker_tokens_input: Option<StrategyMarkerTokensInput>,
    #[serde(default)]
    pub capability_cooldown_key_input: Option<CapabilityCooldownKeyInput>,
    #[serde(default)]
    pub readiness_retry_cooldown_key_input: Option<ReadinessRetryCooldownKeyInput>,
    #[serde(default)]
    pub source_eye_id_input: Option<SourceEyeIdInput>,
    #[serde(default)]
    pub deprioritized_source_proposal_input: Option<DeprioritizedSourceProposalInput>,
    #[serde(default)]
    pub composite_eligibility_min_input: Option<CompositeEligibilityMinInput>,
    #[serde(default)]
    pub clamp_threshold_input: Option<ClampThresholdInput>,
    #[serde(default)]
    pub applied_thresholds_input: Option<AppliedThresholdsInput>,
    #[serde(default)]
    pub extract_eye_from_evidence_ref_input: Option<ExtractEyeFromEvidenceRefInput>,
    #[serde(default)]
    pub total_outcomes_input: Option<TotalOutcomesInput>,
    #[serde(default)]
    pub derive_entity_bias_input: Option<DeriveEntityBiasInput>,
    #[serde(default)]
    pub strategy_profile_input: Option<StrategyProfileInput>,
    #[serde(default)]
    pub active_strategy_variants_input: Option<ActiveStrategyVariantsInput>,
    #[serde(default)]
    pub strategy_scorecard_summaries_input: Option<StrategyScorecardSummariesInput>,
    #[serde(default)]
    pub outcome_fitness_policy_input: Option<OutcomeFitnessPolicyInput>,
    #[serde(default)]
    pub load_eyes_map_input: Option<LoadEyesMapInput>,
    #[serde(default)]
    pub fallback_directive_objective_ids_input: Option<FallbackDirectiveObjectiveIdsInput>,
    #[serde(default)]
    pub queue_pressure_snapshot_input: Option<QueuePressureSnapshotInput>,
    #[serde(default)]
    pub parse_success_criteria_rows_input: Option<ParseSuccessCriteriaRowsInput>,
    #[serde(default)]
    pub collect_outcome_stats_input: Option<CollectOutcomeStatsInput>,
    #[serde(default)]
    pub subdirective_v2_signals_input: Option<SubdirectiveV2SignalsInput>,
    #[serde(default)]
    pub build_overlay_input: Option<BuildOverlayInput>,
    #[serde(default)]
    pub has_adaptive_mutation_signal_input: Option<HasAdaptiveMutationSignalInput>,
    #[serde(default)]
    pub adaptive_mutation_execution_guard_input: Option<AdaptiveMutationExecutionGuardInput>,
    #[serde(default)]
    pub strategy_selection_input: Option<StrategySelectionInput>,
    #[serde(default)]
    pub calibration_deltas_input: Option<CalibrationDeltasInput>,
    #[serde(default)]
    pub strategy_admission_decision_input: Option<StrategyAdmissionDecisionInput>,
    #[serde(default)]
    pub expected_value_score_input: Option<ExpectedValueScoreInput>,
    #[serde(default)]
    pub suggest_run_batch_max_input: Option<SuggestRunBatchMaxInput>,
    #[serde(default)]
    pub backlog_autoscale_snapshot_input: Option<BacklogAutoscaleSnapshotInput>,
    #[serde(default)]
    pub admission_summary_input: Option<AdmissionSummaryInput>,
    #[serde(default)]
    pub unknown_type_quarantine_decision_input: Option<UnknownTypeQuarantineDecisionInput>,
    #[serde(default)]
    pub infer_optimization_delta_input: Option<InferOptimizationDeltaInput>,
    #[serde(default)]
    pub optimization_intent_proposal_input: Option<OptimizationIntentProposalInput>,
    #[serde(default)]
    pub unlinked_optimization_admission_input: Option<UnlinkedOptimizationAdmissionInput>,
    #[serde(default)]
    pub optimization_good_enough_input: Option<OptimizationGoodEnoughInput>,
    #[serde(default)]
    pub proposal_dependency_summary_input: Option<ProposalDependencySummaryInput>,
    #[serde(default)]
    pub choose_selection_mode_input: Option<ChooseSelectionModeInput>,
    #[serde(default)]
    pub explore_quota_for_day_input: Option<ExploreQuotaForDayInput>,
    #[serde(default)]
    pub medium_risk_thresholds_input: Option<MediumRiskThresholdsInput>,
    #[serde(default)]
    pub medium_risk_gate_decision_input: Option<MediumRiskGateDecisionInput>,
    #[serde(default)]
    pub route_block_prefilter_input: Option<RouteBlockPrefilterInput>,
    #[serde(default)]
    pub route_execution_sample_event_input: Option<RouteExecutionSampleEventInput>,
    #[serde(default)]
    pub route_block_telemetry_summary_input: Option<RouteBlockTelemetrySummaryInput>,
    #[serde(default)]
    pub is_stub_proposal_input: Option<IsStubProposalInput>,
    #[serde(default)]
    pub recent_autonomy_run_events_input: Option<RecentAutonomyRunEventsInput>,
    #[serde(default)]
    pub proposal_meta_index_input: Option<ProposalMetaIndexInput>,
    #[serde(default)]
    pub new_log_events_input: Option<NewLogEventsInput>,
    #[serde(default)]
    pub outcome_buckets_input: Option<OutcomeBucketsInput>,
    #[serde(default)]
    pub recent_run_events_input: Option<RecentRunEventsInput>,
    #[serde(default)]
    pub all_decision_events_input: Option<AllDecisionEventsInput>,
    #[serde(default)]
    pub cooldown_active_state_input: Option<CooldownActiveStateInput>,
    #[serde(default)]
    pub bump_count_input: Option<BumpCountInput>,
    #[serde(default)]
    pub lock_age_minutes_input: Option<LockAgeMinutesInput>,
    #[serde(default)]
    pub hash_obj_input: Option<HashObjInput>,
    #[serde(default)]
    pub assess_success_criteria_quality_input: Option<AssessSuccessCriteriaQualityInput>,
    #[serde(default)]
    pub manual_gate_prefilter_input: Option<ManualGatePrefilterInput>,
    #[serde(default)]
    pub execute_confidence_cooldown_active_input: Option<ExecuteConfidenceCooldownActiveInput>,
    #[serde(default)]
    pub top_biases_summary_input: Option<TopBiasesSummaryInput>,
    #[serde(default)]
    pub criteria_pattern_penalty_input: Option<CriteriaPatternPenaltyInput>,
    #[serde(default)]
    pub strategy_threshold_overrides_input: Option<StrategyThresholdOverridesInput>,
    #[serde(default)]
    pub effective_allowed_risks_input: Option<EffectiveAllowedRisksInput>,
    #[serde(default)]
    pub directive_pulse_context_input: Option<DirectivePulseContextInput>,
    #[serde(default)]
    pub directive_pulse_stats_input: Option<DirectivePulseStatsInput>,
    #[serde(default)]
    pub compile_directive_pulse_objectives_input: Option<CompileDirectivePulseObjectivesInput>,
    #[serde(default)]
    pub directive_pulse_objectives_profile_input: Option<DirectivePulseObjectivesProfileInput>,
    #[serde(default)]
    pub recent_directive_pulse_cooldown_count_input: Option<RecentDirectivePulseCooldownCountInput>,
    #[serde(default)]
    pub proposal_directive_text_input: Option<ProposalDirectiveTextInput>,
    #[serde(default)]
    pub objective_ids_from_pulse_context_input: Option<ObjectiveIdsFromPulseContextInput>,
    #[serde(default)]
    pub policy_hold_objective_context_input: Option<PolicyHoldObjectiveContextInput>,
    #[serde(default)]
    pub proposal_semantic_objective_id_input: Option<ProposalSemanticObjectiveIdInput>,
    #[serde(default)]
    pub criteria_pattern_keys_input: Option<CriteriaPatternKeysInput>,
    #[serde(default)]
    pub success_criteria_requirement_input: Option<SuccessCriteriaRequirementInput>,
    #[serde(default)]
    pub success_criteria_policy_for_proposal_input: Option<SuccessCriteriaPolicyForProposalInput>,
    #[serde(default)]
    pub capability_descriptor_input: Option<CapabilityDescriptorInput>,
    #[serde(default)]
    pub normalize_token_usage_shape_input: Option<NormalizeTokenUsageShapeInput>,
    #[serde(default)]
    pub is_directive_clarification_proposal_input: Option<IsDirectiveClarificationProposalInput>,
    #[serde(default)]
    pub is_directive_decomposition_proposal_input: Option<IsDirectiveDecompositionProposalInput>,
    #[serde(default)]
    pub sanitize_directive_objective_id_input: Option<SanitizeDirectiveObjectiveIdInput>,
    #[serde(default)]
    pub sanitized_directive_id_list_input: Option<SanitizedDirectiveIdListInput>,
    #[serde(default)]
    pub parse_first_json_line_input: Option<ParseFirstJsonLineInput>,
    #[serde(default)]
    pub parse_json_objects_from_text_input: Option<ParseJsonObjectsFromTextInput>,
    #[serde(default)]
    pub read_path_value_input: Option<ReadPathValueInput>,
    #[serde(default)]
    pub number_or_null_input: Option<NumberOrNullInput>,
    #[serde(default)]
    pub choose_evidence_selection_mode_input: Option<ChooseEvidenceSelectionModeInput>,
    #[serde(default)]
    pub truthy_flag_input: Option<TruthyFlagInput>,
    #[serde(default)]
    pub falsey_flag_input: Option<TruthyFlagInput>,
    #[serde(default)]
    pub stable_selection_index_input: Option<StableSelectionIndexInput>,
    #[serde(default)]
    pub as_string_array_input: Option<AsStringArrayInput>,
    #[serde(default)]
    pub uniq_sorted_input: Option<UniqSortedInput>,
    #[serde(default)]
    pub normalize_model_ids_input: Option<NormalizeModelIdsInput>,
    #[serde(default)]
    pub selected_model_from_run_event_input: Option<SelectedModelFromRunEventInput>,
    #[serde(default)]
    pub read_first_numeric_metric_input: Option<ReadFirstNumericMetricInput>,
    #[serde(default)]
    pub parse_arg_input: Option<ParseArgInput>,
    #[serde(default)]
    pub date_arg_or_today_input: Option<DateArgOrTodayInput>,
    #[serde(default)]
    pub has_env_numeric_override_input: Option<HasEnvNumericOverrideInput>,
    #[serde(default)]
    pub coalesce_numeric_input: Option<CoalesceNumericInput>,
    #[serde(default)]
    pub clamp_number_input: Option<ClampNumberInput>,
    #[serde(default)]
    pub list_proposal_files_input: Option<ListProposalFilesInput>,
    #[serde(default)]
    pub latest_proposal_date_input: Option<LatestProposalDateInput>,
    #[serde(default)]
    pub parse_directive_file_arg_input: Option<ParseDirectiveFileArgInput>,
    #[serde(default)]
    pub parse_directive_objective_arg_input: Option<ParseDirectiveObjectiveArgInput>,
    #[serde(default)]
    pub now_iso_input: Option<NowIsoInput>,
    #[serde(default)]
    pub today_str_input: Option<TodayStrInput>,
    #[serde(default)]
    pub human_canary_override_approval_phrase_input: Option<HumanCanaryOverrideApprovalPhraseInput>,
    #[serde(default)]
    pub parse_human_canary_override_state_input: Option<ParseHumanCanaryOverrideStateInput>,
    #[serde(default)]
    pub daily_budget_path_input: Option<DailyBudgetPathInput>,
    #[serde(default)]
    pub runs_path_for_input: Option<RunsPathForInput>,
    #[serde(default)]
    pub effective_tier1_policy_input: Option<EffectiveTier1PolicyInput>,
    #[serde(default)]
    pub compact_tier1_exception_input: Option<CompactTier1ExceptionInput>,
    #[serde(default)]
    pub next_human_escalation_clear_at_input: Option<NextHumanEscalationClearAtInput>,
    #[serde(default)]
    pub model_catalog_canary_thresholds_input: Option<ModelCatalogCanaryThresholdsInput>,
    #[serde(default)]
    pub directive_clarification_exec_spec_input: Option<DirectiveClarificationExecSpecInput>,
    #[serde(default)]
    pub directive_decomposition_exec_spec_input: Option<DirectiveDecompositionExecSpecInput>,
    #[serde(default)]
    pub parse_actuation_spec_input: Option<ParseActuationSpecInput>,
    #[serde(default)]
    pub task_from_proposal_input: Option<TaskFromProposalInput>,
    #[serde(default)]
    pub parse_objective_id_from_evidence_refs_input: Option<ParseObjectiveIdFromEvidenceRefsInput>,
    #[serde(default)]
    pub parse_objective_id_from_command_input: Option<ParseObjectiveIdFromCommandInput>,
    #[serde(default)]
    pub objective_id_for_execution_input: Option<ObjectiveIdForExecutionInput>,
    #[serde(default)]
    pub short_text_input: Option<ShortTextInput>,
    #[serde(default)]
    pub normalized_signal_status_input: Option<NormalizedSignalStatusInput>,
    #[serde(default)]
    pub execution_reserve_snapshot_input: Option<ExecutionReserveSnapshotInput>,
    #[serde(default)]
    pub budget_pacing_gate_input: Option<BudgetPacingGateInput>,
    #[serde(default)]
    pub capability_cap_input: Option<CapabilityCapInput>,
    #[serde(default)]
    pub estimate_tokens_for_candidate_input: Option<EstimateTokensForCandidateInput>,
    #[serde(default)]
    pub no_progress_result_input: Option<NoProgressResultInput>,
    #[serde(default)]
    pub attempt_run_event_input: Option<AttemptRunEventInput>,
    #[serde(default)]
    pub safety_stop_run_event_input: Option<SafetyStopRunEventInput>,
    #[serde(default)]
    pub non_yield_category_input: Option<NonYieldCategoryInput>,
    #[serde(default)]
    pub non_yield_reason_input: Option<NonYieldReasonInput>,
    #[serde(default)]
    pub proposal_type_from_run_event_input: Option<ProposalTypeFromRunEventInput>,
    #[serde(default)]
    pub run_event_objective_id_input: Option<RunEventObjectiveIdInput>,
    #[serde(default)]
    pub run_event_proposal_id_input: Option<RunEventProposalIdInput>,
    #[serde(default)]
    pub capacity_counted_attempt_event_input: Option<CapacityCountedAttemptEventInput>,
    #[serde(default)]
    pub repeat_gate_anchor_input: Option<RepeatGateAnchorInput>,
    #[serde(default)]
    pub route_execution_policy_hold_input: Option<RouteExecutionPolicyHoldInput>,
    #[serde(default)]
    pub policy_hold_pressure_input: Option<PolicyHoldPressureInput>,
    #[serde(default)]
    pub policy_hold_pattern_input: Option<PolicyHoldPatternInput>,
    #[serde(default)]
    pub policy_hold_latest_event_input: Option<PolicyHoldLatestEventInput>,
    #[serde(default)]
    pub policy_hold_cooldown_input: Option<PolicyHoldCooldownInput>,
    #[serde(default)]
    pub receipt_verdict_input: Option<ReceiptVerdictInput>,
    #[serde(default)]
    pub default_backlog_autoscale_state_input: Option<DefaultBacklogAutoscaleStateInput>,
    #[serde(default)]
    pub normalize_backlog_autoscale_state_input: Option<NormalizeBacklogAutoscaleStateInput>,
    #[serde(default)]
    pub spawn_allocated_cells_input: Option<SpawnAllocatedCellsInput>,
    #[serde(default)]
    pub spawn_capacity_boost_snapshot_input: Option<SpawnCapacityBoostSnapshotInput>,
    #[serde(default)]
    pub inversion_maturity_score_input: Option<InversionMaturityScoreInput>,
    #[serde(default)]
    pub default_criteria_pattern_memory_input: Option<DefaultCriteriaPatternMemoryInput>,
    #[serde(default)]
    pub strategy_execution_mode_effective_input: Option<StrategyExecutionModeEffectiveInput>,
    #[serde(default)]
    pub strategy_canary_exec_limit_effective_input: Option<StrategyCanaryExecLimitEffectiveInput>,
    #[serde(default)]
    pub strategy_exploration_effective_input: Option<StrategyExplorationEffectiveInput>,
    #[serde(default)]
    pub strategy_budget_effective_input: Option<StrategyBudgetEffectiveInput>,
    #[serde(default)]
    pub preexec_verdict_from_signals_input: Option<PreexecVerdictFromSignalsInput>,
    #[serde(default)]
    pub score_only_proposal_churn_input: Option<ScoreOnlyProposalChurnInput>,
    #[serde(default)]
    pub success_criteria_quality_audit_input: Option<SuccessCriteriaQualityAuditInput>,
    #[serde(default)]
    pub detect_eyes_terminology_drift_input: Option<DetectEyesTerminologyDriftInput>,
    #[serde(default)]
    pub normalize_stored_proposal_row_input: Option<NormalizeStoredProposalRowInput>,
    #[serde(default)]
    pub recent_proposal_key_counts_input: Option<RecentProposalKeyCountsInput>,
    #[serde(default)]
    pub capability_attempt_count_for_date_input: Option<CapabilityAttemptCountForDateInput>,
    #[serde(default)]
    pub capability_outcome_stats_in_window_input: Option<CapabilityOutcomeStatsInWindowInput>,
    #[serde(default)]
    pub execute_confidence_history_input: Option<ExecuteConfidenceHistoryInput>,
    #[serde(default)]
    pub execute_confidence_policy_input: Option<ExecuteConfidencePolicyInput>,
    #[serde(default)]
    pub directive_fit_assessment_input: Option<DirectiveFitAssessmentInput>,
    #[serde(default)]
    pub signal_quality_assessment_input: Option<SignalQualityAssessmentInput>,
    #[serde(default)]
    pub actionability_assessment_input: Option<ActionabilityAssessmentInput>,
}
