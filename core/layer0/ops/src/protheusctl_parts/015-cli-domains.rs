// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use semver::Version;
use std::cmp::Ordering;
use std::fs;

const VERSION_INSTALL_COMMAND: &str =
    "curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full";
const VERSION_CACHE_TTL_MS: i64 = 6 * 60 * 60 * 1000;
const GITHUB_RELEASES_API_URL: &str =
    "https://api.github.com/repos/protheuslabs/InfRing/releases?per_page=12";
const GITHUB_RELEASE_LATEST_API_URL: &str =
    "https://api.github.com/repos/protheuslabs/InfRing/releases/latest";

const CORE_COMPLETION_COMMANDS: &[&str] = &[
    "gateway",
    "start",
    "stop",
    "restart",
    "status",
    "dashboard",
    "doctor",
    "verify-install",
    "dream",
    "compact",
    "proactive_daemon",
    "speculate",
    "setup",
    "help",
];

#[derive(Debug, Clone)]
struct VersionCliOpts {
    command: String,
    json: bool,
    quiet: bool,
    force: bool,
    apply: bool,
}

impl Default for VersionCliOpts {
    fn default() -> Self {
        Self {
            command: "version".to_string(),
            json: false,
            quiet: false,
            force: false,
            apply: false,
        }
    }
}

#[derive(Debug, Clone)]
struct ReleaseCandidate {
    latest_version: String,
    changelog_line: String,
    released_at: String,
    prerelease: bool,
    draft: bool,
}

fn normalize_version_text(raw: &str) -> String {
    clean(raw, 120)
        .trim()
        .trim_start_matches(['v', 'V'])
        .to_string()
}

fn first_meaningful_line(raw: &str, fallback: &str) -> String {
    for row in raw.lines() {
        let line = clean(row, 240);
        if line.is_empty() || line.starts_with('#') || line == "-" || line == "*" {
            continue;
        }
        return line;
    }
    clean(fallback, 240)
}

fn parse_version_cli_args(argv: &[String]) -> VersionCliOpts {
    let mut opts = VersionCliOpts::default();
    let mut tokens = argv
        .iter()
        .map(|row| clean(row, 160))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if let Some(first) = tokens.first() {
        if !first.starts_with("--") {
            opts.command = clean(first, 40).to_ascii_lowercase();
            tokens.remove(0);
        }
    }
    for token in tokens {
        match token.as_str() {
            "--json" | "--json=1" => opts.json = true,
            "--quiet" | "--quiet=1" => opts.quiet = true,
            "--force" | "--force=1" => opts.force = true,
            "--apply" | "--apply=1" => opts.apply = true,
            "--help" | "-h" => opts.command = "help".to_string(),
            _ => {}
        }
    }
    opts
}

fn version_release_tag_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("local")
        .join("state")
        .join("ops")
        .join("install_release_tag.txt")
}

fn version_release_meta_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("local")
        .join("state")
        .join("ops")
        .join("install_release_meta.json")
}

fn version_release_channel_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("client")
        .join("runtime")
        .join("config")
        .join("protheus_release_channel.json")
}

fn version_cache_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("local")
        .join("state")
        .join("ops")
        .join("protheus_version_cli")
        .join("latest.json")
}

