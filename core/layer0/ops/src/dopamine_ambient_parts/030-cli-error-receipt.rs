
fn cli_error_receipt(command: &str, reason: &str, date: &str, exit_code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "dopamine_ambient_error",
        "ts": now_iso(),
        "command": command,
        "date": date,
        "reason": reason,
        "exit_code": exit_code
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn print_receipt(receipt: &Value) {
    println!(
        "{}",
        serde_json::to_string(receipt).unwrap_or_else(|_| "{\"ok\":false}".to_string())
    );
}

fn exit_with_error(command: &str, reason: &str, date: &str, exit_code: i32) -> i32 {
    print_receipt(&cli_error_receipt(command, reason, date, exit_code));
    exit_code
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() {
        usage();
        return exit_with_error("unknown", "missing_command", &now_iso()[..10], 2);
    }
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(command.as_str(), "closeout" | "status" | "evaluate") {
        usage();
        return exit_with_error(&command, "unknown_command", &now_iso()[..10], 2);
    }

    let flags = parse_cli_flags(&argv.iter().skip(1).cloned().collect::<Vec<_>>());
    let date = normalize_date(flags.get("date").map(String::as_str));
    let run_context = clean_text(flags.get("run-context").map(String::as_str), 40);
    let run_context = if run_context.is_empty() {
        "dopamine".to_string()
    } else {
        run_context
    };
    let policy = load_policy(root);

    let mut status_source = "computed";
    let (summary, severity, breached, breach_reasons, sds) = if command == "evaluate" {
        let summary = match parse_summary_from_flags(&flags) {
            Ok(Some(value)) => value,
            Ok(None) => {
                return exit_with_error(&command, "missing_summary_json", &date, 2);
            }
            Err(reason) => {
                return exit_with_error(&command, &reason, &date, 2);
            }
        };
        let (severity, breached, reasons) = classify_threshold(&summary);
        let sds = summary_number(&summary, "sds");
        (summary, severity, breached, reasons, sds)
    } else if command == "closeout" {
        let summary = match run_snapshot(root, &policy, "closeout", &date) {
            Ok(snapshot) => parse_snapshot_summary(&snapshot),
            Err(reason) => {
                return exit_with_error(&command, &reason, &date, 1);
            }
        };
        let (severity, breached, reasons) = classify_threshold(&summary);
        let sds = summary_number(&summary, "sds");
        (summary, severity, breached, reasons, sds)
    } else {
        let latest = read_json(&policy.latest_path).unwrap_or_else(|| json!({}));
        if let Some((summary, severity, breached, reasons, sds)) =
            load_cached_status_from_latest(&latest)
        {
            status_source = "cached_latest";
            (summary, severity, breached, reasons, sds)
        } else {
            status_source = "cold_status";
            let summary = json!({});
            let (severity, breached, reasons) = classify_threshold(&summary);
            (summary, severity, breached, reasons, 0.0)
        }
    };

    let surfaced = should_surface(&policy, &command, &severity, breached);
    let attention_queue = if policy.enabled && policy.push_attention_queue && surfaced {
        match enqueue_attention(
            root,
            &summary,
            &severity,
            &breach_reasons,
            &date,
            &run_context,
        ) {
            Ok(value) => value,
            Err(reason) => {
                return exit_with_error(&command, &reason, &date, 1);
            }
        }
    } else {
        json!({
            "ok": true,
            "queued": false,
            "decision": if command == "status" { "status_probe_no_enqueue" } else if !policy.enabled { "ambient_disabled" } else if !policy.push_attention_queue { "attention_queue_disabled" } else if !surfaced { "below_threshold" } else { "not_enqueued" },
            "routed_via": "rust_attention_queue"
        })
    };

    let mut receipt = json!({
        "ok": true,
        "type": "dopamine_ambient",
        "ts": now_iso(),
        "date": date,
        "command": command,
        "run_context": run_context,
        "status_source": status_source,
        "ambient_mode_active": policy.enabled,
        "threshold_breach_only": policy.threshold_breach_only,
        "surface_levels": policy.surface_levels,
        "severity": severity,
        "threshold_breached": breached,
        "breach_reasons": breach_reasons,
        "surfaced": surfaced,
        "sds": sds,
        "summary": summary,
        "attention_queue": attention_queue
    });
    receipt["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&receipt));

    write_json(&policy.latest_path, &receipt);
    append_jsonl(&policy.receipts_path, &receipt);

    update_mech_suit_status(
        &policy,
        json!({
            "ambient": policy.enabled,
            "threshold_breach_only": policy.threshold_breach_only,
            "surface_levels": policy.surface_levels,
            "last_result": "dopamine_ambient",
            "last_date": date,
            "last_command": command,
            "last_severity": severity,
            "last_sds": sds,
            "last_threshold_breached": breached,
            "last_attention_decision": receipt
                .get("attention_queue")
                .and_then(|v| v.get("decision"))
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        }),
    );

    print_receipt(&receipt);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_threshold_detects_non_positive_score() {
        let summary = json!({
            "sds": 0,
            "drift_minutes": 20,
            "directive_pain": { "active": false }
        });
        let (severity, breached, reasons) = classify_threshold(&summary);
        assert_eq!(severity, "warn");
        assert!(breached);
        assert!(reasons.iter().any(|row| row == "sds_non_positive"));
    }

    #[test]
    fn classify_threshold_detects_directive_pain_as_critical() {
        let summary = json!({
            "sds": 5,
            "drift_minutes": 10,
            "directive_pain": { "active": true }
        });
        let (severity, breached, reasons) = classify_threshold(&summary);
        assert_eq!(severity, "critical");
        assert!(breached);
        assert!(reasons.iter().any(|row| row == "directive_pain_active"));
    }
}
