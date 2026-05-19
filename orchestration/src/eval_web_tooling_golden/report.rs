use serde_json::{json, Value};

use super::super::eval_research_golden_utils::{str_at, u64_at};

pub(super) fn tooling_markdown_report(report: &Value) -> String {
    let summary = report.get("summary").cloned().unwrap_or_else(|| json!({}));
    let web = report
        .pointer("/measurement_split/web_tooling")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let top_layer = str_at(&web, &["operator_metrics", "top_layer"], "unknown");
    let top_failure = str_at(
        &web,
        &["operator_metrics", "top_first_failure", "name"],
        "unknown",
    );
    format!(
        "# Web Tooling Golden\n\n- mode: {}\n- success_rate: {:.3}\n- transport_adjusted_success_rate: {:.3}\n- transport_failures: {}\n- top_layer: {}\n- top_first_failure: {}\n",
        str_at(report, &["mode"], "unknown"),
        summary
            .get("success_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        summary
            .get("transport_adjusted_success_rate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        u64_at(&summary, &["transport_failures"], 0),
        top_layer,
        top_failure
    )
}
