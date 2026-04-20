fn chat_ui_retry_loop_risk_from_diagnostics(diagnostics: &Value) -> Value {
    let receipts = diagnostics
        .get("execution_receipts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let receipt_count = receipts.len() as i64;
    if receipt_count == 0 {
        return json!({
            "detected": false,
            "severity": "none",
            "receipt_count": 0,
            "max_duplicate_signature_count": 0,
            "max_consecutive_signature_streak": 0,
            "dominant_signature": Value::Null,
            "source": "execution_receipts"
        });
    }

    let mut signatures = Vec::<String>::new();
    for row in &receipts {
        let status = clean(row.get("status").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        let error_code = clean(row.get("error_code").and_then(Value::as_str).unwrap_or(""), 120)
            .to_ascii_lowercase();
        let signature = if status.is_empty() && error_code.is_empty() {
            "unknown".to_string()
        } else if error_code.is_empty() {
            status
        } else {
            format!("{status}|{error_code}")
        };
        signatures.push(signature);
    }

    let mut max_duplicate_signature_count = 0_i64;
    let mut dominant_signature = String::new();
    for signature in &signatures {
        let duplicate_count = signatures.iter().filter(|candidate| *candidate == signature).count() as i64;
        if duplicate_count > max_duplicate_signature_count {
            max_duplicate_signature_count = duplicate_count;
            dominant_signature = signature.clone();
        }
    }

    let mut max_consecutive_signature_streak = 0_i64;
    let mut streak = 0_i64;
    let mut last_signature = String::new();
    for signature in &signatures {
        if *signature == last_signature {
            streak += 1;
        } else {
            streak = 1;
            last_signature = signature.clone();
        }
        if streak > max_consecutive_signature_streak {
            max_consecutive_signature_streak = streak;
        }
    }

    let detected = receipt_count >= 3
        && (max_duplicate_signature_count >= 3 || max_consecutive_signature_streak >= 2);
    let severity = if receipt_count >= 4
        && (max_duplicate_signature_count >= 4 || max_consecutive_signature_streak >= 3)
    {
        "high"
    } else if detected {
        "medium"
    } else {
        "none"
    };
    json!({
        "detected": detected,
        "severity": severity,
        "receipt_count": receipt_count,
        "max_duplicate_signature_count": max_duplicate_signature_count,
        "max_consecutive_signature_streak": max_consecutive_signature_streak,
        "dominant_signature": if dominant_signature.is_empty() { Value::Null } else { json!(dominant_signature) },
        "source": "execution_receipts"
    })
}
