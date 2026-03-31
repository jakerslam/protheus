pub fn compute_read_json(input: &ReadJsonInput) -> ReadJsonOutput {
    let fallback = input.fallback.clone().unwrap_or(Value::Null);
    let file_path = input.file_path.as_deref().unwrap_or("").trim();
    if file_path.is_empty() {
        return ReadJsonOutput { value: fallback };
    }
    let path = Path::new(file_path);
    if !path.exists() {
        return ReadJsonOutput { value: fallback };
    }
    let text = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return ReadJsonOutput { value: fallback },
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(v) => {
            if v.is_null() {
                ReadJsonOutput { value: fallback }
            } else {
                ReadJsonOutput { value: v }
            }
        }
        Err(_) => ReadJsonOutput { value: fallback },
    }
}
pub fn compute_read_jsonl(input: &ReadJsonlInput) -> ReadJsonlOutput {
    let file_path = input.file_path.as_deref().unwrap_or("").trim();
    if file_path.is_empty() {
        return ReadJsonlOutput { rows: Vec::new() };
    }
    let path = Path::new(file_path);
    if !path.exists() {
        return ReadJsonlOutput { rows: Vec::new() };
    }
    let text = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return ReadJsonlOutput { rows: Vec::new() },
    };
    let rows = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|row| row.is_object())
        .collect::<Vec<_>>();
    ReadJsonlOutput { rows }
}

pub fn compute_write_json_atomic(input: &WriteJsonAtomicInput) -> WriteJsonAtomicOutput {
    let file_path = input.file_path.as_deref().unwrap_or("").trim();
    if file_path.is_empty() {
        return WriteJsonAtomicOutput { ok: true };
    }
    let value = input.value.clone().unwrap_or(Value::Null);
    let path = Path::new(file_path);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let tmp_path = format!(
        "{}.tmp-{}-{}",
        file_path,
        chrono::Utc::now().timestamp_millis(),
        std::process::id()
    );
    let payload = format!(
        "{}\n",
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| "null".to_string())
    );
    let _ = fs::write(&tmp_path, payload);
    let _ = fs::rename(&tmp_path, file_path);
    WriteJsonAtomicOutput { ok: true }
}

pub fn compute_append_jsonl(input: &AppendJsonlInput) -> AppendJsonlOutput {
    let file_path = input.file_path.as_deref().unwrap_or("").trim();
    if file_path.is_empty() {
        return AppendJsonlOutput { ok: true };
    }
    let row = input.row.clone().unwrap_or(Value::Null);
    let path = Path::new(file_path);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let line = format!(
        "{}\n",
        serde_json::to_string(&row).unwrap_or_else(|_| "null".to_string())
    );
    let mut opts = fs::OpenOptions::new();
    opts.create(true).append(true);
    if let Ok(mut file) = opts.open(path) {
        let _ = std::io::Write::write_all(&mut file, line.as_bytes());
    }
    AppendJsonlOutput { ok: true }
}

pub fn compute_read_text(input: &ReadTextInput) -> ReadTextOutput {
    let fallback = input.fallback.clone().unwrap_or_default();
    let file_path = input.file_path.as_deref().unwrap_or("").trim();
    if file_path.is_empty() {
        return ReadTextOutput { text: fallback };
    }
    let path = Path::new(file_path);
    if !path.exists() {
        return ReadTextOutput { text: fallback };
    }
    let text = fs::read_to_string(path).unwrap_or_else(|_| fallback.clone());
    ReadTextOutput { text }
}

pub fn compute_latest_json_file_in_dir(
    input: &LatestJsonFileInDirInput,
) -> LatestJsonFileInDirOutput {
    let dir = input.dir_path.as_deref().unwrap_or("").trim();
    if dir.is_empty() {
        return LatestJsonFileInDirOutput { file_path: None };
    }
    let dir_path = Path::new(dir);
    if !dir_path.exists() {
        return LatestJsonFileInDirOutput { file_path: None };
    }
    let mut latest_path: Option<PathBuf> = None;
    let mut latest_millis: i128 = i128::MIN;
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            let Ok(elapsed) = modified.elapsed() else {
                continue;
            };
            let score = -(elapsed.as_millis() as i128);
            if score > latest_millis {
                latest_millis = score;
                latest_path = Some(p);
            }
        }
    }
    LatestJsonFileInDirOutput {
        file_path: latest_path.map(|p| p.to_string_lossy().to_string()),
    }
}

