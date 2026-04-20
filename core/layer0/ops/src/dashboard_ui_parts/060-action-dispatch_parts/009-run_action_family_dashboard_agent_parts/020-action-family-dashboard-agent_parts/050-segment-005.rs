            let (shared_fields, changed_fields) = dashboard_agent_task_shared_and_changed(&before, &after);
            let changed_count = changed_fields
                .as_array()
                .map(|rows| rows.len() as i64)
                .unwrap_or(0);
            LaneResult {
                ok: true,
                status: 0,
                argv: vec!["dashboard.agent.task.explainChangesShared".to_string()],
                payload: Some(json!({
                    "ok": true,
                    "type": "dashboard_agent_task_explain_changes_shared",
                    "shared_fields": shared_fields,
                    "changed_fields": changed_fields,
                    "changed_count": changed_count
                })),
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
