
fn usage() {
    println!("stomach-kernel commands:");
    println!("  protheus-ops stomach-kernel run --id=<digest_id> --source-root=<path> --origin=<https://...> [--commit=<hash>] [--refs=refs/heads/main] [--spdx=<MIT>] [--transform=namespace_fix|header_injection|path_remap|adapter_scaffold] [--targets=a.rs,b.rs] [--header=...]");
    println!("  protheus-ops stomach-kernel score --id=<digest_id> --source-root=<path> [--targets=a.rs,b.rs]");
    println!("  protheus-ops stomach-kernel status --id=<digest_id>");
    println!("  protheus-ops stomach-kernel rollback --id=<digest_id> --receipt=<receipt_id> [--reason=<text>]");
    println!("  protheus-ops stomach-kernel retention --id=<digest_id> --action=hold|release|eligible [--reason=<text>] [--retained-until=<epoch_secs>] [--approve-receipt=<receipt_id>]");
    println!("  protheus-ops stomach-kernel purge --id=<digest_id>");
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let prefix = format!("--{key}=");
    for token in argv {
        if let Some(rest) = token.strip_prefix(&prefix) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn csv_list(raw: Option<String>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(|row| row.trim().to_string())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
}

fn candidate_extension_allowed(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref(),
        Some("rs")
            | Some("ts")
            | Some("tsx")
            | Some("toml")
            | Some("json")
            | Some("yaml")
            | Some("yml")
            | Some("md")
            | Some("py")
    )
}

fn should_skip_candidate_path(path: &Path) -> bool {
    let normalized = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    normalized.contains("/.git/")
        || normalized.contains("/target/")
        || normalized.contains("/node_modules/")
        || normalized.contains("/dist/")
        || normalized.contains("/build/")
        || normalized.contains("/local/state/")
}

fn collect_candidate_paths_recursive(
    source_root: &Path,
    current: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<PathBuf>,
    max_files: usize,
) -> Result<(), String> {
    if depth > max_depth || out.len() >= max_files {
        return Ok(());
    }
    let entries = fs::read_dir(current)
        .map_err(|e| format!("stomach_candidate_scan_failed:{}:{e}", current.display()))?;
    for entry in entries {
        if out.len() >= max_files {
            break;
        }
        let entry = entry.map_err(|e| format!("stomach_candidate_entry_failed:{e}"))?;
        let path = entry.path();
        if should_skip_candidate_path(&path) {
            continue;
        }
        if path.is_dir() {
            collect_candidate_paths_recursive(
                source_root,
                &path,
                depth.saturating_add(1),
                max_depth,
                out,
                max_files,
            )?;
            continue;
        }
        if !path.is_file() || !candidate_extension_allowed(&path) {
            continue;
        }
        let rel = path
            .strip_prefix(source_root)
            .map(PathBuf::from)
            .unwrap_or(path.clone());
        out.push(rel);
    }
    Ok(())
}

fn score_authority_risk(path_rel: &str) -> u8 {
    let normalized = path_rel.to_ascii_lowercase();
    let mut score = 1u8;
    if normalized.contains("core/") || normalized.contains("/security/") || normalized.contains("/ops/") {
        score = 5;
    } else if normalized.contains("surface/") || normalized.contains("/autonomy/") {
        score = 4;
    } else if normalized.contains("client/runtime/") {
        score = 3;
    } else if normalized.contains("docs/") || normalized.ends_with(".md") {
        score = 2;
    }
    score.min(5)
}

fn score_migration_potential(path_rel: &str) -> u8 {
    let normalized = path_rel.to_ascii_lowercase();
    let mut score = 2u8;
    if normalized.ends_with(".rs") {
        score = 5;
    } else if normalized.ends_with(".ts") || normalized.ends_with(".tsx") {
        score = 4;
    } else if normalized.ends_with(".toml") || normalized.ends_with(".json") {
        score = 3;
    } else if normalized.ends_with(".md") {
        score = 2;
    }
    if normalized.contains("/tests/") || normalized.contains("/fixtures/") {
        score = score.saturating_sub(1).max(1);
    }
    score.min(5)
}

fn score_concept_opportunity(path_rel: &str) -> u8 {
    let normalized = path_rel.to_ascii_lowercase();
    let mut score = 2u8;
    if normalized.contains("planner")
        || normalized.contains("orchestration")
        || normalized.contains("memory")
        || normalized.contains("autonomy")
        || normalized.contains("tooling")
    {
        score = 5;
    } else if normalized.contains("gateway") || normalized.contains("conduit") || normalized.contains("receipt")
    {
        score = 4;
    } else if normalized.contains("ui/") || normalized.contains("docs/") {
        score = 3;
    }
    score.min(5)
}

fn concept_note_for(path_rel: &str) -> String {
    let normalized = path_rel.replace('\\', "/");
    let leaf = normalized.rsplit('/').next().unwrap_or(path_rel);
    format!("extract reusable concept from {}", clean(leaf, 120))
}

fn priority_score(authority: u8, migration: u8, concept: u8) -> f64 {
    let authority_w = (authority as f64) * 0.5;
    let migration_w = (migration as f64) * 0.3;
    let concept_w = (concept as f64) * 0.2;
    ((authority_w + migration_w + concept_w) * 100.0).round() / 100.0
}