fn read_json_file(path: &std::path::Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn write_json_file(path: &std::path::Path, payload: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(payload) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn read_installed_release_version(root: &std::path::Path) -> Option<String> {
    let meta_path = version_release_meta_path(root);
    if let Some(meta) = read_json_file(&meta_path) {
        if let Some(value) = meta
            .get("release_version_normalized")
            .and_then(Value::as_str)
            .or_else(|| meta.get("release_tag").and_then(Value::as_str))
        {
            let normalized = normalize_version_text(value);
            if !normalized.is_empty() {
                return Some(normalized);
            }
        }
    }

    let tag_path = version_release_tag_path(root);
    let raw = fs::read_to_string(tag_path).ok()?;
    let normalized = normalize_version_text(raw.lines().next().unwrap_or_default());
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn read_current_version(root: &std::path::Path) -> String {
    read_installed_release_version(root)
        .or_else(|| workspace_package_version(root))
        .unwrap_or_else(|| "0.0.0-unknown".to_string())
}

fn compare_versions(a: &str, b: &str) -> Ordering {
    let left = normalize_version_text(a);
    let right = normalize_version_text(b);
    match (Version::parse(&left), Version::parse(&right)) {
        (Ok(la), Ok(lb)) => la.cmp(&lb),
        _ => left.cmp(&right),
    }
}

fn read_release_channel_metadata(root: &std::path::Path) -> (String, String, String) {
    let path = version_release_channel_path(root);
    let Some(cfg) = read_json_file(&path) else {
        return (String::new(), String::new(), String::new());
    };
    (
        normalize_version_text(cfg.get("latest_version").and_then(Value::as_str).unwrap_or("")),
        clean(cfg.get("changelog_line").and_then(Value::as_str).unwrap_or(""), 240),
        clean(cfg.get("released_at").and_then(Value::as_str).unwrap_or(""), 40),
    )
}

fn parse_release_candidate(raw: &Value) -> Option<ReleaseCandidate> {
    let latest_version = normalize_version_text(
        raw.get("tag_name")
            .and_then(Value::as_str)
            .or_else(|| raw.get("name").and_then(Value::as_str))
            .unwrap_or(""),
    );
    if latest_version.is_empty() {
        return None;
    }
    let prerelease = raw
        .get("prerelease")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || Version::parse(&latest_version)
            .ok()
            .map(|v| !v.pre.is_empty())
            .unwrap_or(false);
    Some(ReleaseCandidate {
        latest_version,
        changelog_line: first_meaningful_line(raw.get("body").and_then(Value::as_str).unwrap_or(""), ""),
        released_at: clean(
            raw.get("published_at")
                .and_then(Value::as_str)
                .or_else(|| raw.get("created_at").and_then(Value::as_str))
                .unwrap_or(""),
            40,
        ),
        prerelease,
        draft: raw.get("draft").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn select_release_candidate(raw: &Value, prefer_prerelease: bool) -> Option<ReleaseCandidate> {
    let mut candidates = Vec::<ReleaseCandidate>::new();
    match raw {
        Value::Array(rows) => {
            for row in rows {
                if let Some(candidate) = parse_release_candidate(row) {
                    if !candidate.draft {
                        candidates.push(candidate);
                    }
                }
            }
        }
        Value::Object(_) => {
            if let Some(candidate) = parse_release_candidate(raw) {
                if !candidate.draft {
                    candidates.push(candidate);
                }
            }
        }
        _ => {}
    }
    if candidates.is_empty() {
        return None;
    }

    let mut filtered = if prefer_prerelease {
        candidates
    } else {
        let stable = candidates
            .iter()
            .filter(|row| !row.prerelease)
            .cloned()
            .collect::<Vec<_>>();
        if stable.is_empty() {
            candidates
        } else {
            stable
        }
    };

    filtered.sort_by(|a, b| {
        let cmp = compare_versions(&a.latest_version, &b.latest_version);
        if cmp == Ordering::Equal {
            a.released_at.cmp(&b.released_at)
        } else {
            cmp
        }
    });
    filtered.pop()
}

fn curl_fetch_json(url: &str, timeout_ms: u64) -> Result<Value, String> {
    let timeout_secs = ((timeout_ms + 999) / 1000).max(1);
    let output = Command::new("curl")
        .arg("-fsSL")
        .arg("--max-time")
        .arg(timeout_secs.to_string())
        .arg("-H")
        .arg("Accept: application/vnd.github+json")
        .arg("-H")
        .arg("User-Agent: infring-version-cli")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| clean(format!("curl_spawn_failed:{err}"), 220))?;
    if !output.status.success() {
        let stderr = clean(String::from_utf8_lossy(&output.stderr), 220);
        return Err(if stderr.is_empty() {
            "curl_nonzero_exit".to_string()
        } else {
            format!("curl_nonzero_exit:{stderr}")
        });
    }
    let body = String::from_utf8_lossy(&output.stdout).to_string();
    serde_json::from_str::<Value>(&body).map_err(|_| "github_release_parse_failed".to_string())
}

fn fetch_latest_release(timeout_ms: u64, prefer_prerelease: bool) -> Result<(ReleaseCandidate, String), String> {
    let primary = curl_fetch_json(GITHUB_RELEASES_API_URL, timeout_ms)
        .and_then(|payload| {
            select_release_candidate(&payload, prefer_prerelease)
                .ok_or_else(|| "github_release_tag_missing".to_string())
        })
        .map(|candidate| (candidate, "github_releases_api".to_string()));
    match primary {
        Ok(value) => Ok(value),
        Err(primary_err) => {
            let fallback = curl_fetch_json(GITHUB_RELEASE_LATEST_API_URL, timeout_ms)
                .and_then(|payload| {
                    select_release_candidate(&payload, prefer_prerelease)
                        .ok_or_else(|| "github_release_tag_missing".to_string())
                })
                .map(|candidate| (candidate, "github_latest_api".to_string()));
            fallback.map_err(|fallback_err| {
                clean(
                    format!("github_release_fetch_failed:{fallback_err}|fallback_from:{primary_err}"),
                    220,
                )
            })
        }
    }
}

fn read_version_cache(root: &std::path::Path) -> Option<(i64, Value)> {
    let path = version_cache_path(root);
    let payload = read_json_file(&path)?;
    let checked_at_ms = payload
        .get("checked_at_ms")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)?;
    let result = payload.get("result").cloned()?;
    if !result.is_object() {
        return None;
    }
    Some((checked_at_ms, result))
}

fn write_version_cache(root: &std::path::Path, result: &Value) {
    let path = version_cache_path(root);
    write_json_file(
        &path,
        &json!({
            "type": "protheus_version_cli_cache",
            "schema_version": 1,
            "checked_at": crate::now_iso(),
            "checked_at_ms": chrono::Utc::now().timestamp_millis(),
            "result": result
        }),
    );
}

fn resolve_release_check(root: &std::path::Path, force: bool, timeout_ms: u64) -> Value {
    let current_version = read_current_version(root);
    if !force {
        if let Some((checked_at_ms, cached_result)) = read_version_cache(root) {
            let now_ms = chrono::Utc::now().timestamp_millis();
            if now_ms - checked_at_ms < VERSION_CACHE_TTL_MS {
                let cache_version = normalize_version_text(
                    cached_result
                        .get("current_version")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                );
                if cache_version == normalize_version_text(&current_version) {
                    let mut merged = cached_result;
                    merged["source"] = Value::String(
                        clean(merged.get("source").and_then(Value::as_str).unwrap_or("cache"), 80),
                    );
                    merged["cache_hit"] = Value::Bool(true);
                    return merged;
                }
            }
        }
    }

    let prefer_prerelease = Version::parse(&normalize_version_text(&current_version))
        .ok()
        .map(|v| !v.pre.is_empty())
        .unwrap_or(false);
    let (local_latest, local_changelog, local_released_at) = read_release_channel_metadata(root);
    let remote = fetch_latest_release(timeout_ms, prefer_prerelease);

    let mut latest_version = current_version.clone();
    let mut changelog_line = String::new();
    let mut released_at = String::new();
    let mut source = "local_current".to_string();
    let mut check_warning = String::new();

    match remote {
        Ok((candidate, remote_source)) => {
            latest_version = normalize_version_text(&candidate.latest_version);
            changelog_line = clean(&candidate.changelog_line, 240);
            released_at = clean(&candidate.released_at, 40);
            source = remote_source;
        }
        Err(err) if !local_latest.is_empty() => {
            latest_version = local_latest;
            changelog_line = local_changelog;
            released_at = local_released_at;
            source = "release_channel".to_string();
            check_warning = err;
        }
        Err(err) => {
            check_warning = err;
        }
    }

    if latest_version.is_empty() {
        latest_version = current_version.clone();
    }
    let update_available = compare_versions(&latest_version, &current_version) == Ordering::Greater;
    let mut result = json!({
        "ok": true,
        "type": "protheus_version_cli",
        "current_version": current_version,
        "latest_version": latest_version,
        "update_available": update_available,
        "changelog_line": changelog_line,
        "released_at": released_at,
        "source": clean(source, 80),
        "checked_at": crate::now_iso()
    });
    if !check_warning.is_empty() {
        result["check_warning"] = Value::String(clean(check_warning, 240));
    }
    write_version_cache(root, &result);
    result
}

fn emit_json_line(payload: &Value) {
    println!("{}", serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string()));
}

fn print_version_help() {
    println!("Usage: infring version|update|check-quiet [flags]");
    println!();
    println!("Commands:");
    println!("  version      Show local version and update signal.");
    println!("  update       Check latest release and print upgrade command.");
    println!("  check-quiet  Emit one-line notice only when update is available.");
    println!();
    println!("Flags:");
    println!("  --json        Print JSON payload.");
    println!("  --quiet       Suppress advisory notes.");
    println!("  --force       Bypass cache for remote release check.");
    println!("  --apply       Run installer command after check (`update` only).");
}

fn run_version_mode(root: &std::path::Path, opts: &VersionCliOpts) -> i32 {
    let check = resolve_release_check(root, opts.force, 1800);
    let mut payload = check.clone();
    payload["command"] = Value::String("version".to_string());
    if opts.json {
        emit_json_line(&payload);
        return 0;
    }
    let current = check.get("current_version").and_then(Value::as_str).unwrap_or("0.0.0-unknown");
    println!("infring {current}");
    if check.get("update_available").and_then(Value::as_bool) == Some(true) {
        let latest = check.get("latest_version").and_then(Value::as_str).unwrap_or(current);
        println!("[infring update] available: {latest} (current {current})");
        if let Some(changelog) = check.get("changelog_line").and_then(Value::as_str) {
            if !changelog.trim().is_empty() {
                println!("[infring update] {changelog}");
            }
        }
    }
    if !opts.quiet {
        if let Some(warning) = check.get("check_warning").and_then(Value::as_str) {
            if !warning.trim().is_empty() {
                println!("[infring update] note: {warning}");
            }
        }
    }
    0
}

fn run_update_mode(root: &std::path::Path, opts: &VersionCliOpts) -> i32 {
    let check = resolve_release_check(root, true, 2200);
    let mut apply_status = 0i32;
    if opts.apply {
        println!("[infring update] applying via: {VERSION_INSTALL_COMMAND}");
        let status = Command::new("sh")
            .arg("-c")
            .arg(VERSION_INSTALL_COMMAND)
            .current_dir(root)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();
        apply_status = status.ok().and_then(|row| row.code()).unwrap_or(1);
    }

    let mut payload = check.clone();
    payload["command"] = Value::String("update".to_string());
    payload["install_command"] = Value::String(VERSION_INSTALL_COMMAND.to_string());
    payload["apply_requested"] = Value::Bool(opts.apply);
    payload["apply_exit_code"] = Value::from(apply_status);

    if opts.json {
        emit_json_line(&payload);
        return apply_status;
    }

    let current = check.get("current_version").and_then(Value::as_str).unwrap_or("0.0.0-unknown");
    if check.get("update_available").and_then(Value::as_bool) == Some(true) {
        let latest = check.get("latest_version").and_then(Value::as_str).unwrap_or(current);
        println!("[infring update] update available: {latest} (current {current})");
        if let Some(changelog) = check.get("changelog_line").and_then(Value::as_str) {
            if !changelog.trim().is_empty() {
                println!("[infring update] {changelog}");
            }
        }
        if !opts.apply {
            println!("[infring update] install: {VERSION_INSTALL_COMMAND}");
        }
    } else {
        println!("[infring update] already up to date ({current})");
    }

    if !opts.quiet {
        if let Some(warning) = check.get("check_warning").and_then(Value::as_str) {
            if !warning.trim().is_empty() {
                println!("[infring update] note: {warning}");
            }
        }
    }
    apply_status
}

fn run_check_quiet_mode(root: &std::path::Path, opts: &VersionCliOpts) -> i32 {
    let check = resolve_release_check(root, opts.force, 1200);
    let mut payload = check.clone();
    payload["command"] = Value::String("check-quiet".to_string());
    if opts.json {
        emit_json_line(&payload);
        return 0;
    }
    if check.get("update_available").and_then(Value::as_bool) == Some(true) {
        let latest = check
            .get("latest_version")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let current = check
            .get("current_version")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        println!(
            "[infring update] Update available: {latest} (current {current}). Run: infring update"
        );
    }
    0
}

fn run_version_cli_domain(root: &std::path::Path, args: &[String]) -> i32 {
    let opts = parse_version_cli_args(args);
    let command = clean(&opts.command, 40).to_ascii_lowercase();
    match command.as_str() {
        "help" => {
            print_version_help();
            0
        }
        "version" => run_version_mode(root, &opts),
        "update" => run_update_mode(root, &opts),
        "check-quiet" => run_check_quiet_mode(root, &opts),
        _ => {
            let payload = json!({
                "ok": false,
                "type": "protheus_version_cli",
                "error": "unknown_command",
                "command": clean(&command, 40)
            });
            if opts.json {
                emit_json_line(&payload);
            } else {
                eprintln!(
                    "[infring version] unknown subcommand: {}. Try: infring version --help",
                    payload.get("command").and_then(Value::as_str).unwrap_or("unknown")
                );
            }
            2
        }
    }
}

#[derive(Debug, Clone)]
struct ReleaseSemverContractOpts {
    command: String,
    write: bool,
    strict: bool,
    pretty: bool,
    channel: String,
}

impl Default for ReleaseSemverContractOpts {
    fn default() -> Self {
        Self {
            command: "run".to_string(),
            write: false,
            strict: false,
            pretty: true,
            channel: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct ReleaseCommitRow {
    sha: String,
    subject: String,
    body: String,
}

fn release_semver_parse_bool(raw: &str, fallback: bool) -> bool {
    let token = clean(raw, 24).to_ascii_lowercase();
    if token.is_empty() {
        return fallback;
    }
    matches!(token.as_str(), "1" | "true" | "yes" | "on")
}

fn parse_release_semver_contract_args(argv: &[String]) -> ReleaseSemverContractOpts {
    let mut opts = ReleaseSemverContractOpts::default();
    let mut tokens = argv
        .iter()
        .map(|row| clean(row, 200))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();

    if let Some(first) = tokens.first() {
        if !first.starts_with("--") {
            opts.command = clean(first, 40).to_ascii_lowercase();
            tokens.remove(0);
        }
    }

    for token in tokens {
        if token == "--write" {
            opts.write = true;
        } else if let Some(value) = token.strip_prefix("--write=") {
            opts.write = release_semver_parse_bool(value, false);
        } else if token == "--strict" {
            opts.strict = true;
        } else if let Some(value) = token.strip_prefix("--strict=") {
            opts.strict = release_semver_parse_bool(value, false);
        } else if token == "--pretty" {
            opts.pretty = true;
        } else if let Some(value) = token.strip_prefix("--pretty=") {
            opts.pretty = release_semver_parse_bool(value, true);
        } else if let Some(value) = token.strip_prefix("--channel=") {
            opts.channel = clean(value, 40).to_ascii_lowercase();
        } else if matches!(token.as_str(), "--help" | "-h") {
            opts.command = "help".to_string();
        }
    }

    opts
}

fn release_semver_package_json_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("package.json")
}

fn release_semver_package_lock_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("package-lock.json")
}

fn release_semver_channel_policy_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("client")
        .join("runtime")
        .join("config")
        .join("release_channel_policy.json")
}

fn release_semver_runtime_version_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("client")
        .join("runtime")
        .join("config")
        .join("runtime_version.json")
}

fn release_semver_normalize_channel(raw: &str) -> String {
    let channel = clean(raw, 40).to_ascii_lowercase();
    if matches!(channel.as_str(), "alpha" | "beta" | "stable") {
        channel
    } else {
        "stable".to_string()
    }
}

fn release_semver_channel_policy_default(root: &std::path::Path) -> String {
    let path = release_semver_channel_policy_path(root);
    let configured = read_json_file(&path)
        .and_then(|row| {
            row.get("default_channel")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "stable".to_string());
    release_semver_normalize_channel(&configured)
}

fn release_semver_tag_for_channel(version: &str, channel: &str) -> String {
    let normalized = release_semver_normalize_channel(channel);
    if normalized == "stable" {
        format!("v{}", normalize_version_text(version))
    } else {
        format!("v{}-{normalized}", normalize_version_text(version))
    }
}

fn release_semver_run_git(root: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output();
    let Ok(output) = output else {
        return String::new();
    };
    if !output.status.success() {
        return String::new();
    }
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn release_semver_parse_triplet(raw: &str) -> Option<(u64, u64, u64, String)> {
    let normalized = normalize_version_text(raw);
    let parsed = Version::parse(&normalized).ok()?;
    Some((
        parsed.major,
        parsed.minor,
        parsed.patch,
        format!("{}.{}.{}", parsed.major, parsed.minor, parsed.patch),
    ))
}

fn release_semver_latest_tag(root: &std::path::Path) -> String {
    let output = release_semver_run_git(root, &["tag", "--list", "--sort=-v:refname", "v*"]);
    for row in output.lines() {
        let tag = clean(row, 120);
        if !tag.is_empty() && release_semver_parse_triplet(&tag).is_some() {
            return tag;
        }
    }
    String::new()
}

fn release_semver_base_version(root: &std::path::Path, previous_tag: &str) -> (u64, u64, u64, String) {
    if let Some(base) = release_semver_parse_triplet(previous_tag) {
        return base;
    }

    let package_version = read_json_file(&release_semver_package_json_path(root))
        .and_then(|row| {
            row.get("version")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "0.0.0".to_string());

    if let Some(base) = release_semver_parse_triplet(&package_version) {
        return base;
    }

    (0, 0, 0, "0.0.0".to_string())
}

fn release_semver_read_commits(root: &std::path::Path, range_expr: &str) -> Vec<ReleaseCommitRow> {
    let format = "%H%x1f%s%x1f%b%x1e";
    let mut args = vec!["log".to_string(), format!("--format={format}")];
    if !range_expr.trim().is_empty() {
        args.push(range_expr.to_string());
    }
    let ref_args = args.iter().map(String::as_str).collect::<Vec<_>>();
    let raw = release_semver_run_git(root, &ref_args);
    raw.split('\u{1e}')
        .map(str::trim)
        .filter(|row| !row.is_empty())
        .filter_map(|row| {
            let parts = row.split('\u{1f}').collect::<Vec<_>>();
            if parts.len() < 2 {
                return None;
            }
            let sha = clean(parts.first().copied().unwrap_or_default(), 80);
            let subject = clean(parts.get(1).copied().unwrap_or_default(), 400);
            let body = clean(parts.get(2).copied().unwrap_or_default(), 6000);
            if sha.is_empty() || subject.is_empty() {
                return None;
            }
            Some(ReleaseCommitRow { sha, subject, body })
        })
        .collect()
}

fn release_semver_is_release_chore(subject: &str) -> bool {
    let normalized = clean(subject, 220).to_ascii_lowercase();
    if !normalized.starts_with("chore(release):") {
        return false;
    }
    let remainder = normalized.trim_start_matches("chore(release):").trim();
    release_semver_parse_triplet(remainder).is_some()
}

fn release_semver_conventional_type(subject: &str) -> (String, bool) {
    let row = clean(subject, 260);
    let head = row.split(':').next().unwrap_or_default().trim();
    if head.is_empty() {
        return (String::new(), false);
    }
    let mut lowered = head.to_ascii_lowercase();
    let breaking = lowered.ends_with('!');
    if breaking {
        lowered.pop();
    }
    let ty = lowered
        .split_once('(')
        .map(|(prefix, _)| prefix.to_string())
        .unwrap_or(lowered)
        .trim()
        .to_string();
    if ty.is_empty() {
        (String::new(), breaking)
    } else {
        (ty, breaking)
    }
}

fn release_semver_is_breaking_change(subject: &str, body: &str) -> bool {
    let (_ty, breaking_bang) = release_semver_conventional_type(subject);
    if breaking_bang {
        return true;
    }
    let upper = clean(body, 6000).to_ascii_uppercase();
    upper.contains("BREAKING CHANGE:")
        || upper.contains("BREAKING_CHANGE:")
        || upper.contains("BREAKING-CHANGE:")
}

fn release_semver_classify_bump(commits: &[ReleaseCommitRow]) -> String {
    let mut saw_minor = false;
    let mut saw_patch = false;
    for row in commits {
        if release_semver_is_release_chore(&row.subject) {
            continue;
        }
        if release_semver_is_breaking_change(&row.subject, &row.body) {
            return "major".to_string();
        }
        let (ty, _breaking_bang) = release_semver_conventional_type(&row.subject);
        if ty == "feat" {
            saw_minor = true;
        } else {
            saw_patch = true;
        }
    }
    if saw_minor {
        "minor".to_string()
    } else if saw_patch {
        "patch".to_string()
    } else {
        "none".to_string()
    }
}

fn release_semver_bump_version(base: &(u64, u64, u64, String), bump: &str) -> String {
    let (major, minor, patch, _normalized) = base;
    match bump {
        "major" => format!("{}.0.0", major + 1),
        "minor" => format!("{}.{}.0", major, minor + 1),
        _ => format!("{}.{}.{}", major, minor, patch + 1),
    }
}

fn release_semver_update_package_version(root: &std::path::Path, version: &str) -> bool {
    let path = release_semver_package_json_path(root);
    let Some(mut payload) = read_json_file(&path) else {
        return false;
    };
    let Some(object) = payload.as_object_mut() else {
        return false;
    };
    if object.get("version").and_then(Value::as_str) == Some(version) {
        return false;
    }
    object.insert("version".to_string(), Value::String(version.to_string()));
    write_json_file(&path, &payload);
    true
}

fn release_semver_update_package_lock_version(root: &std::path::Path, version: &str) -> bool {
    let path = release_semver_package_lock_path(root);
    let Some(mut payload) = read_json_file(&path) else {
        return false;
    };
    let Some(object) = payload.as_object_mut() else {
        return false;
    };
    let mut changed = false;

    if object.get("version").and_then(Value::as_str) != Some(version) {
        object.insert("version".to_string(), Value::String(version.to_string()));
        changed = true;
    }

    if let Some(packages) = object.get_mut("packages").and_then(Value::as_object_mut) {
        if let Some(root_pkg) = packages.get_mut("").and_then(Value::as_object_mut) {
            if root_pkg.get("version").and_then(Value::as_str) != Some(version) {
                root_pkg.insert("version".to_string(), Value::String(version.to_string()));
                changed = true;
            }
        }
    }

    if changed {
        write_json_file(&path, &payload);
    }
    changed
}

fn release_semver_write_runtime_version_data(
    root: &std::path::Path,
    version: &str,
    bump_kind: &str,
    previous_tag: &str,
    next_tag: &str,
    release_ready: bool,
) -> bool {
    let path = release_semver_runtime_version_path(root);
    let release_channel = if next_tag.trim().is_empty() || next_tag == "none" {
        "stable".to_string()
    } else if next_tag.to_ascii_lowercase().contains("-alpha") {
        "alpha".to_string()
    } else if next_tag.to_ascii_lowercase().contains("-beta") {
        "beta".to_string()
    } else {
        "stable".to_string()
    };
    let payload = json!({
        "schema_version": 1,
        "version": normalize_version_text(version),
        "tag": if next_tag.trim().is_empty() || next_tag == "none" {
            format!("v{}", normalize_version_text(version))
        } else {
            clean(next_tag, 120)
        },
        "previous_tag": if previous_tag.trim().is_empty() {
            Value::Null
        } else {
            Value::String(clean(previous_tag, 120))
        },
        "release_channel": release_channel,
        "bump": clean(bump_kind, 16),
        "release_ready": release_ready,
        "source": "release_semver_contract",
    });
    let changed = read_json_file(&path).map(|prior| prior != payload).unwrap_or(true);
    write_json_file(&path, &payload);
    changed
}

fn run_release_semver_contract_domain(root: &std::path::Path, args: &[String]) -> i32 {
    let opts = parse_release_semver_contract_args(args);
    if matches!(opts.command.as_str(), "help" | "--help" | "-h") {
        println!("Usage: infring release-semver-contract [run|status] [--write=0|1] [--strict=0|1] [--pretty=0|1] [--channel=alpha|beta|stable]");
        return 0;
    }

    let requested_channel = release_semver_normalize_channel(
        &if !opts.channel.trim().is_empty() {
            opts.channel.clone()
        } else if let Ok(value) = env::var("INFRING_RELEASE_CHANNEL") {
            value
        } else if let Ok(value) = env::var("PROTHEUS_RELEASE_CHANNEL") {
            value
        } else {
            release_semver_channel_policy_default(root)
        },
    );

    let previous_tag = release_semver_latest_tag(root);
    let range = if previous_tag.is_empty() {
        "HEAD".to_string()
    } else {
        format!("{previous_tag}..HEAD")
    };
    let commits = release_semver_read_commits(root, &range)
        .into_iter()
        .filter(|row| !release_semver_is_release_chore(&row.subject))
        .collect::<Vec<_>>();
    let bump = release_semver_classify_bump(&commits);
    let release_ready = bump != "none" && !commits.is_empty();
    let base = release_semver_base_version(root, &previous_tag);
    let current_version = base.3.clone();
    let next_version = if release_ready {
        release_semver_bump_version(&base, &bump)
    } else {
        current_version.clone()
    };
    let next_tag = if release_ready {
        release_semver_tag_for_channel(&next_version, &requested_channel)
    } else {
        "none".to_string()
    };

    let mut wrote_version = false;
    if opts.write && release_ready {
        wrote_version = release_semver_update_package_version(root, &next_version) || wrote_version;
        wrote_version =
            release_semver_update_package_lock_version(root, &next_version) || wrote_version;
        wrote_version = release_semver_write_runtime_version_data(
            root,
            &next_version,
            &bump,
            &previous_tag,
            &next_tag,
            release_ready,
        ) || wrote_version;
    } else if opts.write {
        let stable_version = if release_ready {
            next_version.clone()
        } else {
            current_version.clone()
        };
        let _ = release_semver_write_runtime_version_data(
            root,
            &stable_version,
            &bump,
            &previous_tag,
            &next_tag,
            release_ready,
        );
    }

    let commits_payload = commits
        .iter()
        .take(60)
        .map(|row| {
            let classification = if release_semver_is_breaking_change(&row.subject, &row.body) {
                "major".to_string()
            } else {
                let (ty, _breaking_bang) = release_semver_conventional_type(&row.subject);
                if ty == "feat" {
                    "minor".to_string()
                } else {
                    "patch".to_string()
                }
            };
            json!({
                "sha": row.sha,
                "subject": row.subject,
                "classification": classification
            })
        })
        .collect::<Vec<_>>();

    let output = json!({
        "ok": true,
        "mode": "conventional_commits",
        "release_channel": requested_channel,
        "release_ready": release_ready,
        "previous_tag": if previous_tag.is_empty() { "none".to_string() } else { previous_tag.clone() },
        "next_tag": next_tag,
        "current_version": current_version,
        "next_version": next_version,
        "bump": bump,
        "commits_scanned": commits.len(),
        "commits": commits_payload,
        "write_requested": opts.write,
        "version_bumped": wrote_version
    });

    if opts.pretty {
        if let Ok(raw) = serde_json::to_string_pretty(&output) {
            println!("{raw}");
        } else {
            println!("{{\"ok\":false,\"error\":\"serialize_failed\"}}");
        }
    } else {
        emit_json_line(&output);
    }

    if opts.strict && output.get("ok").and_then(Value::as_bool) != Some(true) {
        1
    } else {
        0
    }
}

fn run_completion_domain(args: &[String]) -> i32 {
    let help_mode = args
        .iter()
        .any(|arg| matches!(arg.trim(), "--help" | "-h"));
    let json_mode = args
        .iter()
        .any(|arg| matches!(arg.trim(), "--json" | "--json=1"));
    if help_mode {
        println!("Usage: infring completion [--json]");
        println!("Prints completion candidates for core entry commands.");
        return 0;
    }
    if json_mode {
        emit_json_line(&json!({
            "ok": true,
            "type": "protheus_completion",
            "commands": CORE_COMPLETION_COMMANDS
        }));
        return 0;
    }
    for row in CORE_COMPLETION_COMMANDS {
        println!("{row}");
    }
    0
}

fn run_repl_domain(root: &std::path::Path, args: &[String]) -> i32 {
    if args
        .iter()
        .any(|arg| matches!(arg.trim(), "--help" | "-h"))
    {
        println!("Usage: infring repl");
        println!("Lightweight REPL bootstrap for constrained installs.");
        return 0;
    }
    let status = crate::command_list_kernel::run(root, &[String::from("--mode=help")]);
    if status == 0 && std::io::stdin().is_terminal() {
        println!(
            "[infring repl] interactive shell is unavailable in slim runtime; showing command index."
        );
    }
    status
}

fn normalize_domain_subcommand(raw: Option<&String>, fallback: &str) -> String {
    let token = raw
        .map(|row| clean(row, 80).to_ascii_lowercase())
        .unwrap_or_else(|| fallback.to_string());
    if token.trim().is_empty() {
        fallback.to_string()
    } else {
        token
    }
}

fn normalize_resurrection_protocol_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 160))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["status".to_string()];
    }
    let mut head = rows[0].to_ascii_lowercase();
    let mut tail = rows[1..].to_vec();
    if head == "resurrection-protocol" || head == "resurrection_protocol" {
        head = tail
            .first()
            .map(|row| clean(row, 40).to_ascii_lowercase())
            .unwrap_or_else(|| "status".to_string());
        tail = tail.into_iter().skip(1).collect();
    }
    if head.trim().is_empty() {
        return vec!["status".to_string()];
    }
    if matches!(head.as_str(), "run" | "build" | "bundle") {
        return std::iter::once("checkpoint".to_string())
            .chain(tail)
            .collect();
    }
    if head == "verify" {
        return vec!["status".to_string()];
    }
    if matches!(head.as_str(), "status" | "restore") {
        return std::iter::once(head).chain(tail).collect();
    }
    std::iter::once(head).chain(tail).collect()
}

fn normalize_session_continuity_vault_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 160))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["status".to_string()];
    }
    let head = rows[0].to_ascii_lowercase();
    let tail = rows[1..].to_vec();
    if head == "session-continuity-vault" || head == "session_continuity_vault" {
        return normalize_session_continuity_vault_args(&tail);
    }
    if head.trim().is_empty() {
        return vec!["status".to_string()];
    }
    if head == "restore" {
        return std::iter::once("get".to_string()).chain(tail).collect();
    }
    if head == "archive" {
        return std::iter::once("put".to_string()).chain(tail).collect();
    }
    if head == "verify" {
        return std::iter::once("status".to_string()).chain(tail).collect();
    }
    std::iter::once(head).chain(tail).collect()
}

