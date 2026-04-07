pub fn run_pipeline(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let contract = read_json_or(
        root,
        PIPELINE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "item_pipeline_contract",
            "stages": ["validate","dedupe","enrich"],
            "allowed_export_formats": ["json","csv"]
        }),
    );
    let items_json = parse_json_flag_or_path(root, parsed, "items-json", "items-path", json!([]));
    let pipeline_json = parse_json_flag_or_path(
        root,
        parsed,
        "pipeline-json",
        "pipeline-path",
        json!([
            {"stage":"validate","required_fields":["url","title"]},
            {"stage":"dedupe","key":"url"},
            {"stage":"enrich","add":{"source":"research"}}
        ]),
    );
    let export_format = clean(
        parsed
            .flags
            .get("export-format")
            .cloned()
            .unwrap_or_else(|| "json".to_string()),
        16,
    )
    .to_ascii_lowercase();
    let export_path_rel = parsed.flags.get("export-path").cloned().unwrap_or_else(|| {
        state_root(root)
            .join("pipeline")
            .join(format!("latest.{export_format}"))
            .display()
            .to_string()
    });
    let export_path = if Path::new(&export_path_rel).is_absolute() {
        PathBuf::from(&export_path_rel)
    } else {
        root.join(&export_path_rel)
    };

    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("pipeline_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "item_pipeline_contract"
    {
        errors.push("pipeline_contract_kind_invalid".to_string());
    }
    let allowed_formats = contract
        .get("allowed_export_formats")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if !allowed_formats.iter().any(|v| v == &export_format) {
        errors.push("export_format_not_allowed".to_string());
    }
    let mut items = items_json.unwrap_or_else(|err| {
        errors.push(err);
        json!([])
    });
    let pipeline = pipeline_json.unwrap_or_else(|err| {
        errors.push(err);
        json!([])
    });
    if !items.is_array() {
        errors.push("items_payload_must_be_array".to_string());
        items = json!([]);
    }
    if !errors.is_empty() {
        return finalize_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "research_plane_item_pipeline",
            "errors": errors
        }));
    }

    let mut stage_receipts = Vec::<Value>::new();
    let mut rows = items.as_array().cloned().unwrap_or_default();
    for stage in pipeline.as_array().cloned().unwrap_or_default() {
        let stage_name = stage
            .get("stage")
            .and_then(Value::as_str)
            .map(|v| v.to_ascii_lowercase())
            .unwrap_or_else(|| "unknown".to_string());
        let before = rows.len();
        if stage_name == "validate" {
            let required = stage
                .get("required_fields")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
                .map(|v| v.to_string())
                .collect::<Vec<_>>();
            rows.retain(|row| {
                required
                    .iter()
                    .all(|k| row.get(k).map(|v| !v.is_null()).unwrap_or(false))
            });
        } else if stage_name == "dedupe" {
            let key = stage
                .get("key")
                .and_then(Value::as_str)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "url".to_string());
            let mut seen = BTreeSet::<String>::new();
            rows.retain(|row| {
                let v = row
                    .get(&key)
                    .map(|x| clean(x.to_string(), 600))
                    .unwrap_or_default();
                if v.is_empty() || seen.contains(&v) {
                    false
                } else {
                    seen.insert(v);
                    true
                }
            });
        } else if stage_name == "enrich" {
            let add = stage
                .get("add")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            for row in &mut rows {
                if !row.is_object() {
                    continue;
                }
                for (k, v) in &add {
                    row[k] = v.clone();
                }
            }
        }
        stage_receipts.push(json!({
            "stage": stage_name,
            "before": before,
            "after": rows.len()
        }));
    }
    if let Some(parent) = export_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let export_body = if export_format == "csv" {
        let headers = rows
            .iter()
            .filter_map(Value::as_object)
            .flat_map(|row| row.keys().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut lines = vec![headers.join(",")];
        for row in &rows {
            let obj = row.as_object().cloned().unwrap_or_default();
            let line = headers
                .iter()
                .map(|h| clean(obj.get(h).cloned().unwrap_or(Value::Null).to_string(), 600))
                .collect::<Vec<_>>()
                .join(",");
            lines.push(line);
        }
        lines.join("\n")
    } else {
        serde_json::to_string_pretty(&rows).unwrap_or_else(|_| "[]".to_string())
    };
    let _ = fs::write(&export_path, format!("{export_body}\n"));

    let out = finalize_receipt(json!({
        "ok": true,
        "strict": strict,
        "type": "research_plane_item_pipeline",
        "lane": "core/layer0/ops",
        "stage_receipts": stage_receipts,
        "item_count": rows.len(),
        "export": {
            "format": export_format,
            "path": export_path.display().to_string(),
            "sha256": sha256_hex_str(&export_body)
        },
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-002.3",
                "claim": "item_pipeline_stages_and_feed_exporters_are_governed_and_receipted",
                "evidence": {"stages": stage_receipts.len()}
            }
        ]
    }));
    out
}

