fn inferred_family_for(id: &str) -> Option<(&'static str, &'static str)> {
    if id.starts_with("V10-ULTIMATE-001.") {
        return Some((
            "ultimate_evolution",
            "viral_replication_metacognition_exotic_hardware_tokenomics_and_universal_adapters",
        ));
    }
    if id.starts_with("V10-CORE-001.") {
        return Some((
            "ultimate_evolution",
            "core_metakernel_evolution_bootstrap_and_replication_controls",
        ));
    }
    if id.starts_with("V10-ULTIMATE-002.") {
        return Some((
            "ultimate_evolution",
            "ultimate_expansion_lane_for_operator_and_runtime_evolution_controls",
        ));
    }
    if id.starts_with("V10-PHONE-001.") {
        return Some((
            "ecosystem_scale_v11",
            "phone_surface_runtime_integration_and_operator_control_plane",
        ));
    }
    if id.starts_with("V10-SWARM-INF-001.") {
        return Some((
            "swarm_runtime_scaling",
            "swarm_infrastructure_scaling_consensus_and_resilience_controls",
        ));
    }
    if id.starts_with("V10-PERF-001.") {
        return Some((
            "competitive_execution_moat",
            "receipt_batching_simd_lockfree_coordination_pgo_slab_allocation_and_throughput_regression_guards",
        ));
    }
    if id.starts_with("V6-WORKFLOW-") || id.starts_with("V6-EXECUTION-001.") {
        return Some((
            "swarm_runtime_scaling",
            "workflow_orchestration_parallel_execution_and_checkpoint_recovery",
        ));
    }
    if id.starts_with("V6-CODE-REVIEW-") {
        return Some((
            "swarm_runtime_scaling",
            "code_review_automation_orchestration_and_recovery_controls",
        ));
    }
    if id.starts_with("V6-COLLAB-002.") {
        return Some((
            "swarm_runtime_scaling",
            "collaboration_handoff_and_inter_agent_coordination_controls",
        ));
    }
    if id.starts_with("V6-SCHEDULER-002.") || id.starts_with("V6-DASHBOARD-001.") {
        return Some((
            "automation_mission_stack",
            "scheduler_hardening_handoff_memory_security_and_dashboard_control_plane",
        ));
    }
    if id.starts_with("V6-DASHBOARD-007.") {
        return Some((
            "automation_mission_stack",
            "dashboard_queue_conduit_cockpit_autoremediation_and_attention_compaction_under_runtime_pressure",
        ));
    }
    if id.starts_with("V6-DASHBOARD-008.") {
        return Some((
            "automation_mission_stack",
            "dashboard_auto_router_selection_preflight_and_receipted_model_routing",
        ));
    }
    if id.starts_with("V6-DASHBOARD-009.") {
        return Some((
            "automation_mission_stack",
            "chat_source_run_grouping_boot_retry_and_error_status_artifacts",
        ));
    }
    if id.starts_with("V6-INFRING-GAP-001.") {
        return Some((
            "infring_assimilation_stack",
            "llm_runtime_http_ws_channels_security_and_hands_assimilation_parity",
        ));
    }
    if id.starts_with("V6-APP-023.") {
        return Some((
            "automation_mission_stack",
            "app_plane_operator_runtime_controls_and_dashboard_governance",
        ));
    }
    if id.starts_with("V6-MEMORY-") {
        return Some((
            "memory_depth_stack",
            "memory_depth_decay_compaction_and_provenance_preserving_retrieval",
        ));
    }
    if id.starts_with("V6-SKILLS-") {
        return Some((
            "skills_runtime_pack",
            "skills_runtime_expansion_focus_templates_and_deployment_pack",
        ));
    }
    if id.starts_with("V6-SECURITY-") {
        return Some((
            "security_sandbox_redteam",
            "security_gate_expansion_sandboxing_and_adversarial_resilience",
        ));
    }
    if id.starts_with("V6-LEARNING-") || id.starts_with("V6-INFERENCE-") {
        return Some((
            "learning_rsi_pipeline",
            "learning_and_inference_feedback_loops_distillation_and_policy_retraining",
        ));
    }
    if id.starts_with("V6-ADAPTER-") {
        return Some((
            "competitor_surface_expansion",
            "adapter_surface_expansion_with_provider_router_and_domain_controls",
        ));
    }
    if id.starts_with("V6-BEAT-INFRING-") {
        return Some((
            "competitive_execution_moat",
            "infring_surpass_execution_moat_with_receipted_performance_controls",
        ));
    }
    if id.starts_with("V6-BLINDSPOT-") {
        return Some((
            "autonomy_opportunity_engine",
            "blindspot_detection_and_autonomous_opportunity_prioritization",
        ));
    }
    if id.starts_with("V6-INFRING-DETACH-001.") {
        return Some((
            "infring_detachment_stack",
            "assimilate_infring_home_assets_into_infring_runtime_state_and_determine_local_independence_surfaces",
        ));
    }
    if id.starts_with("V6-ECONOMY-003.") {
        return Some((
            "ecosystem_scale_v11",
            "economy_loop_growth_governance_and_marketplace_alignment",
        ));
    }
    if id.starts_with("V8-AUTOMATION-016.") {
        return Some((
            "automation_mission_stack",
            "cron_handoff_memory_security_and_dashboard_hardening",
        ));
    }
    if id.starts_with("V8-AUTONOMY-012.") {
        return Some((
            "autonomy_opportunity_engine",
            "opportunity_scanning_inefficiency_detection_and_monetization_prioritization",
        ));
    }
    if id.starts_with("V8-CLI-001.") {
        return Some((
            "cli_surface_hardening",
            "single_rust_binary_state_machine_and_node_optional_wrapper_hardening",
        ));
    }
    if id.starts_with("V8-CLIENT-010.") {
        return Some((
            "client_model_access",
            "vibe_proxy_and_model_access_store_with_policy_controls",
        ));
    }
    if id.starts_with("V8-COMPETE-001.") {
        return Some((
            "competitive_execution_moat",
            "aot_performance_signed_receipts_non_divergence_and_resilience_flywheel",
        ));
    }
    if id.starts_with("V8-EYES-009.") {
        return Some((
            "eyes_media_assimilation",
            "video_transcription_course_assimilation_podcast_generation_and_swarm_integration",
        ));
    }
    if id.starts_with("V8-EYES-010.") {
        return Some((
            "eyes_computer_use",
            "browser_computer_use_navigation_reliability_voice_and_safety_gate",
        ));
    }
    if id.starts_with("V8-EYES-011.") {
        return Some((
            "eyes_lightpanda_router",
            "lightpanda_speed_profile_and_multi_backend_router_with_session_archival",
        ));
    }
    if id.starts_with("V8-LEARNING-") {
        return Some((
            "learning_rsi_pipeline",
            "signal_extraction_distillation_distributed_training_and_policy_retraining",
        ));
    }
    if id.starts_with("V8-MEMORY-") {
        return Some((
            "memory_depth_stack",
            "hierarchical_retrieval_lossless_sync_ast_indexing_and_provenance_memory",
        ));
    }
    if id.starts_with("V8-ORGANISM-") {
        return Some((
            "organism_parallel_intelligence",
            "side_sessions_hub_spoke_coordination_model_generation_and_evolution_archive",
        ));
    }
    if id.starts_with("V8-PERSONA-015.") {
        return Some((
            "persona_enterprise_pack",
            "ai_ceo_departmental_pack_cross_agent_memory_sync_and_role_extension",
        ));
    }
    if id.starts_with("V8-SAFETY-022.") {
        return Some((
            "safety_error_taxonomy",
            "structured_error_taxonomy_and_fail_closed_safety_receipts",
        ));
    }
    if id.starts_with("V8-SECURITY-") {
        return Some((
            "security_sandbox_redteam",
            "wasm_sandbox_credential_injection_privacy_plane_and_attack_chain_simulation",
        ));
    }
    if id.starts_with("V8-SKILLS-") {
        return Some((
            "skills_runtime_pack",
            "hf_cli_focus_templates_prompt_chaining_scaffolding_and_deployment_pack",
        ));
    }
    if id.starts_with("V8-SWARM-") {
        return Some((
            "swarm_runtime_scaling",
            "sentiment_swarm_role_routing_work_stealing_watchdog_and_real_time_dashboard",
        ));
    }
    if id.starts_with("V9-AUDIT-026.") {
        return Some((
            "audit_self_healing_stack",
            "self_healing_audit_stack_with_cross_agent_verification_and_human_gate",
        ));
    }
    if id.starts_with("V9-CLIENT-020.") {
        return Some((
            "client_wasm_bridge",
            "rust_wasm_bridge_structured_concurrency_demo_generation_and_artifact_archival",
        ));
    }
    if id.starts_with("V9-ORGANISM-025.") {
        return Some((
            "organism_adlc",
            "adlc_goals_replanning_parallel_subagents_and_live_feedback_testing",
        ));
    }
    if id.starts_with("V9-TINYMAX-021.") {
        return Some((
            "tinymax_extreme_profile",
            "trait_swappable_tinymax_core_and_sub5mb_idle_memory_mode",
        ));
    }
    None
}

