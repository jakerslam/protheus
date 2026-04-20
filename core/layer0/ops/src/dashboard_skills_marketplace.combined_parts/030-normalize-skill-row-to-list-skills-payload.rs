
fn normalize_skill_row(mut row: Value, fallback_name: &str, source: Value) -> Value {
    let name = clean_text(
        row.get("name")
            .and_then(Value::as_str)
            .unwrap_or(fallback_name),
        120,
    );
    row["name"] = Value::String(name.clone());
    row["description"] = Value::String(clean_text(
        row.get("description")
            .and_then(Value::as_str)
            .unwrap_or("No description provided."),
        300,
    ));
    row["version"] = Value::String(clean_text(
        row.get("version").and_then(Value::as_str).unwrap_or("v1"),
        40,
    ));
    row["author"] = Value::String(clean_text(
        row.get("author")
            .and_then(Value::as_str)
            .unwrap_or("Unknown"),
        120,
    ));
    row["runtime"] = Value::String(clean_text(
        row.get("runtime")
            .and_then(Value::as_str)
            .unwrap_or("prompt_only"),
        40,
    ));
    row["tools_count"] = json!(parse_u64(row.get("tools_count"), 0));
    if !row.get("tags").map(Value::is_array).unwrap_or(false) {
        row["tags"] = Value::Array(default_tags());
    }
    row["enabled"] = Value::Bool(row.get("enabled").and_then(Value::as_bool).unwrap_or(true));
    row["has_prompt_context"] = Value::Bool(
        row.get("has_prompt_context")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    );
    row["source"] = source;
    row
}

