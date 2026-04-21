
fn run_screenshot(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let sid = session_id(parsed);
    let annotate = parse_bool(parsed.flags.get("annotate"), false);
    let delay_max_ms = 10_000_u64;
    let (requested_delay_ms, delay_source) = if let Some(raw) = parsed.flags.get("delay-ms") {
        (parse_u64(Some(raw), 500), "delay-ms")
    } else if let Some(raw) = parsed.flags.get("delay_ms") {
        (parse_u64(Some(raw), 500), "delay_ms")
    } else if let Some(raw) = parsed.flags.get("delay") {
        (parse_u64(Some(raw), 500), "delay")
    } else if let Some(raw) = parsed.flags.get("settle-ms") {
        (parse_u64(Some(raw), 500), "settle-ms")
    } else if let Some(raw) = parsed.flags.get("settle_ms") {
        (parse_u64(Some(raw), 500), "settle_ms")
    } else if let Some(raw) = parsed.flags.get("settle") {
        (parse_u64(Some(raw), 500), "settle")
    } else {
        (500, "default")
    };
    let delay_ms = requested_delay_ms.min(delay_max_ms);
    let delay_clamped = requested_delay_ms != delay_ms;
    let delay_defaulted = delay_source == "default";
    let delay_alias_used = !matches!(delay_source, "delay-ms" | "settle-ms" | "default");
    if delay_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }
    let session = read_json(&session_state_path(root, &sid)).unwrap_or_else(|| {
        json!({
            "session_id": sid,
            "target_url": "about:blank"
        })
    });
    let target_url = clean(
        session
            .get("target_url")
            .and_then(Value::as_str)
            .unwrap_or("about:blank"),
        240,
    );
    let annotations = if annotate {
        vec![
            json!({"id":"a1","label":"Primary CTA","x":90,"y":44}),
            json!({"id":"a2","label":"Navigation","x":16,"y":18}),
        ]
    } else {
        Vec::new()
    };

    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1024\" height=\"576\"><rect width=\"100%\" height=\"100%\" fill=\"#0b1020\"/><text x=\"24\" y=\"48\" fill=\"#ffffff\" font-size=\"20\">Session {}</text><text x=\"24\" y=\"78\" fill=\"#9ca3af\" font-size=\"14\">{}</text></svg>",
        sid, target_url
    );
    let svg_path = screenshot_svg_path(root);
    ensure_parent(&svg_path);
    let _ = fs::write(&svg_path, svg);

    let map = json!({
        "version": "v1",
        "session_id": sid,
        "target_url": target_url,
        "annotated": annotate,
        "requested_delay_ms": requested_delay_ms,
        "delay_ms": delay_ms,
        "delay_clamped": delay_clamped,
        "delay_defaulted": delay_defaulted,
        "delay_alias_used": delay_alias_used,
        "delay_max_ms": delay_max_ms,
        "delay_unit": "ms",
        "delay_source": delay_source,
        "annotations": annotations,
        "captured_at": crate::now_iso()
    });
    let map_path = screenshot_map_path(root);
    let _ = write_json(&map_path, &map);
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "vbrowser_plane_screenshot",
        "lane": "core/layer0/ops",
        "map": map,
        "artifact": {
            "svg_path": svg_path.display().to_string(),
            "map_path": map_path.display().to_string()
        },
        "claim_evidence": [
            {
                "id": "V6-VBROWSER-002.2",
                "claim": "screenshot_operation_emits_visual_artifact_and_coordinate_map",
                "evidence": {"session_id": sid, "annotated": annotate}
            }
        ]
    });
    stamp_receipt(&mut out);
    out
}

fn normalize_key_token(raw: &str) -> String {
    let token = clean(raw, 40);
    if token.is_empty() {
        return String::new();
    }
    if token.len() == 1 && token.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return token.to_ascii_uppercase();
    }
    let upper = token.to_ascii_uppercase();
    match upper.as_str() {
        "ENTER" | "RETURN" => "Enter".to_string(),
        "ESC" | "ESCAPE" => "Escape".to_string(),
        "BACKSPACE" => "Backspace".to_string(),
        "TAB" => "Tab".to_string(),
        "SPACE" => "Space".to_string(),
        "DELETE" | "DEL" => "Delete".to_string(),
        "ARROWUP" | "ARROW_UP" | "UP" => "ArrowUp".to_string(),
        "ARROWDOWN" | "ARROW_DOWN" | "DOWN" => "ArrowDown".to_string(),
        "ARROWLEFT" | "ARROW_LEFT" | "LEFT" => "ArrowLeft".to_string(),
        "ARROWRIGHT" | "ARROW_RIGHT" | "RIGHT" => "ArrowRight".to_string(),
        "CTRL" | "CONTROL" => "Control".to_string(),
        "CMD" | "COMMAND" | "META" | "SUPER" | "WINDOWS" | "WIN" => "Meta".to_string(),
        "OPTION" | "ALT" => "Alt".to_string(),
        "SHIFT" => "Shift".to_string(),
        "HOME" => "Home".to_string(),
        "END" => "End".to_string(),
        "PAGEUP" | "PAGE_UP" | "PGUP" => "PageUp".to_string(),
        "PAGEDOWN" | "PAGE_DOWN" | "PGDN" => "PageDown".to_string(),
        _ => token,
    }
}

fn normalize_key_combo(raw: &str) -> String {
    let cleaned = clean(raw, 240);
    if cleaned.is_empty() {
        return String::new();
    }
    cleaned
        .split('+')
        .map(normalize_key_token)
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
        .join("+")
}

fn key_input_variable_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(clean(text, 240)),
        Value::Number(num) => Some(clean(num.to_string(), 240)),
        Value::Bool(flag) => Some(if *flag {
            "true".to_string()
        } else {
            "false".to_string()
        }),
        Value::Object(map) => map.get("value").and_then(key_input_variable_text),
        _ => None,
    }
}

fn substitute_key_input_variables(
    template: &str,
    variables: &serde_json::Map<String, Value>,
) -> String {
    let mut rendered = template.to_string();
    for (key, value) in variables {
        if let Some(replacement) = key_input_variable_text(value) {
            let token = format!("%{}%", key);
            if rendered.contains(&token) {
                rendered = rendered.replace(&token, &replacement);
            }
        }
    }
    rendered
}