pub fn compute_normalize_output_channel(
    input: &NormalizeOutputChannelInput,
) -> NormalizeOutputChannelOutput {
    let base = input.base_out.as_ref();
    let src = input.src_out.as_ref();
    NormalizeOutputChannelOutput {
        enabled: to_bool_like(
            src.and_then(|v| v.as_object())
                .and_then(|m| m.get("enabled")),
            map_bool_key(base, "enabled", false),
        ),
        live_enabled: to_bool_like(
            src.and_then(|v| v.as_object())
                .and_then(|m| m.get("live_enabled")),
            map_bool_key(base, "live_enabled", false),
        ),
        test_enabled: to_bool_like(
            src.and_then(|v| v.as_object())
                .and_then(|m| m.get("test_enabled")),
            map_bool_key(base, "test_enabled", false),
        ),
        require_sandbox_verification: to_bool_like(
            src.and_then(|v| v.as_object())
                .and_then(|m| m.get("require_sandbox_verification")),
            map_bool_key(base, "require_sandbox_verification", false),
        ),
        require_explicit_emit: to_bool_like(
            src.and_then(|v| v.as_object())
                .and_then(|m| m.get("require_explicit_emit")),
            map_bool_key(base, "require_explicit_emit", false),
        ),
    }
}

pub fn compute_normalize_repo_path(input: &NormalizeRepoPathInput) -> NormalizeRepoPathOutput {
    let fallback = input.fallback.as_deref().unwrap_or("").to_string();
    let raw = clean_text_runtime(input.value.as_deref().unwrap_or(""), 420);
    if raw.is_empty() {
        return NormalizeRepoPathOutput { path: fallback };
    }
    let path = Path::new(&raw);
    if path.is_absolute() {
        return NormalizeRepoPathOutput {
            path: raw.to_string(),
        };
    }
    let root = input.root.as_deref().unwrap_or("");
    let joined = Path::new(root).join(raw);
    NormalizeRepoPathOutput {
        path: joined.to_string_lossy().to_string(),
    }
}

pub fn compute_runtime_paths(input: &RuntimePathsInput) -> RuntimePathsOutput {
    let root = input.root.as_deref().unwrap_or("");
    let default_state_dir = input.default_state_dir.as_deref().unwrap_or("");
    let policy_path = input.policy_path.as_deref().unwrap_or("").to_string();
    let state_dir = {
        let env = input
            .inversion_state_dir_env
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string();
        if env.is_empty() {
            default_state_dir.to_string()
        } else if Path::new(&env).is_absolute() {
            env
        } else {
            Path::new(root).join(env).to_string_lossy().to_string()
        }
    };
    let dual_brain_policy_path = {
        let env = input
            .dual_brain_policy_path_env
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string();
        if env.is_empty() {
            Path::new(root)
                .join("config")
                .join("dual_brain_policy.json")
                .to_string_lossy()
                .to_string()
        } else if Path::new(&env).is_absolute() {
            env
        } else {
            Path::new(root).join(env).to_string_lossy().to_string()
        }
    };

    let mk = |parts: &[&str]| -> String {
        let mut p = PathBuf::from(&state_dir);
        for part in parts {
            p = p.join(part);
        }
        p.to_string_lossy().to_string()
    };

    RuntimePathsOutput {
        paths: json!({
            "policy_path": policy_path,
            "state_dir": state_dir,
            "latest_path": mk(&["latest.json"]),
            "history_path": mk(&["history.jsonl"]),
            "maturity_path": mk(&["maturity.json"]),
            "tier_governance_path": mk(&["tier_governance.json"]),
            "observer_approvals_path": mk(&["observer_approvals.jsonl"]),
            "harness_state_path": mk(&["maturity_harness.json"]),
            "active_sessions_path": mk(&["active_sessions.json"]),
            "library_path": mk(&["library.jsonl"]),
            "receipts_path": mk(&["receipts.jsonl"]),
            "first_principles_dir": mk(&["first_principles"]),
            "first_principles_latest_path": mk(&["first_principles", "latest.json"]),
            "first_principles_history_path": mk(&["first_principles", "history.jsonl"]),
            "first_principles_lock_path": mk(&["first_principles", "lock_state.json"]),
            "code_change_proposals_dir": mk(&["code_change_proposals"]),
            "code_change_proposals_latest_path": mk(&["code_change_proposals", "latest.json"]),
            "code_change_proposals_history_path": mk(&["code_change_proposals", "history.jsonl"]),
            "organ_dir": mk(&["organ"]),
            "organ_latest_path": mk(&["organ", "latest.json"]),
            "organ_history_path": mk(&["organ", "history.jsonl"]),
            "tree_latest_path": mk(&["tree", "latest.json"]),
            "tree_history_path": mk(&["tree", "history.jsonl"]),
            "interfaces_dir": mk(&["interfaces"]),
            "interfaces_latest_path": mk(&["interfaces", "latest.json"]),
            "interfaces_history_path": mk(&["interfaces", "history.jsonl"]),
            "events_dir": mk(&["events"]),
            "dual_brain_policy_path": dual_brain_policy_path
        }),
    }
}

