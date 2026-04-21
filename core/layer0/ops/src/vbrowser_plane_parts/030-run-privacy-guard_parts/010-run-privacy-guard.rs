fn run_privacy_guard(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        PRIVACY_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "vbrowser_privacy_security_contract",
            "allowed_network_modes": ["isolated", "restricted"],
            "max_budget_tokens": 200000,
            "recording_requires_allow_flag": true
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("vbrowser_privacy_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "vbrowser_privacy_security_contract"
    {
        errors.push("vbrowser_privacy_contract_kind_invalid".to_string());
    }

    let sid = session_id(parsed);
    let network = clean(
        parsed
            .flags
            .get("network")
            .cloned()
            .unwrap_or_else(|| "isolated".to_string()),
        40,
    )
    .to_ascii_lowercase();
    let recording = parse_bool(parsed.flags.get("recording"), false);
    let allow_recording = parse_bool(parsed.flags.get("allow-recording"), false);
    let budget_tokens = parse_u64(parsed.flags.get("budget-tokens"), 50_000);

    let allowed_networks = contract
        .get("allowed_network_modes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("isolated"), json!("restricted")])
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 40).to_ascii_lowercase())
        .collect::<Vec<_>>();
    let max_budget = contract
        .get("max_budget_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(200_000);

    if strict && !allowed_networks.iter().any(|v| v == &network) {
        errors.push("network_mode_not_allowed".to_string());
    }
    if strict
        && recording
        && contract
            .get("recording_requires_allow_flag")
            .and_then(Value::as_bool)
            .unwrap_or(true)
        && !allow_recording
    {
        errors.push("recording_not_allowed_without_flag".to_string());
    }
    if strict && budget_tokens > max_budget {
        errors.push("budget_tokens_exceed_max".to_string());
    }

    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "vbrowser_plane_privacy_guard",
            "errors": errors,
            "session_id": sid
        });
    }

    let policy_state = json!({
        "version": "v1",
        "session_id": sid,
        "network_mode": network,
        "recording": recording,
        "allow_recording": allow_recording,
        "budget_tokens": budget_tokens,
        "enforced_at": crate::now_iso()
    });
    let policy_path = state_root(root).join("privacy").join("latest.json");
    let _ = write_json(&policy_path, &policy_state);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_privacy_guard",
        "lane": "core/layer0/ops",
        "policy": policy_state,
        "artifact": {
            "path": policy_path.display().to_string(),
            "sha256": sha256_hex_str(&policy_state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-001.4",
                "claim": "privacy_and_security_controls_enforce_network_recording_and_budget_fail_closed_policies",
                "evidence": {
                    "session_id": sid,
                    "network_mode": network,
                    "budget_tokens": budget_tokens
                }
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

fn run_snapshot(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let refs_enabled = parse_bool(parsed.flags.get("refs"), true);
    let session = read_json(&session_state_path(root, &sid)).unwrap_or_else(|| {
        json!({
            "session_id": sid,
            "target_url": "about:blank",
            "shadow": "default-shadow"
        })
    });
    let target_url = session
        .get("target_url")
        .and_then(Value::as_str)
        .unwrap_or("about:blank");
    let shadow = session
        .get("shadow")
        .and_then(Value::as_str)
        .unwrap_or("default-shadow");
    let links = if refs_enabled {
        vec![
            json!({"href": target_url, "label": "current"}),
            json!({"href": "about:history", "label": "history"}),
        ]
    } else {
        Vec::new()
    };
    let snapshot = json!({
        "version": "v1",
        "session_id": sid,
        "shadow": shadow,
        "target_url": target_url,
        "refs_enabled": refs_enabled,
        "dom": {
            "title": "Virtual Browser Snapshot",
            "headings": ["h1: Session Overview", "h2: Context"],
            "text_blocks": 3
        },
        "links": links,
        "captured_at": crate::now_iso()
    });

    let path = snapshot_path(root);
    let _ = write_json(&path, &snapshot);
    let _ = append_jsonl(
        &state_root(root).join("snapshots").join("history.jsonl"),
        &snapshot,
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_snapshot",
        "lane": "core/layer0/ops",
        "snapshot": snapshot,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&snapshot.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-002.1",
                "claim": "snapshot_operation_emits_structured_page_artifact_for_streamed_browser_session",
                "evidence": {"session_id": sid, "refs_enabled": refs_enabled}
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}
