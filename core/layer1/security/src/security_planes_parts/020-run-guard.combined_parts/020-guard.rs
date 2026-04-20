
pub fn run_guard(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let first = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "run".to_string());
    if first == "status" {
        return (
            json!({
                "ok": true,
                "type": "security_guard_status",
                "ts": now_iso(),
                "zones": guard_zones().iter().map(|z| json!({"prefix": z.prefix, "min_clearance": z.min_clearance, "label": z.label})).collect::<Vec<_>>(),
                "protected_files": guard_protected_files(),
                "default_clearance": 2
            }),
            0,
        );
    }

    let files = if let Some(csv) = flag(&parsed, "files") {
        split_csv(csv, 200)
    } else {
        parsed
            .positional
            .iter()
            .filter(|v| *v != "run")
            .map(normalize_rel)
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>()
    };
    if files.is_empty() {
        return (
            json!({
                "ok": false,
                "blocked": true,
                "type": "security_guard",
                "error": "files_required",
                "usage": "security-plane guard --files=<path1,path2,...>"
            }),
            2,
        );
    }

    let clearance = std::env::var("CLEARANCE")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(2)
        .clamp(1, 4);
    let break_glass = std::env::var("BREAK_GLASS")
        .ok()
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);
    let approval_note = clean(
        std::env::var("APPROVAL_NOTE")
            .or_else(|_| std::env::var("SECOND_APPROVAL_NOTE"))
            .unwrap_or_default(),
        400,
    );
    let request_source = clean(
        std::env::var("REQUEST_SOURCE").unwrap_or_else(|_| "local".to_string()),
        60,
    )
    .to_ascii_lowercase();
    let request_action = clean(
        std::env::var("REQUEST_ACTION").unwrap_or_else(|_| "apply".to_string()),
        60,
    )
    .to_ascii_lowercase();
    let remote_source = matches!(
        request_source.as_str(),
        "slack" | "discord" | "webhook" | "email" | "api" | "remote" | "moltbook"
    );
    let proposal_action = matches!(
        request_action.as_str(),
        "propose" | "proposal" | "dry_run" | "dry-run" | "audit"
    );

    // Integrity gate remains authoritative before clearance checks.
    let (integrity, _) =
        crate::run_integrity_reseal(repo_root, &["check".to_string(), "--staged=0".to_string()]);
    let integrity_ok = integrity
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !integrity_ok {
        return (
            json!({
                "ok": false,
                "blocked": true,
                "break_glass": false,
                "reason": "integrity_violation",
                "ts": now_iso(),
                "integrity": integrity
            }),
            1,
        );
    }

    let mut requirements = Vec::<Value>::new();
    let mut required_max = 1i64;
    for file in &files {
        let (min_clearance, label) = guard_match_zone(file);
        required_max = required_max.max(min_clearance);
        requirements.push(json!({
            "file": file,
            "min_clearance": min_clearance,
            "label": label
        }));
    }
    let clearance_ok = clearance >= required_max;

    let break_glass_allowed = break_glass
        && approval_note.len() >= 12
        && (!remote_source
            || proposal_action
            || (std::env::var("REMOTE_DIRECT_OVERRIDE").ok().as_deref() == Some("1")
                && !clean(std::env::var("APPROVER_ID").unwrap_or_default(), 120).is_empty()
                && !clean(std::env::var("SECOND_APPROVER_ID").unwrap_or_default(), 120)
                    .is_empty()));

    let blocked = !clearance_ok && !break_glass_allowed;
    let ok = !blocked;
    let reason = if blocked {
        "clearance_insufficient"
    } else if !clearance_ok && break_glass_allowed {
        "break_glass"
    } else {
        "approved"
    };

    let (break_glass_log, remote_log, risky_log) = guard_state_logs(repo_root);
    if break_glass {
        let _ = append_jsonl(
            &break_glass_log,
            &json!({
                "ts": now_iso(),
                "type": "break_glass_attempt",
                "ok": ok,
                "reason": reason,
                "request_source": request_source,
                "request_action": request_action,
                "approval_note_len": approval_note.len(),
                "required_clearance": required_max,
                "clearance": clearance,
                "files": files
            }),
        );
    }
    if remote_source {
        let _ = append_jsonl(
            &remote_log,
            &json!({
                "ts": now_iso(),
                "type": "remote_request_gate",
                "ok": ok,
                "reason": reason,
                "request_source": request_source,
                "request_action": request_action,
                "proposal_action": proposal_action
            }),
        );
    }
    let risky_toggles = [
        "AUTONOMY_ENABLED",
        "AUTONOMY_MODEL_CATALOG_AUTO_APPLY",
        "AUTONOMY_MODEL_CATALOG_AUTO_BREAK_GLASS",
        "REMOTE_DIRECT_OVERRIDE",
        "BREAK_GLASS",
    ]
    .iter()
    .filter_map(|k| {
        std::env::var(k)
            .ok()
            .map(|v| (k.to_string(), v))
            .filter(|(_, v)| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
    })
    .collect::<Vec<_>>();
    if !risky_toggles.is_empty() {
        let _ = append_jsonl(
            &risky_log,
            &json!({
                "ts": now_iso(),
                "type": "risky_env_toggle_gate",
                "ok": ok,
                "reason": reason,
                "toggles": risky_toggles
            }),
        );
    }

    (
        json!({
            "ok": ok,
            "blocked": blocked,
            "break_glass": reason == "break_glass",
            "reason": reason,
            "ts": now_iso(),
            "request_source": request_source,
            "request_action": request_action,
            "clearance": clearance,
            "required_clearance": required_max,
            "requirements": requirements
        }),
        if ok { 0 } else { 1 },
    )
}