fn normalized_rust_hotpath_inventory_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 160))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["status".to_string()];
    }
    let first = rows[0].to_ascii_lowercase();
    let is_cmd = !first.starts_with("--");
    let command = if is_cmd { first } else { "status".to_string() };
    let normalized = if matches!(command.as_str(), "run" | "status" | "inventory") {
        command
    } else {
        "status".to_string()
    };
    let tail = if is_cmd {
        rows.into_iter().skip(1).collect::<Vec<_>>()
    } else {
        rows
    };
    std::iter::once(normalized).chain(tail).collect()
}

fn normalized_coverage_badge_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 200))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["run".to_string()];
    }
    let first = rows[0].to_ascii_lowercase();
    if first.starts_with("--") {
        return std::iter::once("run".to_string()).chain(rows).collect();
    }
    rows
}

fn normalized_local_runtime_partitioner_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 200))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["status".to_string()];
    }
    let first = rows[0].to_ascii_lowercase();
    let is_cmd = !first.starts_with("--");
    let normalized = if matches!(first.as_str(), "status" | "init" | "reset") {
        first
    } else {
        "status".to_string()
    };
    let tail = if is_cmd {
        rows.into_iter().skip(1).collect::<Vec<_>>()
    } else {
        rows
    };
    std::iter::once(normalized).chain(tail).collect()
}

