
pub fn run_anti_sabotage_shield(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let policy = load_anti_sabotage_policy(repo_root, &parsed);
    match cmd.as_str() {
        "snapshot" => match anti_sabotage_snapshot(repo_root, &policy, flag(&parsed, "label")) {
            Ok(out) => (out, 0),
            Err(err) => (
                json!({"ok": false, "type":"anti_sabotage_snapshot", "error": clean(err, 220)}),
                1,
            ),
        },
        "verify" => {
            let strict = bool_flag(&parsed, "strict", policy.verify_strict_default);
            let auto_reset = bool_flag(&parsed, "auto-reset", policy.auto_reset_default);
            let snapshot_ref = flag(&parsed, "snapshot").unwrap_or("latest");
            match anti_sabotage_verify(repo_root, &policy, snapshot_ref, strict, auto_reset) {
                Ok(result) => result,
                Err(err) => (
                    json!({"ok": false, "type":"anti_sabotage_verify", "error": clean(err, 220)}),
                    1,
                ),
            }
        }
        "watch" => {
            let strict = bool_flag(&parsed, "strict", policy.watcher_strict_default);
            let auto_reset = bool_flag(&parsed, "auto-reset", policy.watcher_auto_reset_default);
            let interval_ms = flag(&parsed, "interval-ms")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(policy.watcher_interval_ms.max(250) as u64)
                .clamp(250, 300_000);
            let iterations = flag(&parsed, "iterations")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(1)
                .clamp(1, 1000);
            if bool_flag(&parsed, "bootstrap-snapshot", false) {
                let _ = anti_sabotage_snapshot(repo_root, &policy, Some("watch-bootstrap"));
            }
            let snapshot_ref = flag(&parsed, "snapshot").unwrap_or("latest").to_string();
            let mut last = json!({"ok": true, "type": "anti_sabotage_watch", "iterations": 0});
            let mut last_code = 0;
            for idx in 0..iterations {
                match anti_sabotage_verify(repo_root, &policy, &snapshot_ref, strict, auto_reset) {
                    Ok((verify, code)) => {
                        last = json!({
                            "ok": verify.get("ok").and_then(Value::as_bool).unwrap_or(false),
                            "type": "anti_sabotage_watch",
                            "iteration": idx + 1,
                            "iterations": iterations,
                            "verify": verify
                        });
                        last_code = code;
                    }
                    Err(err) => {
                        last = json!({"ok": false, "type":"anti_sabotage_watch", "error": clean(err, 220)});
                        last_code = 1;
                    }
                }
                if idx + 1 < iterations {
                    thread::sleep(Duration::from_millis(interval_ms));
                }
            }
            (last, last_code)
        }
        "status" => (anti_sabotage_status(repo_root, &policy), 0),
        _ => (
            json!({
                "ok": false,
                "type": "anti_sabotage_shield",
                "error": "unknown_command",
                "usage": [
                    "anti-sabotage-shield snapshot [--label=<id>]",
                    "anti-sabotage-shield verify [--snapshot=latest|<id>] [--strict=1|0] [--auto-reset=1|0]",
                    "anti-sabotage-shield watch [--snapshot=latest|<id>] [--strict=1|0] [--auto-reset=1|0] [--interval-ms=<n>] [--iterations=<n>]",
                    "anti-sabotage-shield status"
                ]
            }),
            2,
        ),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct ConstitutionPolicy {
    version: String,
    constitution_path: String,
    state_dir: String,
    veto_window_days: i64,
    min_approval_note_chars: usize,
    require_dual_approval: bool,
    enforce_inheritance_lock: bool,
    emergency_rollback_requires_approval: bool,
}

impl Default for ConstitutionPolicy {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            constitution_path: "docs/workspace/AGENT-CONSTITUTION.md".to_string(),
            state_dir: "local/state/security/constitution_guardian".to_string(),
            veto_window_days: 14,
            min_approval_note_chars: 12,
            require_dual_approval: true,
            enforce_inheritance_lock: true,
            emergency_rollback_requires_approval: true,
        }
    }
}

fn load_constitution_policy(repo_root: &Path, parsed: &ParsedArgs) -> ConstitutionPolicy {
    let policy_path = flag(parsed, "policy")
        .map(|v| resolve_runtime_or_state(repo_root, v))
        .unwrap_or_else(|| runtime_config_path(repo_root, "constitution_guardian_policy.json"));
    if !policy_path.exists() {
        return ConstitutionPolicy::default();
    }
    match fs::read_to_string(&policy_path) {
        Ok(raw) => serde_json::from_str::<ConstitutionPolicy>(&raw).unwrap_or_default(),
        Err(_) => ConstitutionPolicy::default(),
    }
}

#[derive(Debug, Clone)]
struct ConstitutionPaths {
    constitution: PathBuf,
    state_dir: PathBuf,
    genesis: PathBuf,
    proposals_dir: PathBuf,
    events: PathBuf,
    history_dir: PathBuf,
    active_state: PathBuf,
}

fn constitution_paths(repo_root: &Path, policy: &ConstitutionPolicy) -> ConstitutionPaths {
    let constitution = resolve_runtime_or_state(repo_root, &policy.constitution_path);
    let state_dir = resolve_runtime_or_state(repo_root, &policy.state_dir);
    ConstitutionPaths {
        constitution,
        genesis: state_dir.join("genesis.json"),
        proposals_dir: state_dir.join("proposals"),
        events: state_dir.join("events.jsonl"),
        history_dir: state_dir.join("history"),
        active_state: state_dir.join("active_state.json"),
        state_dir,
    }
}

fn proposal_path(paths: &ConstitutionPaths, proposal_id: &str) -> PathBuf {
    paths.proposals_dir.join(proposal_id).join("proposal.json")
}

fn load_proposal(paths: &ConstitutionPaths, proposal_id: &str) -> Option<Value> {
    let path = proposal_path(paths, proposal_id);
    if !path.exists() {
        return None;
    }
    Some(read_json_or(&path, Value::Null))
}

fn save_proposal(
    paths: &ConstitutionPaths,
    proposal_id: &str,
    value: &Value,
) -> Result<(), String> {
    write_json_atomic(&proposal_path(paths, proposal_id), value)
}

fn proposal_status(value: &Value) -> String {
    clean(
        value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        64,
    )
}
