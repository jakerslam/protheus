
    #[test]
    fn agents_routes_create_message_config_and_git_tree_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        init_git_repo(root.path());
        let created = handle(
            root.path(),
            "POST",
            "/api/agents",
            br#"{"name":"Jarvis","role":"director","provider":"ollama","model":"qwen:4b"}"#,
            &json!({"ok": true}),
        )
        .expect("create agent");
        assert_eq!(created.status, 200);
        assert_eq!(
            created.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        let agent_id = clean_text(
            created
                .payload
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!agent_id.is_empty());

        let listed = handle(root.path(), "GET", "/api/agents", &[], &json!({"ok": true}))
            .expect("list agents");
        let rows = listed.payload.as_array().cloned().unwrap_or_default();
        assert!(rows.iter().any(|row| {
            clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 180) == agent_id
        }));

        let details = handle(
            root.path(),
            "GET",
            &format!("/api/agents/{agent_id}"),
            &[],
            &json!({"ok": true}),
        )
        .expect("agent details");
        assert_eq!(details.status, 200);
        assert_eq!(
            details.payload.get("name").and_then(Value::as_str),
            Some("Jarvis")
        );

        let message = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"hello there"}"#,
            &json!({"ok": true}),
        )
        .expect("agent message");
        assert_eq!(message.status, 200);
        assert_eq!(
            message.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        assert!(message
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("hello there"));

        let new_session = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/sessions"),
            br#"{"label":"Ops"}"#,
            &json!({"ok": true}),
        )
        .expect("create session");
        let sid = clean_text(
            new_session
                .payload
                .get("active_session_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!sid.is_empty());
        let switched = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/sessions/{sid}/switch"),
            &[],
            &json!({"ok": true}),
        )
        .expect("switch session");
        assert_eq!(
            switched.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            switched
                .payload
                .get("active_session_id")
                .and_then(Value::as_str),
            Some(sid.as_str())
        );
        let cross_session = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"What did I say earlier?"}"#,
            &json!({"ok": true}),
        )
        .expect("cross session message");
        assert_eq!(cross_session.status, 200);
        assert!(
            cross_session
                .payload
                .pointer("/context_pool/pool_messages")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 2
        );
        assert_eq!(
            cross_session
                .payload
                .pointer("/context_pool/cross_session_memory_enabled")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            cross_session
                .payload
                .get("response")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("Persistent memory is enabled"),
            "cross-session recall should be remediated to persistent memory summary"
        );

        let configured = handle(
            root.path(),
            "PATCH",
            &format!("/api/agents/{agent_id}/config"),
            br#"{
              "mode":"focus",
              "git_branch":"feature/jarvis",
              "identity":{"emoji":"robot","color":"00ff00","archetype":"director","vibe":"direct"}
            }"#,
            &json!({"ok": true}),
        )
        .expect("config");
        assert_eq!(
            configured.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );

        let model = handle(
            root.path(),
            "PUT",
            &format!("/api/agents/{agent_id}/model"),
            br#"{"model":"openai/gpt-5"}"#,
            &json!({"ok": true}),
        )
        .expect("set model");
        assert_eq!(
            model.payload.get("provider").and_then(Value::as_str),
            Some("openai")
        );
        assert_eq!(
            model.payload.get("model").and_then(Value::as_str),
            Some("gpt-5")
        );

        let after_model = handle(
            root.path(),
            "GET",
            &format!("/api/agents/{agent_id}"),
            &[],
            &json!({"ok": true}),
        )
        .expect("agent after model");
        assert_eq!(
            after_model
                .payload
                .get("model_provider")
                .and_then(Value::as_str),
            Some("openai")
        );
        assert_eq!(
            after_model
                .payload
                .get("model_name")
                .and_then(Value::as_str),
            Some("gpt-5")
        );
        assert_eq!(
            after_model
                .payload
                .pointer("/identity/vibe")
                .and_then(Value::as_str),
            Some("direct")
        );

        let trees = handle(
            root.path(),
            "GET",
            &format!("/api/agents/{agent_id}/git-trees"),
            &[],
            &json!({"ok": true}),
        )
        .expect("git trees");
        let options = trees
            .payload
            .get("options")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(options.iter().any(|row| {
            row.get("branch")
                .and_then(Value::as_str)
                .map(|v| v == "main")
                .unwrap_or(false)
        }));
        let switched_tree = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/git-tree/switch"),
            br#"{"branch":"feature/jarvis"}"#,
            &json!({"ok": true}),
        )
        .expect("git tree switch");
        assert_eq!(
            switched_tree
                .payload
                .pointer("/current/git_branch")
                .and_then(Value::as_str),
            Some("feature/jarvis")
        );
    }

    #[test]
    fn agent_message_runtime_probe_uses_authoritative_runtime_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        init_git_repo(root.path());
        let created = handle(
            root.path(),
            "POST",
            "/api/agents",
            br#"{"name":"Runtime Probe","role":"analyst"}"#,
            &json!({"ok": true}),
        )
        .expect("create agent");
        let agent_id = clean_text(
            created
                .payload
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!agent_id.is_empty());

        let message = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"Report runtime sync now. What changed in queue depth, cockpit blocks, conduit signals, and memory context?"}"#,
            &json!({"ok": true}),
        )
        .expect("agent runtime probe");
        assert_eq!(message.status, 200);
        assert_eq!(
            message.payload.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        let response = message
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(response.contains("Current queue depth:"));
        assert!(response.contains("Runtime memory context"));
        assert!(message
            .payload
            .get("runtime_sync")
            .and_then(Value::as_object)
            .is_some());
    }

    #[test]
    fn memory_denial_variant_is_remediated_to_persistent_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        init_git_repo(root.path());
        let created = handle(
            root.path(),
            "POST",
            "/api/agents",
            br#"{"name":"Memory Probe","role":"analyst"}"#,
            &json!({"ok": true}),
        )
        .expect("create agent");
        let agent_id = clean_text(
            created
                .payload
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!agent_id.is_empty());

        let seeded = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"Remember this exactly: favorite animal is octopus and codename aurora-7."}"#,
            &json!({"ok": true}),
        )
        .expect("seed memory");
        assert_eq!(seeded.status, 200);

        let second = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/sessions"),
            br#"{"label":"Session 2"}"#,
            &json!({"ok": true}),
        )
        .expect("create second session");
        let sid = clean_text(
            second
                .payload
                .get("active_session_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            180,
        );
        assert!(!sid.is_empty());
        let switched = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/sessions/{sid}/switch"),
            &[],
            &json!({"ok": true}),
        )
        .expect("switch second session");
        assert_eq!(switched.status, 200);

        let denial_variant = handle(
            root.path(),
            "POST",
            &format!("/api/agents/{agent_id}/message"),
            br#"{"message":"I still do not see any stored memory context from earlier in this session. I do not retain information between exchanges unless you explicitly use a memory conduit, and I can only work with what is in the current message."}"#,
            &json!({"ok": true}),
        )
        .expect("denial variant message");
        assert_eq!(denial_variant.status, 200);
        let response = denial_variant
            .payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            response.contains("Persistent memory is enabled"),
            "memory denial variant should be remediated to persistent memory summary"
        );
        assert!(
            !response
                .to_ascii_lowercase()
                .contains("do not retain information between exchanges"),
            "raw denial text should not leak back to caller"
        );
    }
