fn compat_api_response_with_nexus(
    route_label: &str,
    mut response: CompatApiResponse,
) -> CompatApiResponse {
    let runtime_web_tooling_contract =
        compat_runtime_web_tooling_contract_snapshot(clean_text(route_label, 180).as_str());
    let route_kind = compat_route_nexus_kind(route_label);
    if route_kind == CompatRouteNexusKind::None {
        if let Some(obj) = response.payload.as_object_mut() {
            obj.insert(
                "runtime_web_tooling_contract".to_string(),
                runtime_web_tooling_contract,
            );
        }
        return response;
    }
    let auth_result = match route_kind {
        CompatRouteNexusKind::Tool => {
            crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(route_label)
        }
        CompatRouteNexusKind::Terminal => {
            crate::dashboard_tool_turn_loop::authorize_ingress_terminal_command_with_nexus(
                route_label,
            )
        }
        CompatRouteNexusKind::None => Ok(None),
    };
    match auth_result {
        Ok(Some(meta)) => {
            if let Some(obj) = response.payload.as_object_mut() {
                obj.insert("nexus_connection".to_string(), meta);
                obj.insert(
                    "runtime_web_tooling_contract".to_string(),
                    runtime_web_tooling_contract,
                );
            }
            response
        }
        Ok(None) => {
            if let Some(obj) = response.payload.as_object_mut() {
                obj.insert(
                    "runtime_web_tooling_contract".to_string(),
                    runtime_web_tooling_contract,
                );
            }
            response
        }
        Err(err) => CompatApiResponse {
            status: 403,
            payload: json!({
                "ok": false,
                "error": "nexus_route_denied",
                "route_label": clean_text(route_label, 180),
                "reason": clean_text(&err, 240),
                "fail_closed": true,
                "runtime_web_tooling_contract": runtime_web_tooling_contract
            }),
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompatRouteNexusKind {
    None,
    Tool,
    Terminal,
}

fn compat_route_nexus_kind(route_label: &str) -> CompatRouteNexusKind {
    let lowered = clean_text(route_label, 180).to_ascii_lowercase();
    if lowered.starts_with("terminal:") || lowered.contains("/terminal") {
        return CompatRouteNexusKind::Terminal;
    }
    if lowered.starts_with("tool:") || lowered.contains("/tool") {
        return CompatRouteNexusKind::Tool;
    }
    CompatRouteNexusKind::None
}

fn compat_runtime_web_tooling_auth_sources() -> Vec<String> {
    let env_candidates = [
        "BRAVE_API_KEY",
        "EXA_API_KEY",
        "TAVILY_API_KEY",
        "PERPLEXITY_API_KEY",
        "SERPAPI_API_KEY",
        "GOOGLE_SEARCH_API_KEY",
        "GOOGLE_CSE_ID",
        "FIRECRAWL_API_KEY",
        "XAI_API_KEY",
        "MOONSHOT_API_KEY",
        "OPENAI_API_KEY",
    ];
    let mut sources = Vec::<String>::new();
    for env_name in env_candidates {
        let present = std::env::var(env_name)
            .ok()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        if present {
            sources.push(format!("env:{env_name}"));
        }
    }
    sources
}

fn compat_runtime_web_tooling_contract_snapshot(route_label: &str) -> Value {
    let auth_sources = compat_runtime_web_tooling_auth_sources();
    json!({
        "contract_version": "dashboard_web_tooling_contract_v1",
        "route_label": clean_text(route_label, 180),
        "strict_auth_required": std::env::var("INFRING_WEB_TOOLING_STRICT_AUTH")
            .ok()
            .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "y" | "on"))
            .unwrap_or(true),
        "auth_present": !auth_sources.is_empty(),
        "auth_sources": auth_sources,
        "provider_aliases": {
            "google": "google_search",
            "xai": "grok",
            "moonshot": "kimi",
            "serp": "serpapi"
        },
        "provider_contract_targets": [
            "exa",
            "firecrawl",
            "google",
            "perplexity",
            "tavily"
        ]
    })
}

pub fn handle(
    root: &Path,
    method: &str,
    path: &str,
    body: &[u8],
    snapshot: &Value,
) -> Option<CompatApiResponse> {
    handle_with_headers(root, method, path, body, &[], snapshot)
}

#[cfg(test)]
mod tests {
    include!("../config_payload_tests_parts/010-init-git-repo.rs");
    include!("../config_payload_tests_parts/020-agent-create-without-name-returns-non-generic-id.rs");
    include!("../config_payload_tests_parts/030-memory-kv-http-routes-round-trip-and-feed-context.rs");
    include!("../config_payload_tests_parts/040-terminated-agent-endpoints-round-trip.rs");
    include!("../config_payload_tests_parts/050-compact-session-keyframes.rs");
    include!("../config_payload_tests_parts/060-context-telemetry-and-auto-compact.rs");
    include!("../config_payload_tests_parts/070-cron-command-routing.rs");
    include!("../config_payload_tests_parts/080-conversation-search-includes-archived.rs");
    include!("../config_payload_tests_parts/090-latent-tool-discovery-and-rollback.rs");
    include!("../config_payload_tests_parts/100-governance-and-semantic-memory.rs");
    include!("../config_payload_tests_parts/110-agent-capability-gauntlet.rs");
    include!("../config_payload_tests_parts/120-receipts-lineage-route.rs");
}
