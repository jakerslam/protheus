fn run_action_family_dashboard_troubleshooting(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    match normalized {
        "dashboard.troubleshooting.state" => dashboard_troubleshooting_state_lane(root, payload),
        "dashboard.troubleshooting.eval.drain" => {
            dashboard_troubleshooting_eval_drain_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.flush" => {
            dashboard_troubleshooting_outbox_flush_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.state" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.health" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.metrics" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.overview" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.queue" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.freshness" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.health.metrics" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.health.summary" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.compaction" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.metrics" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.actions" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.priority" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.lane" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.escalation" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.runbook" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.sla" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.escalation_lane" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.deadline" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.deadline_remaining" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.breach" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.breach_detected" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.reason" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.breach_reason" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.blocking_kind" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.auto_retry_allowed" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.execution_policy" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.manual_gate_required" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.manual_gate_reason" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.requeue_strategy" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.can_execute_without_human" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.execution_window" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.manual_gate_timeout" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.next_action_after_seconds" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.next_action_kind" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.retry_window_class" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.readiness_state" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.readiness_reason" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.automation_safe" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.decision_vector" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.decision_vector_key" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.decision_route_hint" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.decision_urgency_tier" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.decision_retry_budget_class" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.decision_lane_token" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.decision_dispatch_mode" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.decision_manual_ack_required" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.decision_execution_guard" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.decision_followup_required" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.decision_vector_version" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_version" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_family" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_priority" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_action_hint" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_escalation_lane" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_runbook" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_owner" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_blocking_kind" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_auto_retry_allowed" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_execution_policy" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_manual_gate_required" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_manual_gate_reason" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_requeue_strategy" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_can_execute_without_human" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_execution_window" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_manual_gate_timeout" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_next_action_after_seconds" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_next_action_kind" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_retry_window_class" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_readiness_state" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_readiness_reason" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_automation_safe" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_decision_vector" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_decision_vector_key" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_decision_route_hint" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_decision_urgency_tier" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_decision_retry_budget_class" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_decision_lane_token" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_decision_dispatch_mode" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_decision_manual_ack_required" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_decision_execution_guard" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_decision_followup_required" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_decision_vector_version" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_deadline" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_breach_reason" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.contract_object" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.pressure.snapshot" => {
            dashboard_troubleshooting_outbox_state_lane(root, payload)
        }
        "dashboard.troubleshooting.summary" => dashboard_troubleshooting_summary_lane(root, payload),
        "dashboard.troubleshooting.summary.metrics" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.health" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.queue_health" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.health.metrics" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.outbox.health" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.outbox.compaction" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.outbox.pressure" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.queue_pressure" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.actions" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.priority" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.lane" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.escalation" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.runbook" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.sla" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.escalation_lane" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.deadline" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.deadline_remaining" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.breach" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.breach_detected" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.reason" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.breach_reason" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.blocking_kind" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.auto_retry_allowed" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.execution_policy" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.manual_gate_required" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.manual_gate_reason" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.requeue_strategy" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.can_execute_without_human" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.execution_window" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.manual_gate_timeout" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.next_action_after_seconds" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.next_action_kind" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.retry_window_class" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.readiness_state" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.readiness_reason" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.automation_safe" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.decision_vector" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.decision_vector_key" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.decision_route_hint" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.decision_urgency_tier" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.decision_retry_budget_class" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.decision_lane_token" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.decision_dispatch_mode" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.decision_manual_ack_required" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.decision_execution_guard" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.decision_followup_required" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.decision_vector_version" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_version" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_family" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_priority" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_action_hint" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_escalation_lane" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_runbook" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_owner" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_blocking_kind" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_auto_retry_allowed" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_execution_policy" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_manual_gate_required" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_manual_gate_reason" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_requeue_strategy" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_can_execute_without_human" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_execution_window" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_manual_gate_timeout" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_next_action_after_seconds" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_next_action_kind" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_retry_window_class" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_readiness_state" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_readiness_reason" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_automation_safe" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_decision_vector" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_decision_vector_key" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_decision_route_hint" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_decision_urgency_tier" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_decision_retry_budget_class" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_decision_lane_token" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_decision_dispatch_mode" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_decision_manual_ack_required" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_decision_execution_guard" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_decision_followup_required" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_decision_vector_version" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_deadline" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_breach_reason" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.contract_object" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.pressure.snapshot" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.filtered" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.by_error" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.by_classification" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.recent" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.window" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.summary.by_time" => {
            dashboard_troubleshooting_summary_lane(root, payload)
        }
        "dashboard.troubleshooting.overview" => dashboard_troubleshooting_summary_lane(root, payload),
        "dashboard.troubleshooting.outbox.preview" => dashboard_troubleshooting_outbox_flush_lane(
            root,
            &merge_json_objects(payload, &json!({"dry_run": true})),
        ),
        "dashboard.troubleshooting.deadletter.state" => {
            dashboard_troubleshooting_deadletter_state_lane(root, payload)
        }
        "dashboard.troubleshooting.deadletter.inspect" => {
            dashboard_troubleshooting_deadletter_state_lane(root, payload)
        }
        "dashboard.troubleshooting.deadletter.requeue" => {
            dashboard_troubleshooting_deadletter_requeue_lane(root, payload)
        }
        "dashboard.troubleshooting.deadletter.requeue.preview" => {
            dashboard_troubleshooting_deadletter_requeue_lane(
                root,
                &merge_json_objects(payload, &json!({"dry_run": true})),
            )
        }
        "dashboard.troubleshooting.deadletter.purge" => {
            dashboard_troubleshooting_deadletter_purge_lane(root, payload)
        }
        "dashboard.troubleshooting.deadletter.purge.preview" => {
            dashboard_troubleshooting_deadletter_purge_lane(
                root,
                &merge_json_objects(payload, &json!({"dry_run": true})),
            )
        }
        "dashboard.troubleshooting.report_message" => {
            dashboard_troubleshooting_report_message_lane(root, payload)
        }
        _ => run_action_family_dashboard_terminal(root, normalized, payload),
    }
}
