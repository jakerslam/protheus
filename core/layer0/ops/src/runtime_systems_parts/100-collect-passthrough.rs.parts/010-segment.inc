fn collect_passthrough(args: &[String]) -> Vec<String> {
    args.iter()
        .filter_map(|row| {
            let t = row.trim();
            if t.is_empty() {
                return None;
            }
            if t.starts_with("--system-id")
                || t.starts_with("--lane-id")
                || t.starts_with("--id")
                || t.starts_with("--apply")
                || t.starts_with("--payload-json")
                || t.starts_with("--strict")
            {
                return None;
            }
            Some(t.to_string())
        })
        .collect::<Vec<_>>()
}

fn payload_object(raw: Option<&str>) -> Result<Value, String> {
    let parsed = match raw {
        Some(v) => parse_json(Some(v))?,
        None => json!({}),
    };
    if parsed.is_object() {
        Ok(parsed)
    } else {
        Err("payload_must_be_json_object".to_string())
    }
}

fn merge_object_values(mut base: Value, extra: Value) -> Value {
    if let (Some(base_obj), Some(extra_obj)) = (base.as_object_mut(), extra.as_object()) {
        for (key, value) in extra_obj {
            base_obj.insert(key.clone(), value.clone());
        }
    }
    base
}

fn infring_assimilation_defaults() -> Value {
    json!({
        "provider_agnostic_driver_enabled": true,
        "context_budget_compaction_enabled": true,
        "llm_driver_registry_count": 4,
        "http_api_endpoints_count": 76,
        "websocket_streaming_enabled": true,
        "channel_adapters": ["slack", "matrix", "email", "whatsapp"],
        "taint_tracking_enabled": true,
        "merkle_audit_chain_enabled": true,
        "manifest_signing_enabled": true,
        "ssrf_deny_paths_enabled": true,
        "hands_registry_enabled": true,
        "skills_promotion_pipeline_enabled": true,
        "hands_fail_closed_enabled": true
    })
}

