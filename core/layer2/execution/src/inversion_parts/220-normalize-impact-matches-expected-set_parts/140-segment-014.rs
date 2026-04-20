    #[test]
    fn extract_mode_literals_ignores_dynamic_template_modes() {
        let text = r#"
const a = runInversionPrimitive("alpha", {});
const b = runInversionPrimitive(`beta_${suffix}`, {});
const c = runInversionPrimitive(modeName, {});
"#;
        let parsed = extract_mode_literals(text, "runInversionPrimitive");
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_mode_literals_ignores_commented_calls() {
        let text = r#"
// runInversionPrimitive("ignored_line", {});
/* runInversionPrimitive("ignored_block", {}); */
const a = runInversionPrimitive(
  "alpha",
  {}
);
"#;
        let parsed = extract_mode_literals(text, "runInversionPrimitive");
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_dispatch_modes_accepts_if_and_else_if() {
        let text = r#"
if mode == "alpha" {
}
else if mode == "beta" {
}
if another == "gamma" {
}
"#;
        let parsed = extract_dispatch_modes(text);
        let expected = ["alpha", "beta"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn extract_dispatch_modes_ignores_commented_branches() {
        let text = r#"
// if mode == "ignored_line" {
// }
/* else if mode == "ignored_block" {
} */
if mode == "alpha" {
}
"#;
        let parsed = extract_dispatch_modes(text);
        let expected = ["alpha"]
            .iter()
            .map(|value| value.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(parsed, expected);
    }

    fn read_optional_autonomy_surface(rel: &str) -> String {
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel))
            .unwrap_or_default()
    }

    #[test]
    fn inversion_bridge_is_wrapper_only_in_coreized_layout() {
        let ts_autonomy = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/autonomy_controller.ts",
        );
        let ts_inversion = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/inversion_controller.ts",
        );
        let bridge = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/backlog_autoscale_rust_bridge.ts",
        );
        let mut called = extract_mode_literals(&ts_inversion, "runInversionPrimitive");
        called.extend(extract_mode_literals(&ts_autonomy, "runInversionPrimitive"));
        if bridge.is_empty() {
            assert!(
                called.is_empty(),
                "coreized wrappers should not carry inversion mode calls"
            );
            return;
        }
        assert!(
            bridge.contains("createLegacyRetiredModule"),
            "backlog_autoscale_rust_bridge.js must remain a thin wrapper"
        );
        assert!(
            !bridge.contains("fieldByMode"),
            "wrapper-only bridge must not contain legacy inversion mode maps"
        );
        assert!(
            called.is_empty(),
            "coreized wrappers should not carry inversion mode calls"
        );
    }

    #[test]
    fn controller_callsite_modes_are_dispatched_by_rust_inversion_json() {
        let ts_autonomy = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/autonomy_controller.ts",
        );
        let ts_inversion = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/inversion_controller.ts",
        );
        let rust_src = include_str!("../inversion.rs");
        let mut called = extract_mode_literals(&ts_inversion, "runInversionPrimitive");
        called.extend(extract_mode_literals(&ts_autonomy, "runInversionPrimitive"));
        if !(ts_autonomy.is_empty() && ts_inversion.is_empty()) {
            assert!(
                ts_autonomy.contains("createOpsLaneBridge")
                    || ts_inversion.contains("createLegacyRetiredModule"),
                "expected thin-wrapper bridge markers in autonomy wrappers"
            );
        }
        let dispatched = extract_dispatch_modes(rust_src);
        let missing = called.difference(&dispatched).cloned().collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "controller TS sources use inversion modes not dispatched by Rust inversion_json: {:?}",
            missing
        );
    }

