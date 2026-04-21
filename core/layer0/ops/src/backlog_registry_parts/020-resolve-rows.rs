fn canonical_registry_status(raw: &str) -> String {
    let token = raw.trim().to_ascii_lowercase().replace('-', "_");
    match token.as_str() {
        "inprogress" => "in_progress".to_string(),
        "blocked_pending" => "blocked".to_string(),
        "ready" | "todo" => "queued".to_string(),
        _ => token,
    }
}

fn canonical_registry_class(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace(' ', "-")
}

fn normalize_dependencies(deps: &[String]) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for dep in deps {
        let id = dep.trim();
        if id.is_empty() {
            continue;
        }
        let normalized = id.to_ascii_uppercase();
        if !out.contains(&normalized) {
            out.push(normalized);
        }
    }
    out.sort();
    out
}

fn resolve_rows(parsed: Vec<ParsedRow>) -> (Vec<RegistryRow>, Vec<Value>) {
    let mut by_id: BTreeMap<String, Vec<ParsedRow>> = BTreeMap::new();
    for row in parsed {
        by_id.entry(row.row.id.clone()).or_default().push(row);
    }

    let mut conflicts = Vec::new();
    let mut out = Vec::new();

    for (id, rows) in by_id {
        let mut statuses = BTreeSet::new();
        for row in &rows {
            statuses.insert(row.row.status.clone());
        }

        let pick = rows
            .iter()
            .max_by_key(|r| {
                let mut score =
                    status_weight(&canonical_registry_status(&r.row.status)) * 10_000;
                if r.canonical {
                    score += 200;
                }
                score += (r.row.acceptance.len().min(500) + r.row.problem.len().min(500)) as i32;
                score += r.source_index as i32;
                score
            })
            .expect("at least one row");

        if statuses.len() > 1 {
            conflicts.push(json!({
                "id": id,
                "statuses": statuses,
                "selected_status": pick.row.status,
                "selected_title": pick.row.title
            }));
        }

        let mut chosen = pick.row.clone();
        chosen.status = canonical_registry_status(&chosen.status);
        chosen.class = canonical_registry_class(&chosen.class);
        chosen.dependencies = normalize_dependencies(&chosen.dependencies);
        out.push(chosen);
    }

    (out, conflicts)
}

fn render_table_view(title: &str, rows: &[RegistryRow], generated_at: &str) -> String {
    let mut lines = vec![
        format!("# {title}"),
        String::new(),
        format!("Generated: {generated_at}"),
        String::new(),
        "| ID | Class | Wave | Status | Title | Dependencies |".to_string(),
        "|---|---|---|---|---|---|".to_string(),
    ];

    for row in rows {
        let deps = if row.dependencies.is_empty() {
            String::new()
        } else {
            row.dependencies.join(", ")
        };
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            row.id, row.class, row.wave, row.status, row.title, deps
        ));
    }

    lines.push(String::new());
    lines.join("\n")
}

fn impact_for_class(class: &str) -> i32 {
    match class {
        "primitive-upgrade" => 18,
        "hardening" => 12,
        "governance" => 10,
        "scale-readiness" => 8,
        "launch-polish" => 4,
        _ => 6,
    }
}

fn risk_for_status(status: &str) -> i32 {
    match status {
        "blocked" => 16,
        "in_progress" => 10,
        "queued" => 8,
        "proposed" => 6,
        _ => 4,
    }
}

fn status_bonus(status: &str) -> i32 {
    match status {
        "in_progress" => 10,
        "queued" => 8,
        "proposed" => 6,
        "blocked" => 4,
        _ => 0,
    }
}

