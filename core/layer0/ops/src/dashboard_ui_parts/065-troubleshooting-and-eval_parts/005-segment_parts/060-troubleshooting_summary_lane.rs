fn dashboard_troubleshooting_summary_lane(root: &Path, payload: &Value) -> LaneResult {
    include!("060-troubleshooting_summary_lane_parts/010-summary-filtering-and-histograms.rs");
    include!("060-troubleshooting_summary_lane_parts/020-summary-queue-health-and-pressure-contracts.rs");
    include!("060-troubleshooting_summary_lane_parts/030-summary-lane-health-and-response-payload.rs");
}
