            let (shared_fields, changed_fields) = dashboard_agent_task_shared_and_changed(&before, &after);
            let changed_count = changed_fields
                .as_array()
                .map(|rows| rows.len() as i64)
                .unwrap_or(0);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(json!({
                    "ok": true,
                    "type": if normalized == "dashboard.agent.task.taskCompletionViewChanges" {
                        "dashboard_agent_task_completion_view_changes"
                    } else {
                        "dashboard_agent_task_explain_changes_shared"
                    },
                    "shared_fields": shared_fields,
                    "changed_fields": changed_fields,
                    "changed_count": changed_count,
                    "view_changes": changed_fields
                })),
            }
        }
        "dashboard.agent.task.favorite" | "dashboard.agent.task.toggleFavorite" => {
            let task_id = payload
                .get("task_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("taskId").and_then(Value::as_str))
                .or_else(|| payload.get("id").and_then(Value::as_str))
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            let is_favorited = payload
                .get("is_favorited")
                .and_then(Value::as_bool)
                .or_else(|| payload.get("isFavorited").and_then(Value::as_bool))
                .or_else(|| payload.get("favorite").and_then(Value::as_bool))
                .unwrap_or(true);
            let result = dashboard_agent_task_apply_favorite(root, &task_id, is_favorited);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.agent.task.feedback" => {
            let task_id = payload
                .get("task_id")
                .and_then(Value::as_str)
                .or_else(|| payload.get("taskId").and_then(Value::as_str))
                .or_else(|| payload.get("id").and_then(Value::as_str))
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            let feedback = payload
                .get("feedback")
                .and_then(Value::as_str)
                .or_else(|| payload.get("value").and_then(Value::as_str))
                .or_else(|| payload.get("type").and_then(Value::as_str))
                .map(|v| clean_text(v, 64))
                .unwrap_or_default();
            let result = dashboard_agent_task_apply_feedback(root, &task_id, &feedback);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.initializeWebview" => {
            let result = dashboard_ui_controller_initialize(root, payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.initializeWebview".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.setTerminalExecutionMode" => {
            let result = dashboard_ui_controller_set_terminal_execution_mode(root, payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.setTerminalExecutionMode".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToAddToInput" | "dashboard.ui.event.addToInput" => {
            let result = dashboard_ui_controller_record_subscription(root, "add_to_input", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToChatButtonClicked" | "dashboard.ui.event.chatButtonClicked" => {
            let result =
                dashboard_ui_controller_record_subscription(root, "chat_button_clicked", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToHistoryButtonClicked"
        | "dashboard.ui.event.historyButtonClicked" => {
            let result = dashboard_ui_controller_record_subscription(
                root,
                "history_button_clicked",
                payload,
            );
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.getWebviewHtml" => {
            let result = dashboard_ui_controller_get_webview_html(root);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.getWebviewHtml".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.onDidShowAnnouncement" => {
            let result = dashboard_ui_controller_on_did_show_announcement(root, payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.onDidShowAnnouncement".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.openUrl" => {
            let result = dashboard_ui_controller_open_url(root, payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.openUrl".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.openWalkthrough" => {
            let result = dashboard_ui_controller_open_walkthrough(root);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.openWalkthrough".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.scrollToSettings" => {
            let result = dashboard_ui_controller_scroll_to_settings(root, payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.ui.scrollToSettings".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToAccountButtonClicked"
        | "dashboard.ui.event.accountButtonClicked" => {
            let result =
                dashboard_ui_controller_record_subscription(root, "account_button_clicked", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToMcpButtonClicked" | "dashboard.ui.event.mcpButtonClicked" => {
            let result =
                dashboard_ui_controller_record_subscription(root, "mcp_button_clicked", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToPartialMessage" | "dashboard.ui.event.partialMessage" => {
            let result =
                dashboard_ui_controller_record_subscription(root, "partial_message", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToRelinquishControl"
        | "dashboard.ui.event.relinquishControl" => {
            let result =
                dashboard_ui_controller_record_subscription(root, "relinquish_control", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToSettingsButtonClicked"
        | "dashboard.ui.event.settingsButtonClicked" => {
            let result = dashboard_ui_controller_record_subscription(
                root,
                "settings_button_clicked",
                payload,
            );
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToShowWebview" | "dashboard.ui.event.showWebview" => {
            let result = dashboard_ui_controller_record_subscription(root, "show_webview", payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.ui.subscribeToWorktreesButtonClicked"
        | "dashboard.ui.event.worktreesButtonClicked" => {
            let result = dashboard_ui_controller_record_subscription(
                root,
                "worktrees_button_clicked",
                payload,
            );
            LaneResult {
                ok: true,
                status: 0,
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.web.checkIsImageUrl" => {
            let result = dashboard_web_check_is_image_url(payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.web.checkIsImageUrl".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.web.fetchOpenGraphData" => {
            let result = dashboard_web_fetch_open_graph_data(payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.web.fetchOpenGraphData".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.web.openInBrowser" => {
            let result = dashboard_web_open_in_browser(root, payload);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.web.openInBrowser".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.listWorktrees" => {
            let result = dashboard_worktree_list(root);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.worktree.listWorktrees".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.getAvailableBranches" => {
            let result = dashboard_worktree_get_available_branches(root);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.worktree.getAvailableBranches".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.createWorktree" => {
            let result = dashboard_worktree_create(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.createWorktree".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.deleteWorktree" => {
            let result = dashboard_worktree_delete(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.deleteWorktree".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.switchWorktree" => {
            let result = dashboard_worktree_switch(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.switchWorktree".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.checkoutBranch" => {
            let result = dashboard_worktree_checkout_branch(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.checkoutBranch".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.createWorktreeInclude" => {
            let result = dashboard_worktree_create_include(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.createWorktreeInclude".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.getWorktreeDefaults" => {
            let result = dashboard_worktree_get_defaults(root);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.worktree.getWorktreeDefaults".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.getWorktreeIncludeStatus" => {
            let result = dashboard_worktree_get_include_status(root);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.worktree.getWorktreeIncludeStatus".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.mergeWorktree" => {
            let result = dashboard_worktree_merge(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.mergeWorktree".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.worktree.trackWorktreeViewOpened" => {
            let result = dashboard_worktree_track_view_opened(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.worktree.trackWorktreeViewOpened".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.hooks.registry.register"
        | "dashboard.hooks.registry.list"
        | "dashboard.hooks.discoveryCache.get"
        | "dashboard.hooks.discoveryCache.refresh"
        | "dashboard.hooks.process.start"
        | "dashboard.hooks.process.complete"
        | "dashboard.hooks.process.registry" => {
            let result = dashboard_hook_route(root, normalized, payload);
            let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
            LaneResult {
                ok,
                status: if ok { 0 } else { 2 },
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        "dashboard.hooks.test.setupFixture"
        | "dashboard.hooks.test.factory.validate"
        | "dashboard.hooks.test.modelContext.build"
        | "dashboard.hooks.test.process.simulate"
        | "dashboard.hooks.test.utils.normalize"
        | "dashboard.hooks.test.notification.emit"
        | "dashboard.hooks.test.shellEscape.inspect"
        | "dashboard.hooks.test.taskCancel.simulate"
        | "dashboard.hooks.test.taskComplete.simulate"
        | "dashboard.hooks.test.taskResume.simulate"
        | "dashboard.hooks.test.taskStart.simulate"
        | "dashboard.hooks.test.userPromptSubmit.simulate"
        | "dashboard.hooks.test.precompact.evaluate"
        | "dashboard.hooks.test.templates.render"
        | "dashboard.hooks.test.templates.placeholders"
        | "dashboard.hooks.test.utils.digest"
        | "dashboard.hooks.test.ignore.evaluate" => {
            let result = dashboard_hook_test_route(root, normalized, payload);
            let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
            LaneResult {
                ok,
                status: if ok { 0 } else { 2 },
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        _ if dashboard_prompt_route_supported(normalized) => {
            let result = dashboard_lock_permission_prompt_route(root, normalized, payload);
            let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(false);
            LaneResult {
                ok,
                status: if ok { 0 } else { 2 },
                argv: vec![normalized.to_string()],
                payload: Some(result),
            }
        }
        _ => LaneResult {
            ok: false,
            status: 2,
            argv: Vec::new(),
            payload: Some(json!({
                "ok": false,
                "type": "infring_dashboard_action_error",
                "error": format!("unsupported_action:{normalized}")
            })),
        },
    }
}
