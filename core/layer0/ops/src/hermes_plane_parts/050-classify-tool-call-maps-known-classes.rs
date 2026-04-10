
#[cfg(test)]
mod tests {
    use super::*;

    fn cockpit_parsed() -> crate::ParsedArgs {
        crate::parse_args(&["cockpit".to_string(), "--max-blocks=8".to_string()])
    }

    fn lane_block<'a>(out: &'a Value, lane: &str) -> &'a Value {
        out["cockpit"]["render"]["stream_blocks"]
            .as_array()
            .expect("stream blocks")
            .iter()
            .find(|entry| entry.get("lane").and_then(Value::as_str) == Some(lane))
            .expect("lane block should be present")
    }

    #[test]
    fn classify_tool_call_maps_known_classes() {
        assert_eq!(classify_tool_call("skills_plane_run"), "skills");
        assert_eq!(classify_tool_call("binary_vuln_plane_scan"), "security");
        assert_eq!(
            classify_tool_call("vbrowser_plane_session_start"),
            "browser"
        );
    }

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["discover".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "discover");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn continuity_snapshot_paths_are_stable() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = continuity_snapshot_path(root.path(), "session-a");
        assert!(path.to_string_lossy().contains("session-a"));
    }

    #[test]
    fn cockpit_latest_reader_accepts_status_and_event_type_fallbacks() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_dir = root
            .path()
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("benchmark_sanity");
        let latest = lane_dir.join("latest.json");
        let _ = write_json(
            &latest,
            &json!({
                "status": "ok",
                "event_type": "benchmark_sanity_gate",
                "generated_at": "2026-03-22T00:00:00.000Z"
            }),
        );

        let rows = collect_recent_ops_latest(root.path(), 16);
        let row = rows
            .iter()
            .find(|entry| entry.get("lane").and_then(Value::as_str) == Some("benchmark_sanity"))
            .expect("benchmark_sanity row should be present");

        assert_eq!(row.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            row.get("type").and_then(Value::as_str),
            Some("benchmark_sanity_gate")
        );
        assert_eq!(
            row.get("ts").and_then(Value::as_str),
            Some("2026-03-22T00:00:00.000Z")
        );
    }

    #[test]
    fn cockpit_marks_conduit_enforced_from_lane_payload() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_dir = root
            .path()
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("alpha_lane");
        let latest = lane_dir.join("latest.json");
        let _ = write_json(
            &latest,
            &json!({
                "ok": true,
                "type": "alpha_task",
                "ts": "2026-03-22T00:00:00.000Z",
                "conduit_enforcement": {
                    "ok": true,
                    "type": "alpha_conduit_enforcement"
                }
            }),
        );

        let parsed = cockpit_parsed();
        let out = run_cockpit(root.path(), &parsed, true);
        let row = lane_block(&out, "alpha_lane");
        assert_eq!(
            row.get("conduit_enforced").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn cockpit_duration_tracks_timestamp_age() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_dir = root
            .path()
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("age_lane");
        let latest = lane_dir.join("latest.json");
        let _ = write_json(
            &latest,
            &json!({
                "ok": true,
                "type": "age_task",
                "ts": "2000-01-01T00:00:00.000Z"
            }),
        );

        let parsed = cockpit_parsed();
        let out = run_cockpit(root.path(), &parsed, true);
        let row = lane_block(&out, "age_lane");
        assert!(
            row.get("duration_ms").and_then(Value::as_u64).unwrap_or(0) > 1_000,
            "duration_ms should reflect parsed timestamp age"
        );
    }

    #[test]
    fn cockpit_duration_falls_back_to_latest_mtime_when_ts_missing() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_dir = root
            .path()
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("mtime_lane");
        let latest = lane_dir.join("latest.json");
        let _ = write_json(
            &latest,
            &json!({
                "ok": true,
                "type": "mtime_task"
            }),
        );

        let parsed = cockpit_parsed();
        let out = run_cockpit(root.path(), &parsed, true);
        let row = lane_block(&out, "mtime_lane");
        assert_eq!(
            row.get("duration_source").and_then(Value::as_str),
            Some("latest_mtime")
        );
    }

    #[test]
    fn cockpit_metrics_include_active_and_stale_block_counts() {
        let root = tempfile::tempdir().expect("tempdir");
        let stale_lane_dir = root
            .path()
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("stale_lane");
        let fresh_lane_dir = root
            .path()
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("fresh_lane");
        let _ = write_json(
            &stale_lane_dir.join("latest.json"),
            &json!({
                "ok": true,
                "type": "stale_task",
                "ts": "2000-01-01T00:00:00.000Z"
            }),
        );
        let _ = write_json(
            &fresh_lane_dir.join("latest.json"),
            &json!({
                "ok": true,
                "type": "fresh_task",
                "ts": crate::now_iso()
            }),
        );

        let parsed = cockpit_parsed();
        let out = run_cockpit(root.path(), &parsed, true);
        let metrics = out["cockpit"]["metrics"].clone();
        let stale_count = metrics
            .get("stale_block_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let reclaimed_count = metrics
            .get("stale_reclaimed_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        assert!(
            stale_count + reclaimed_count >= 1
        );
        assert!(
            metrics
                .get("active_block_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1
        );
    }
}
