use super::*;

#[test]
fn template_governance_handles_missing_contract_with_receipt() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    let parsed = ParsedArgs {
        flags: std::collections::HashMap::new(),
        positional: Vec::new(),
    };
    let payload = run_template_governance(root, &parsed, false);
    assert_eq!(
        payload.get("type").and_then(Value::as_str),
        Some("research_plane_template_governance")
    );
    assert!(payload.get("receipt_hash").is_some());
}
