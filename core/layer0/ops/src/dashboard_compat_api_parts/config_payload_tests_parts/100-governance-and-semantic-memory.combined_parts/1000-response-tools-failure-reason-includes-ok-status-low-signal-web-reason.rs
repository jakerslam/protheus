fn response_tools_failure_reason_includes_ok_status_low_signal_web_reason() {
    let reason = response_tools_failure_reason_for_user(
        &[json!({
            "name": "batch_query",
            "status": "ok",
            "blocked": false,
            "is_error": false,
            "result": "Web retrieval returned low-signal snippets without synthesis. Retry with a narrower query or one specific source URL for source-backed findings."
        })],
        4,
    );
    let lowered = reason.to_ascii_lowercase();
    assert!(lowered.contains("tool run hit issues"));
    assert!(lowered.contains("low-signal"));
    assert!(!lowered.contains("don't have usable tool findings from this turn yet"));
}

// Decomposed for backend file-size/cohesion remediation; behavior preserved via ordered includes.
include!("040-part_parts/000-part.rs");
include!("040-part_parts/001-part.rs");
include!("040-part_parts/002-part.rs");

use std::sync::Mutex;