fn render_priority_queue(rows: &[RegistryRow], active_statuses: &BTreeSet<String>) -> String {
    let generated_at = now_iso();

    let mut unlock_map: HashMap<String, i32> = HashMap::new();
    for row in rows {
        for dep in &row.dependencies {
            *unlock_map.entry(dep.clone()).or_insert(0) += 1;
        }
    }

    let done_set: BTreeSet<String> = rows
        .iter()
        .filter(|r| {
            matches!(
                r.status.as_str(),
                "done" | "reviewed" | "archived" | "dropped" | "obsolete"
            )
        })
        .map(|r| r.id.clone())
        .collect();

    #[derive(Clone)]
    struct Ranked {
        id: String,
        status: String,
        title: String,
        priority: i32,
        impact: i32,
        risk: i32,
        unresolved: i32,
        unlock_count: i32,
    }

    let mut ranked = Vec::new();
    for row in rows {
        if !active_statuses.contains(&row.status) {
            continue;
        }
        let unresolved = row
            .dependencies
            .iter()
            .filter(|dep| !done_set.contains((*dep).as_str()))
            .count() as i32;
        let unlock = *unlock_map.get(&row.id).unwrap_or(&0);
        let impact = impact_for_class(&row.class);
        let risk = risk_for_status(&row.status);
        let priority = impact + risk + status_bonus(&row.status) + (unlock * 2) - (unresolved * 3);
        ranked.push(Ranked {
            id: row.id.clone(),
            status: row.status.clone(),
            title: row.title.clone(),
            priority,
            impact,
            risk,
            unresolved,
            unlock_count: unlock,
        });
    }

    ranked.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));

    let total_rows = rows.len();
    let active_rows = ranked.len();
    let completed_rows = rows
        .iter()
        .filter(|r| !active_statuses.contains(&r.status))
        .count();

    let mut lines = vec![
        "# Backlog Priority Queue".to_string(),
        String::new(),
        format!("Generated: {generated_at}"),
        String::new(),
        "Scoring model: impact + risk + dependency pressure (unblocks and unresolved deps), with status weighting.".to_string(),
        String::new(),
        "## Summary".to_string(),
        String::new(),
        format!("- Total rows: {total_rows}"),
        format!("- Active rows: {active_rows}"),
        format!("- Completed rows: {completed_rows}"),
        String::new(),
        "## Active Execution Order".to_string(),
        String::new(),
        "| Rank | ID | Status | Priority | Impact | Risk | Unresolved Deps | Unlock Count | Title |".to_string(),
        "|---|---|---|---:|---:|---:|---:|---:|---|".to_string(),
    ];

    for (idx, row) in ranked.iter().enumerate() {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            idx + 1,
            row.id,
            row.status,
            row.priority,
            row.impact,
            row.risk,
            row.unresolved,
            row.unlock_count,
            row.title
        ));
    }

    lines.push(String::new());
    lines.join("\n")
}

fn render_reviewed(rows: &[RegistryRow], active_statuses: &BTreeSet<String>) -> String {
    let generated_at = now_iso();
    let total = rows.len();

    let mut reviewed_count = 0usize;
    let mut pass = 0usize;
    let mut warn = 0usize;
    let mut blocked = 0usize;

    let mut lines = vec![
        "# Backlog Reviewed View".to_string(),
        String::new(),
        format!("Generated: {generated_at}"),
        String::new(),
        "| ID | Status | Reviewed Status | Review Result | Reviewed | Title |".to_string(),
        "|---|---|---|---|---|---|".to_string(),
    ];

    for row in rows {
        let (reviewed_status, result, reviewed) = match row.status.as_str() {
            "done" | "reviewed" | "archived" | "dropped" | "obsolete" => {
                reviewed_count += 1;
                pass += 1;
                ("reviewed", "pass", "yes")
            }
            "blocked" => {
                blocked += 1;
                ("blocked", "blocked", "no")
            }
            _ => {
                warn += 1;
                let rs = if active_statuses.contains(&row.status) {
                    row.status.as_str()
                } else {
                    "queued"
                };
                (rs, "needs_implementation", "no")
            }
        };

        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            row.id, row.status, reviewed_status, result, reviewed, row.title
        ));
    }

    lines.insert(
        4,
        format!(
            "Summary: reviewed {reviewed_count}/{total} | pass {pass} | warn {warn} | fail 0 | blocked {blocked}"
        ),
    );
    lines.insert(5, String::new());

    lines.push(String::new());
    lines.join("\n")
}

