
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    #[cfg(unix)]
    fn write_dispatch_script(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        fs::write(path, body).expect("write script");
        let mut perms = fs::metadata(path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("set perms");
    }

    #[test]
    fn execute_contract_writes_latest_and_history() {
        let tmp = tempdir().expect("tmp");
        let root = tmp.path();
        let id = "V6-TEST-900.1";
        let cpath = root.join(CONTRACT_ROOT).join(format!("{id}.json"));
        if let Some(parent) = cpath.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(
            &cpath,
            serde_json::to_string_pretty(&json!({
                "id": id,
                "upgrade": "Test Contract",
                "layer_map": "0/1/2",
                "deliverables": [{"type":"contract","path":"planes/contracts/srs/V6-TEST-900.1.json"}]
            }))
            .expect("encode"),
        )
        .expect("write contract");

        let receipt = execute_contract(root, id).expect("execute");
        assert_eq!(receipt.get("ok").and_then(Value::as_bool), Some(true));
        assert!(latest_path(root, id).exists());
        assert!(history_path(root).exists());
    }

    #[test]
    fn execute_contract_rejects_missing_contract() {
        let tmp = tempdir().expect("tmp");
        let root = tmp.path();
        let err = execute_contract(root, "V6-TEST-404.1").expect_err("missing");
        assert_eq!(err, "contract_not_found");
    }

    #[test]
    fn parse_id_list_supports_csv_and_file() {
        let tmp = tempdir().expect("tmp");
        let root = tmp.path();
        let ids_file = root.join("ids.txt");
        fs::write(&ids_file, "V6-TEST-100.1\nv6-test-100.2").expect("write ids");

        let argv = vec![
            "run-many".to_string(),
            "--ids=V6-TEST-100.1,V6-TEST-100.3".to_string(),
            format!("--ids-file={}", ids_file.display()),
        ];
        let ids = parse_id_list(root, &argv).expect("ids");
        assert_eq!(
            ids,
            vec![
                "V6-TEST-100.1".to_string(),
                "V6-TEST-100.2".to_string(),
                "V6-TEST-100.3".to_string()
            ]
        );
    }

    #[test]
    fn runtime_lane_targets_map_known_planes() {
        let contract = json!({
            "deliverables": [
                {"type":"runtime_lane","path":"core/layer0/ops/src/canyon_plane.rs"},
                {"type":"runtime_lane","path":"core/layer0/ops/src/skills_plane.rs"},
                {"type":"core_authority","path":"core/layer0/ops/src/security_plane.rs"},
                {"type":"runtime_lane","path":"core/layer0/ops/src/unknown_plane.rs"},
                {"type":"runtime_lane","path":"core/layer0/ops/src/canyon_plane.rs"}
            ]
        });
        let targets = runtime_lane_targets(&contract);
        assert_eq!(targets.len(), 3);
        assert!(targets.iter().any(|row| row.plane == "canyon-plane"));
        assert!(targets.iter().any(|row| row.plane == "skills-plane"));
        assert!(targets.iter().any(|row| row.plane == "security-plane"));
    }

    #[test]
    fn runtime_lane_targets_honor_action_and_argv_overrides() {
        let contract = json!({
            "deliverables": [
                {"type":"runtime_lane","path":"core/layer0/ops/src/canyon_plane.rs","action":"status"},
                {"type":"runtime_lane","path":"core/layer0/ops/src/skills_plane.rs","argv":["skills-plane","run","--skill=compat_skill"]},
                {"type":"runtime_lane","path":"core/layer0/ops/src/skills_plane.rs","argv":["skills-plane","run","--skill=compat_skill"]}
            ]
        });
        let targets = runtime_lane_targets(&contract);
        assert_eq!(targets.len(), 2);
        assert!(targets
            .iter()
            .any(|row| row.argv == vec!["canyon-plane".to_string(), "status".to_string()]));
        assert!(targets.iter().any(|row| row.argv
            == vec![
                "skills-plane".to_string(),
                "run".to_string(),
                "--skill=compat_skill".to_string()
            ]));
    }

    #[test]
    #[cfg(unix)]
    fn execute_contract_dispatches_runtime_lanes_when_enabled() {
        let _guard = env_guard();
        let tmp = tempdir().expect("tmp");
        let root = tmp.path();
        let id = "V7-TEST-901.1";
        let cpath = root.join(CONTRACT_ROOT).join(format!("{id}.json"));
        if let Some(parent) = cpath.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(
            &cpath,
            serde_json::to_string_pretty(&json!({
                "id": id,
                "upgrade": "Dispatch Contract",
                "layer_map": "0/1/2",
                "deliverables": [
                    {"type":"runtime_lane","path":"core/layer0/ops/src/canyon_plane.rs"},
                    {"type":"runtime_lane","path":"core/layer0/ops/src/skills_plane.rs"}
                ]
            }))
            .expect("encode"),
        )
        .expect("write contract");

        let dispatch_bin = root.join("mock_dispatch_ok.sh");
        write_dispatch_script(
            &dispatch_bin,
            r#"#!/bin/sh
printf '{"ok":true,"type":"mock_plane_status","plane":"%s"}\n' "$1"
"#,
        );

        std::env::set_var(
            "PROTHEUS_SRS_DISPATCH_BIN",
            dispatch_bin.display().to_string(),
        );
        let receipt = execute_contract_with_options(root, id, true, true).expect("execute");
        std::env::remove_var("PROTHEUS_SRS_DISPATCH_BIN");

        assert_eq!(receipt.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            receipt
                .pointer("/dispatch/target_count")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            receipt.pointer("/dispatch/failed").and_then(Value::as_u64),
            Some(0)
        );
        assert!(
            receipt
                .pointer("/dispatch/results")
                .and_then(Value::as_array)
                .map(|rows| rows
                    .iter()
                    .all(|row| row.get("ok").and_then(Value::as_bool) == Some(true)))
                .unwrap_or(false),
            "dispatch results should all pass"
        );
        assert!(
            receipt
                .get("claim_evidence")
                .and_then(Value::as_array)
                .map(
                    |rows| rows.iter().any(|row| row.get("id").and_then(Value::as_str)
                        == Some("srs_contract_runtime_dispatch"))
                )
                .unwrap_or(false),
            "missing srs_contract_runtime_dispatch claim"
        );
    }

    #[test]
    #[cfg(unix)]
    fn execute_contract_dispatch_strict_fails_when_target_fails() {
        let _guard = env_guard();
        let tmp = tempdir().expect("tmp");
        let root = tmp.path();
        let id = "V7-TEST-901.2";
        let cpath = root.join(CONTRACT_ROOT).join(format!("{id}.json"));
        if let Some(parent) = cpath.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(
            &cpath,
            serde_json::to_string_pretty(&json!({
                "id": id,
                "upgrade": "Dispatch Contract Fail",
                "layer_map": "0/1/2",
                "deliverables": [
                    {"type":"runtime_lane","path":"core/layer0/ops/src/canyon_plane.rs"}
                ]
            }))
            .expect("encode"),
        )
        .expect("write contract");

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
