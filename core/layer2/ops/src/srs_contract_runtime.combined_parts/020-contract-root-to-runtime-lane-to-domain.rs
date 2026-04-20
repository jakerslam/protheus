
const CONTRACT_ROOT: &str = "planes/contracts/srs";
const STATE_ROOT: &str = "local/state/ops/srs_contract_runtime";
const HISTORY_FILE: &str = "history.jsonl";

fn contract_path(root: &Path, id: &str) -> PathBuf {
    root.join(CONTRACT_ROOT).join(format!("{id}.json"))
}

fn latest_path(root: &Path, id: &str) -> PathBuf {
    root.join(STATE_ROOT).join(id).join("latest.json")
}

fn history_path(root: &Path) -> PathBuf {
    root.join(STATE_ROOT).join(HISTORY_FILE)
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir_failed:{e}"))?;
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read_failed:{e}"))?;
    serde_json::from_str::<Value>(&raw).map_err(|e| format!("parse_failed:{e}"))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let mut body = serde_json::to_string_pretty(value).map_err(|e| format!("encode_failed:{e}"))?;
    body.push('\n');
    fs::write(path, body).map_err(|e| format!("write_failed:{e}"))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let line = serde_json::to_string(value).map_err(|e| format!("encode_failed:{e}"))?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open_failed:{e}"))?;
    use std::io::Write;
    file.write_all(format!("{line}\n").as_bytes())
        .map_err(|e| format!("append_failed:{e}"))
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let pref = format!("--{key}=");
    let long = format!("--{key}");
    let mut idx = 0usize;
    while idx < argv.len() {
        let token = argv[idx].trim();
        if let Some(v) = token.strip_prefix(&pref) {
            return Some(v.to_string());
        }
        if token == long && idx + 1 < argv.len() {
            return Some(argv[idx + 1].clone());
        }
        idx += 1;
    }
    None
}

fn parse_id(argv: &[String]) -> Option<String> {
    parse_flag(argv, "id")
        .or_else(|| {
            argv.iter()
                .skip(1)
                .find(|row| !row.trim().starts_with('-'))
                .cloned()
        })
        .map(|v| v.trim().to_ascii_uppercase())
        .filter(|v| !v.is_empty())
}

fn parse_bool(raw: Option<String>, fallback: bool) -> bool {
    match raw {
        Some(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        None => fallback,
    }
}

fn normalize_id(raw: &str) -> Option<String> {
    let id = raw.trim().to_ascii_uppercase();
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

fn parse_id_list(root: &Path, argv: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();

    if let Some(csv) = parse_flag(argv, "ids") {
        for token in csv.split(',') {
            if let Some(id) = normalize_id(token) {
                out.push(id);
            }
        }
    }

    if let Some(file) = parse_flag(argv, "ids-file") {
        let fpath = if Path::new(&file).is_absolute() {
            PathBuf::from(file)
        } else {
            root.join(file)
        };
        let raw = fs::read_to_string(&fpath).map_err(|e| format!("ids_file_read_failed:{e}"))?;
        for line in raw.lines() {
            for token in line.split(',') {
                if let Some(id) = normalize_id(token) {
                    out.push(id);
                }
            }
        }
    }

    if out.is_empty() {
        if let Some(id) = parse_id(argv) {
            out.push(id);
        }
    }

    if out.is_empty() {
        return Err("missing_ids".to_string());
    }

    out.sort();
    out.dedup();
    Ok(out)
}

fn validate_contract_shape(id: &str, contract: &Value) -> Result<(), String> {
    let cid = contract
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "contract_missing_id".to_string())?;
    if cid != id {
        return Err("contract_id_mismatch".to_string());
    }
    if contract
        .get("upgrade")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        return Err("contract_missing_upgrade".to_string());
    }
    if contract
        .get("layer_map")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        return Err("contract_missing_layer_map".to_string());
    }
    if contract
        .get("deliverables")
        .and_then(Value::as_array)
        .map(|rows| rows.is_empty())
        .unwrap_or(true)
    {
        return Err("contract_missing_deliverables".to_string());
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct DispatchTarget {
    plane: String,
    source_path: String,
    argv: Vec<String>,
}

fn runtime_lane_to_domain(path: &str) -> Option<&'static str> {
    match path.trim() {
        "core/layer0/ops/src/business_plane.rs" => Some("business-plane"),
        "core/layer0/ops/src/canyon_plane.rs" => Some("canyon-plane"),
        "core/layer0/ops/src/f100_readiness_program.rs" => Some("f100-readiness-program"),
        "core/layer0/ops/src/nexus_plane.rs" => Some("nexus-plane"),
        "core/layer0/ops/src/runtime_systems.rs" => Some("runtime-systems"),
        "core/layer0/ops/src/security_plane.rs" => Some("security-plane"),
        "core/layer0/ops/src/skills_plane.rs" => Some("skills-plane"),
        "core/layer0/ops/src/workflow_controller.rs" => Some("workflow-controller"),
        "core/layer0/ops/src/workflow_executor.rs" => Some("workflow-executor"),
        _ => None,
    }
}
