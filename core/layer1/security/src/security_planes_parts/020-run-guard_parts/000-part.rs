// FILE_SIZE_EXCEPTION: reason=Atomic multi-plane security dispatch block requires staged semantic extraction; owner=jay; expires=2026-04-12
fn parse_last_json_line(raw: &str) -> Option<Value> {
    let lines = raw.lines().collect::<Vec<_>>();
    for line in lines.iter().rev() {
        let candidate = line.trim();
        if !candidate.starts_with('{') {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(candidate) {
            return Some(value);
        }
    }
    None
}

#[derive(Debug, Clone)]
struct GuardZone {
    prefix: &'static str,
    min_clearance: i64,
    label: &'static str,
}

fn guard_zones() -> Vec<GuardZone> {
    vec![
        GuardZone {
            prefix: "systems/",
            min_clearance: 3,
            label: "infrastructure",
        },
        GuardZone {
            prefix: "config/",
            min_clearance: 3,
            label: "configuration",
        },
        GuardZone {
            prefix: "memory/",
            min_clearance: 3,
            label: "memory_tools",
        },
        GuardZone {
            prefix: "habits/",
            min_clearance: 2,
            label: "habits_reflexes",
        },
        GuardZone {
            prefix: "local/state/",
            min_clearance: 1,
            label: "state_data",
        },
    ]
}

fn guard_protected_files() -> Vec<&'static str> {
    vec![
        "docs/workspace/AGENT-CONSTITUTION.md",
        "config/constitution_guardian_policy.json",
    ]
}

fn guard_match_zone(file_rel: &str) -> (i64, String) {
    if guard_protected_files().contains(&file_rel) {
        return (4, "protected_core".to_string());
    }
    for zone in guard_zones() {
        if file_rel.starts_with(zone.prefix) {
            return (zone.min_clearance, zone.label.to_string());
        }
    }
    (3, "default_protect".to_string())
}

fn guard_state_logs(repo_root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let base = local_state_root(repo_root).join("security");
    (
        base.join("break_glass.jsonl"),
        base.join("remote_request_gate.jsonl"),
        base.join("risky_env_toggle_gate.jsonl"),
    )
}

