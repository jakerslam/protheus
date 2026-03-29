#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_ids() {
        assert_eq!(normalize_app_id("chat_starter"), "chat-starter");
        assert_eq!(normalize_app_id("chatui"), "chat-ui");
        assert_eq!(normalize_app_id("codeengineer"), "code-engineer");
    }

    #[test]
    fn code_engineer_run_creates_scaffold() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_code_engineer(
            root.path(),
            &crate::parse_args(&[
                "run".to_string(),
                "--app=code-engineer".to_string(),
                "--prompt=build an api".to_string(),
                "--strict=1".to_string(),
            ]),
            true,
            "run",
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(code_engineer_runs_path(root.path()).exists());
    }

    #[test]
    fn chat_starter_roundtrip_writes_session() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_chat_starter(
            root.path(),
            &crate::parse_args(&[
                "run".to_string(),
                "--app=chat-starter".to_string(),
                "--session-id=s1".to_string(),
                "--message=hello".to_string(),
            ]),
            true,
            "run",
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(chat_starter_session_path(root.path(), "s1").exists());
    }

    #[test]
    fn code_engineer_build_requires_reasoning_approval_for_high_risk() {
        let root = tempfile::tempdir().expect("tempdir");
        let denied = run_code_engineer(
            root.path(),
            &crate::parse_args(&[
                "build".to_string(),
                "--app=code-engineer".to_string(),
                "--goal=deploy production payment migration".to_string(),
                "--strict=1".to_string(),
            ]),
            true,
            "build",
        );
        assert_eq!(denied.get("ok").and_then(Value::as_bool), Some(false));
        let allowed = run_code_engineer(
            root.path(),
            &crate::parse_args(&[
                "build".to_string(),
                "--app=code-engineer".to_string(),
                "--goal=deploy production payment migration".to_string(),
                "--approved=1".to_string(),
                "--strict=1".to_string(),
            ]),
            true,
            "build",
        );
        assert_eq!(allowed.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn code_engineer_ingress_supports_slack_and_telegram() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_code_engineer(
            root.path(),
            &crate::parse_args(&[
                "ingress".to_string(),
                "--app=code-engineer".to_string(),
                "--provider=slack".to_string(),
                "--goal=build backlog dashboard".to_string(),
                "--strict=1".to_string(),
            ]),
            true,
            "ingress",
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let bad = run_code_engineer(
            root.path(),
            &crate::parse_args(&[
                "ingress".to_string(),
                "--app=code-engineer".to_string(),
                "--provider=discord".to_string(),
                "--goal=build backlog dashboard".to_string(),
                "--strict=1".to_string(),
            ]),
            true,
            "ingress",
        );
        assert_eq!(bad.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn code_engineer_template_governance_installs_builders_templates() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_code_engineer(
            root.path(),
            &crate::parse_args(&[
                "template-governance".to_string(),
                "--app=code-engineer".to_string(),
                "--op=install".to_string(),
                "--template-id=builders://starter/api".to_string(),
                "--strict=1".to_string(),
            ]),
            true,
            "template-governance",
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(code_engineer_templates_path(root.path()).exists());
    }
}