fn render_execution_path(rows: &[RegistryRow], active_statuses: &BTreeSet<String>) -> String {
    let generated_at = now_iso();

    let mut done_ids = BTreeSet::new();
    for row in rows {
        if matches!(
            row.status.as_str(),
            "done" | "reviewed" | "archived" | "dropped" | "obsolete"
        ) {
            done_ids.insert(row.id.clone());
        }
    }

    #[derive(Clone)]
    struct QueueRow {
        row: RegistryRow,
        open_deps: Vec<String>,
        priority: i32,
    }

    let mut queued = Vec::new();
    let mut blocked = Vec::new();

    let mut unlock_map: HashMap<String, i32> = HashMap::new();
    for row in rows {
        for dep in &row.dependencies {
            *unlock_map.entry(dep.clone()).or_insert(0) += 1;
        }
    }

    for row in rows {
        if !active_statuses.contains(&row.status) {
            continue;
        }
        let open_deps = row
            .dependencies
            .iter()
            .filter(|dep| !done_ids.contains((*dep).as_str()))
            .cloned()
            .collect::<Vec<_>>();

        let unresolved = open_deps.len() as i32;
        let unlock = *unlock_map.get(&row.id).unwrap_or(&0);
        let priority = impact_for_class(&row.class)
            + risk_for_status(&row.status)
            + status_bonus(&row.status)
            + (unlock * 2)
            - (unresolved * 3);

        let q = QueueRow {
            row: row.clone(),
            open_deps,
            priority,
        };

        if row.status == "blocked" {
            blocked.push(q);
        } else if matches!(row.status.as_str(), "queued" | "proposed" | "in_progress") {
            queued.push(q);
        }
    }

    queued.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| a.row.id.cmp(&b.row.id))
    });
    blocked.sort_by(|a, b| a.row.id.cmp(&b.row.id));

    let mut lines = vec![
        "# Backlog Execution Path".to_string(),
        String::new(),
        format!("Generated: {generated_at}"),
        String::new(),
        "## Summary".to_string(),
        String::new(),
        format!("- Active rows: {}", queued.len() + blocked.len()),
        format!("- Queued rows: {}", queued.len()),
        format!("- Blocked rows: {}", blocked.len()),
        "- Ordering strategy: impact-first with dependency-valid sequencing.".to_string(),
        String::new(),
        "## Impact + Dependency Execution Order".to_string(),
        String::new(),
    ];

    if queued.is_empty() {
        lines.push("No queued backlog rows remain in this view.".to_string());
    } else {
        lines.push("| Rank | ID | Status | Priority | Open Dependencies | Title |".to_string());
        lines.push("|---|---|---|---:|---|---|".to_string());
        for (idx, item) in queued.iter().enumerate() {
            let deps = if item.open_deps.is_empty() {
                String::new()
            } else {
                item.open_deps.join(", ")
            };
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} |",
                idx + 1,
                item.row.id,
                item.row.status,
                item.priority,
                deps,
                item.row.title
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Deferred / Blocked".to_string());
    lines.push(String::new());
    lines.push("| ID | Class | Status | Block Reason |".to_string());
    lines.push("|---|---|---|---|".to_string());
    for item in blocked {
        let reason = if item.open_deps.is_empty() {
            "Blocked status in SRS".to_string()
        } else {
            format!("Open dependencies: {}", item.open_deps.join(", "))
        };
        lines.push(format!(
            "| {} | {} | {} | {} |",
            item.row.id, item.row.class, item.row.status, reason
        ));
    }

    lines.push(String::new());
    lines.join("\n")
}

fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

fn write_text_atomic(path: &Path, text: &str) -> Result<(), String> {
    ensure_parent(path);
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp, text).map_err(|e| format!("write_tmp_failed:{}", e))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename_tmp_failed:{}", e))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path);
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open_jsonl_failed:{}", e))?;
    let line = serde_json::to_string(value).map_err(|e| format!("encode_jsonl_failed:{}", e))?;
    f.write_all(line.as_bytes())
        .and_then(|_| f.write_all(b"\n"))
        .map_err(|e| format!("append_jsonl_failed:{}", e))
}

fn canonical_rows_hash(rows: &[RegistryRow]) -> String {
    let value = serde_json::to_value(rows).unwrap_or_else(|_| json!([]));
    crate::deterministic_receipt_hash(&value)
}

fn normalize_text_compare(text: &str) -> String {
    text.replace("\r\n", "\n")
        .lines()
        .filter(|line| !line.trim_start().starts_with("Generated: "))
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}