fn installed_from_core(root: &Path) -> Vec<Value> {
    let registry =
        read_json(&state_path(root, CORE_SKILLS_REGISTRY_REL)).unwrap_or_else(|| json!({}));
    let installed = registry
        .get("installed")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut rows = installed
        .iter()
        .map(|(name, row)| {
            normalize_skill_row(
                row.clone(),
                name,
                json!({"type":"local","path": row.get("path").cloned().unwrap_or(Value::Null)}),
            )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    rows
}

fn merged_installed_rows(root: &Path) -> Vec<Value> {
    let mut by_name = BTreeMap::<String, Value>::new();
    for row in installed_from_core(root) {
        let key = normalize_name(row.get("name").and_then(Value::as_str).unwrap_or(""));
        if !key.is_empty() {
            by_name.insert(key, row);
        }
    }

    let state = load_dashboard_state(root);
    for section in ["installed", "created"] {
        let rows = state
            .get(section)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (name, row) in rows {
            let key = normalize_name(&name);
            if key.is_empty() {
                continue;
            }
            let source = row
                .get("source")
                .cloned()
                .unwrap_or_else(|| json!({"type":"local"}));
            by_name.insert(key, normalize_skill_row(row, &name, source));
        }
    }

    by_name.values().cloned().collect::<Vec<_>>()
}

pub(super) fn skills_prompt_context(root: &Path, max_skills: usize, max_chars: usize) -> String {
    let mut rows = merged_installed_rows(root);
    rows.sort_by(|a, b| {
        clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    let mut lines = Vec::<String>::new();
    for row in rows {
        if lines.len() >= max_skills {
            break;
        }
        let enabled = row.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        if !enabled {
            continue;
        }
        let context = clean_text(
            row.get("prompt_context")
                .and_then(Value::as_str)
                .unwrap_or(""),
            1200,
        );
        if context.is_empty() {
            continue;
        }
        let name = clean_text(
            row.get("name").and_then(Value::as_str).unwrap_or("plugin"),
            120,
        );
        lines.push(format!("- {name}: {context}"));
    }
    if lines.is_empty() {
        return String::new();
    }
    let text = format!(
        "Installed plugin context (apply naturally when relevant):\n{}",
        lines.join("\n")
    );
    text.chars().take(max_chars).collect::<String>()
}

fn marketplace_catalog() -> Vec<Value> {
    vec![
        json!({"slug":"repo-architect","name":"repo-architect","title":"Repo Architect","description":"Deep repository navigation and refactor planning agent skill.","author":"Infring","runtime":"prompt_only","tags":["coding","architecture"],"downloads":7421,"stars":1294,"updated_at":"2026-03-20T00:00:00Z","source":{"type":"clawhub","slug":"repo-architect"},"prompt_context":"Plan safe, incremental repository refactors with risk and rollback awareness."}),
        json!({"slug":"incident-commander","name":"incident-commander","title":"Incident Commander","description":"Operational incident triage and mitigation playbook automation.","author":"Infring","runtime":"prompt_only","tags":["devops","reliability"],"downloads":6200,"stars":1112,"updated_at":"2026-03-22T00:00:00Z","source":{"type":"clawhub","slug":"incident-commander"},"prompt_context":"Triage incidents, prioritize mitigation, and maintain operator-ready runbooks."}),
        json!({"slug":"whatsapp-bridge","name":"whatsapp-bridge","title":"WhatsApp Bridge","description":"Channel bridge for WhatsApp workflows and escalation routing.","author":"Infring","runtime":"node","tags":["communication","messaging"],"downloads":5902,"stars":980,"updated_at":"2026-03-19T00:00:00Z","source":{"type":"clawhub","slug":"whatsapp-bridge"},"prompt_context":"Bridge inbound and outbound WhatsApp workflows with strict audit receipts."}),
        json!({"slug":"slack-warroom","name":"slack-warroom","title":"Slack Warroom","description":"Coordinate incidents and launches in Slack with structured updates.","author":"Infring","runtime":"node","tags":["communication","devops"],"downloads":5544,"stars":932,"updated_at":"2026-03-18T00:00:00Z","source":{"type":"clawhub","slug":"slack-warroom"},"prompt_context":"Drive war-room workflows in Slack with concise, timestamped decision logs."}),
        json!({"slug":"signal-sentry","name":"signal-sentry","title":"Signal Sentry","description":"Signal channel adapter and high-priority alert fanout skill.","author":"Infring","runtime":"node","tags":["communication","security"],"downloads":5120,"stars":854,"updated_at":"2026-03-17T00:00:00Z","source":{"type":"clawhub","slug":"signal-sentry"},"prompt_context":"Relay critical security and reliability alerts into Signal threads."}),
        json!({"slug":"model-router-pro","name":"model-router-pro","title":"Model Router Pro","description":"Provider-agnostic model routing based on scope, latency, and budget.","author":"Infring","runtime":"prompt_only","tags":["ai","routing"],"downloads":8420,"stars":1543,"updated_at":"2026-03-24T00:00:00Z","source":{"type":"clawhub","slug":"model-router-pro"},"prompt_context":"Route tasks across local/cloud models by complexity and cost ceilings."}),
        json!({"slug":"code-audit-pack","name":"code-audit-pack","title":"Code Audit Pack","description":"Security and regression audit checklist for large codebases.","author":"Infring","runtime":"prompt_only","tags":["coding","security"],"downloads":7310,"stars":1381,"updated_at":"2026-03-23T00:00:00Z","source":{"type":"clawhub","slug":"code-audit-pack"},"prompt_context":"Perform high-signal code audits with severity-ranked findings and remediation plans."}),
        json!({"slug":"release-captain","name":"release-captain","title":"Release Captain","description":"Release orchestration, changelog synthesis, and rollback planning.","author":"Infring","runtime":"prompt_only","tags":["devops","release"],"downloads":4888,"stars":811,"updated_at":"2026-03-21T00:00:00Z","source":{"type":"clawhub","slug":"release-captain"},"prompt_context":"Coordinate release readiness checks, rollout, and rollback plans."}),
        json!({"slug":"conduit-optimizer","name":"conduit-optimizer","title":"Conduit Optimizer","description":"Analyze queue pressure and recommend conduit scaling remediations.","author":"Infring","runtime":"prompt_only","tags":["ops","reliability"],"downloads":5333,"stars":902,"updated_at":"2026-03-22T00:00:00Z","source":{"type":"clawhub","slug":"conduit-optimizer"},"prompt_context":"Tune queue, conduit, and cockpit pressure with deterministic remediation guidance."}),
        json!({"slug":"docs-distiller","name":"docs-distiller","title":"Docs Distiller","description":"Condense long docs into implementation-grade summaries.","author":"Infring","runtime":"prompt_only","tags":["docs","productivity"],"downloads":4511,"stars":776,"updated_at":"2026-03-20T00:00:00Z","source":{"type":"clawhub","slug":"docs-distiller"},"prompt_context":"Extract requirements and decisions from long documentation without losing constraints."}),
        json!({"slug":"browser-runner","name":"browser-runner","title":"Browser Runner","description":"Browser automation workflows with policy gates and approvals.","author":"Infring","runtime":"node","tags":["browser","automation"],"downloads":6922,"stars":1205,"updated_at":"2026-03-23T00:00:00Z","source":{"type":"clawhub","slug":"browser-runner"},"prompt_context":"Automate browser tasks with receipts and explicit approval checkpoints."}),
        json!({"slug":"research-deepdive","name":"research-deepdive","title":"Research Deepdive","description":"Structured research synthesis with source confidence tagging.","author":"Infring","runtime":"prompt_only","tags":["research","ai"],"downloads":5833,"stars":1009,"updated_at":"2026-03-21T00:00:00Z","source":{"type":"clawhub","slug":"research-deepdive"},"prompt_context":"Deliver concise, source-grounded research outputs with explicit confidence labels."}),
    ]
}

fn paginate(mut rows: Vec<Value>, query: &Map<String, Value>) -> Value {
    let limit = parse_u64(query.get("limit"), 20).clamp(1, 50) as usize;
    let cursor = parse_u64(query.get("cursor"), 0) as usize;
    let total = rows.len();
    if cursor >= rows.len() {
        rows.clear();
    } else {
        rows = rows
            .into_iter()
            .skip(cursor)
            .take(limit)
            .collect::<Vec<_>>();
    }
    let next = cursor.saturating_add(limit);
    json!({
        "ok": true,
        "items": rows,
        "next_cursor": if next < total { Value::String(next.to_string()) } else { Value::Null }
    })
}

fn list_skills_payload(root: &Path) -> Value {
    json!({"ok": true, "skills": merged_installed_rows(root)})
}
