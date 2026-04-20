
const TERMINAL_PERMISSION_POLICY_REL: &str =
    "client/runtime/config/terminal_command_permission_policy.json";
const CLIENT_INGRESS_SUB_NEXUS: &str = "client_ingress";
const CLIENT_INGRESS_BRIDGE_SUB_NEXUS: &str = "client_ingress_bridge";
const NEXUS_INGRESS_ISSUER: &str = "dashboard_tool_turn_loop";

#[derive(Clone, Copy)]
struct IngressRouteDescriptor {
    target: &'static str,
    schema_id: &'static str,
    verb: &'static str,
    required_verity: VerityClass,
    trust_class: TrustClass,
}

fn route_descriptor(
    target: &'static str,
    schema_id: &'static str,
    verb: &'static str,
    required_verity: VerityClass,
    trust_class: TrustClass,
) -> IngressRouteDescriptor {
    IngressRouteDescriptor {
        target,
        schema_id,
        verb,
        required_verity,
        trust_class,
    }
}

fn sub_nexus_contract(sub_nexus_id: &str) -> (ModuleKind, TrustClass, VerityClass) {
    match sub_nexus_id {
        "stomach" => (
            ModuleKind::Stomach,
            TrustClass::InterModuleData,
            VerityClass::High,
        ),
        "context_stacks" => (
            ModuleKind::ContextStacks,
            TrustClass::InterModuleData,
            VerityClass::High,
        ),
        CLIENT_INGRESS_SUB_NEXUS => (
            ModuleKind::ClientIngress,
            TrustClass::ClientIngressBoundary,
            VerityClass::Standard,
        ),
        _ => (
            ModuleKind::Other,
            TrustClass::ClientIngressBoundary,
            VerityClass::Standard,
        ),
    }
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn normalize_tool_name(raw: &str) -> String {
    clean_text(raw, 80)
        .to_ascii_lowercase()
        .replace('-', "_")
        .replace(' ', "_")
}

fn tool_is_autonomous_spawn(normalized: &str) -> bool {
    matches!(
        normalized,
        "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn"
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|delta| delta.as_millis() as u64)
        .unwrap_or(0)
}

fn bool_env(name: &str, fallback: bool) -> bool {
    match std::env::var(name) {
        Ok(raw) => match clean_text(&raw, 40).to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => fallback,
        },
        Err(_) => fallback,
    }
}

fn ingress_nexus_enabled() -> bool {
    bool_env("PROTHEUS_HIERARCHICAL_NEXUS_V1", true)
}

fn ingress_force_block_pair_enabled() -> bool {
    bool_env(
        "PROTHEUS_HIERARCHICAL_NEXUS_BLOCK_CLIENT_INGRESS_ROUTE",
        false,
    )
}

fn web_tooling_relaxed_test_mode_enabled() -> bool {
    bool_env("INFRING_WEB_TOOLING_RELAXED_TEST_MODE", false)
        || bool_env("PROTHEUS_WEB_TOOLING_RELAXED_TEST_MODE", false)
}

fn ingress_nexus_relaxed_bypass_allowed_tool(tool_name: &str) -> bool {
    matches!(
        normalize_tool_name(tool_name).as_str(),
        "web_search"
            | "search_web"
            | "search"
            | "web_query"
            | "web_fetch"
            | "browse"
            | "web_conduit_fetch"
            | "batch_query"
    )
}

fn parse_module_lifecycle(raw: &str) -> Option<ModuleLifecycleState> {
    let lowered = clean_text(raw, 40).to_ascii_lowercase();
    match lowered.as_str() {
        "active" => Some(ModuleLifecycleState::Active),
        "quiesced" => Some(ModuleLifecycleState::Quiesced),
        "detached" => Some(ModuleLifecycleState::Detached),
        "maintenance" => Some(ModuleLifecycleState::Maintenance),
        "draining" => Some(ModuleLifecycleState::Draining {
            drain_deadline_ms: now_ms().saturating_add(30_000),
        }),
        _ => None,
    }
}

fn ingress_lifecycle_override_from_env() -> Option<ModuleLifecycleState> {
    std::env::var("PROTHEUS_HIERARCHICAL_NEXUS_CLIENT_INGRESS_LIFECYCLE")
        .ok()
        .and_then(|raw| parse_module_lifecycle(raw.as_str()))
}

fn ingress_route_for_tool(tool_name: &str) -> IngressRouteDescriptor {
    let normalized = normalize_tool_name(tool_name);
    if matches!(
        normalized.as_str(),
        "web_search"
            | "search_web"
            | "search"
            | "web_query"
            | "web_fetch"
            | "browse"
            | "web_conduit_fetch"
            | "batch_query"
            | "file_read"
            | "file_read_many"
    ) {
        return route_descriptor(
            "context_stacks",
            "client_ingress.tool.retrieval",
            "invoke",
            VerityClass::High,
            TrustClass::InterModuleData,
        );
    }
    if normalized.starts_with("stomach_") {
        return route_descriptor(
            "stomach",
            "client_ingress.tool.stomach",
            "invoke",
            VerityClass::High,
            TrustClass::InterModuleData,
        );
    }
    route_descriptor(
        CLIENT_INGRESS_BRIDGE_SUB_NEXUS,
        "client_ingress.tool.execute",
        "invoke",
        VerityClass::Standard,
        TrustClass::ClientIngressBoundary,
    )
}

fn terminal_ingress_route() -> IngressRouteDescriptor {
    route_descriptor(
        CLIENT_INGRESS_BRIDGE_SUB_NEXUS,
        "client_ingress.terminal.exec",
        "execute",
        VerityClass::Standard,
        TrustClass::ClientIngressBoundary,
    )
}

fn ensure_sub_nexus_registered(
    nexus: &mut MainNexusControlPlane,
    sub_nexus_id: &str,
) -> Result<(), String> {
    if nexus.registry().contains(sub_nexus_id) {
        return Ok(());
    }
    let (module_kind, trust_class, verity_class) = sub_nexus_contract(sub_nexus_id);
    let registration =
        SubNexusRegistration::new(sub_nexus_id, module_kind, trust_class, verity_class);
    let _ = nexus.register_sub_nexus(NEXUS_INGRESS_ISSUER, registration)?;
    Ok(())
}
