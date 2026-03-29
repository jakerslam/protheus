
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::test_env_guard()
    }

    fn temp_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("protheus_seed_protocol_{name}_{nonce}"));
        fs::create_dir_all(&root).expect("mkdir");
        root
    }

    fn allow(root: &Path, directive: &str) {
        std::env::set_var("DIRECTIVE_KERNEL_SIGNING_KEY", "seed-test-sign-key");
        assert_eq!(
            directive_kernel::run(
                root,
                &[
                    "prime-sign".to_string(),
                    format!("--directive={directive}"),
                    "--signer=tester".to_string(),
                ]
            ),
            0
        );
    }

    #[test]
    fn deploy_viral_writes_packet_and_replications() {
        let _guard = env_guard();
        let root = temp_root("deploy_viral");
        allow(&root, "allow:seed:*");
        let exit = run(
            &root,
            &[
                "deploy".to_string(),
                "--profile=viral".to_string(),
                "--targets=node-a,node-b".to_string(),
                "--apply=1".to_string(),
            ],
        );
        assert_eq!(exit, 0);
        let latest = read_json(&latest_path(&root)).expect("latest");
        assert_eq!(latest.get("profile").and_then(Value::as_str), Some("viral"));
        assert!(latest
            .get("packet_path")
            .and_then(Value::as_str)
            .map(|v| !v.is_empty())
            .unwrap_or(false));
        std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn migrate_immortal_selects_low_power_target() {
        let _guard = env_guard();
        let root = temp_root("migrate_immortal");
        allow(&root, "allow:seed:*");
        let exit = run(
            &root,
            &[
                "migrate".to_string(),
                "--profile=immortal".to_string(),
                "--node=node-z".to_string(),
                "--energy=0.12".to_string(),
                "--threat=high".to_string(),
                "--apply=1".to_string(),
            ],
        );
        assert_eq!(exit, 0);
        let latest = read_json(&latest_path(&root)).expect("latest");
        assert_eq!(
            latest
                .get("migration")
                .and_then(|v| v.get("target_class"))
                .and_then(Value::as_str),
            Some("ultra_low_power")
        );
        std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn enforce_denied_quarantines_node() {
        let _guard = env_guard();
        let root = temp_root("enforce_denied");
        let exit = run(
            &root,
            &[
                "enforce".to_string(),
                "--profile=viral".to_string(),
                "--operation=replicate".to_string(),
                "--node=rogue-1".to_string(),
                "--apply=1".to_string(),
            ],
        );
        assert_eq!(exit, 2);
        let latest = read_json(&latest_path(&root)).expect("latest");
        assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(false));
        let state = load_state(&root);
        let q = state
            .get("quarantine")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert!(q.contains_key("rogue-1"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn select_uses_network_balances() {
        let _guard = env_guard();
        let root = temp_root("select");
        allow(&root, "allow:seed:*");
        allow(&root, "allow:tokenomics");
        assert_eq!(
            crate::network_protocol::run(
                &root,
                &[
                    "reward".to_string(),
                    "--action=reward".to_string(),
                    "--agent=node-a".to_string(),
                    "--amount=120".to_string(),
                    "--reason=tokenomics".to_string()
                ]
            ),
            0
        );
        assert_eq!(
            crate::network_protocol::run(
                &root,
                &[
                    "reward".to_string(),
                    "--action=reward".to_string(),
                    "--agent=node-b".to_string(),
                    "--amount=10".to_string(),
                    "--reason=tokenomics".to_string()
                ]
            ),
            0
        );
        let exit = run(
            &root,
            &[
                "select".to_string(),
                "--profile=viral".to_string(),
                "--top=1".to_string(),
                "--apply=1".to_string(),
            ],
        );
        assert_eq!(exit, 0);
        let latest = read_json(&latest_path(&root)).expect("latest");
        let first = latest
            .get("selected")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("node"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert_eq!(first, "node-a");
        std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn archive_updates_merkle_root() {
        let _guard = env_guard();
        let root = temp_root("archive");
        allow(&root, "allow:seed:*");
        assert_eq!(
            run(
                &root,
                &[
                    "archive".to_string(),
                    "--profile=immortal".to_string(),
                    "--lineage-id=lineage-alpha".to_string(),
                    "--apply=1".to_string(),
                ],
            ),
            0
        );
        let state = load_state(&root);
        assert!(state
            .get("archive_merkle_root")
            .and_then(Value::as_str)
            .map(|v| !v.is_empty())
            .unwrap_or(false));
        std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
        let _ = fs::remove_dir_all(root);
    }
}


