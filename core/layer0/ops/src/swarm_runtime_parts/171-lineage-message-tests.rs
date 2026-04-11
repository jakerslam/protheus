#[cfg(test)]
mod lineage_message_tests {
    use super::*;
    use std::path::PathBuf;

    fn spawn_options_for_lineage() -> SpawnOptions {
        SpawnOptions {
            verify: true,
            timeout_ms: 100,
            metrics_detailed: true,
            simulate_unreachable: false,
            byzantine: false,
            corruption_type: "data_falsification".to_string(),
            token_budget: None,
            token_warning_threshold: 0.8,
            budget_exhaustion_action: BudgetAction::FailHard,
            adaptive_complexity: false,
            execution_mode: ExecutionMode::TaskOriented,
            role: None,
            capabilities: Vec::new(),
            auto_publish_results: false,
            agent_label: None,
            result_value: None,
            result_text: None,
            result_confidence: 1.0,
            verification_status: "not_verified".to_string(),
        }
    }

    #[test]
    fn sessions_send_allows_parent_and_child_delivery() {
        let mut state = SwarmState::default();
        let parent = spawn_single(&mut state, None, "parent", 8, &spawn_options_for_lineage())
            .expect("parent spawn")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("parent id")
            .to_string();
        let child = spawn_single(
            &mut state,
            Some(&parent),
            "child",
            8,
            &spawn_options_for_lineage(),
        )
        .expect("child spawn")
        .get("session_id")
        .and_then(Value::as_str)
        .expect("child id")
        .to_string();

        let down = send_session_message(
            &mut state,
            &parent,
            &child,
            "directive",
            DeliveryGuarantee::AtMostOnce,
            false,
            DEFAULT_MESSAGE_TTL_MS,
        )
        .expect("parent to child");
        assert_eq!(
            down.get("recipient_session_id").and_then(Value::as_str),
            Some(child.as_str())
        );

        let up = send_session_message(
            &mut state,
            &child,
            &parent,
            "report",
            DeliveryGuarantee::AtMostOnce,
            false,
            DEFAULT_MESSAGE_TTL_MS,
        )
        .expect("child to parent");
        assert_eq!(
            up.get("recipient_session_id").and_then(Value::as_str),
            Some(parent.as_str())
        );
    }

    fn temp_state_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "infring-swarm-lineage-{label}-{}.json",
            now_epoch_ms()
        ))
    }

    #[test]
    fn persisted_state_allows_parent_and_child_delivery() {
        let path = temp_state_path("persisted-parent-child");
        let mut state = SwarmState::default();
        let parent = spawn_single(&mut state, None, "parent", 8, &spawn_options_for_lineage())
            .expect("parent spawn")
            .get("session_id")
            .and_then(Value::as_str)
            .expect("parent id")
            .to_string();
        let child = spawn_single(
            &mut state,
            Some(&parent),
            "child",
            8,
            &spawn_options_for_lineage(),
        )
        .expect("child spawn")
        .get("session_id")
        .and_then(Value::as_str)
        .expect("child id")
        .to_string();

        save_state(&path, &state).expect("save state");
        let mut loaded = load_state(&path).expect("load state");
        let down = send_session_message(
            &mut loaded,
            &parent,
            &child,
            "directive",
            DeliveryGuarantee::AtMostOnce,
            false,
            DEFAULT_MESSAGE_TTL_MS,
        )
        .expect("parent to child after load");
        assert_eq!(
            down.get("recipient_session_id").and_then(Value::as_str),
            Some(child.as_str())
        );

        save_state(&path, &loaded).expect("resave state");
        let mut reloaded = load_state(&path).expect("reload state");
        let up = send_session_message(
            &mut reloaded,
            &child,
            &parent,
            "report",
            DeliveryGuarantee::AtMostOnce,
            false,
            DEFAULT_MESSAGE_TTL_MS,
        )
        .expect("child to parent after reload");
        assert_eq!(
            up.get("recipient_session_id").and_then(Value::as_str),
            Some(parent.as_str())
        );

        let _ = std::fs::remove_file(&path);
    }
}
