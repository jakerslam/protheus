mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_text(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, text).expect("write text");
    }

    fn write_policy(root: &Path) {
        write_text(
            &root.join("client/runtime/config/sdlc_change_control_policy.json"),
            &json!({
                "strict_default": true,
                "required_approvers_major": 1,
                "required_approvers_high_risk": 2,
                "require_rfc_for_major": true,
                "require_adr_for_high_risk": true,
                "require_rollback_drill_for_high_risk": true,
                "require_approval_receipts_for_major": true,
                "high_risk_path_prefixes": ["core/layer0/security/", "client/runtime/systems/security/"],
                "major_path_prefixes": ["core/layer0/ops/", "client/runtime/systems/ops/"],
                "outputs": {
                    "latest_path": "local/state/ops/sdlc_change_control/latest.json",
                    "history_path": "local/state/ops/sdlc_change_control/history.jsonl"
                }
            })
            .to_string(),
        );
    }

    #[test]
    fn high_risk_change_requires_full_approval_bundle() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        write_policy(root);

        write_text(
            &root.join("local/state/ops/sdlc_change_control/pr_body.md"),
            "- Risk class: high-risk\n- Rollback plan: revert and freeze\n- Rollback owner: ops-oncall\n- Approvers: alice\n- Approval receipts: docs/client/approvals/one.md\n- Rollback drill receipt: docs/client/drills/rollback.json\n",
        );
        write_text(
            &root.join("local/state/ops/sdlc_change_control/changed_paths.txt"),
            "core/layer0/security/src/lib.rs\n",
        );

        write_text(&root.join("docs/client/approvals/one.md"), "ok");
        write_text(&root.join("docs/client/drills/rollback.json"), "{}");

        let code = run(
            root,
            &[
                "run".to_string(),
                "--strict=1".to_string(),
                "--pr-body-path=local/state/ops/sdlc_change_control/pr_body.md".to_string(),
                "--changed-paths-path=local/state/ops/sdlc_change_control/changed_paths.txt"
                    .to_string(),
            ],
        );
        assert_eq!(code, 1);

        let latest =
            fs::read_to_string(root.join("local/state/ops/sdlc_change_control/latest.json"))
                .unwrap();
        let payload: Value = serde_json::from_str(&latest).unwrap();
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(false));
        assert!(payload
            .get("blocking_checks")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .any(|v| v.as_str() == Some("adr_link_requirement"))
            })
            .unwrap_or(false));
    }

    #[test]
    fn major_change_passes_with_rfc_and_single_approver() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        write_policy(root);

        write_text(&root.join("docs/client/rfc/RFC-1.md"), "rfc");
        write_text(&root.join("docs/client/approvals/approve-1.md"), "receipt");

        write_text(
            &root.join("local/state/ops/sdlc_change_control/pr_body.md"),
            "- Risk class: major\n- RFC link: docs/client/rfc/RFC-1.md\n- Rollback plan: git revert\n- Rollback owner: platform\n- Approvers: alice\n- Approval receipts: docs/client/approvals/approve-1.md\n",
        );
        write_text(
            &root.join("local/state/ops/sdlc_change_control/changed_paths.txt"),
            "core/layer0/ops/src/main.rs\n",
        );

        let code = run(
            root,
            &[
                "run".to_string(),
                "--strict=1".to_string(),
                "--pr-body-path=local/state/ops/sdlc_change_control/pr_body.md".to_string(),
                "--changed-paths-path=local/state/ops/sdlc_change_control/changed_paths.txt"
                    .to_string(),
            ],
        );
        assert_eq!(code, 0);

        let latest =
            fs::read_to_string(root.join("local/state/ops/sdlc_change_control/latest.json"))
                .unwrap();
        let payload: Value = serde_json::from_str(&latest).unwrap();
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("declared_risk_class").and_then(Value::as_str),
            Some("major")
        );
    }
}
