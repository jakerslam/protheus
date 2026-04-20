
    #[test]
    fn status_bootstraps_default_policy_and_receipts_surface() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_status(tmp.path());
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(out.get("policy").is_some());
        assert!(out
            .get("fetch_provider_catalog")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("provider_catalog")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("default_provider_chain")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("default_fetch_provider_chain")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn providers_surface_returns_ranked_catalog() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_providers(tmp.path());
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("web_conduit_providers")
        );
        let providers = out
            .get("providers")
            .and_then(Value::as_array)
            .expect("provider catalog");
        assert!(providers
            .iter()
            .any(|row| row.get("provider").and_then(Value::as_str) == Some("duckduckgo")));
        assert!(providers.iter().all(|row| row.get("auto_detect_rank").is_some()));
        let fetch_providers = out
            .get("fetch_providers")
            .and_then(Value::as_array)
            .expect("fetch provider catalog");
        assert!(fetch_providers.iter().any(|row| {
            row.get("provider").and_then(Value::as_str) == Some("direct_http")
        }));
    }

    #[test]
    fn sensitive_domain_requires_explicit_human_approval() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(
            tmp.path(),
            &json!({"url": "https://accounts.google.com/login", "human_approved": false}),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.pointer("/policy_decision/reason")
                .and_then(Value::as_str),
            Some("human_approval_required_for_sensitive_domain")
        );
        assert_eq!(
            out.get("approval_required").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.get("tool_execution_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.get("cache_status").and_then(Value::as_str),
            Some("skipped_validation")
        );
        assert_eq!(
            out.get("cache_skip_reason").and_then(Value::as_str),
            Some("approval_required")
        );
        assert_eq!(
            out.pointer("/tool_execution_gate/reason")
                .and_then(Value::as_str),
            Some("policy_denied")
        );
        assert!(out.pointer("/approval/id").is_some());
    }

    #[test]
    fn approved_token_allows_sensitive_domain_policy_gate() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let first = api_fetch(
            tmp.path(),
            &json!({"url": "https://accounts.google.com/login", "human_approved": false}),
        );
        let approval_id = first
            .pointer("/approval/id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(!approval_id.is_empty());

        let mut approvals = load_approvals(tmp.path());
        if let Some(row) = approvals.iter_mut().find(|row| {
            clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 160) == approval_id
        }) {
            row["status"] = json!("approved");
            row["updated_at"] = json!(crate::now_iso());
        }
        save_approvals(tmp.path(), &approvals).expect("save approvals");

        let second = api_fetch(
            tmp.path(),
            &json!({
                "url": "https://accounts.google.com/login",
                "approval_id": approval_id,
                "summary_only": true
            }),
        );
        assert_eq!(
            second
                .pointer("/policy_decision/allow")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn fetch_example_com_and_summarize_smoke() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(
            tmp.path(),
            &json!({"url": "https://example.com", "summary_only": true}),
        );
        assert!(out.get("receipt").is_some());
        assert_eq!(
            out.get("provider").and_then(Value::as_str),
            Some("direct_http")
        );
        assert!(out
            .get("provider_chain")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            assert!(out
                .get("summary")
                .and_then(Value::as_str)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false));
        } else {
            assert!(out.get("error").is_some());
        }
    }
