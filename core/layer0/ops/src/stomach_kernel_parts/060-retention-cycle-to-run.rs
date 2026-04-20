
fn retention_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").ok_or_else(|| "stomach_missing_id".to_string())?;
    let action = parse_flag(argv, "action")
        .unwrap_or_else(|| "hold".to_string())
        .to_ascii_lowercase();
    let reason = parse_flag(argv, "reason").unwrap_or_else(|| "manual_hold".to_string());
    let retained_until = parse_flag(argv, "retained-until").and_then(|raw| raw.parse::<u64>().ok());
    let approve_receipt = parse_flag(argv, "approve-receipt");

    let state_root = stomach_state_root(root);
    let state_path = state_root.join("state").join(format!("{digest_id}.json"));
    let raw = read_json(&state_path)?;
    let mut state: DigestState =
        serde_json::from_value(raw).map_err(|e| format!("stomach_state_decode_failed:{e}"))?;

    if let Some(epoch_secs) = retained_until {
        transition_retention(
            &mut state.retention,
            RetentionEvent::SetRetainedUntil { epoch_secs },
        )?;
    }
    if let Some(receipt_id) = approve_receipt {
        transition_retention(
            &mut state.retention,
            RetentionEvent::ApprovePurge { receipt_id },
        )?;
    }

    match action.as_str() {
        "hold" => transition_retention(
            &mut state.retention,
            RetentionEvent::PlaceHold {
                reason: reason.clone(),
            },
        )?,
        "release" => transition_retention(&mut state.retention, RetentionEvent::ReleaseHold)?,
        "eligible" => {
            transition_retention(&mut state.retention, RetentionEvent::MarkEligibleForPurge)?
        }
        _ => return Err("stomach_retention_unknown_action".to_string()),
    }

    write_json(
        &state_path,
        &serde_json::to_value(&state).map_err(|e| format!("stomach_state_encode_failed:{e}"))?,
    )?;
    let out = json_receipt(
        "stomach_kernel_retention",
        json!({
            "digest_id": digest_id,
            "action": action,
            "retention_state": state.retention_state(),
            "retained_until": state.retention.retained_until,
            "explicit_purge_approval_receipt": state.retention.explicit_purge_approval_receipt
        }),
    );
    append_jsonl(&state_root.join("receipts.jsonl"), &out)?;
    Ok(out)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let nexus_connection = if nexus_enabled(argv) {
        match authorize_stomach_command_with_nexus(command.as_str()) {
            Ok(meta) => Some(meta),
            Err(err) => {
                print_json_line(&json_error("stomach_kernel_nexus_error", &err));
                return 1;
            }
        }
    } else {
        Some(json!({
            "enabled": false,
            "reason": "nexus_disabled_by_flag_or_env"
        }))
    };
    let response = match command.as_str() {
        "score" => score_cycle(root, &argv[1..]),
        "run" => run_cycle(root, &argv[1..]),
        "status" => status_cycle(root, &argv[1..]),
        "rollback" => rollback_cycle(root, &argv[1..]),
        "retention" => retention_cycle(root, &argv[1..]),
        "purge" => purge_cycle(root, &argv[1..]),
        _ => Err("stomach_unknown_command".to_string()),
    };
    match response {
        Ok(mut value) => {
            if let Some(meta) = nexus_connection {
                value["nexus_connection"] = meta;
            }
            print_json_line(&value);
            0
        }
        Err(err) => {
            print_json_line(&json_error("stomach_kernel_error", &err));
            1
        }
    }
}