fn build_profiles() -> Vec<RuntimeSystemContractProfile> {
    let mut out = BTreeMap::new();
    for group in CONTRACT_FAMILIES {
        for id in group.ids {
            out.insert(
                *id,
                RuntimeSystemContractProfile {
                    id: *id,
                    family: group.family,
                    objective: group.objective,
                    strict_conduit_only: true,
                    strict_fail_closed: true,
                },
            );
        }
    }
    for id in NEW_ACTIONABLE_IDS {
        let (family, objective) =
            inferred_family_for(id).unwrap_or(("unknown_contract_family", "unknown_objective"));
        out.insert(
            *id,
            RuntimeSystemContractProfile {
                id: *id,
                family,
                objective,
                strict_conduit_only: true,
                strict_fail_closed: true,
            },
        );
    }
    out.into_values().collect()
}

fn profiles_registry() -> &'static [RuntimeSystemContractProfile] {
    static REGISTRY: OnceLock<Vec<RuntimeSystemContractProfile>> = OnceLock::new();
    REGISTRY.get_or_init(build_profiles).as_slice()
}

fn profile_index() -> &'static BTreeMap<&'static str, RuntimeSystemContractProfile> {
    static INDEX: OnceLock<BTreeMap<&'static str, RuntimeSystemContractProfile>> = OnceLock::new();
    INDEX.get_or_init(|| {
        profiles_registry()
            .iter()
            .copied()
            .map(|profile| (profile.id, profile))
            .collect()
    })
}