pub fn run_guard(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let first = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "run".to_string());
    if first == "status" {
        return (
            json!({
                "ok": true,
                "type": "security_guard_status",
                "ts": now_iso(),
                "zones": guard_zones().iter().map(|z| json!({"prefix": z.prefix, "min_clearance": z.min_clearance, "label": z.label})).collect::<Vec<_>>(),
                "protected_files": guard_protected_files(),
                "default_clearance": 2
            }),
            0,
        );
    }

    let files = if let Some(csv) = flag(&parsed, "files") {
        split_csv(csv, 200)
    } else {
        parsed
            .positional
            .iter()
            .filter(|v| *v != "run")
            .map(normalize_rel)
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>()
    };
    if files.is_empty() {
        return (
            json!({
                "ok": false,
                "blocked": true,
                "type": "security_guard",
                "error": "files_required",
                "usage": "security-plane guard --files=<path1,path2,...>"
            }),
            2,
        );
    }

    let clearance = std::env::var("CLEARANCE")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(2)
        .clamp(1, 4);
    let break_glass = std::env::var("BREAK_GLASS")
        .ok()
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);
    let approval_note = clean(
        std::env::var("APPROVAL_NOTE")
            .or_else(|_| std::env::var("SECOND_APPROVAL_NOTE"))
            .unwrap_or_default(),
        400,
    );
    let request_source = clean(
        std::env::var("REQUEST_SOURCE").unwrap_or_else(|_| "local".to_string()),
        60,
    )
    .to_ascii_lowercase();
    let request_action = clean(
        std::env::var("REQUEST_ACTION").unwrap_or_else(|_| "apply".to_string()),
        60,
    )
    .to_ascii_lowercase();
    let remote_source = matches!(
        request_source.as_str(),
        "slack" | "discord" | "webhook" | "email" | "api" | "remote" | "moltbook"
    );
    let proposal_action = matches!(
        request_action.as_str(),
        "propose" | "proposal" | "dry_run" | "dry-run" | "audit"
    );

    // Integrity gate remains authoritative before clearance checks.
    let (integrity, _) =
        crate::run_integrity_reseal(repo_root, &["check".to_string(), "--staged=0".to_string()]);
    let integrity_ok = integrity
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !integrity_ok {
        return (
            json!({
                "ok": false,
                "blocked": true,
                "break_glass": false,
                "reason": "integrity_violation",
                "ts": now_iso(),
                "integrity": integrity
            }),
            1,
        );
    }

    let mut requirements = Vec::<Value>::new();
    let mut required_max = 1i64;
    for file in &files {
        let (min_clearance, label) = guard_match_zone(file);
        required_max = required_max.max(min_clearance);
        requirements.push(json!({
            "file": file,
            "min_clearance": min_clearance,
            "label": label
        }));
    }
    let clearance_ok = clearance >= required_max;

    let break_glass_allowed = break_glass
        && approval_note.len() >= 12
        && (!remote_source
            || proposal_action
            || (std::env::var("REMOTE_DIRECT_OVERRIDE").ok().as_deref() == Some("1")
                && !clean(std::env::var("APPROVER_ID").unwrap_or_default(), 120).is_empty()
                && !clean(std::env::var("SECOND_APPROVER_ID").unwrap_or_default(), 120)
                    .is_empty()));

    let blocked = !clearance_ok && !break_glass_allowed;
    let ok = !blocked;
    let reason = if blocked {
        "clearance_insufficient"
    } else if !clearance_ok && break_glass_allowed {
        "break_glass"
    } else {
        "approved"
    };

    let (break_glass_log, remote_log, risky_log) = guard_state_logs(repo_root);
    if break_glass {
        let _ = append_jsonl(
            &break_glass_log,
            &json!({
                "ts": now_iso(),
                "type": "break_glass_attempt",
                "ok": ok,
                "reason": reason,
                "request_source": request_source,
                "request_action": request_action,
                "approval_note_len": approval_note.len(),
                "required_clearance": required_max,
                "clearance": clearance,
                "files": files
            }),
        );
    }
    if remote_source {
        let _ = append_jsonl(
            &remote_log,
            &json!({
                "ts": now_iso(),
                "type": "remote_request_gate",
                "ok": ok,
                "reason": reason,
                "request_source": request_source,
                "request_action": request_action,
                "proposal_action": proposal_action
            }),
        );
    }
    let risky_toggles = [
        "AUTONOMY_ENABLED",
        "AUTONOMY_MODEL_CATALOG_AUTO_APPLY",
        "AUTONOMY_MODEL_CATALOG_AUTO_BREAK_GLASS",
        "REMOTE_DIRECT_OVERRIDE",
        "BREAK_GLASS",
    ]
    .iter()
    .filter_map(|k| {
        std::env::var(k)
            .ok()
            .map(|v| (k.to_string(), v))
            .filter(|(_, v)| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
    })
    .collect::<Vec<_>>();
    if !risky_toggles.is_empty() {
        let _ = append_jsonl(
            &risky_log,
            &json!({
                "ts": now_iso(),
                "type": "risky_env_toggle_gate",
                "ok": ok,
                "reason": reason,
                "toggles": risky_toggles
            }),
        );
    }

    (
        json!({
            "ok": ok,
            "blocked": blocked,
            "break_glass": reason == "break_glass",
            "reason": reason,
            "ts": now_iso(),
            "request_source": request_source,
            "request_action": request_action,
            "clearance": clearance,
            "required_clearance": required_max,
            "requirements": requirements
        }),
        if ok { 0 } else { 1 },
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct AntiSabotagePolicy {
    version: String,
    protected_roots: Vec<String>,
    extensions: Vec<String>,
    state_dir: String,
    quarantine_dir: String,
    snapshots_dir: String,
    incident_log: String,
    state_file: String,
    watcher_state_file: String,
    watcher_interval_ms: i64,
    max_snapshots: usize,
    verify_strict_default: bool,
    auto_reset_default: bool,
    watcher_strict_default: bool,
    watcher_auto_reset_default: bool,
}

impl Default for AntiSabotagePolicy {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            protected_roots: vec![
                "systems".to_string(),
                "config".to_string(),
                "lib".to_string(),
                "adaptive".to_string(),
            ],
            extensions: vec![
                ".js".to_string(),
                ".ts".to_string(),
                ".json".to_string(),
                ".yaml".to_string(),
                ".yml".to_string(),
            ],
            state_dir: "local/state/security/anti_sabotage".to_string(),
            quarantine_dir: "local/state/security/anti_sabotage/quarantine".to_string(),
            snapshots_dir: "local/state/security/anti_sabotage/snapshots".to_string(),
            incident_log: "local/state/security/anti_sabotage/incidents.jsonl".to_string(),
            state_file: "local/state/security/anti_sabotage/state.json".to_string(),
            watcher_state_file: "local/state/security/anti_sabotage/watcher_state.json".to_string(),
            watcher_interval_ms: 30_000,
            max_snapshots: 20,
            verify_strict_default: true,
            auto_reset_default: true,
            watcher_strict_default: false,
            watcher_auto_reset_default: true,
        }
    }
}

fn load_anti_sabotage_policy(repo_root: &Path, parsed: &ParsedArgs) -> AntiSabotagePolicy {
    let policy_path = flag(parsed, "policy")
        .map(|v| resolve_runtime_or_state(repo_root, v))
        .unwrap_or_else(|| runtime_config_path(repo_root, "anti_sabotage_policy.json"));
    if !policy_path.exists() {
        return AntiSabotagePolicy::default();
    }
    match fs::read_to_string(&policy_path) {
        Ok(raw) => serde_json::from_str::<AntiSabotagePolicy>(&raw).unwrap_or_default(),
        Err(_) => AntiSabotagePolicy::default(),
    }
}

fn anti_sabotage_walk_files(
    repo_root: &Path,
    policy: &AntiSabotagePolicy,
) -> Vec<(String, PathBuf)> {
    let runtime = runtime_root(repo_root);
    let ext_set = policy
        .extensions
        .iter()
        .map(|v| {
            let c = clean(v, 16).to_ascii_lowercase();
            if c.starts_with('.') {
                c
            } else {
                format!(".{c}")
            }
        })
        .collect::<BTreeSet<_>>();
    let mut out = Vec::<(String, PathBuf)>::new();
    for rel_root in &policy.protected_roots {
        let root = runtime.join(normalize_rel(rel_root));
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|v| v.to_str())
                .map(|v| format!(".{}", v.to_ascii_lowercase()))
                .unwrap_or_default();
            if !ext_set.is_empty() && !ext_set.contains(&ext) {
                continue;
            }
            let rel = path
                .strip_prefix(&runtime)
                .ok()
                .map(|v| normalize_rel(v.to_string_lossy()))
                .unwrap_or_else(|| normalize_rel(path.to_string_lossy()));
            out.push((rel, path.to_path_buf()));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn anti_sabotage_paths(
    repo_root: &Path,
    policy: &AntiSabotagePolicy,
