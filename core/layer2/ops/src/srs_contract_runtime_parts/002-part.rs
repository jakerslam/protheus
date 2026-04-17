
        let dispatch_bin = root.join("mock_dispatch_fail.sh");
        write_dispatch_script(
            &dispatch_bin,
            r#"#!/bin/sh
printf '{"ok":false,"type":"mock_plane_status"}\n'
exit 1
"#,
        );

        std::env::set_var(
            "PROTHEUS_SRS_DISPATCH_BIN",
            dispatch_bin.display().to_string(),
        );
        let receipt = execute_contract_with_options(root, id, true, true).expect("execute");
        std::env::remove_var("PROTHEUS_SRS_DISPATCH_BIN");

        assert_eq!(receipt.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            receipt.pointer("/dispatch/failed").and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    #[cfg(unix)]
    fn execute_contract_defaults_to_dispatch_strict_mode() {
        let _guard = env_guard();
        let tmp = tempdir().expect("tmp");
        let root = tmp.path();
        let id = "V7-TEST-901.3";
        let cpath = root.join(CONTRACT_ROOT).join(format!("{id}.json"));
        if let Some(parent) = cpath.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(
            &cpath,
            serde_json::to_string_pretty(&json!({
                "id": id,
                "upgrade": "Dispatch Contract Strict Default",
                "layer_map": "0/1/2",
                "deliverables": [
                    {"type":"runtime_lane","path":"core/layer0/ops/src/canyon_plane.rs"}
                ]
            }))
            .expect("encode"),
        )
        .expect("write contract");

        let dispatch_bin = root.join("mock_dispatch_fail_default.sh");
        write_dispatch_script(
            &dispatch_bin,
            r#"#!/bin/sh
printf '{"ok":false,"type":"mock_plane_status"}\n'
exit 1
"#,
        );
        std::env::set_var(
            "PROTHEUS_SRS_DISPATCH_BIN",
            dispatch_bin.display().to_string(),
        );
        let receipt = execute_contract(root, id).expect("execute");
        std::env::remove_var("PROTHEUS_SRS_DISPATCH_BIN");

        assert_eq!(receipt.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            receipt.pointer("/dispatch/strict").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            receipt.pointer("/dispatch/failed").and_then(Value::as_u64),
            Some(1)
        );
    }
}
