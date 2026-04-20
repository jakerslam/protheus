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
