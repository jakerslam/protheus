
#[derive(Debug, Clone)]
struct ActiveDirectiveRow {
    id: String,
    tier: i64,
    status: String,
    reason: String,
    auto_generated: bool,
    parent_directive_id: String,
}

fn directive_hierarchy_paths(repo_root: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let directives_dir = runtime_config_path(repo_root, "directives");
    let active_path = directives_dir.join("ACTIVE.yaml");
    let strategies_dir = runtime_config_path(repo_root, "strategies");
    let audit_path = runtime_state_root(repo_root)
        .join("security")
        .join("directive_hierarchy_audit.jsonl");
    (directives_dir, active_path, strategies_dir, audit_path)
}

fn directive_tier_from_id(id: &str) -> i64 {
    let clean = id.trim();
    if !clean.starts_with('T') {
        return 99;
    }
    let rest = &clean[1..];
    let mut digits = String::new();
    for ch in rest.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            break;
        }
    }
    digits.parse::<i64>().unwrap_or(99)
}

fn normalize_directive_id(v: &str) -> String {
    let text = clean_text(v, 160);
    let mut chars = text.chars();
    if chars.next() != Some('T') {
        return String::new();
    }
    let mut digit_count = 0usize;
    let mut seen_underscore = false;
    for ch in text.chars().skip(1) {
        if !seen_underscore && ch.is_ascii_digit() {
            digit_count += 1;
            continue;
        }
        if ch == '_' {
            if seen_underscore || digit_count == 0 {
                return String::new();
            }
            seen_underscore = true;
            continue;
        }
        let ok = ch.is_ascii_alphanumeric() || ch == '_';
        if !ok {
            return String::new();
        }
    }
    if !seen_underscore {
        return String::new();
    }
    text
}

fn parse_active_yaml(path: &Path) -> Vec<ActiveDirectiveRow> {
    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut rows = Vec::<ActiveDirectiveRow>::new();
    let mut cur: Option<ActiveDirectiveRow> = None;
    for line in raw.lines() {
        let t = line.trim();
        if let Some(value) = t.strip_prefix("- id:") {
            if let Some(prev) = cur.take() {
                rows.push(prev);
            }
            cur = Some(ActiveDirectiveRow {
                id: normalize_directive_id(value.trim().trim_matches('"').trim_matches('\'')),
                tier: 0,
                status: "active".to_string(),
                reason: String::new(),
                auto_generated: false,
                parent_directive_id: String::new(),
            });
            continue;
        }
        if !t.starts_with(|c: char| c.is_ascii_alphabetic()) {
            continue;
        }
        if let Some(row) = cur.as_mut() {
            if let Some(v) = t.strip_prefix("id:") {
                row.id = normalize_directive_id(v.trim().trim_matches('"').trim_matches('\''));
            } else if let Some(v) = t.strip_prefix("tier:") {
                row.tier = v
                    .trim()
                    .parse::<i64>()
                    .unwrap_or_else(|_| directive_tier_from_id(&row.id));
            } else if let Some(v) = t.strip_prefix("status:") {
                row.status = normalize_token(v, 40);
            } else if let Some(v) = t.strip_prefix("reason:") {
                row.reason = clean_text(v.trim().trim_matches('"').trim_matches('\''), 280);
            } else if let Some(v) = t.strip_prefix("auto_generated:") {
                row.auto_generated = bool_from_str(Some(v.trim()), false);
            } else if let Some(v) = t.strip_prefix("parent_directive_id:") {
                row.parent_directive_id =
                    normalize_directive_id(v.trim().trim_matches('"').trim_matches('\''));
            }
        }
    }
    if let Some(prev) = cur.take() {
        rows.push(prev);
    }
    let mut seen_ids = HashSet::<String>::new();
    rows.into_iter()
        .filter(|row| !row.id.is_empty())
        .filter_map(|mut row| {
            if row.tier <= 0 {
                row.tier = directive_tier_from_id(&row.id);
            }
            if row.status.is_empty() {
                row.status = "active".to_string();
            }
            if row.parent_directive_id == row.id {
                row.parent_directive_id.clear();
            }
            if !row.parent_directive_id.is_empty()
                && directive_tier_from_id(&row.parent_directive_id) >= row.tier
            {
                row.parent_directive_id.clear();
            }
            if !seen_ids.insert(row.id.clone()) {
                return None;
            }
            Some(row)
        })
        .collect::<Vec<_>>()
}

