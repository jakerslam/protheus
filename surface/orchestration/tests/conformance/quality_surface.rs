use super::*;

#[test]
fn non_legacy_surface_fixture_quality_stays_within_surface_thresholds() {
    #[derive(Default, Clone, Copy)]
    struct SurfaceStats {
        total: usize,
        fallback: usize,
        low_confidence: usize,
    }

    let fixtures = vec![
        OrchestrationRequest {
            session_id: "sdk-quality-1".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Sdk,
            payload: json!({
                "sdk": {
                    "operation_kind": "search",
                    "resource_kind": "web",
                    "request_kind": "direct",
                    "targets": [{ "kind": "url", "value": "https://example.com/releases" }]
                },
                "core_probe_envelope": {
                    "web_search": {
                        "tool_available": true,
                        "transport_available": true
                    }
                }
            }),
        },
        OrchestrationRequest {
            session_id: "gateway-quality-1".to_string(),
            intent: "opaque".to_string(),
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
                    "tool_route": {
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
            session_id: "dashboard-quality-1".to_string(),
            intent: "opaque".to_string(),
            surface: RequestSurface::Dashboard,
            payload: json!({
                "dashboard": {
                    "operation_kind": "read",
                    "resource_kind": "workspace",
                    "targets": [{ "kind": "workspace_path", "value": "README.md" }]
                }
            }),
        },
        OrchestrationRequest {
            session_id: "dashboard-quality-fallback".to_string(),
            intent: "".to_string(),
            surface: RequestSurface::Dashboard,
            payload: json!({
                "dashboard": {
                    "selection_mode": "panel"
                }
            }),
        },
    ];
    let mut runtime = OrchestrationSurfaceRuntime::new();
    let mut sdk = SurfaceStats::default();
    let mut gateway = SurfaceStats::default();
    let mut dashboard = SurfaceStats::default();

    for (idx, request) in fixtures.into_iter().enumerate() {
        let surface = request.surface;
        let package = runtime.orchestrate(request, 4_700 + idx as u64);
        let low_confidence = package
            .classification
            .reasons
            .iter()
            .any(|reason| reason == "parse_confidence_below_threshold");
        let stats = match surface {
            RequestSurface::Sdk => &mut sdk,
            RequestSurface::Gateway => &mut gateway,
            RequestSurface::Dashboard => &mut dashboard,
            RequestSurface::Legacy | RequestSurface::Cli => continue,
        };
        stats.total += 1;
        if package.classification.surface_adapter_fallback {
            stats.fallback += 1;
        }
        if low_confidence {
            stats.low_confidence += 1;
        }
    }

    let sdk_fallback_rate = sdk.fallback as f32 / sdk.total as f32;
    let sdk_low_confidence_rate = sdk.low_confidence as f32 / sdk.total as f32;
    let gateway_fallback_rate = gateway.fallback as f32 / gateway.total as f32;
    let gateway_low_confidence_rate = gateway.low_confidence as f32 / gateway.total as f32;
    let dashboard_fallback_rate = dashboard.fallback as f32 / dashboard.total as f32;
    let dashboard_low_confidence_rate = dashboard.low_confidence as f32 / dashboard.total as f32;

    assert!(sdk_fallback_rate <= 0.05, "sdk fallback rate regression");
    assert!(
        sdk_low_confidence_rate <= 0.05,
        "sdk low-confidence rate regression"
    );
    assert!(
        gateway_fallback_rate <= 0.05,
        "gateway fallback rate regression"
    );
    assert!(
        gateway_low_confidence_rate <= 0.05,
        "gateway low-confidence rate regression"
    );
    assert!(
        dashboard_fallback_rate <= 0.50,
        "dashboard fallback rate regression"
    );
    assert!(
        dashboard_low_confidence_rate <= 0.50,
        "dashboard low-confidence rate regression"
    );

    println!(
        "surface_quality_metrics={}",
        json!({
            "sdk": {
                "total": sdk.total,
                "fallback_rate": sdk_fallback_rate,
                "low_confidence_rate": sdk_low_confidence_rate
            },
            "gateway": {
                "total": gateway.total,
                "fallback_rate": gateway_fallback_rate,
                "low_confidence_rate": gateway_low_confidence_rate
            },
            "dashboard": {
                "total": dashboard.total,
                "fallback_rate": dashboard_fallback_rate,
                "low_confidence_rate": dashboard_low_confidence_rate
            }
        })
    );
}