fn normalized_top50_roi_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 160))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return vec!["run".to_string()];
    }
    let first = rows[0].to_ascii_lowercase();
    let is_cmd = !first.starts_with("--");
    let command = if is_cmd { first } else { "run".to_string() };
    let normalized = if matches!(command.as_str(), "run" | "queue" | "status") {
        command
    } else {
        "run".to_string()
    };
    let tail = if is_cmd {
        rows.into_iter().skip(1).collect::<Vec<_>>()
    } else {
        rows
    };
    std::iter::once(normalized).chain(tail).collect()
}

fn contains_token(args: &[String], token: &str) -> bool {
    args.iter()
        .any(|row| clean(row, 80).eq_ignore_ascii_case(token))
}

fn normalize_status_dashboard_args(args: &[String]) -> Vec<String> {
    let rows = args
        .iter()
        .map(|row| clean(row, 200))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let has_web = contains_token(args, "--web") || contains_token(args, "web");
    if has_web {
        let filtered = rows
            .into_iter()
            .filter(|row| {
                !matches!(
                    row.as_str(),
                    "--dashboard" | "dashboard" | "--web" | "web"
                )
            })
            .collect::<Vec<_>>();
        return std::iter::once("start".to_string())
            .chain(filtered)
            .collect();
    }
    if rows.is_empty() {
        return vec!["status".to_string()];
    }
    let first = rows[0].to_ascii_lowercase();
    if first.starts_with("--") {
        return std::iter::once("status".to_string()).chain(rows).collect();
    }
    rows
}

