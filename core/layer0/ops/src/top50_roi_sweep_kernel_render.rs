// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::fs;
use std::path::Path;

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

pub(super) const HOTPATH_CSV_REL: &str = "docs/client/generated/RUST60_TS_HOTPATHS_RANKED_FULL.csv";
pub(super) const HOTPATH_MD_REL: &str = "docs/client/generated/RUST60_TS_HOTPATHS_RANKED_FULL.md";
pub(super) const QUEUE_JSON_REL: &str = "docs/client/generated/RUST60_EXECUTION_QUEUE_261.json";
pub(super) const QUEUE_MD_REL: &str = "docs/client/generated/RUST60_EXECUTION_QUEUE_261.md";

fn row_u64(row: &Value, key: &str) -> u64 {
    row.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn row_f64(row: &Value, key: &str) -> f64 {
    row.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn row_str<'a>(row: &'a Value, key: &str) -> &'a str {
    row.get(key).and_then(Value::as_str).unwrap_or("")
}

pub(super) fn render_csv(rows: &[Value]) -> String {
    let mut lines = vec![
        "rank,path,loc,weight,impact_score,cumulative_migrated_ts_lines,projected_rust_percent_after_lane".to_string(),
    ];
    for row in rows {
        lines.push(format!(
            "{},{},{},{},{},{},{}",
            row_u64(row, "rank"),
            row_str(row, "path"),
            row_u64(row, "loc"),
            row_f64(row, "weight"),
            row_f64(row, "impact_score"),
            row_u64(row, "cumulative_migrated_ts_lines"),
            row_f64(row, "projected_rust_percent_after_lane")
        ));
    }
    format!("{}\n", lines.join("\n"))
}

pub(super) fn render_md(title: &str, rows: &[Value]) -> String {
    let mut out = vec![
        format!("# {title}"),
        String::new(),
        format!("Generated: {}", now_iso()),
        String::new(),
        "| Rank | Path | LOC | Impact | Cumulative TS Migrated | Projected Rust % |".to_string(),
        "|---:|---|---:|---:|---:|---:|".to_string(),
    ];
    for row in rows {
        out.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            row_u64(row, "rank"),
            row_str(row, "path"),
            row_u64(row, "loc"),
            row_f64(row, "impact_score"),
            row_u64(row, "cumulative_migrated_ts_lines"),
            row_f64(row, "projected_rust_percent_after_lane")
        ));
    }
    out.push(String::new());
    format!("{}\n", out.join("\n"))
}

pub(super) fn write_outputs(root: &Path, queue: &Value) -> Result<(), String> {
    let lanes = queue
        .get("lanes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let hotpaths = lanes
        .iter()
        .map(|lane| {
            json!({
                "rank": row_u64(lane, "rank"),
                "path": row_str(lane, "path"),
                "loc": row_u64(lane, "loc"),
                "impact_score": row_f64(lane, "impact_score"),
                "cumulative_migrated_ts_lines": row_u64(lane, "cumulative_migrated_ts_lines"),
                "projected_rust_percent_after_lane": row_f64(lane, "projected_rust_percent_after_lane"),
            })
        })
        .collect::<Vec<_>>();

    let hotpath_csv = root.join(HOTPATH_CSV_REL);
    let hotpath_md = root.join(HOTPATH_MD_REL);
    let queue_json = root.join(QUEUE_JSON_REL);
    let queue_md = root.join(QUEUE_MD_REL);

    write_text(&hotpath_csv, &render_csv(&hotpaths))
        .map_err(|err| format!("write_hotpath_csv_failed:{err}"))?;
    write_text(
        &hotpath_md,
        &render_md("RUST60 Live TS Hotpaths", &hotpaths),
    )
    .map_err(|err| format!("write_hotpath_md_failed:{err}"))?;
    lane_utils::write_json(&queue_json, queue)
        .map_err(|err| format!("write_queue_json_failed:{err}"))?;
    write_text(&queue_md, &render_md("RUST60 Live Execution Queue", &lanes))
        .map_err(|err| format!("write_queue_md_failed:{err}"))?;
    Ok(())
}

pub(super) fn summary_payload(queue: &Value) -> Value {
    let ts = queue
        .get("ts")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(now_iso);
    json!({
        "ok": true,
        "type": "top50_roi_sweep",
        "ts": ts,
        "current_rust_percent": queue.get("current_rust_percent").and_then(Value::as_f64).unwrap_or(0.0),
        "target_already_met": queue.get("target_already_met").and_then(Value::as_bool).unwrap_or(false),
        "queue_size": queue.get("queue_size").and_then(Value::as_u64).unwrap_or(0),
        "bridge_wrappers_excluded": queue.get("bridge_wrappers_excluded").and_then(Value::as_u64).unwrap_or(0),
        "extension_surfaces_excluded": queue.get("extension_surfaces_excluded").and_then(Value::as_u64).unwrap_or(0),
        "top_count": queue.get("top").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "output_files": {
            "hotpath_csv": HOTPATH_CSV_REL,
            "hotpath_md": HOTPATH_MD_REL,
            "queue_json": QUEUE_JSON_REL,
            "queue_md": QUEUE_MD_REL,
        }
    })
}

fn write_text(path: &Path, body: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(path, body).map_err(|err| err.to_string())
}
