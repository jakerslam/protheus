// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0

pub(crate) fn web_tooling_error_guidance(error_code: &str) -> serde_json::Value {
    let code = crate::dashboard_compat_api::clean_text(error_code, 120).to_ascii_lowercase();
    let (summary, next_steps): (&str, Vec<&str>) = match code.as_str() {
        "web_tool_auth_missing" => (
            "No server-side credential was found for this tooling path.",
            vec![
                "Set the required provider token in server env.",
                "Re-run probe and then retry the same query.",
            ],
        ),
        "web_tool_policy_blocked" => (
            "The tooling request was blocked by policy.",
            vec![
                "Narrow the query to one source or one domain.",
                "Use domain-first mode if broad queries are blocked.",
            ],
        ),
        "web_tool_invalid_response" => (
            "The provider returned an invalid or malformed response.",
            vec![
                "Retry with one precise query.",
                "Switch preferred provider order for this lane.",
            ],
        ),
        "web_tool_timeout" => (
            "The request timed out before a valid response arrived.",
            vec![
                "Reduce breadth and run fewer queries.",
                "Retry with a known-good domain source.",
            ],
        ),
        "web_tool_http_429" => (
            "Provider rate limit was hit.",
            vec![
                "Back off and retry shortly.",
                "Reduce query fan-out and use narrower prompts.",
            ],
        ),
        _ => (
            "No critical remediation is required yet.",
            vec![
                "Run a single-source probe query.",
                "Confirm status route remains healthy before broad search.",
            ],
        ),
    };
    serde_json::json!({
        "error_code": if code.is_empty() { "none" } else { &code },
        "summary": summary,
        "next_steps": next_steps
    })
}

include!("dashboard_compat_api_reference_gap_closure.rs.inc");
