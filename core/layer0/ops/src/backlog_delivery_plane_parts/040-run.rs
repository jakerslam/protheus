pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    if command == "status" {
        let payload = plane_status(
            root,
            STATE_ENV,
            STATE_SCOPE,
            "backlog_delivery_plane_status",
        );
        return emit_attached_plane_receipt(root, STATE_ENV, STATE_SCOPE, false, payload, None);
    }

    if command != "run" {
        return emit_attached_plane_receipt(
            root,
            STATE_ENV,
            STATE_SCOPE,
            true,
            json!({
                "ok": false,
                "type": "backlog_delivery_plane_error",
                "error": "unknown_command",
                "command": command
            }),
            None,
        );
    }

    let strict = strict_mode(&parsed);
    let id = normalize_id(&parsed);
    if id.is_empty() || !id.starts_with('V') {
        return emit_attached_plane_receipt(
            root,
            STATE_ENV,
            STATE_SCOPE,
            strict,
            json!({
                "ok": false,
                "type": "backlog_delivery_plane_run",
                "error": "missing_or_invalid_id"
            }),
            None,
        );
    }

    let conduit = build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        &format!("backlog_delivery:{id}"),
        "backlog_delivery_conduit_enforcement",
        "core/layer0/ops/backlog_delivery_plane",
        conduit_bypass_requested(&parsed.flags),
        "backlog_delivery_actions_route_through_layer0_conduit_with_fail_closed_bypass_rejection",
        &[&id],
    );
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let payload = attach_conduit(
            json!({
                "ok": false,
                "type": "backlog_delivery_plane_run",
                "id": id,
                "error": "conduit_enforcement_failed"
            }),
            Some(&conduit),
        );
        return emit_attached_plane_receipt(root, STATE_ENV, STATE_SCOPE, strict, payload, None);
    }

    let mut payload = run_id(root, &id, &parsed);
    if payload.get("type").is_none() {
        payload["type"] = Value::String("backlog_delivery_plane_run".to_string());
    }
    payload["id"] = Value::String(id);
    payload["lane"] = Value::String("core/layer0/ops".to_string());
    payload["strict"] = Value::Bool(strict);
    payload = attach_conduit(payload, Some(&conduit));

    emit_attached_plane_receipt(root, STATE_ENV, STATE_SCOPE, strict, payload, None)
}

