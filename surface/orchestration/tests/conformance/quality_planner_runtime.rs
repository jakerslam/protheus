use super::*;

#[test]
fn planner_quality_fixture_metrics_stay_within_thresholds() {
    let fixtures = vec![
        OrchestrationRequest {
            session_id: "planner-quality-sdk".to_string(),
            intent: "search release notes".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                },
                "core_probe_envelope": {
                    "execute_tool": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        OrchestrationRequest {
            session_id: "planner-quality-compare".to_string(),
            intent: "compare workspace and web".to_string(),
            surface: RequestSurface::Gateway,
            payload: json!({
                "gateway": {
                    "route": "compare.resource",
                    "resource_kind": "mixed",
                    "targets": [
                        { "kind": "workspace_path", "value": "README.md" },
                        { "kind": "url", "value": "https://example.com/docs" }
                    ]
                },
                "core_probe_envelope": {
                    "execute_tool": {
                        "tool_available": true,
                        "transport_available": true
                    },
                    "verify_claim": {
                        "transport_available": true
                    }
                }
            }),
        },
        OrchestrationRequest {
            session_id: "planner-quality-legacy".to_string(),
            intent: "search the web for release notes".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        OrchestrationRequest {
            session_id: "planner-quality-ambiguous".to_string(),
            intent: "maybe do something".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        OrchestrationRequest {
            session_id: "planner-quality-mutation".to_string(),
            intent: "implement requested mutation".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "mutate",
                    "resource_kind": "task_graph",
                    "request_kind": "direct",
                    "mutability": "mutation",
                    "targets": [{ "kind": "task_id", "value": "task-42" }]
                },
                "core_probe_envelope": {
                    "mutate_task": {
                        "target_supplied": true,
                        "target_syntactically_valid": true,
                        "target_exists": true,
                        "authorization_valid": false,
                        "policy_allows": true
                    }
                }
            }),
        },
        OrchestrationRequest {
            session_id: "planner-quality-sdk-2".to_string(),
            intent: "search release notes".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                },
                "core_probe_envelope": {
                    "execute_tool": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        OrchestrationRequest {
            session_id: "planner-quality-compare-2".to_string(),
            intent: "compare workspace and web".to_string(),
            surface: RequestSurface::Gateway,
            payload: json!({
                "gateway": {
                    "route": "compare.resource",
                    "resource_kind": "mixed",
                    "targets": [
                        { "kind": "workspace_path", "value": "README.md" },
                        { "kind": "url", "value": "https://example.com/docs" }
                    ]
                },
                "core_probe_envelope": {
                    "execute_tool": {
                        "tool_available": true,
                        "transport_available": true
                    },
                    "verify_claim": {
                        "transport_available": true
                    }
                }
            }),
        },
        OrchestrationRequest {
            session_id: "planner-quality-legacy-2".to_string(),
            intent: "search the web for release notes".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        OrchestrationRequest {
            session_id: "planner-quality-ambiguous-2".to_string(),
            intent: "maybe do something".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
        OrchestrationRequest {
            session_id: "planner-quality-mutation-2".to_string(),
            intent: "implement requested mutation".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "mutate",
                    "resource_kind": "task_graph",
                    "request_kind": "direct",
                    "mutability": "mutation",
                    "targets": [{ "kind": "task_id", "value": "task-42" }]
                },
                "core_probe_envelope": {
                    "mutate_task": {
                        "target_supplied": true,
                        "target_syntactically_valid": true,
                        "target_exists": true,
                        "authorization_valid": false,
                        "policy_allows": true
                    }
                }
            }),
        },
    ];
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let mut candidate_counts = Vec::new();
    let mut clarification_first_selected = 0usize;
    let mut degraded_selected = 0usize;
    let mut selected_plan_degraded = 0usize;
    let mut selected_plan_requires_clarification = 0usize;
    let mut heuristic_probe_selected = 0usize;
    let mut zero_executable_selected = 0usize;
    let mut all_candidates_clarification_selected = 0usize;
    let mut all_candidates_degraded_selected = 0usize;

    for (idx, request) in fixtures.into_iter().enumerate() {
        let package = runtime.orchestrate(request, 4_760 + idx as u64);
        let candidate_count = 1 + package.alternative_plans.len();
        candidate_counts.push(candidate_count);
        if package.selected_plan.variant
            == infring_orchestration_surface_v1::contracts::PlanVariant::ClarificationFirst
        {
            clarification_first_selected += 1;
        }
        if package.execution_state.plan_status
            == infring_orchestration_surface_v1::contracts::PlanStatus::Degraded
            || package.selected_plan.variant
                == infring_orchestration_surface_v1::contracts::PlanVariant::DegradedFallback
        {
            degraded_selected += 1;
        }
        if package.runtime_quality.selected_plan_degraded {
            selected_plan_degraded += 1;
        }
        if package.runtime_quality.selected_plan_requires_clarification {
            selected_plan_requires_clarification += 1;
        }
        if package.selected_plan.capability_probes.iter().any(|probe| {
            probe
                .probe_sources
                .iter()
                .any(|source| source.starts_with("heuristic."))
        }) {
            heuristic_probe_selected += 1;
        }
        if package.runtime_quality.zero_executable_candidates {
            zero_executable_selected += 1;
        }
        if package.runtime_quality.all_candidates_require_clarification {
            all_candidates_clarification_selected += 1;
        }
        if package.runtime_quality.all_candidates_degraded {
            all_candidates_degraded_selected += 1;
        }
    }

    let total = candidate_counts.len() as f32;
    let average_candidate_count = candidate_counts.iter().sum::<usize>() as f32 / total.max(1.0);
    let clarification_first_rate = clarification_first_selected as f32 / total.max(1.0);
    let degraded_rate = degraded_selected as f32 / total.max(1.0);
    let selected_plan_requires_clarification_rate =
        selected_plan_requires_clarification as f32 / total.max(1.0);
    let selected_plan_degraded_rate = selected_plan_degraded as f32 / total.max(1.0);
    let heuristic_probe_rate = heuristic_probe_selected as f32 / total.max(1.0);
    let zero_executable_candidate_rate = zero_executable_selected as f32 / total.max(1.0);
    let all_candidates_require_clarification_rate =
        all_candidates_clarification_selected as f32 / total.max(1.0);
    let all_candidates_degraded_rate = all_candidates_degraded_selected as f32 / total.max(1.0);

    assert!(
        candidate_counts.len() >= 10,
        "planner fixture request-count regression"
    );
    assert!(
        candidate_counts.iter().all(|count| *count >= 2),
        "planner candidate diversity regression"
    );
    assert!(
        average_candidate_count >= 2.0,
        "planner average candidate count regression"
    );
    assert!(
        clarification_first_rate <= 0.40,
        "clarification-first selection rate regression"
    );
    assert!(degraded_rate <= 0.45, "degraded selection rate regression");
    assert!(
        heuristic_probe_rate <= 0.45,
        "heuristic probe dependence regression"
    );
    assert!(
        zero_executable_candidate_rate <= 0.45,
        "zero-executable candidate rate regression"
    );
    assert!(
        all_candidates_require_clarification_rate <= 0.45,
        "all-candidates-clarification rate regression"
    );
    assert!(
        all_candidates_degraded_rate <= 0.35,
        "all-candidates-degraded rate regression"
    );

    println!(
        "planner_quality_metrics={}",
        json!({
            "request_count": candidate_counts.len(),
            "average_candidate_count": average_candidate_count,
            "clarification_first_rate": clarification_first_rate,
            "degraded_rate": degraded_rate,
            "selected_plan_requires_clarification_rate": selected_plan_requires_clarification_rate,
            "selected_plan_degraded_rate": selected_plan_degraded_rate,
            "heuristic_probe_rate": heuristic_probe_rate,
            "zero_executable_candidate_rate": zero_executable_candidate_rate,
            "all_candidates_require_clarification_rate": all_candidates_require_clarification_rate,
            "all_candidates_degraded_rate": all_candidates_degraded_rate
        })
    );
}