fn render_active_yaml(rows: &[ActiveDirectiveRow]) -> String {
    let mut out = Vec::new();
    out.push("metadata:".to_string());
    out.push(format!("  updated_at: \"{}\"", now_iso()));
    out.push("active_directives:".to_string());
    for row in rows {
        out.push(format!("  - id: {}", row.id));
        out.push(format!("    tier: {}", row.tier));
        out.push(format!(
            "    status: {}",
            if row.status.is_empty() {
                "active"
            } else {
                &row.status
            }
        ));
        if !row.reason.is_empty() {
            out.push(format!("    reason: \"{}\"", row.reason.replace('"', "'")));
        }
        if row.auto_generated {
            out.push("    auto_generated: true".to_string());
        }
        if !row.parent_directive_id.is_empty() {
            out.push(format!(
                "    parent_directive_id: {}",
                row.parent_directive_id
            ));
        }
    }
    out.push(String::new());
    out.join("\n")
}

fn directive_child_id(
    parent_id: &str,
    tier: i64,
    kind: &str,
    existing: &HashSet<String>,
) -> String {
    let base = parent_id
        .split_once('_')
        .map(|(_, b)| b.to_string())
        .unwrap_or_else(|| "parent".to_string());
    let stem = normalize_token(base, 120);
    let candidate = format!("T{}_{}_{}_auto", tier, stem, kind);
    if !existing.contains(&candidate) {
        return candidate;
    }
    let mut i = 2_u64;
    loop {
        let next = format!("{candidate}_{i}");
        if !existing.contains(&next) {
            return next;
        }
        i += 1;
    }
}

fn write_child_directive_file(
    path: &Path,
    child_id: &str,
    parent_id: &str,
    kind: &str,
) -> Result<(), String> {
    ensure_parent(path)?;
    let body = format!(
        "id: {child_id}\n\
tier: {}\n\
status: active\n\
metadata:\n\
  parent_directive_id: {parent_id}\n\
  decomposition_kind: {kind}\n\
  auto_generated: true\n\
  created_at: \"{}\"\n\
summary: \"Auto-generated child directive ({kind}) from {parent_id}.\"\n\
risk_limits:\n\
  max_cost_usd: 100\n\
  max_token_usage: 1500\n\
success_criteria:\n\
  - \"Demonstrate measurable progress toward parent objective\"\n\
",
        directive_tier_from_id(child_id),
        now_iso()
    );
    fs::write(path, body)
        .map_err(|err| format!("write_child_directive_failed:{}:{err}", path.display()))
}

