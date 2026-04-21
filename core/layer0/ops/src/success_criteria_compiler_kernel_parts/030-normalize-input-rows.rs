
fn normalize_input_rows(
    rows: Option<&Value>,
    source: &str,
) -> Vec<(String, String, String, String)> {
    let src = if source.trim().is_empty() {
        "success_criteria".to_string()
    } else {
        source.trim().to_string()
    };
    let mut out = Vec::new();
    for row in as_array(rows) {
        if let Some(text) = row.as_str() {
            let target = normalize_spaces_str(text);
            if !target.is_empty() {
                out.push((src.clone(), String::new(), target, String::new()));
            }
            continue;
        }
        let Some(obj) = row.as_object() else {
            continue;
        };
        let metric = normalize_spaces(obj.get("metric").or_else(|| obj.get("name")));
        let target = normalize_spaces(
            obj.get("target")
                .or_else(|| obj.get("threshold"))
                .or_else(|| obj.get("description"))
                .or_else(|| obj.get("goal")),
        );
        let horizon = normalize_spaces(
            obj.get("horizon")
                .or_else(|| obj.get("window"))
                .or_else(|| obj.get("by")),
        );
        if metric.is_empty() && target.is_empty() && horizon.is_empty() {
            continue;
        }
        out.push((src.clone(), metric, target, horizon));
    }
    out
}

pub(crate) fn compile_success_criteria_rows(rows: Option<&Value>, source: &str) -> Vec<Value> {
    let raw_rows = normalize_input_rows(rows, source);
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (row_source, row_metric, row_target, row_horizon) in raw_rows {
        let metric = classify_metric(&row_metric, &row_target, &row_source);
        let horizon = if row_horizon.is_empty() {
            parse_horizon(&row_target)
        } else {
            row_horizon
        };
        let target = normalize_target(&metric, &row_target, &horizon);
        let key = format!("{metric}|{target}|{horizon}|{row_source}").to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        out.push(json!({
            "source": row_source,
            "metric": metric,
            "target": target,
            "horizon": horizon,
            "measurable": true
        }));
    }
    out
}

pub(crate) fn compile_proposal_success_criteria(payload: &Map<String, Value>) -> Vec<Value> {
    let proposal = as_object(payload.get("proposal"))
        .cloned()
        .unwrap_or_default();
    let action_spec = as_object(proposal.get("action_spec"))
        .cloned()
        .unwrap_or_default();
    let opts = as_object(payload.get("opts")).cloned().unwrap_or_default();
    let include_verify = opts
        .get("include_verify")
        .map(|v| lane_utils::parse_bool(Some(&as_str(Some(v))), true))
        .unwrap_or(true);
    let include_validation = opts
        .get("include_validation")
        .map(|v| lane_utils::parse_bool(Some(&as_str(Some(v))), true))
        .unwrap_or(true);
    let allow_fallback = opts
        .get("allow_fallback")
        .map(|v| lane_utils::parse_bool(Some(&as_str(Some(v))), true))
        .unwrap_or(true);
    let capability_key = normalize_capability_key(opts.get("capability_key"));

    let mut compiled = Vec::new();
    compiled.extend(compile_success_criteria_rows(
        proposal.get("success_criteria"),
        "success_criteria",
    ));
    compiled.extend(compile_success_criteria_rows(
        action_spec.get("success_criteria"),
        "action_spec.success_criteria",
    ));
    if include_verify {
        compiled.extend(compile_success_criteria_rows(
            action_spec.get("verify"),
            "action_spec.verify",
        ));
    }
    if include_validation {
        compiled.extend(compile_success_criteria_rows(
            proposal.get("validation"),
            "validation",
        ));
    }

    if compiled.is_empty() && allow_fallback {
        compiled.push(json!({
            "source": "compiler_fallback",
            "metric": "execution_success",
            "target": "execution success",
            "horizon": "",
            "measurable": true
        }));
    }

    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for row in compiled {
        let metric = remap_metric_for_capability(&as_str(row.get("metric")), &capability_key);
        let horizon = normalize_spaces(row.get("horizon"));
        let target = normalize_target(&metric, &as_str(row.get("target")), &horizon);
        let source = {
            let normalized = normalize_spaces(row.get("source"));
            if normalized.is_empty() {
                "success_criteria".to_string()
            } else {
                normalized
            }
        };
        let key = format!("{source}|{metric}|{target}|{horizon}").to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        out.push(json!({
            "source": source,
            "metric": metric,
            "target": target,
            "horizon": horizon,
            "measurable": true
        }));
    }
    out
}

fn to_action_spec_rows(rows: Option<&Value>) -> Vec<Value> {
    as_array(rows)
        .iter()
        .map(|row| {
            let metric = {
                let metric = as_str(row.get("metric"));
                if metric.is_empty() {
                    "execution_success".to_string()
                } else {
                    metric
                }
            };
            let target = {
                let target = as_str(row.get("target"));
                if target.is_empty() {
                    "execution success".to_string()
                } else {
                    target
                }
            };
            json!({
                "metric": metric,
                "target": target,
                "horizon": normalize_spaces(row.get("horizon"))
            })
        })
        .collect()
}

fn run_command(command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "compile-rows" => {
            let source = clean_text(payload.get("source"), 120);
            Ok(json!({
                "ok": true,
                "rows": compile_success_criteria_rows(payload.get("rows"), if source.is_empty() { "success_criteria" } else { &source })
            }))
        }
        "compile-proposal" => Ok(json!({
            "ok": true,
            "rows": compile_proposal_success_criteria(payload)
        })),
        "to-action-spec-rows" => Ok(json!({
            "ok": true,
            "rows": to_action_spec_rows(payload.get("rows"))
        })),
        _ => Err("success_criteria_compiler_kernel_unknown_command".to_string()),
    }
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|v| v.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("success_criteria_compiler_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();
    match run_command(command, &payload) {
        Ok(out) => {
            print_json_line(&cli_receipt("success_criteria_compiler_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("success_criteria_compiler_kernel", &err));
            1
        }
    }
}
