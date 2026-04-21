
fn runs_for_workflow(state: &Value, workflow_id: &str) -> Vec<Value> {
    state
        .pointer(&format!("/runs_by_workflow/{}", clean_id(workflow_id, 120)))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn summarize_evicted_runs(evicted: &[Value]) -> Value {
    if evicted.is_empty() {
        return Value::Null;
    }
    let mut status_counts = BTreeMap::<String, i64>::new();
    let mut duration_total = 0i64;
    let mut duration_count = 0i64;
    let mut samples = Vec::<String>::new();
    for row in evicted {
        let status = clean_text(
            row.get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            40,
        );
        *status_counts.entry(status).or_insert(0) += 1;
        if let Some(ms) = row.get("duration_ms").and_then(Value::as_i64) {
            if ms > 0 {
                duration_total += ms;
                duration_count += 1;
            }
        }
        if samples.len() < 2 {
            let sample = clean_text(row.get("output").and_then(Value::as_str).unwrap_or(""), 220);
            if !sample.is_empty() {
                samples.push(sample);
            }
        }
    }
    let status_json = status_counts
        .into_iter()
        .map(|(status, count)| (status, json!(count)))
        .collect::<Map<String, Value>>();
    json!({
        "keyframe_id": make_id("wf-kf", &json!({"ts": crate::now_iso(), "size": evicted.len()})),
        "created_at": crate::now_iso(),
        "evicted_runs": evicted.len(),
        "status_counts": Value::Object(status_json),
        "avg_duration_ms": if duration_count > 0 { duration_total / duration_count } else { 0 },
        "sample_outputs": samples
    })
}

fn set_runs_for_workflow(state: &mut Value, workflow_id: &str, mut runs: Vec<Value>) {
    let mut evicted = Vec::<Value>::new();
    if runs.len() > 200 {
        let keep_from = runs.len().saturating_sub(200);
        evicted = runs.iter().take(keep_from).cloned().collect::<Vec<_>>();
        runs = runs.into_iter().skip(keep_from).collect::<Vec<_>>();
    }
    if !state
        .get("runs_by_workflow")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["runs_by_workflow"] = Value::Object(Map::new());
    }
    if let Some(map) = state
        .get_mut("runs_by_workflow")
        .and_then(Value::as_object_mut)
    {
        map.insert(clean_id(workflow_id, 120), Value::Array(runs));
    }

    let keyframe = summarize_evicted_runs(&evicted);
    if keyframe.is_null() {
        return;
    }
    if !state
        .get("keyframes_by_workflow")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["keyframes_by_workflow"] = Value::Object(Map::new());
    }
    if let Some(map) = state
        .get_mut("keyframes_by_workflow")
        .and_then(Value::as_object_mut)
    {
        let key = clean_id(workflow_id, 120);
        let mut rows = map
            .get(&key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        rows.push(keyframe);
        if rows.len() > 50 {
            let keep_from = rows.len().saturating_sub(50);
            rows = rows.into_iter().skip(keep_from).collect::<Vec<_>>();
        }
        map.insert(key, Value::Array(rows));
    }
}

fn workflow_output(input: &str, workflow: &Value) -> (String, Vec<Value>) {
    let steps = workflow
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut current = clean_text(input, 10_000);
    let mut rows = Vec::<Value>::new();
    for (idx, step) in steps.iter().enumerate() {
        let name = clean_text(
            step.get("name")
                .and_then(Value::as_str)
                .unwrap_or(&format!("step-{}", idx + 1)),
            120,
        );
        let prompt = clean_text(
            step.get("prompt_template")
                .and_then(Value::as_str)
                .unwrap_or("{{input}}"),
            4000,
        );
        let rendered = if prompt.contains("{{input}}") {
            prompt.replace("{{input}}", &current)
        } else if current.is_empty() {
            prompt
        } else {
            format!("{prompt}\n\nInput:\n{current}")
        };
        let output = clean_text(&rendered, 16_000);
        rows.push(json!({
            "step": if name.is_empty() { format!("step-{}", idx + 1) } else { name },
            "output": output
        }));
        current = output;
    }
    (current, rows)
}

fn normalize_schedule(schedule: &Value) -> Value {
    if let Some(kind) = schedule.get("kind").and_then(Value::as_str) {
        if kind == "cron" {
            return json!({
                "kind": "cron",
                "expr": clean_text(schedule.get("expr").and_then(Value::as_str).unwrap_or("* * * * *"), 120)
            });
        }
        if kind == "every" {
            return json!({
                "kind": "every",
                "every_secs": as_i64(schedule.get("every_secs"), 300).max(30)
            });
        }
        if kind == "at" {
            return json!({
                "kind": "at",
                "at": clean_text(schedule.get("at").and_then(Value::as_str).unwrap_or(""), 120)
            });
        }
    }
    if let Some(expr) = schedule.get("expr").and_then(Value::as_str) {
        return json!({"kind": "cron", "expr": clean_text(expr, 120)});
    }
    if let Some(expr) = schedule.as_str() {
        return json!({"kind": "cron", "expr": clean_text(expr, 120)});
    }
    json!({"kind": "cron", "expr": "* * * * *"})
}

fn schedule_next_run(schedule: &Value) -> Value {
    let kind = clean_text(
        schedule
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("cron"),
        24,
    )
    .to_ascii_lowercase();
    if kind == "at" {
        let at = clean_text(
            schedule.get("at").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if !at.is_empty() {
            return Value::String(at);
        }
        return Value::Null;
    }
    if kind == "every" {
        let secs = as_i64(schedule.get("every_secs"), 300).max(30);
        return Value::String((Utc::now() + Duration::seconds(secs)).to_rfc3339());
    }
    let expr = clean_text(
        schedule.get("expr").and_then(Value::as_str).unwrap_or(""),
        120,
    );
    if expr.is_empty() || expr == "* * * * *" {
        return Value::String(now_plus(1));
    }
    if expr.starts_with("*/") {
        let mins = expr
            .trim_start_matches("*/")
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(5);
        return Value::String(now_plus(mins));
    }
    if expr == "0 * * * *" {
        return Value::String(now_plus(60));
    }
    if expr.starts_with("0 */") {
        let hours = expr
            .trim_start_matches("0 */")
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(1);
        return Value::String(now_plus(hours * 60));
    }
    Value::String(now_plus(15))
}
