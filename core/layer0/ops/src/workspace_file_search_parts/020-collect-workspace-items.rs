
fn collect_workspace_items(
    workspace: &WorkspaceSpec,
    fetch_limit: usize,
) -> Result<Vec<SearchItem>, String> {
    let rg_binary = std::env::var("PROTHEUS_RG_BINARY")
        .ok()
        .map(|row| row.trim().to_string())
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| "rg".to_string());
    let output = Command::new(&rg_binary)
        .arg("--files")
        .arg("--follow")
        .arg("--hidden")
        .arg("-g")
        .arg("!**/{node_modules,.git,.github,out,dist,__pycache__,.venv,.env,venv,env,.cache,tmp,temp}/**")
        .current_dir(&workspace.path)
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                format!(
                    "workspace_file_scan_failed:rg_not_found:{}:install_hint={}",
                    rg_binary,
                    ripgrep_install_hint()
                )
            } else {
                format!("workspace_file_scan_failed:{err}")
            }
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("workspace_file_scan_failed:{stderr}"));
    }

    let mut out = Vec::<SearchItem>::new();
    let mut dirs = BTreeSet::<String>::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if out.len() >= fetch_limit {
            break;
        }
        let rel = normalize_rel_path(line);
        if rel.is_empty() {
            continue;
        }
        out.push(SearchItem {
            workspace_name: workspace.name.clone(),
            path: rel.clone(),
            item_type: "file".to_string(),
            label: Path::new(&rel)
                .file_name()
                .and_then(|row| row.to_str())
                .map(|row| crate::clean(row, 240))
                .unwrap_or_else(|| rel.clone()),
            score_span: None,
            score_gaps: None,
        });
        let mut cursor = Path::new(&rel).parent().map(|row| row.to_path_buf());
        while let Some(parent) = cursor {
            if parent.as_os_str().is_empty() || parent == Path::new(".") {
                break;
            }
            let parent_rel = normalize_rel_path(parent.to_string_lossy().as_ref());
            if parent_rel.is_empty() {
                break;
            }
            dirs.insert(parent_rel);
            cursor = parent.parent().map(|row| row.to_path_buf());
        }
    }

    out.extend(dirs.into_iter().map(|dir| {
        SearchItem {
            workspace_name: workspace.name.clone(),
            label: Path::new(&dir)
                .file_name()
                .and_then(|row| row.to_str())
                .map(|row| crate::clean(row, 240))
                .unwrap_or_else(|| dir.clone()),
            path: dir,
            item_type: "folder".to_string(),
            score_span: None,
            score_gaps: None,
        }
    }));
    Ok(out)
}

fn subsequence_gap_score(query: &str, candidate: &str) -> Option<(usize, usize)> {
    let q_chars = query.to_ascii_lowercase().chars().collect::<Vec<_>>();
    if q_chars.is_empty() {
        return Some((0, 0));
    }
    let c_chars = candidate.to_ascii_lowercase().chars().collect::<Vec<_>>();
    let mut positions = Vec::<usize>::with_capacity(q_chars.len());
    let mut cursor = 0usize;
    for q in q_chars {
        let mut found = None;
        while cursor < c_chars.len() {
            if c_chars[cursor] == q {
                found = Some(cursor);
                cursor += 1;
                break;
            }
            cursor += 1;
        }
        match found {
            Some(pos) => positions.push(pos),
            None => return None,
        }
    }
    let mut gaps = 0usize;
    for pair in positions.windows(2) {
        if pair[1] > pair[0] + 1 {
            gaps += 1;
        }
    }
    let span = positions.last().copied().unwrap_or(0) - positions.first().copied().unwrap_or(0) + 1;
    Some((gaps, span))
}

fn item_type_matches(item: &SearchItem, selected: &str) -> bool {
    if selected.is_empty() {
        return true;
    }
    item.item_type.eq_ignore_ascii_case(selected)
}

fn search_items(
    items: &[SearchItem],
    query: &str,
    selected_type: &str,
    limit: usize,
) -> Vec<SearchItem> {
    let query_clean = crate::clean(query, 200).to_ascii_lowercase();
    let mut filtered = Vec::<SearchItem>::new();
    for item in items {
        if !item_type_matches(item, selected_type) {
            continue;
        }
        if query_clean.is_empty() {
            filtered.push(item.clone());
            continue;
        }
        let candidate = format!("{} {} {}", item.label, item.label, item.path);
        if let Some((gaps, span)) = subsequence_gap_score(&query_clean, &candidate) {
            let mut row = item.clone();
            row.score_gaps = Some(gaps);
            row.score_span = Some(span);
            filtered.push(row);
        }
    }

    filtered.sort_by(|a, b| {
        let ag = a.score_gaps.unwrap_or(usize::MAX);
        let bg = b.score_gaps.unwrap_or(usize::MAX);
        let aspan = a.score_span.unwrap_or(usize::MAX);
        let bspan = b.score_span.unwrap_or(usize::MAX);
        match ag.cmp(&bg) {
            Ordering::Equal => match aspan.cmp(&bspan) {
                Ordering::Equal => match a.path.len().cmp(&b.path.len()) {
                    Ordering::Equal => a.path.cmp(&b.path),
                    other => other,
                },
                other => other,
            },
            other => other,
        }
    });
    filtered.truncate(limit);
    filtered
}

fn append_receipt(root: &Path, payload: &Value) {
    let path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("workspace_file_search_receipts.jsonl");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(
            format!(
                "{}\n",
                serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string())
            )
            .as_bytes(),
        );
    }
}

fn run_search(root: &Path, parsed: &crate::ParsedArgs, default_query: &str) -> Value {
    let specs = match resolve_workspace_specs(root, parsed) {
        Ok(rows) => rows,
        Err(error) => return json!({"ok": false, "status": "blocked", "error": error}),
    };
    let selected_type = parsed
        .flags
        .get("type")
        .map(|row| crate::clean(row, 16))
        .unwrap_or_default();
    let query = parsed
        .flags
        .get("q")
        .or_else(|| parsed.flags.get("query"))
        .map(|row| crate::clean(row, 200))
        .unwrap_or_else(|| crate::clean(default_query, 200));
    let limit = parse_usize_flag(parsed, "limit", 20, 5000);
    let fetch_limit = parse_usize_flag(parsed, "fetch-limit", 5000, 20000);

    let mut all_items = Vec::<SearchItem>::new();
    let mut warnings = Vec::<String>::new();
    for workspace in &specs {
        match collect_workspace_items(workspace, fetch_limit) {
            Ok(rows) => all_items.extend(rows),
            Err(error) => warnings.push(format!("{}:{error}", workspace.name)),
        }
    }
    let results = search_items(&all_items, &query, &selected_type, limit);
    let receipt = json!({
        "type": "workspace_file_search_receipt",
        "ts": crate::now_iso(),
        "source": "cline/src/services/search/file-search.ts",
        "query": query,
        "selected_type": selected_type,
        "workspace_count": specs.len(),
        "item_count": all_items.len(),
        "result_count": results.len(),
        "warnings": warnings,
    });
    append_receipt(root, &receipt);
    json!({
        "ok": true,
        "type": "workspace_file_search",
        "source": "cline:file-search",
        "query": query,
        "selected_type": selected_type,
        "workspace_count": specs.len(),
        "results": results,
        "warnings": warnings,
    })
}
