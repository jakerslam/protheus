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
    "verify",
    "inspect",
    "replay",
    "verify-install",
    "dream",
    "compact",
    "proactive_daemon",
    "kairos",
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
