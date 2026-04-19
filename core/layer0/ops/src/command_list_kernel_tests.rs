mod tests {
    use super::*;

    #[test]
    fn parse_args_detects_mode_and_json() {
        let argv = vec!["--mode=help".to_string(), "--json".to_string()];
        let (mode, json_out) = parse_args(&argv);
        assert_eq!(mode, "help");
        assert!(json_out);
    }

    #[test]
    fn commands_json_has_expected_shape() {
        let out = commands_json();
        let rows = out.as_array().cloned().unwrap_or_default();
        assert!(!rows.is_empty());
        let first = rows
            .first()
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert_eq!(first.get("synopsis").and_then(Value::as_str), Some("help"));
        assert_eq!(first.get("command").and_then(Value::as_str), Some("help"));
        assert_eq!(first.get("tier").and_then(Value::as_str), Some("tier1"));
        assert_eq!(
            first.get("availability").and_then(Value::as_str),
            Some("core_native")
        );
    }

    #[test]
    fn command_registry_includes_assimilate_as_experimental_runtime_surface() {
        let out = commands_json();
        let rows = out.as_array().cloned().unwrap_or_default();
        let assimilate = rows.iter().find(|row| {
            row.get("synopsis").and_then(Value::as_str) == Some(
                "assimilate <target> [--payload-base64=...] [--strict=1] [--showcase=1] [--duration-ms=<n>] [--json=1] [--allow-local-simulation=1] [--plan-only=1] [--hard-selector=<selector>] [--selector-bypass=1]",
            )
        });
        let row = assimilate
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            row.get("tier").and_then(Value::as_str),
            Some("experimental")
        );
        assert_eq!(
            row.get("handler").and_then(Value::as_str),
            Some("runtime_script")
        );
        assert_eq!(
            row.get("script_rel").and_then(Value::as_str),
            Some("client/runtime/systems/tools/assimilation_cli_bridge.ts")
        );
    }

    #[test]
    fn tier1_runtime_entrypoints_are_unique() {
        let entries = tier1_runtime_entrypoints();
        let set: BTreeSet<String> = entries.iter().map(|row| (*row).to_string()).collect();
        assert_eq!(entries.len(), set.len());
        assert!(entries
            .iter()
            .any(|row| *row == "client/runtime/systems/ops/protheusd.ts"));
    }

    #[test]
    fn command_registry_integrity_reports_no_duplicates() {
        let summary = command_registry_integrity();
        assert_eq!(summary.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            summary
                .get("duplicates")
                .and_then(Value::as_array)
            .map(|rows| rows.is_empty()),
            Some(true)
        );
        assert_eq!(
            summary
                .get("alias_collisions")
                .and_then(Value::as_array)
                .map(|rows| rows.is_empty()),
            Some(true)
        );
    }

    #[test]
    fn command_aliases_resolve_collision_safe() {
        assert_eq!(
            canonical_command_name("dashboard-ui").as_deref(),
            Some("dashboard")
        );
        assert_eq!(canonical_command_name("kairos").as_deref(), Some("proactive_daemon"));
        assert!(canonical_command_name("not_a_real_command").is_none());
    }
}