pub fn run_signals(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let contract = read_json_or(
        root,
        SIGNAL_BUS_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "signal_bus_contract",
            "supported_signals": ["spider_opened","item_scraped","spider_closed"]
        }),
    );
    let events_json = parse_json_flag_or_path(
        root,
        parsed,
        "events-json",
        "events-path",
        json!([
            {"signal":"spider_opened","payload":{"spider_id":"default"}},
            {"signal":"item_scraped","payload":{"url":"https://example.com"}}
        ]),
    );
    let handlers_json = parse_json_flag_or_path(
        root,
        parsed,
        "handlers-json",
        "handlers-path",
        json!([
            {"id":"metrics","signal":"item_scraped"},
            {"id":"lifecycle","signal":"spider_opened"}
        ]),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("signal_bus_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "signal_bus_contract"
    {
        errors.push("signal_bus_contract_kind_invalid".to_string());
    }
    let events = events_json.unwrap_or_else(|err| {
        errors.push(err);
        json!([])
    });
    let handlers = handlers_json.unwrap_or_else(|err| {
        errors.push(err);
        json!([])
    });
    if !events.is_array() || !handlers.is_array() {
        errors.push("signal_payloads_must_be_arrays".to_string());
    }
    if !errors.is_empty() {
        return finalize_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "research_plane_signal_bus",
            "errors": errors
        }));
    }

    let supported = contract
        .get("supported_signals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_string())
        .collect::<BTreeSet<_>>();

    let mut dispatch = Vec::<Value>::new();
    for event in events.as_array().cloned().unwrap_or_default() {
        let signal = event
            .get("signal")
            .and_then(Value::as_str)
            .map(|v| v.to_string())
            .unwrap_or_default();
        if !supported.contains(&signal) {
            dispatch.push(json!({
                "signal": signal,
                "status": "rejected",
                "reason": "unsupported_signal"
            }));
            continue;
        }
        let matched = handlers
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|h| h.get("signal").and_then(Value::as_str) == Some(signal.as_str()))
            .map(|h| {
                json!({
                    "handler_id": h.get("id").and_then(Value::as_str).unwrap_or("anonymous"),
                    "signal": signal
                })
            })
            .collect::<Vec<_>>();
        dispatch.push(json!({
            "signal": signal,
            "status": "dispatched",
            "handler_count": matched.len(),
            "handlers": matched
        }));
    }

    let out = finalize_receipt(json!({
        "ok": true,
        "strict": strict,
        "type": "research_plane_signal_bus",
        "lane": "core/layer0/ops",
        "dispatch_receipts": dispatch,
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-002.4",
                "claim": "signal_bus_dispatches_policy_gated_events_with_deterministic_receipts",
                "evidence": {"events": events.as_array().map(|v| v.len()).unwrap_or(0)}
            }
        ]
    }));
    out
}

pub fn run_console(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let contract = read_json_or(
        root,
        CONSOLE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "crawl_console_contract",
            "default_token_env": "RESEARCH_CONSOLE_TOKEN",
            "allow_ops": ["status","stats","queue","pause","resume","enqueue"]
        }),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        64,
    )
    .to_ascii_lowercase();
    let token_env = contract
        .get("default_token_env")
        .and_then(Value::as_str)
        .unwrap_or("RESEARCH_CONSOLE_TOKEN");
    let expected = std::env::var(token_env).unwrap_or_else(|_| "local-dev-token".to_string());
    let supplied = parsed.flags.get("auth-token").cloned().unwrap_or_default();
    let auth_ok = !expected.is_empty() && supplied == expected;

    let allowed_ops = contract
        .get("allow_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| v.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if !allowed_ops.iter().any(|v| v == &op) {
        return finalize_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "research_plane_console",
            "errors": ["console_op_not_allowed"]
        }));
    }

    let console_state_path = state_root(root).join("console").join("state.json");
    let mut state = read_json(&console_state_path).unwrap_or_else(|| {
        json!({
            "paused": false,
            "queue": [],
            "last_op": "init",
            "updated_at": now_iso()
        })
    });
    if !auth_ok {
        return finalize_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "research_plane_console",
            "op": op,
            "auth": "denied",
            "errors": ["auth_failed"],
            "claim_evidence": [
                {
                    "id": "V6-RESEARCH-002.5",
                    "claim": "crawl_console_requires_authenticated_control_path",
                    "evidence": {"op": op}
                }
            ]
        }));
    }

    if op == "pause" {
        state["paused"] = Value::Bool(true);
    } else if op == "resume" {
        state["paused"] = Value::Bool(false);
    } else if op == "enqueue" {
        let url = clean(parsed.flags.get("url").cloned().unwrap_or_default(), 1800);
        if !url.is_empty() {
            if !state.get("queue").map(Value::is_array).unwrap_or(false) {
                state["queue"] = Value::Array(Vec::new());
            }
            state["queue"]
                .as_array_mut()
                .expect("queue array")
                .push(Value::String(url));
        }
    }
    state["last_op"] = Value::String(op.clone());
    state["updated_at"] = Value::String(now_iso());
    let _ = write_json(&console_state_path, &state);

    let out = finalize_receipt(json!({
        "ok": true,
        "strict": strict,
        "type": "research_plane_console",
        "lane": "core/layer0/ops",
        "op": op,
        "auth": "ok",
        "state": state,
        "stats": {
            "paused": state.get("paused").and_then(Value::as_bool).unwrap_or(false),
            "queue_len": state.get("queue").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0)
        },
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-002.5",
                "claim": "authenticated_console_controls_pause_resume_queue_with_receipts",
                "evidence": {"op": op}
            }
        ]
    }));
    out
}

