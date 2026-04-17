fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops verity-plane <status|drift-status|vector-check|record-event|refine-event> [flags]");
    println!("  Flags:");
    println!("    --limit=<n>                 recent rows limit for status commands");
    println!("    --operation=<id>            operation type for record-event");
    println!("    --fidelity=<0..1>           fidelity override for record-event");
    println!("    --vector=<0..1>             vector alignment override for record-event");
    println!("    --drift-delta=<-1..1>       explicit drift delta for record-event");
    println!("    --tags=<csv>                comma-delimited truth tags for record-event");
    println!("    --metadata-json=<json>      JSON metadata payload for record-event");
    println!("    --target=<0..1>             vector alignment target for vector-check");
    println!("    --falsehood=<text>          falsehood description for refine-event");
    println!("    --refined=<text>            refinement summary for refine-event");
    println!("    --before-fidelity=<0..1>    before fidelity for refine-event");
    println!("    --after-fidelity=<0..1>     after fidelity for refine-event");
}

fn attach_execution_receipt(mut payload: Value, cmd: &str) -> Value {
    let status = if payload.get("ok").and_then(Value::as_bool) == Some(true) {
        "success"
    } else {
        "error"
    };
    payload["execution_receipt"] = json!({
        "lane": "verity_plane",
        "command": cmd,
        "status": status,
        "source": VERITY_EXECUTION_RECEIPT_SOURCE,
        "tool_runtime_class": "receipt_wrapped"
    });
    payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));
    payload
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    let payload = match cmd.as_str() {
        "status" => status_payload(root, argv),
        "drift-status" | "drift" => drift_status_payload(root, argv),
        "vector-check" | "vector" => run_vector_check(root, argv),
        "record-event" | "record" => run_record_event(root, argv),
        "refine-event" | "refinement-event" | "molt" => run_refine_event(root, argv),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => {
            usage();
            let mut out = json!({
                "ok": false,
                "type": "verity_plane_error",
                "error": "unknown_command",
                "command": cmd,
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            out
        }
    };
    let payload = attach_execution_receipt(payload, cmd.as_str());
    print_json(&payload);
    if payload.get("ok").and_then(Value::as_bool) == Some(true) {
        0
    } else {
        2
    }
}
