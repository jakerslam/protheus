
fn authorize_client_ingress_route_with_nexus_inner(
    route_label: &str,
    route: IngressRouteDescriptor,
    force_block_pair: bool,
    source_lifecycle_override: Option<ModuleLifecycleState>,
) -> Result<Value, String> {
    let mut policy = DefaultNexusPolicy::default();
    if force_block_pair {
        policy.block_pair(CLIENT_INGRESS_SUB_NEXUS, route.target);
    }
    let mut nexus = MainNexusControlPlane::new(
        NexusFeatureFlags {
            hierarchical_nexus_enabled: true,
            coexist_with_flat_routing: true,
        },
        policy,
    );
    let _ = nexus.register_v1_adapters(NEXUS_INGRESS_ISSUER)?;
    ensure_sub_nexus_registered(&mut nexus, route.target)?;
    if let Some(next) = source_lifecycle_override {
        let _ = nexus.set_module_lifecycle(NEXUS_INGRESS_ISSUER, CLIENT_INGRESS_SUB_NEXUS, next)?;
    }

    let lease = nexus.issue_route_lease(
        NEXUS_INGRESS_ISSUER,
        LeaseIssueRequest {
            source: CLIENT_INGRESS_SUB_NEXUS.to_string(),
            target: route.target.to_string(),
            schema_ids: vec![route.schema_id.to_string()],
            verbs: vec![route.verb.to_string()],
            required_verity: route.required_verity,
            trust_class: route.trust_class,
            requested_ttl_ms: 45_000,
            template_id: None,
            template_version: None,
        },
    )?;
    let delivery = nexus.authorize_direct_delivery(
        NEXUS_INGRESS_ISSUER,
        DeliveryAuthorizationInput {
            source: CLIENT_INGRESS_SUB_NEXUS.to_string(),
            target: route.target.to_string(),
            schema_id: route.schema_id.to_string(),
            verb: route.verb.to_string(),
            offered_verity: route.required_verity,
            lease_id: Some(lease.lease_id.clone()),
            now_ms: None,
        },
    );
    if !delivery.allowed {
        return Err(format!(
            "client_ingress_nexus_delivery_denied:{}",
            delivery.reason
        ));
    }
    let receipt_ids = nexus
        .receipts()
        .iter()
        .map(|row| Value::String(row.receipt_id.clone()))
        .collect::<Vec<_>>();
    Ok(json!({
      "enabled": true,
      "source": CLIENT_INGRESS_SUB_NEXUS,
      "target": route.target,
      "schema_id": route.schema_id,
      "verb": route.verb,
      "route_label": clean_text(route_label, 200),
      "lease_id": lease.lease_id,
      "policy_decision_ref": lease.policy_decision_ref,
      "delivery": {"allowed": delivery.allowed, "reason": delivery.reason, "local_resolution": delivery.local_resolution, "conduit_link_id": delivery.conduit_link_id},
      "metrics": nexus.metrics(),
      "receipt_ids": receipt_ids
    }))
}

pub(crate) fn authorize_ingress_tool_call_with_nexus(
    tool_name: &str,
) -> Result<Option<Value>, String> {
    if !ingress_nexus_enabled() {
        return Ok(None);
    }
    if web_tooling_relaxed_test_mode_enabled()
        && ingress_nexus_relaxed_bypass_allowed_tool(tool_name)
    {
        let route = ingress_route_for_tool(tool_name);
        return Ok(Some(json!({
            "enabled": true,
            "source": CLIENT_INGRESS_SUB_NEXUS,
            "target": route.target,
            "schema_id": route.schema_id,
            "verb": route.verb,
            "route_label": format!("tool:{}", normalize_tool_name(tool_name)),
            "delivery": {
                "allowed": true,
                "reason": "relaxed_test_mode_bypass",
                "local_resolution": true
            },
            "policy_bypass": true,
            "bypass_reason": "web_tooling_relaxed_test_mode"
        })));
    }
    let route = ingress_route_for_tool(tool_name);
    let connection = authorize_client_ingress_route_with_nexus_inner(
        &format!("tool:{tool_name}"),
        route,
        ingress_force_block_pair_enabled(),
        ingress_lifecycle_override_from_env(),
    )?;
    Ok(Some(connection))
}

pub(crate) fn authorize_ingress_terminal_command_with_nexus(
    command: &str,
) -> Result<Option<Value>, String> {
    if !ingress_nexus_enabled() {
        return Ok(None);
    }
    if web_tooling_relaxed_test_mode_enabled() {
        let route = terminal_ingress_route();
        return Ok(Some(json!({
            "enabled": true,
            "source": CLIENT_INGRESS_SUB_NEXUS,
            "target": route.target,
            "schema_id": route.schema_id,
            "verb": route.verb,
            "route_label": format!("terminal:{}", clean_text(command, 220)),
            "delivery": {
                "allowed": true,
                "reason": "relaxed_test_mode_bypass",
                "local_resolution": true
            },
            "policy_bypass": true,
            "bypass_reason": "web_tooling_relaxed_test_mode"
        })));
    }
    let connection = authorize_client_ingress_route_with_nexus_inner(
        &format!("terminal:{}", clean_text(command, 220)),
        terminal_ingress_route(),
        ingress_force_block_pair_enabled(),
        ingress_lifecycle_override_from_env(),
    )?;
    Ok(Some(connection))
}

fn load_permission_rules(root: &Path) -> (Vec<String>, Vec<String>) {
    let mut deny = Vec::<String>::new();
    let mut ask = Vec::<String>::new();
    let path = root.join(TERMINAL_PERMISSION_POLICY_REL);
    if let Ok(raw) = fs::read_to_string(&path) {
        if let Ok(value) = serde_json::from_str::<Value>(&raw) {
            let (policy_deny, policy_ask) =
                crate::command_permission_kernel::collect_permission_rules_for_kernel(Some(&value));
            deny.extend(policy_deny);
            ask.extend(policy_ask);
        }
    }
    deny.sort();
    deny.dedup();
    ask.sort();
    ask.dedup();
    (deny, ask)
}

fn input_confirmed(input: &Value) -> bool {
    input
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || !clean_text(
            input
                .get("approval_note")
                .and_then(Value::as_str)
                .unwrap_or(""),
            200,
        )
        .is_empty()
}
