// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn redact_hides_tokens_and_bearer_headers() {
    let out = redact_secrets(
        "Authorization: Bearer abcdefghijklmnopqrstuvwxyz123456\nmoltbook_sk_abcdefghijklmnopqrstuvwxyz1234567890",
    );
    assert!(out.contains("Authorization: Bearer [REDACTED]"));
    assert!(out.contains("moltbook_sk_****7890"));
}

#[test]
fn extract_summary_reports_ids_and_urls() {
    let payload = json!({
        "id": "abcdef123456",
        "total_count": 4,
        "url": "https://example.com/long/path",
        "status": "error"
    });
    let summary = extract_summary_rows(&payload, "tool");
    assert!(summary.iter().any(|row| row.contains("IDs:")));
    assert!(summary.iter().any(|row| row.contains("URLs:")));
    assert!(summary.iter().any(|row| row.contains("Status: error")));
}
