
fn command_route_preflight(root: &Path, cmd: &str, route: &Route) -> Result<(), Value> {
    let canonical = crate::command_list_kernel::canonical_command_name(cmd);
    let Some(canonical_cmd) = canonical else {
        return Ok(());
    };
    let Some(item) = crate::command_list_kernel::command_registry_item(&canonical_cmd) else {
        return Ok(());
    };
    if route.script_rel == "core://unknown-command" {
        return Err(json!({
            "ok": false,
            "type": "protheusctl_dispatch",
            "error": "command_contract_preflight_failed",
            "reason": "known_command_routed_to_unknown",
            "root_cause_code": "INF-REGISTRY-004-KNOWN-ROUTED-UNKNOWN",
            "command": clean(cmd, 120),
            "canonical_command": canonical_cmd,
            "expected_script": item.expected_script(),
            "resolved_script": route.script_rel
        }));
    }
    if route.script_rel != item.expected_script() {
        return Err(json!({
            "ok": false,
            "type": "protheusctl_dispatch",
            "error": "command_contract_preflight_failed",
            "reason": "registry_expected_script_mismatch",
            "root_cause_code": "INF-REGISTRY-005-EXPECTED-SCRIPT-MISMATCH",
            "command": clean(cmd, 120),
            "canonical_command": canonical_cmd,
            "expected_script": item.expected_script(),
            "resolved_script": route.script_rel
        }));
    }
    match item.handler_kind() {
        crate::command_list_kernel::CommandHandlerKind::CoreDomain
            if !route.script_rel.starts_with("core://") =>
        {
            return Err(json!({
                "ok": false,
                "type": "protheusctl_dispatch",
                "error": "command_contract_preflight_failed",
                "reason": "core_domain_handler_must_resolve_to_core_route",
                "root_cause_code": "INF-REGISTRY-006-HANDLER-ROUTE-MISMATCH",
                "command": clean(cmd, 120),
                "canonical_command": canonical_cmd,
                "resolved_script": route.script_rel
            }));
        }
        crate::command_list_kernel::CommandHandlerKind::RuntimeScript
            if route.script_rel.starts_with("core://") =>
        {
            return Err(json!({
                "ok": false,
                "type": "protheusctl_dispatch",
                "error": "command_contract_preflight_failed",
                "reason": "runtime_script_handler_must_resolve_to_runtime_script",
                "root_cause_code": "INF-REGISTRY-006-HANDLER-ROUTE-MISMATCH",
                "command": clean(cmd, 120),
                "canonical_command": canonical_cmd,
                "resolved_script": route.script_rel
            }));
        }
        _ => {}
    }
    if !route_script_exists(root, &route.script_rel) {
        return Err(json!({
            "ok": false,
            "type": "protheusctl_dispatch",
            "error": "command_contract_preflight_failed",
            "reason": "runtime_script_missing",
            "root_cause_code": "INF-REGISTRY-007-RUNTIME-SCRIPT-MISSING",
            "command": clean(cmd, 120),
            "canonical_command": canonical_cmd,
            "resolved_script": route.script_rel
        }));
    }
    if let Some(reason) = command_mode_reason(&canonical_cmd, route) {
        return Err(json!({
            "ok": false,
            "type": "protheusctl_dispatch",
            "error": "command_contract_preflight_failed",
            "reason": reason,
            "root_cause_code": "INF-REGISTRY-008-MODE-INCOMPATIBLE",
            "command": clean(cmd, 120),
            "canonical_command": canonical_cmd,
            "resolved_script": route.script_rel,
            "resolved_args": route.args
        }));
    }
    Ok(())
}
