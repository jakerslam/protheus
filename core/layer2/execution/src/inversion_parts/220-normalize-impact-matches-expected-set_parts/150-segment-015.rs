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
