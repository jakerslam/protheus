fn run_action_family_dashboard_core(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    match normalized {
        "dashboard.assimilate" => {
            let target = payload
                .get("target")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "codex".to_string());
            run_lane(
                root,
                "app-plane",
                &[
                    "run".to_string(),
                    "--app=chat-ui".to_string(),
                    format!("--input=assimilate target {target} with receipt-first safety"),
                ],
            )
        }
        "dashboard.benchmark" => run_lane(root, "health-status", &["dashboard".to_string()]),
        "dashboard.models.catalog" => {
            let runtime_flags = Flags {
                mode: "snapshot".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: DEFAULT_TEAM.to_string(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let snapshot = build_snapshot(root, &runtime_flags);
            let result = dashboard_model_catalog::catalog_payload(root, &snapshot);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.models.catalog".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.model.routeDecision" => {
            let runtime_flags = Flags {
                mode: "snapshot".to_string(),
                host: DEFAULT_HOST.to_string(),
                port: DEFAULT_PORT,
                team: DEFAULT_TEAM.to_string(),
                refresh_ms: DEFAULT_REFRESH_MS,
                pretty: false,
            };
            let snapshot = build_snapshot(root, &runtime_flags);
            let result = dashboard_model_catalog::route_decision_payload(root, &snapshot, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: 0,
                argv: vec!["dashboard.model.routeDecision".to_string()],
                payload: Some(result),
            }
        }
        _ => run_action_family_dashboard_github(root, normalized, payload),
    }
}
