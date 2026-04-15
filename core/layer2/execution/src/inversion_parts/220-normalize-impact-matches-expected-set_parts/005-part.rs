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

    #[test]
    fn rust_dispatch_covers_all_inversion_bridge_modes() {
        let bridge = read_optional_autonomy_surface(
            "../../../client/runtime/systems/autonomy/backlog_autoscale_rust_bridge.ts",
        );
        let rust_src = include_str!("../inversion.rs");
        if bridge.is_empty() {
            return;
        }
        let mapped = extract_bridge_modes(&bridge, "runInversionPrimitive");
        if mapped.is_empty() {
            assert!(
                bridge.contains("createLegacyRetiredModule"),
                "wrapper-only bridge expected when inversion map literals are retired"
            );
            return;
        }
        let dispatched = extract_dispatch_modes(rust_src);
        let missing = mapped.difference(&dispatched).cloned().collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "inversion bridge maps modes not dispatched by Rust inversion_json: {:?}",
            missing
        );
    }
}