#[test]
fn runtime_quality_telemetry_metrics_stay_within_thresholds() {
    let fixtures = vec![
        OrchestrationRequest {
            session_id: "runtime-quality-sdk".to_string(),
            intent: "search release notes".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                },
                "core_probe_envelope": {
                    "execute_tool": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        OrchestrationRequest {
            session_id: "runtime-quality-gateway".to_string(),
            intent: "compare workspace and web".to_string(),
            surface: RequestSurface::Gateway,
            payload: json!({
                "gateway": {
                    "route": "compare.resource",
                    "resource_kind": "mixed",
                    "targets": [
                        { "kind": "workspace_path", "value": "README.md" },
                        { "kind": "url", "value": "https://example.com/docs" }
                    ]
                },
                "core_probe_envelope": {
                    "execute_tool": {
                        "tool_available": true,
                        "transport_available": true
                    },
                    "verify_claim": {
                        "transport_available": true
                    }
                }
            }),
        },
        OrchestrationRequest {
            session_id: "runtime-quality-dashboard-fallback".to_string(),
            intent: "".to_string(),
            surface: RequestSurface::Dashboard,
            payload: json!({
                "dashboard": {
                    "selection_mode": "panel"
                }
            }),
        },
        OrchestrationRequest {
            session_id: "runtime-quality-legacy".to_string(),
            intent: "search web for release notes".to_string(),
            surface: RequestSurface::Legacy,
            payload: json!({}),
        },
    ];
    let fixture_count = fixtures.len().max(1);

    let mut runtime = OrchestrationSurfaceRuntime::new();
    let mut non_legacy_total = 0usize;
    let mut non_legacy_fallback = 0usize;
    let mut non_legacy_heuristic = 0usize;
    let mut non_legacy_clarification = 0usize;
    let mut non_legacy_zero_executable = 0usize;
    let mut non_legacy_all_candidates_degraded = 0usize;
    let mut candidate_total = 0usize;

    for (idx, request) in fixtures.into_iter().enumerate() {
        let package = runtime.orchestrate(request, 4_900 + idx as u64);
        candidate_total += package.runtime_quality.candidate_count as usize;
        if package.classification.request_class == RequestClass::ReadOnly {
            assert!(
                package.runtime_quality.candidate_count >= 1,
                "runtime telemetry candidate_count should always be populated"
            );
        }
        if !matches!(package.classification.request_class, RequestClass::ReadOnly)
            && package.runtime_quality.used_heuristic_probe
        {
            assert!(
                !package.selected_plan.capability_probes.is_empty(),
                "runtime heuristic probe signal must correspond to probe rows"
            );
        }
        if !matches!(package.classification.request_class, RequestClass::ReadOnly)
            && package.runtime_quality.selected_plan_requires_clarification
        {
            assert!(
                package.classification.needs_clarification
                    || package.selected_plan.requires_clarification,
                "runtime clarification signal must match selected plan/classification"
            );
        }
        if !matches!(package.classification.request_class, RequestClass::ReadOnly)
            && package.runtime_quality.selected_plan_degraded
        {
            assert!(
                !package.selected_plan.degradation.is_empty()
                    || package.execution_state.plan_status
                        == infring_orchestration_surface_v1::contracts::PlanStatus::Degraded,
                "runtime degraded signal must match plan degradation state"
            );
        }

        if package.classification.surface_adapter_used
            || package.classification.surface_adapter_fallback
        {
            non_legacy_total += 1;
            if package.runtime_quality.surface_adapter_fallback {
                non_legacy_fallback += 1;
            }
            if package.runtime_quality.used_heuristic_probe {
                non_legacy_heuristic += 1;
            }
            if package.runtime_quality.selected_plan_requires_clarification {
                non_legacy_clarification += 1;
            }
            if package.runtime_quality.zero_executable_candidates {
                non_legacy_zero_executable += 1;
            }
            if package.runtime_quality.all_candidates_degraded {
                non_legacy_all_candidates_degraded += 1;
            }
            if package.runtime_quality.zero_executable_candidates {
                assert_eq!(
                    package.runtime_quality.executable_candidate_count, 0,
                    "zero executable flag must align with executable count"
                );
            }
            if package.runtime_quality.all_candidates_degraded {
                assert_eq!(
                    package.runtime_quality.degraded_candidate_count,
                    package.runtime_quality.candidate_count,
                    "all-candidates-degraded flag must align with counts"
                );
            }
        }
    }

    let total = non_legacy_total.max(1) as f32;
    let fallback_rate = non_legacy_fallback as f32 / total;
    let heuristic_probe_rate = non_legacy_heuristic as f32 / total;
    let clarification_rate = non_legacy_clarification as f32 / total;
    let zero_executable_rate = non_legacy_zero_executable as f32 / total;
    let all_candidates_degraded_rate = non_legacy_all_candidates_degraded as f32 / total;
    let average_candidate_count = candidate_total as f32 / fixture_count as f32;

    assert!(
        non_legacy_total >= 3,
        "runtime non-legacy sample size regression"
    );
    assert!(fallback_rate <= 0.35, "runtime fallback rate regression");
    assert!(
        heuristic_probe_rate <= 0.40,
        "runtime heuristic probe rate regression"
    );
    assert!(
        clarification_rate <= 0.40,
        "runtime clarification rate regression"
    );
    assert!(
        zero_executable_rate <= 0.40,
        "runtime zero-executable rate regression"
    );
    assert!(
        all_candidates_degraded_rate <= 0.40,
        "runtime all-candidates-degraded rate regression"
    );
    assert!(
        average_candidate_count >= 1.5,
        "runtime candidate count regression"
    );

    println!(
        "runtime_quality_metrics={}",
        json!({
            "sample_size_non_legacy": non_legacy_total,
            "fallback_rate_non_legacy": fallback_rate,
            "heuristic_probe_rate_non_legacy": heuristic_probe_rate,
            "clarification_rate_non_legacy": clarification_rate,
            "zero_executable_rate_non_legacy": zero_executable_rate,
            "all_candidates_degraded_rate_non_legacy": all_candidates_degraded_rate,
            "average_candidate_count": average_candidate_count
        })
    );
}