pub fn actionable_profiles() -> &'static [RuntimeSystemContractProfile] {
    profiles_registry()
}

pub fn actionable_ids() -> &'static [&'static str] {
    static IDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    IDS.get_or_init(|| profiles_registry().iter().map(|row| row.id).collect())
        .as_slice()
}

pub fn profile_for(system_id: &str) -> Option<RuntimeSystemContractProfile> {
    let wanted = system_id.trim();
    profile_index().get(wanted).copied()
}

pub fn looks_like_contract_id(system_id: &str) -> bool {
    let id = system_id.trim();
    id.starts_with('V') && id.contains('-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn actionable_registry_has_expected_cardinality_and_no_duplicates() {
        let profiles = actionable_profiles();
        let mut expected = BTreeSet::new();
        for group in CONTRACT_FAMILIES {
            for id in group.ids {
                expected.insert((*id).to_string());
            }
        }
        for id in NEW_ACTIONABLE_IDS {
            expected.insert((*id).to_string());
        }
        assert_eq!(
            profiles.len(),
            expected.len(),
            "runtime contract profile count should match static registry inputs"
        );
        let mut seen = BTreeSet::new();
        for profile in profiles {
            assert!(
                seen.insert(profile.id.to_string()),
                "duplicate contract id in runtime registry: {}",
                profile.id
            );
            assert!(profile.strict_conduit_only);
            assert!(profile.strict_fail_closed);
        }
    }

    #[test]
    fn profile_lookup_resolves_known_and_rejects_unknown_ids() {
        assert!(profile_for("V8-ACT-001.1").is_some());
        assert!(profile_for("V11-ECOSYSTEM-001.7").is_some());
        assert!(profile_for("V6-COMPANY-003.5").is_some());
        assert!(profile_for("V6-EXECUTION-002.4").is_some());
        assert!(profile_for("V6-RUNTIME-CLEANUP-001.7").is_some());
        assert!(profile_for("V5-HOLD-001").is_some());
        assert!(profile_for("V5-RUST-HYB-010").is_some());
        assert!(profile_for("V5-RUST-PROD-012").is_some());
        assert!(profile_for("V10-ULTIMATE-001.6").is_some());
        assert!(profile_for("V10-PERF-001.6").is_some());
        assert!(profile_for("V6-WORKFLOW-026.5").is_some());
        assert!(profile_for("V6-DASHBOARD-007.8").is_some());
        assert!(profile_for("V6-DASHBOARD-008.4").is_some());
        assert!(profile_for("V6-INFRING-DETACH-001.2").is_some());
        assert!(profile_for("V6-INFRING-DETACH-001.4").is_some());
        assert!(profile_for("V8-SWARM-012.10").is_some());
        assert!(profile_for("V9-TINYMAX-021.2").is_some());
        assert!(profile_for("X-UNKNOWN-404.1").is_none());
    }

    #[test]
    fn infring_detach_xtask_contract_remains_non_runtime_profile() {
        let xtask_contract_id = "V6-INFRING-DETACH-001.7";
        assert!(
            profile_for(xtask_contract_id).is_none(),
            "xtask detach contract should stay outside runtime-systems profile registry"
        );
    }

    #[test]
    fn inferred_family_covers_every_new_actionable_id() {
        for id in NEW_ACTIONABLE_IDS {
            assert!(
                inferred_family_for(id).is_some(),
                "new actionable id missing inferred family: {id}"
            );
        }
    }
}
