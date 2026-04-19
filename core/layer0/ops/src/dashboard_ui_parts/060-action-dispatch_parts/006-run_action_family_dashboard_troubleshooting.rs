fn run_action_family_dashboard_troubleshooting(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    match normalized {
        "dashboard.troubleshooting.state" => dashboard_troubleshooting_state_lane(root, payload),
        "dashboard.troubleshooting.eval.drain" => {
            dashboard_troubleshooting_eval_drain_lane(root, payload)
        }
        "dashboard.troubleshooting.outbox.flush" => {
            dashboard_troubleshooting_outbox_flush_lane(root, payload)
        }
        "dashboard.troubleshooting.report_message" => {
            dashboard_troubleshooting_report_message_lane(root, payload)
        }
        _ => run_action_family_dashboard_terminal(root, normalized, payload),
    }
}
