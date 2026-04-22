    #[test]
    fn dashboard_agent_task_history_supports_favorites_search_and_sort_contract() {
        let root = tempfile::tempdir().expect("tempdir");
        let tasks_path = root
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/comms_tasks.json");
        if let Some(parent) = tasks_path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir tasks");
        }
        std::fs::write(
            &tasks_path,
            serde_json::to_string_pretty(&json!({
                "tasks": [
                    {
                        "id": "task-alpha-small",
                        "title": "Alpha Small",
                        "description": "alpha low token",
                        "status": "running",
                        "is_favorited": true,
                        "ts": 100,
                        "tokens_in": 10,
                        "tokens_out": 5
                    },
                    {
                        "id": "task-alpha-big",
                        "title": "Alpha Big",
                        "description": "alpha high token",
                        "status": "running",
                        "is_favorited": true,
                        "ts": 200,
                        "tokens_in": 90,
                        "tokens_out": 45
                    },
                    {
                        "id": "task-beta",
                        "title": "Beta",
                        "description": "non matching",
                        "status": "completed",
                        "is_favorited": false,
                        "ts": 300,
                        "tokens_in": 2,
                        "tokens_out": 1
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let lane = run_action(
            root.path(),
            "dashboard.agent.task.history",
            &json!({
                "favorites_only": true,
                "search_query": "alpha",
                "sort_by": "mostTokens",
                "limit": 10
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("dashboard_agent_task_history")
        );
        assert_eq!(payload.get("total_count").and_then(Value::as_i64), Some(2));
        assert_eq!(
            payload
                .pointer("/rows/0/id")
                .and_then(Value::as_str),
            Some("task-alpha-big")
        );
        assert_eq!(
            payload
                .pointer("/filters/favorites_only")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/filters/sort_by")
                .and_then(Value::as_str),
            Some("mosttokens")
        );
    }

    #[test]
    fn dashboard_agent_task_feedback_and_ui_controller_routes_persist_state_contract() {
        let root = tempfile::tempdir().expect("tempdir");
        let tasks_path = root
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/comms_tasks.json");
        if let Some(parent) = tasks_path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir tasks");
        }
        std::fs::write(
            &tasks_path,
            serde_json::to_string_pretty(&json!({
                "tasks": [
                    {
                        "id": "task-live",
                        "title": "Live Task",
                        "description": "feedback target",
                        "status": "running",
                        "is_favorited": false
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let favorite = run_action(
            root.path(),
            "dashboard.agent.task.favorite",
            &json!({
                "task_id": "task-live",
                "is_favorited": true
            }),
        );
        assert!(favorite.ok);

        let feedback = run_action(
            root.path(),
            "dashboard.agent.task.feedback",
            &json!({
                "task_id": "task-live",
                "value": "thumbs_up"
            }),
        );
        assert!(feedback.ok);

        let history = run_action(
            root.path(),
            "dashboard.agent.task.history",
            &json!({
                "search_query": "live",
                "limit": 5
            }),
        );
        assert!(history.ok);
        let history_payload = history.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            history_payload
                .pointer("/rows/0/is_favorited")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            history_payload
                .pointer("/rows/0/feedback")
                .and_then(Value::as_str),
            Some("thumbs_up")
        );

        let init = run_action(root.path(), "dashboard.ui.initializeWebview", &json!({}));
        assert!(init.ok);
        let mode = run_action(
            root.path(),
            "dashboard.ui.setTerminalExecutionMode",
            &json!({"value": true}),
        );
        assert!(mode.ok);
        let add_to_input = run_action(
            root.path(),
            "dashboard.ui.subscribeToAddToInput",
            &json!({"text": "hello"}),
        );
        assert!(add_to_input.ok);
        let chat = run_action(root.path(), "dashboard.ui.subscribeToChatButtonClicked", &json!({}));
        assert!(chat.ok);
        let history_btn = run_action(
            root.path(),
            "dashboard.ui.subscribeToHistoryButtonClicked",
            &json!({}),
        );
        assert!(history_btn.ok);

        let ui_state_path = root
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/ui_controller_state.json");
        let ui_state = serde_json::from_str::<Value>(
            &std::fs::read_to_string(ui_state_path).expect("read ui state"),
        )
        .expect("parse ui state");
        assert_eq!(
            ui_state
                .get("terminal_execution_mode")
                .and_then(Value::as_str),
            Some("backgroundExec")
        );
        assert_eq!(
            ui_state
                .pointer("/subscriptions/add_to_input/count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            ui_state
                .pointer("/subscriptions/chat_button_clicked/count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            ui_state
                .pointer("/subscriptions/history_button_clicked/count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            ui_state.get("source").and_then(Value::as_str),
            Some("dashboard.ui.controller")
        );
        assert!(
            ui_state
                .get("source_sequence")
                .and_then(Value::as_str)
                .unwrap_or("")
                .len()
                > 8
        );
        assert_eq!(ui_state.get("stale").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn dashboard_ui_controller_additional_codex_subscription_routes_persist_state_contract() {
        let root = tempfile::tempdir().expect("tempdir");

        let webview = run_action(root.path(), "dashboard.ui.getWebviewHtml", &json!({}));
        assert!(webview.ok);
        assert_eq!(
            webview
                .payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_ui_get_webview_html")
        );

        assert!(run_action(
            root.path(),
            "dashboard.ui.onDidShowAnnouncement",
            &json!({"announcement_id":"announce-1"}),
        )
        .ok);
        assert!(run_action(
            root.path(),
            "dashboard.ui.openUrl",
            &json!({"url":"https://example.com"}),
        )
        .ok);
        assert!(run_action(root.path(), "dashboard.ui.openWalkthrough", &json!({})).ok);
        assert!(run_action(
            root.path(),
            "dashboard.ui.scrollToSettings",
            &json!({"value":"models"}),
        )
        .ok);
        assert!(run_action(
            root.path(),
            "dashboard.ui.subscribeToAccountButtonClicked",
            &json!({}),
        )
        .ok);
        assert!(run_action(
            root.path(),
            "dashboard.ui.subscribeToMcpButtonClicked",
            &json!({}),
        )
        .ok);
        assert!(run_action(
            root.path(),
            "dashboard.ui.subscribeToPartialMessage",
            &json!({"value":"partial-response"}),
        )
        .ok);
        assert!(run_action(
            root.path(),
            "dashboard.ui.subscribeToRelinquishControl",
            &json!({}),
        )
        .ok);
        assert!(run_action(
            root.path(),
            "dashboard.ui.subscribeToSettingsButtonClicked",
            &json!({}),
        )
        .ok);

        let ui_state_path = root
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/ui_controller_state.json");
        let ui_state = serde_json::from_str::<Value>(
            &std::fs::read_to_string(ui_state_path).expect("read ui state"),
        )
        .expect("parse ui state");
        assert_eq!(
            ui_state
                .get("last_shown_announcement_id")
                .and_then(Value::as_str),
            Some("announce-1")
        );
        assert_eq!(
            ui_state.get("announcement_should_show").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            ui_state.get("last_open_url").and_then(Value::as_str),
            Some("https://example.com")
        );
        assert_eq!(
            ui_state
                .get("last_scroll_to_settings")
                .and_then(Value::as_str),
            Some("models")
        );
        assert_eq!(
            ui_state
                .pointer("/subscriptions/account_button_clicked/count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            ui_state
                .pointer("/subscriptions/mcp_button_clicked/count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            ui_state
                .pointer("/subscriptions/partial_message/count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            ui_state
                .pointer("/subscriptions/relinquish_control/count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            ui_state
                .pointer("/subscriptions/settings_button_clicked/count")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn dashboard_ui_web_and_worktree_controller_routes_persist_contract_state() {
        let root = tempfile::tempdir().expect("tempdir");

        let show_webview = run_action(
            root.path(),
            "dashboard.ui.subscribeToShowWebview",
            &json!({"preserve_editor_focus": true}),
        );
        assert!(show_webview.ok);
        let worktrees_button = run_action(
            root.path(),
            "dashboard.ui.subscribeToWorktreesButtonClicked",
            &json!({}),
        );
        assert!(worktrees_button.ok);

        let img = run_action(
            root.path(),
            "dashboard.web.checkIsImageUrl",
            &json!({"value":"https://example.com/logo.png"}),
        );
        assert!(img.ok);
        assert_eq!(
            img.payload
                .unwrap_or_else(|| json!({}))
                .get("is_image")
                .and_then(Value::as_bool),
            Some(true)
        );

        let og = run_action(
            root.path(),
            "dashboard.web.fetchOpenGraphData",
            &json!({"url":"https://example.com/docs/intro"}),
        );
        assert!(og.ok);
        assert_eq!(
            og.payload
                .unwrap_or_else(|| json!({}))
                .get("domain")
                .and_then(Value::as_str),
            Some("example.com")
        );

        let open = run_action(
            root.path(),
            "dashboard.web.openInBrowser",
            &json!({"url":"https://example.com"}),
        );
        assert!(open.ok);

        let create = run_action(
            root.path(),
            "dashboard.worktree.createWorktree",
            &json!({"path":"/tmp/worktree-a","branch":"feature/a"}),
        );
        assert!(create.ok);
        let list = run_action(root.path(), "dashboard.worktree.listWorktrees", &json!({}));
        assert!(list.ok);
        assert_eq!(
            list.payload
                .clone()
                .unwrap_or_else(|| json!({}))
                .pointer("/worktrees/0/path")
                .and_then(Value::as_str),
            Some("/tmp/worktree-a")
        );

        let branches = run_action(
            root.path(),
            "dashboard.worktree.getAvailableBranches",
            &json!({}),
        );
        assert!(branches.ok);
        let branches_payload = branches.payload.unwrap_or_else(|| json!({}));
        let local = branches_payload
            .get("local_branches")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(local.iter().any(|row| row.as_str() == Some("feature/a")));

        let switch = run_action(
            root.path(),
            "dashboard.worktree.switchWorktree",
            &json!({"path":"/tmp/worktree-a","new_window":false}),
        );
        assert!(switch.ok);
        let delete = run_action(
            root.path(),
            "dashboard.worktree.deleteWorktree",
            &json!({"path":"/tmp/worktree-a"}),
        );
        assert!(delete.ok);
        let list_after_delete = run_action(root.path(), "dashboard.worktree.listWorktrees", &json!({}));
        assert!(list_after_delete.ok);
        assert_eq!(
            list_after_delete
                .payload
                .unwrap_or_else(|| json!({}))
                .get("worktrees")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(0)
        );

        let ui_state_path = root
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/ui_controller_state.json");
        let ui_state = serde_json::from_str::<Value>(
            &std::fs::read_to_string(ui_state_path).expect("read ui state"),
        )
        .expect("parse ui state");
        assert_eq!(
            ui_state
                .pointer("/subscriptions/show_webview/count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            ui_state
                .pointer("/subscriptions/worktrees_button_clicked/count")
                .and_then(Value::as_i64),
            Some(1)
        );
    }