pub fn compute_normalize_axiom_list(input: &NormalizeAxiomListInput) -> NormalizeAxiomListOutput {
    let src = input
        .raw_axioms
        .as_ref()
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let fallback = input
        .base_axioms
        .as_ref()
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let rows = if src.is_empty() { fallback } else { src };
    let mut out = Vec::new();
    for row in rows {
        let item = row.as_object();
        let id = normalize_token_runtime(&value_to_string(item.and_then(|m| m.get("id"))), 80);
        let patterns = item
            .and_then(|m| m.get("patterns"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|x| clean_text_runtime(&value_to_string(Some(x)), 140).to_lowercase())
                    .filter(|x| !x.is_empty())
                    .take(20)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let regex = item
            .and_then(|m| m.get("regex"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|x| clean_text_runtime(&value_to_string(Some(x)), 220))
                    .filter(|x| !x.is_empty())
                    .take(20)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let intent_tags = compute_normalize_list(&NormalizeListInput {
            value: item.and_then(|m| m.get("intent_tags")).cloned(),
            max_len: Some(80),
        })
        .items
        .into_iter()
        .take(24)
        .collect::<Vec<_>>();

        let signals = item
            .and_then(|m| m.get("signals"))
            .and_then(|v| v.as_object());
        let action_terms = signals
            .and_then(|m| m.get("action_terms"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|x| clean_text_runtime(&value_to_string(Some(x)), 80).to_lowercase())
                    .filter(|x| !x.is_empty())
                    .take(24)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let subject_terms = signals
            .and_then(|m| m.get("subject_terms"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|x| clean_text_runtime(&value_to_string(Some(x)), 80).to_lowercase())
                    .filter(|x| !x.is_empty())
                    .take(24)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let object_terms = signals
            .and_then(|m| m.get("object_terms"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|x| clean_text_runtime(&value_to_string(Some(x)), 80).to_lowercase())
                    .filter(|x| !x.is_empty())
                    .take(24)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let default_groups = (if !action_terms.is_empty() { 1 } else { 0 })
            + (if !subject_terms.is_empty() { 1 } else { 0 })
            + (if !object_terms.is_empty() { 1 } else { 0 });
        let min_signal_groups = parse_number_like(item.and_then(|m| m.get("min_signal_groups")))
            .unwrap_or(default_groups as f64)
            .floor() as i64;
        let min_signal_groups = min_signal_groups.clamp(0, 3);

        let semantic_req = item
            .and_then(|m| m.get("semantic_requirements"))
            .and_then(|v| v.as_object());
        let semantic_actions = compute_normalize_list(&NormalizeListInput {
            value: semantic_req.and_then(|m| m.get("actions")).cloned(),
            max_len: Some(80),
        })
        .items
        .into_iter()
        .take(24)
        .collect::<Vec<_>>();
        let semantic_subjects = compute_normalize_list(&NormalizeListInput {
            value: semantic_req.and_then(|m| m.get("subjects")).cloned(),
            max_len: Some(80),
        })
        .items
        .into_iter()
        .take(24)
        .collect::<Vec<_>>();
        let semantic_objects = compute_normalize_list(&NormalizeListInput {
            value: semantic_req.and_then(|m| m.get("objects")).cloned(),
            max_len: Some(80),
        })
        .items
        .into_iter()
        .take(24)
        .collect::<Vec<_>>();
        let has_semantic_requirements = !semantic_actions.is_empty()
            || !semantic_subjects.is_empty()
            || !semantic_objects.is_empty();

        if id.is_empty()
            || (patterns.is_empty()
                && regex.is_empty()
                && intent_tags.is_empty()
                && !has_semantic_requirements)
        {
            continue;
        }

        out.push(json!({
            "id": id,
            "patterns": patterns,
            "regex": regex,
            "intent_tags": intent_tags,
            "signals": {
                "action_terms": action_terms,
                "subject_terms": subject_terms,
                "object_terms": object_terms
            },
            "min_signal_groups": min_signal_groups,
            "semantic_requirements": {
                "actions": semantic_actions,
                "subjects": semantic_subjects,
                "objects": semantic_objects
            }
        }));
    }
    NormalizeAxiomListOutput { axioms: out }
}