fn load_strategy_conflicts(strategies_dir: &Path, parent_id: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(strategies_dir) {
        Ok(v) => v,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        let row = read_json_or(&p, json!({}));
        if row
            .get("status")
            .and_then(Value::as_str)
            .map(|v| v.eq_ignore_ascii_case("active"))
            .unwrap_or(true)
            == false
        {
            continue;
        }
        let blocked = row
            .get("admission_policy")
            .and_then(|v| v.get("blocked_types"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(|v| v.to_ascii_lowercase())
            .collect::<Vec<_>>();
        if blocked.iter().any(|v| v == "directive_decomposition") {
            out.push(json!({
                "strategy_id": row.get("id").cloned().unwrap_or(Value::String(
                    p.file_stem().and_then(|v| v.to_str()).unwrap_or("unknown").to_string()
                )),
                "reason": "strategy_blocks_directive_decomposition",
                "directive_id": parent_id
            }));
        }
    }
    out
}

pub fn run_directive_hierarchy_controller(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let args = parse_cli_args(argv);
    let cmd = args
        .positional
        .first()
        .map(|v| normalize_token(v, 80))
        .unwrap_or_else(|| "status".to_string());
    let (directives_dir, active_path, strategies_dir, audit_path) =
        directive_hierarchy_paths(repo_root);
    let rows = parse_active_yaml(&active_path);

    if cmd == "status" {
        let filter_id = args
            .flags
            .get("id")
            .map(|v| normalize_directive_id(v))
            .unwrap_or_default();
        let records = if filter_id.is_empty() {
            rows.clone()
        } else {
            rows.into_iter()
                .filter(|row| row.id == filter_id || row.parent_directive_id == filter_id)
                .collect::<Vec<_>>()
        };
        return (
            json!({
                "ok": true,
                "type": "directive_hierarchy_status",
                "ts": now_iso(),
                "count": records.len(),
                "records": records
                    .iter()
                    .map(|row| {
                        json!({
                            "id": row.id,
                            "tier": row.tier,
                            "status": row.status,
                            "reason": row.reason,
                            "auto_generated": row.auto_generated,
                            "parent_directive_id": row.parent_directive_id
                        })
                    })
                    .collect::<Vec<_>>()
            }),
            0,
        );
    }

    if cmd != "decompose" {
        return (
            json!({
                "ok": false,
                "type": "directive_hierarchy_error",
                "reason": format!("unknown_command:{cmd}")
            }),
            2,
        );
    }

    let parent_id = args
        .flags
        .get("id")
        .map(|v| normalize_directive_id(v))
        .unwrap_or_default();
    if parent_id.is_empty() {
        return (
            json!({
                "ok": false,
                "type": "directive_hierarchy_decompose",
                "reason": "missing_or_invalid_parent_id"
            }),
            2,
        );
    }
    let apply = bool_from_str(args.flags.get("apply").map(String::as_str), false);
    let dry_run = bool_from_str(args.flags.get("dry-run").map(String::as_str), false);

    let parent = rows
        .iter()
        .find(|row| row.id == parent_id && row.status == "active")
        .cloned();
    if parent.is_none() {
        return (
            json!({
                "ok": false,
                "type": "directive_hierarchy_decompose",
                "reason": "parent_not_active_or_missing",
                "parent_id": parent_id
            }),
            1,
        );
    }
    let parent = parent.unwrap_or(ActiveDirectiveRow {
        id: parent_id.clone(),
        tier: 1,
        status: "active".to_string(),
        reason: String::new(),
        auto_generated: false,
        parent_directive_id: String::new(),
    });
    let min_tier = std::env::var("DIRECTIVE_DECOMPOSE_PARENT_MIN_TIER")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1);
    let max_tier = std::env::var("DIRECTIVE_DECOMPOSE_PARENT_MAX_TIER")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1);
    if parent.tier < min_tier || parent.tier > max_tier {
        return (
            json!({
                "ok": false,
                "type": "directive_hierarchy_decompose",
                "reason": "parent_tier_out_of_bounds",
                "parent_tier": parent.tier
            }),
            1,
        );
    }

    let conflicts = load_strategy_conflicts(&strategies_dir, &parent_id);
    if !conflicts.is_empty() {
        return (
            json!({
                "ok": false,
                "type": "directive_hierarchy_decompose",
                "reason": "campaign_conflict",
                "conflicts": conflicts
            }),
            1,
        );
    }

    let mut existing_ids = rows
        .iter()
        .map(|row| row.id.clone())
        .collect::<HashSet<_>>();
    let active_children = rows
        .iter()
        .filter(|row| row.parent_directive_id == parent_id && row.status == "active")
        .cloned()
        .collect::<Vec<_>>();
    let has_plan = active_children.iter().any(|row| row.id.contains("_plan_"));
    let has_execute = active_children
        .iter()
        .any(|row| row.id.contains("_execute_") || row.id.contains("_execution_"));

    let child_tier = parent.tier + 1;
    let mut generated = Vec::<Value>::new();
    if !has_plan {
        let id = directive_child_id(&parent_id, child_tier, "plan", &existing_ids);
        existing_ids.insert(id.clone());
        generated.push(json!({
            "id": id,
            "kind": "plan"
        }));
    }
    if !has_execute {
        let id = directive_child_id(&parent_id, child_tier, "execute", &existing_ids);
        existing_ids.insert(id.clone());
        generated.push(json!({
            "id": id,
            "kind": "execute"
        }));
    }

    let mut next_rows = rows.clone();
    if apply && !dry_run {
        for row in &generated {
            let child_id = row.get("id").and_then(Value::as_str).unwrap_or("");
            let kind = row.get("kind").and_then(Value::as_str).unwrap_or("plan");
            if child_id.is_empty() {
                continue;
            }
            let file_path = directives_dir.join(format!("{child_id}.yaml"));
            if let Err(err) = write_child_directive_file(&file_path, child_id, &parent_id, kind) {
                return (
                    json!({
                        "ok": false,
                        "type": "directive_hierarchy_decompose",
                        "reason": format!("write_child_failed:{err}")
                    }),
                    1,
                );
            }
            next_rows.push(ActiveDirectiveRow {
                id: child_id.to_string(),
                tier: child_tier,
                status: "active".to_string(),
                reason: format!("auto_decomposed_from:{parent_id}"),
                auto_generated: true,
                parent_directive_id: parent_id.clone(),
            });
        }
        let rendered = render_active_yaml(&next_rows);
        if let Err(err) = ensure_parent(&active_path).and_then(|_| {
            fs::write(&active_path, rendered)
                .map_err(|e| format!("write_active_yaml_failed:{}:{e}", active_path.display()))
        }) {
            return (
                json!({
                    "ok": false,
                    "type": "directive_hierarchy_decompose",
                    "reason": err
                }),
                1,
            );
        }
    }

    let out = json!({
        "ok": true,
        "type": "directive_hierarchy_decompose",
        "ts": now_iso(),
        "parent_id": parent_id,
        "dry_run": dry_run,
        "applied": apply && !dry_run,
        "generated": generated,
        "generated_count": generated.len(),
        "existing_children_count": active_children.len(),
        "result": if generated.is_empty() { "no_change" } else { "decomposed" }
    });
    let _ = append_jsonl(&audit_path, &out);
    (out, 0)
}

// -------------------------------------------------------------------------------------------------
// Truth-Seeking Rule Gate
// -------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct TruthGateIdentityBinding {
    required: bool,
}

impl Default for TruthGateIdentityBinding {
    fn default() -> Self {
        Self { required: true }
    }
}