fn maybe_redirect_ts_wrapper_to_core_domain(script_rel: &str, args: &[String]) -> Option<(String, Vec<String>)> {
    match script_rel {
        "client/runtime/systems/ops/protheus_control_plane.ts" => {
            let sub = normalize_domain_subcommand(args.first(), "status");
            let normalized = match sub.as_str() {
                "health" => "status".to_string(),
                "job-submit" | "audit" => "run".to_string(),
                _ => sub,
            };
            let mapped = std::iter::once(normalized)
                .chain(args.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(("protheus-control-plane".to_string(), mapped))
        }
        "client/runtime/systems/ops/protheus_debug_diagnostics.ts" => {
            let sub = normalize_domain_subcommand(args.first(), "status");
            let mapped = if matches!(sub.as_str(), "status" | "health") {
                std::iter::once("status".to_string())
                    .chain(args.iter().skip(1).cloned())
                    .collect::<Vec<_>>()
            } else {
                std::iter::once("run".to_string())
                    .chain(args.iter().cloned())
                    .collect::<Vec<_>>()
            };
            Some(("protheus-control-plane".to_string(), mapped))
        }
        "client/runtime/systems/ops/protheus_status_dashboard.ts" => Some((
            "daemon-control".to_string(),
            normalize_status_dashboard_args(args),
        )),
        "client/runtime/systems/ops/protheus_unknown_guard.ts" => Some((
            "unknown-command".to_string(),
            args.to_vec(),
        )),
        "client/runtime/systems/ops/backlog_github_sync.ts" => {
            let sub = normalize_domain_subcommand(args.first(), "status");
            let mapped = std::iter::once(sub)
                .chain(args.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(("backlog-github-sync".to_string(), mapped))
        }
        "client/runtime/systems/ops/backlog_registry.ts" => {
            let sub = normalize_domain_subcommand(args.first(), "status");
            let normalized = if matches!(sub.as_str(), "metrics" | "triage") {
                "status".to_string()
            } else {
                sub
            };
            let mapped = std::iter::once(normalized)
                .chain(args.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(("backlog-registry".to_string(), mapped))
        }
        "client/runtime/systems/ops/rust50_migration_program.ts" => {
            let sub = normalize_domain_subcommand(args.first(), "status");
            let mapped = std::iter::once(sub)
                .chain(args.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(("rust50-migration-program".to_string(), mapped))
        }
        "client/runtime/systems/ops/rust_enterprise_productivity_program.ts" => {
            let sub = normalize_domain_subcommand(args.first(), "status");
            let mapped = std::iter::once(sub)
                .chain(args.iter().skip(1).cloned())
                .collect::<Vec<_>>();
            Some(("rust-enterprise-productivity-program".to_string(), mapped))
        }
        "client/runtime/systems/ops/rust_hotpath_inventory.ts" => Some((
            "rust-hotpath-inventory-kernel".to_string(),
            normalized_rust_hotpath_inventory_args(args),
        )),
        "client/runtime/systems/ops/benchmark_autonomy_gate.ts" => Some((
            "benchmark-autonomy-gate".to_string(),
            if args.is_empty() {
                vec!["run".to_string()]
            } else {
                args.to_vec()
            },
        )),
        "client/runtime/systems/ops/generate_coverage_badge.ts" => Some((
            "coverage-badge-kernel".to_string(),
            normalized_coverage_badge_args(args),
        )),
        "client/runtime/systems/ops/local_runtime_partitioner.ts" => Some((
            "local-runtime-partitioner".to_string(),
            normalized_local_runtime_partitioner_args(args),
        )),
        "client/runtime/systems/ops/security_layer_inventory_gate.ts" => Some((
            "security-layer-inventory-gate-kernel".to_string(),
            if args.is_empty() {
                vec!["run".to_string()]
            } else {
                args.to_vec()
            },
        )),
        "client/runtime/systems/ops/top50_roi_sweep.ts" => Some((
            "top50-roi-sweep-kernel".to_string(),
            normalized_top50_roi_args(args),
        )),
        "client/runtime/systems/ops/readiness_bridge_pack.ts" => Some((
            "readiness-bridge-pack-kernel".to_string(),
            if args.is_empty() {
                vec!["run".to_string()]
            } else {
                args.to_vec()
            },
        )),
        "client/runtime/systems/ops/system_health_audit_runner.ts" => Some((
            "system-health-audit-runner-kernel".to_string(),
            if args.is_empty() {
                vec!["run".to_string()]
            } else {
                args.to_vec()
            },
        )),
        "client/runtime/systems/ops/release_semver_contract.ts" => Some((
            "release-semver-contract".to_string(),
            if args.is_empty() {
                vec!["status".to_string()]
            } else {
                args.to_vec()
            },
        )),
        "client/runtime/systems/continuity/resurrection_protocol.ts" => Some((
            "continuity-runtime".to_string(),
            std::iter::once("resurrection-protocol".to_string())
                .chain(normalize_resurrection_protocol_args(args))
                .collect::<Vec<_>>(),
        )),
        "client/runtime/systems/continuity/session_continuity_vault.ts" => Some((
            "continuity-runtime".to_string(),
            std::iter::once("session-continuity-vault".to_string())
                .chain(normalize_session_continuity_vault_args(args))
                .collect::<Vec<_>>(),
        )),
        "client/runtime/systems/continuity/sovereign_resurrection_substrate.ts" => Some((
            "runtime-systems".to_string(),
            std::iter::once(
                "--system-id=SYSTEMS-CONTINUITY-SOVEREIGN_RESURRECTION_SUBSTRATE".to_string(),
            )
            .chain(args.iter().cloned())
            .collect::<Vec<_>>(),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod cli_domain_wrapper_redirect_tests {
    use super::*;

    #[test]
    fn debug_wrapper_redirects_to_control_plane() {
        let (domain, args) = maybe_redirect_ts_wrapper_to_core_domain(
            "client/runtime/systems/ops/protheus_debug_diagnostics.ts",
            &["status".to_string()],
        )
        .expect("redirect");
        assert_eq!(domain, "protheus-control-plane");
        assert_eq!(args, vec!["status"]);
    }

    #[test]
    fn continuity_wrapper_redirects_with_surface_prefix() {
        let (domain, args) = maybe_redirect_ts_wrapper_to_core_domain(
            "client/runtime/systems/continuity/session_continuity_vault.ts",
            &["restore".to_string(), "--id=s1".to_string()],
        )
        .expect("redirect");
        assert_eq!(domain, "continuity-runtime");
        assert_eq!(
            args,
            vec![
                "session-continuity-vault".to_string(),
                "get".to_string(),
                "--id=s1".to_string()
            ]
        );
    }

    #[test]
    fn rust_hotpath_wrapper_redirects_to_kernel_with_status_default() {
        let (domain, args) = maybe_redirect_ts_wrapper_to_core_domain(
            "client/runtime/systems/ops/rust_hotpath_inventory.ts",
            &["--policy=foo.json".to_string()],
        )
        .expect("redirect");
        assert_eq!(domain, "rust-hotpath-inventory-kernel");
        assert_eq!(args, vec!["status".to_string(), "--policy=foo.json".to_string()]);
    }

    #[test]
    fn top50_wrapper_redirects_with_run_default() {
        let (domain, args) = maybe_redirect_ts_wrapper_to_core_domain(
            "client/runtime/systems/ops/top50_roi_sweep.ts",
            &["--max=25".to_string()],
        )
        .expect("redirect");
        assert_eq!(domain, "top50-roi-sweep-kernel");
        assert_eq!(args, vec!["run".to_string(), "--max=25".to_string()]);
    }

    #[test]
    fn release_semver_wrapper_redirects_to_core_domain() {
        let (domain, args) = maybe_redirect_ts_wrapper_to_core_domain(
            "client/runtime/systems/ops/release_semver_contract.ts",
            &["run".to_string(), "--write=1".to_string()],
        )
        .expect("redirect");
        assert_eq!(domain, "release-semver-contract");
        assert_eq!(args, vec!["run".to_string(), "--write=1".to_string()]);
    }

    #[test]
    fn status_dashboard_wrapper_redirects_web_to_daemon_start() {
        let (domain, args) = maybe_redirect_ts_wrapper_to_core_domain(
            "client/runtime/systems/ops/protheus_status_dashboard.ts",
            &["--web".to_string(), "--dashboard-open=1".to_string()],
        )
        .expect("redirect");
        assert_eq!(domain, "daemon-control");
        assert_eq!(
            args,
            vec!["start".to_string(), "--dashboard-open=1".to_string()]
        );
    }
}