fn contract_defaults(profile: RuntimeSystemContractProfile) -> Value {
    match profile.family {
        "audit_self_healing_stack" => json!({
            "drift_detection_enabled": true,
            "self_healing_playbooks_enabled": true,
            "confidence_scoring_enabled": true,
            "cross_agent_verification_enabled": true,
            "human_review_gate_enforced": true,
            "conduit_only_enforced": true,
            "poll_interval_minutes": 15.0,
            "verification_agents": 2.0,
            "confidence_high_threshold": 0.9,
            "audit_targets": [
                "origin_integrity",
                "supply_chain_provenance_v2",
                "alpha_readiness"
            ],
            "self_healing_actions": [
                "refresh_spine_receipt",
                "rebuild_supply_chain_bundle",
                "reconcile_workspace_churn"
            ],
            "confidence_range": "0.0-1.0",
            "consensus_mode": "strict_match"
        }),
        "act_critical_judgment" => json!({
            "critical_judgment_gate": true,
            "pairwise_training_enabled": true,
            "self_mod_gate_mode": "is_change_better",
            "benchmark_lane": "alfworld_webshop_scienceworld_gpqa"
        }),
        "company_revenue_automation" => json!({
            "crm_boundary": "conduit_only",
            "auto_followup_enabled": true,
            "lead_routing_mode": "warm_inbound",
            "funnel_metrics_required": true
        }),
        "competitor_surface_expansion" => json!({
            "provider_router_mode": "governed",
            "channel_adapter_expansion": true,
            "domain_hands_expansion": true
        }),
        "go_to_market_crush" => json!({
            "enterprise_licensing_guard": "active",
            "migration_bridge": "crewai_workflow_graph_autogen_openhands",
            "ga_lts_contract": true,
            "governance_observability": "required"
        }),
        "compatibility_mole" => json!({
            "compatibility_shim_mode": "silent",
            "safety_absorption_engine": "active",
            "receipt_anchoring": "permanent"
        }),
        "power_execution" => json!({
            "release_speed_mode": "accelerated_release_profile",
            "predictive_router_intelligence": true,
            "endurance_window": "week_scale",
            "byzantine_consensus": true,
            "external_blocker_closure": true
        }),
        "swarm_orchestration" => json!({
            "parallel_swarm_enabled": true,
            "implicit_planning": true,
            "compaction_engine": "self_engineered",
            "shared_memory_mode": "swarm_aware"
        }),
        "ecosystem_scale_v11" => json!({
            "migration_hub": true,
            "persistent_actions": true,
            "marketplace": "infring_hub",
            "sdk_surface": ["rust", "python", "typescript", "go", "wasm"],
            "economic_governance_layer": true
        }),
        "ecosystem_scale_v8" => json!({
            "persistent_runtime_24x7": true,
            "skills_import_mode": "plug_and_play",
            "wifi_pose_eye_substrate": true,
            "swarm_prediction_engine": true,
            "voice_companion_mode": "realtime"
        }),
        "memory_bank_v2" => json!({
            "working_memory_state": "working_memory.json",
            "tiering": ["hot", "warm", "cold"],
            "importance_decay": true,
            "decision_log": true,
            "task_scoped_slots": true,
            "session_continuation": true,
            "uncertainty_surface": true,
            "cross_reference_graph": true
        }),
        "f100_assurance" => json!({
            "enterprise_zero_trust": true,
            "assurance_super_gate": true,
            "signed_jwt_required": true,
            "cmek_required": true,
            "private_link_required": true
        }),
        "v5_hold_remediation" => json!({
            "unchanged_state_hold_rate": 0.62,
            "low_confidence_hold_rate": 0.41,
            "cap_hold_rate": 0.33,
            "route_hold_rate": 0.28,
            "budget_hold_rate": 0.09
        }),
        "v5_rust_hybrid" => json!({
            "target_min_rust_pct": 15.0,
            "target_max_rust_pct": 25.0,
            "wrapper_integrity_ok": true
        }),
        "v5_rust_productivity" => json!({
            "throughput_ops_sec": 12000.0,
            "p95_ms": 45.0,
            "p99_ms": 90.0,
            "unit_cost_per_user": 0.012,
            "canary_enabled": true
        }),
        "ultimate_evolution" => json!({
            "replication_policy_gate": true,
            "self_awareness_journal": true,
            "exotic_hardware_abstraction": true,
            "tokenomics_ledger_enforced": true,
            "symbiosis_interface": true,
            "universal_adapter_skeleton_key": true,
            "universal_adapter_coverage_pct": 92.0
        }),
        "automation_mission_stack" => json!({
            "cron_scheduler_enabled": true,
            "multi_agent_handoff_enabled": true,
            "persistent_memory_enabled": true,
            "security_hardening_enabled": true,
            "mission_dashboard_enabled": true,
            "metadata_hover_scope": "message_only",
            "hover_pushdown_layout_enabled": true,
            "stack_interrupts_on_notifications": true,
            "boot_retry_enabled": true,
            "boot_retry_max_attempts": 5,
            "boot_retry_backoff_ms": 1000,
            "server_status_emitted": true,
            "checkpoint_interval_items": 10.0,
            "checkpoint_interval_minutes": 2.0
        }),
        "duality_resilience_stack" => json!({
            "duality_bundle_emitted": true,
            "harmony_score": 0.92,
            "toll_prediction_enabled": true,
            "imbalance_debt": 0.03,
            "fractal_balance_score": 0.89,
            "macro_composition_enabled": true,
            "dual_memory_tagging_enabled": true,
            "inversion_candidate_tagging_enabled": true
        }),
        "autonomy_opportunity_engine" => json!({
            "opportunity_discovery_engine": true,
            "inefficiency_scanner": true,
            "monetization_evaluator": true,
            "hindsight_ranking_engine": true,
            "creative_mode_signal_floor": 0.8
        }),
        "cli_surface_hardening" => json!({
            "single_static_rust_binary": true,
            "rust_state_machine_core": true,
            "ts_cli_opt_in_extension": true,
            "thin_shim_wrapper": true,
            "node_absence_doctor_message": true,
            "static_binary_mb": 1.3
        }),
        "client_model_access" => json!({
            "vibe_proxy_layer_enabled": true,
            "model_access_store_encrypted": true,
            "model_access_store_policy_gate": true
        }),
        "competitive_execution_moat" => json!({
            "aot_musl_zerocopy_lanes": true,
            "signed_receipt_export_sub_ms": true,
            "non_divergence_pre_execution_gate": true,
            "autonomous_swarm_workflow_evolution": true,
            "kernel_native_observability_governance": true,
            "edge_to_cloud_uniform_plan": true,
            "production_resilience_flywheel": true,
            "receipt_batching_enabled": true,
            "receipt_batch_size": 32,
            "simd_hotpaths_enabled": true,
            "simd_profile": "portable",
            "lock_free_coordination_enabled": true,
            "coordination_contention_ratio": 0.05,
            "pgo_enabled": true,
            "lto_enabled": true,
            "hierarchy_slab_allocator_enabled": true,
            "memory_fragmentation_percent": 4.0,
            "throughput_regression_guard_enabled": true,
            "throughput_drop_threshold_pct": 4.0,
            "throughput_ops_sec": 12600.0,
            "p95_ms": 12.0
        }),
        "eyes_media_assimilation" => json!({
            "video_transcription_enabled": true,
            "course_assimilation_pipeline": true,
            "podcast_generator_enabled": true,
            "swarm_opportunity_integration": true,
            "transcript_quality_floor": 0.86
        }),
        "eyes_computer_use" => json!({
            "parchi_computer_use_engine": true,
            "frontend_navigation_reliability": true,
            "computer_use_safety_gate": true,
            "superwhisper_voice_engine": true,
            "voice_session_blob_archival": true,
            "interaction_success_floor": 0.91
        }),
        "eyes_lightpanda_router" => json!({
            "lightpanda_backend_enabled": true,
            "ultra_speed_profile_enabled": true,
            "seamless_multi_backend_router": true,
            "browser_session_blob_archival": true,
            "target_speedup_x": 31.0
        }),
        "learning_rsi_pipeline" => json!({
            "signal_extraction_prm_judge": true,
            "hindsight_on_policy_distillation": true,
            "async_four_loop_training": true,
            "interaction_trajectory_blob_integration": true,
            "distributed_gym_factory": true,
            "adversarial_verification_pipeline": true,
            "training_flywheel_export": true,
            "real_world_product_verifier": true,
            "local_overnight_self_improvement": true,
            "real_usage_feedback_reinforcement": true,
            "single_directive_rl_engine": true,
            "emergent_strategy_discovery": true,
            "weekly_policy_retraining": true,
            "auto_rollback_enabled": true,
            "low_cost_overnight_loop": true,
            "training_loops_per_day": 3.0
        }),
        "memory_depth_stack" => json!({
            "hierarchical_tree_index_builder": true,
            "agentic_tree_reasoning_retriever": true,
            "vision_page_retrieval": true,
            "tree_index_trace_blob_archival": true,
            "lossless_folder_backend": true,
            "automatic_sync_perfect_recall": true,
            "blob_lossless_hybrid_mirroring": true,
            "tinymax_lossless_mode": true,
            "tree_sitter_ast_indexer": true,
            "blast_radius_analyzer": true,
            "auto_codebase_wiki_generator": true,
            "mcp_graph_integration": true,
            "persistent_case_facts_scratchpad": true,
            "claim_source_provenance_mapping": true,
            "recall_budget_ms": 220.0
        }),
        "organism_parallel_intelligence" => json!({
            "side_chat_forking_engine": true,
            "non_capturing_overlay_renderer": true,
            "file_overlap_peek_safety": true,
            "persistent_side_session_blob_integration": true,
            "hub_spoke_coordinator": true,
            "plan_vs_explore_subagent_separation": true,
            "autonomous_model_generator": true,
            "self_critique_alternative_perspectives": true,
            "explainer_slide_visual_synthesis": true,
            "model_evolution_archive": true
        }),
        "persona_enterprise_pack" => json!({
            "ai_ceo_persona_core": true,
            "departmental_agent_pack": true,
            "cross_agent_memory_sync": true,
            "role_based_agent_addition": true
        }),
        "safety_error_taxonomy" => json!({
            "structured_error_taxonomy": true,
            "error_fail_closed_mapping": true
        }),
        "security_sandbox_redteam" => json!({
            "wasm_capability_sandbox": true,
            "credential_injection_isolation": true,
            "verifiable_privacy_plane": true,
            "long_horizon_attack_chain_simulation": true,
            "zero_to_full_context_accumulation": true,
            "attack_trajectory_blob_archival": true,
            "max_escape_rate": 0.0
        }),
        "skills_runtime_pack" => json!({
            "native_hf_cli_skill": true,
            "autonomous_model_dataset_pipeline": true,
            "hf_pure_context_mode": true,
            "hf_output_swarm_integration": true,
            "native_pomodoro_skill": true,
            "interactive_tui_focus_mode": true,
            "shell_composable_focus_status": true,
            "focus_session_blob_integration": true,
            "raspberry_pi_edge_template": true,
            "self_healing_server_agent": true,
            "orion_team_coordinator": true,
            "productivity_workflow_pack": true,
            "lens_scribe_code_agent_pack": true,
            "claude_style_prompt_chaining": true,
            "iterative_refinement_loop": true,
            "component_fullstack_scaffolding": true,
            "one_click_deployment_flow": true
        }),
        "swarm_runtime_scaling" => json!({
            "sentiment_swarm_core": true,
            "scenario_injection_live_consensus_mapper": true,
            "prediction_market_sentiment_oracle": true,
            "swarm_trajectory_storage_dream_refinement": true,
            "role_based_model_assignment": true,
            "automatic_parallel_exploration": true,
            "visual_subagent_dashboard": true,
            "subagent_edit_permission_gate": true,
            "planning_as_tool_engine": true,
            "filesystem_native_persistent_memory": true,
            "isolated_subagent_spawning": true,
            "shell_execution_safety_gates": true,
            "worker_heartbeat": true,
            "automatic_work_stealing": true,
            "supervisor_watchdog_respawn": true,
            "output_schema_enforcement": true,
            "frequent_checkpoint_recovery": true,
            "scope_boundary_validation": true,
            "realtime_aggregation_dashboard": true,
            "capability_advertisement_adaptive_partitioning": true,
            "cross_agent_dedup_reconciliation": true,
            "timeout_graceful_degradation": true,
            "max_timeout_seconds": 60.0
        }),
        "client_wasm_bridge" => json!({
            "rust_wasm_bridge_engine": true,
            "browser_structured_concurrency": true,
            "standalone_html_demo_generator": true,
            "wasm_artifact_blob_archival": true
        }),
        "organism_adlc" => json!({
            "adlc_core_engine": true,
            "evolving_goals_replanning": true,
            "parallel_subagent_coordination": true,
            "continuous_testing_live_feedback": true
        }),
        "tinymax_extreme_profile" => json!({
            "trait_driven_swappable_tinymax_core": true,
            "sub5mb_idle_memory_mode": true,
            "idle_memory_mb": 1.4
        }),
        "execution_streaming_stack" => json!({
            "ssd_streaming_enabled": true,
            "quantization_bridge_enabled": true,
            "os_page_cache_first": true,
            "kernel_bridge_enabled": true,
            "target_tokens_per_sec": 96.0,
            "resident_memory_gb": 12.0,
            "prefetch_window": 4,
            "quantization_bits": 4,
            "metal_mode": "bridge"
        }),
        "execution_worktree_stack" => json!({
            "worktree_manager_enabled": true,
            "branch_isolation_enforced": true,
            "swarm_worktree_dispatch_enabled": true,
            "cleanup_enabled": true,
            "cleanup_interval_seconds": 75,
            "max_residual_worktrees": 3,
            "agent_id": "agent-default",
            "base_branch": "main"
        }),
        "assimilate_fast_stack" => json!({
            "fast_mode_enabled": true,
            "skeleton_cache_enabled": true,
            "progress_receipts_enabled": true,
            "parallel_microtasks_enabled": true,
            "warmup_enabled": true,
            "safety_guard_enabled": true,
            "target_latency_ms": 5000.0,
            "cache_ttl_seconds": 3600,
            "max_parallel_microtasks": 8
        }),
        "workflow_open_swe_stack" => json!({
            "loop_registry_enabled": true,
            "git_bridge_enabled": true,
            "approval_middleware_enabled": true,
            "eval_harness_enabled": true,
            "memory_continuity_enabled": true,
            "eval_pass_floor": 0.8
        }),
        "memory_context_maintenance" => json!({
            "staleness_tracking_enabled": true,
            "pre_generation_pruning_enabled": true,
            "emergency_compact_enabled": true,
            "context_observability_enabled": true,
            "safe_config_validation_enabled": true,
            "context_budget_tokens": 250000,
            "window_usage_tokens": 120000,
            "staleness_reset_seconds": 30
        }),
        "integration_lakehouse_stack" => json!({
            "unity_catalog_bridge_enabled": true,
            "mosaic_mapping_enabled": true,
            "mlflow_provider_enabled": true,
            "vector_automl_bridge_enabled": true,
            "dbrx_provider_enabled": true,
            "drift_monitoring_enabled": true,
            "catalog": "main",
            "schema": "default"
        }),
        "inference_adaptive_routing" => json!({
            "live_scoring_enabled": true,
            "rule_routing_enabled": true,
            "ordered_failover_enabled": true,
            "provider_observability_enabled": true,
            "min_success_rate": 0.8,
            "max_latency_ms": 1500.0,
            "preferred_model": "kimi2.5:cloud",
            "providers": [
                {
                    "name": "kimi2.5:cloud",
                    "latency_ms": 320,
                    "cost_per_1k": 0.0015,
                    "success_rate": 0.96
                },
                {
                    "name": "gpt-5.4",
                    "latency_ms": 410,
                    "cost_per_1k": 0.0030,
                    "success_rate": 0.97
                },
                {
                    "name": "qwen-max",
                    "latency_ms": 290,
                    "cost_per_1k": 0.0012,
                    "success_rate": 0.91
                }
            ],
            "rules": [
                { "id": "large_context", "max_cost_per_1k": 0.0035, "min_context_tokens": 64000, "force_provider": "kimi2.5:cloud" },
                { "id": "strict_latency", "max_latency_ms": 350, "force_provider": "qwen-max" }
            ]
        }),
        "runtime_cleanup_autonomous" => json!({
            "eviction_matrix_enabled": true,
            "multi_trigger_scheduler_enabled": true,
            "tiered_pressure_enabled": true,
            "device_profiles_enabled": true,
            "protected_state_invariants_enabled": true,
            "audit_controls_enabled": true,
            "boundedness_gate_enabled": true,
            "cleanup_interval_minutes": 5,
            "memory_threshold_percent": 75,
            "disk_threshold_percent": 10,
            "disk_free_percent": 22,
            "device_profile": "desktop",
            "max_storage_mb": 500
        }),
        "erp_agentic_stack" => json!({
            "erp_template_registry_enabled": true,
            "closed_loop_enabled": true,
            "lineage_gate_enabled": true,
            "max_loop_latency_ms": 1500,
            "team": "procurement"
        }),
        "tooling_uv_ruff_stack" => json!({
            "uv_bridge_enabled": true,
            "ruff_bridge_enabled": true,
            "isolated_env_enabled": true,
            "autowire_pipeline_enabled": true,
            "tooling_gate_enabled": true,
            "max_resolution_time_seconds": 300,
            "venv_name": "default",
            "tiny_mode_no_regression": true,
            "pure_mode_no_regression": true
        }),
        "workflow_visual_bridge_stack" => json!({
            "canvas_bridge_enabled": true,
            "prompt_chain_enabled": true,
            "rag_integration_enabled": true,
            "tool_eval_enabled": true,
            "enterprise_observability_enabled": true,
            "cold_start_guard_ms": 3000
        }),
        "infring_assimilation_stack" => infring_assimilation_defaults(),
        "infring_detachment_stack" => merge_object_values(infring_assimilation_defaults(), json!({
            "source_assimilation_enabled": true,
            "nursery_migration_enabled": true,
            "external_dependency_detached": true,
            "operator_state_capture_enabled": true,
            "local_runtime_paths_enforced": true,
            "source_controlled_mirror_enabled": true,
            "llm_runtime_registry_enabled": true,
            "source_files_required_min": 1,
            "llm_registry_models_min": 1,
            "max_assimilation_copy_mb": 1024,
            "source_root": ".."
        })),
        _ => json!({}),
    }
}
