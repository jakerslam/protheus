use super::*;

fn assert_non_silent_cli_outcome(payload: &Value) {
    assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(false));
    assert!(payload.get("type").and_then(Value::as_str).is_some());
    assert!(
        payload.get("receipt_hash").and_then(Value::as_str).is_some()
            || payload.get("claim_evidence").and_then(Value::as_array).is_some()
            || payload.get("error").is_some()
            || payload.get("reason").is_some()
    );
}

#[test]
fn cli_error_receipt_is_deterministic() {
    let out = cli_error_receipt("unknown_domain", 1, Some("nope"), None);
    assert_non_silent_cli_outcome(&out);
    assert_eq!(
        out.get("type").and_then(Value::as_str),
        Some("protheus_ops_cli_error")
    );
    assert!(out.get("claim_evidence").is_some());
    assert!(out.get("persona_lenses").is_some());
    let ts = out.get("ts").and_then(Value::as_str).expect("ts");
    let date = out.get("date").and_then(Value::as_str).expect("date");
    assert!(ts.starts_with(date));

    let expected_hash = out
        .get("receipt_hash")
        .and_then(Value::as_str)
        .expect("hash")
        .to_string();
    let mut unhashed = out.clone();
    unhashed
        .as_object_mut()
        .expect("object")
        .remove("receipt_hash");
    assert_eq!(crate::deterministic_receipt_hash(&unhashed), expected_hash);
}
