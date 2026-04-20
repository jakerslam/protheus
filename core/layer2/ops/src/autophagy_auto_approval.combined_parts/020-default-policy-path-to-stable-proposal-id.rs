
const DEFAULT_POLICY_PATH: &str = "client/runtime/config/autophagy_auto_approval_policy.json";
const DEFAULT_STATE_PATH: &str = "local/state/autonomy/autophagy_auto_approval/state.json";
const DEFAULT_LATEST_PATH: &str = "local/state/autonomy/autophagy_auto_approval/latest.json";
const DEFAULT_RECEIPTS_PATH: &str = "local/state/autonomy/autophagy_auto_approval/receipts.jsonl";
const DEFAULT_REGRETS_PATH: &str = "local/state/autonomy/autophagy_auto_approval/regrets.jsonl";

const USAGE: &[&str] = &[
    "Usage:",
    "  protheus-ops autophagy-auto-approval evaluate --proposal-json=<json>|--proposal-file=<path> [--apply=1|0] [--policy=<path>] [--state-path=<path>] [--latest-path=<path>] [--receipts-path=<path>] [--regrets-path=<path>]",
    "  protheus-ops autophagy-auto-approval monitor --proposal-id=<id> [--drift=<float>] [--yield-drop=<float>] [--apply=1|0] [--policy=<path>] [--state-path=<path>] [--latest-path=<path>] [--receipts-path=<path>] [--regrets-path=<path>]",
    "  protheus-ops autophagy-auto-approval commit --proposal-id=<id> [--reason=<text>] [--policy=<path>] [--state-path=<path>] [--latest-path=<path>] [--receipts-path=<path>] [--regrets-path=<path>]",
    "  protheus-ops autophagy-auto-approval rollback --proposal-id=<id> [--reason=<text>] [--policy=<path>] [--state-path=<path>] [--latest-path=<path>] [--receipts-path=<path>] [--regrets-path=<path>]",
    "  protheus-ops autophagy-auto-approval status [--policy=<path>] [--state-path=<path>]",
];

#[derive(Clone, Debug)]
struct Policy {
    enabled: bool,
    min_confidence: f64,
    min_historical_success_rate: f64,
    max_impact_score: f64,
    excluded_types: Vec<String>,
    auto_rollback_on_degradation: bool,
    max_drift_delta: f64,
    max_yield_drop: f64,
    rollback_window_minutes: i64,
    regret_issue_label: String,
    state_path: PathBuf,
    latest_path: PathBuf,
    receipts_path: PathBuf,
    regrets_path: PathBuf,
}

#[derive(Clone, Debug)]
struct ProposalSummary {
    id: String,
    title: String,
    proposal_type: String,
    confidence: f64,
    historical_success_rate: f64,
    impact_score: f64,
    raw: Value,
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn usage() {
    for line in USAGE {
        println!("{line}");
    }
}

fn parse_bool(raw: Option<&str>, fallback: bool) -> bool {
    let Some(v) = raw else {
        return fallback;
    };
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn parse_f64(raw: Option<&str>) -> Option<f64> {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
}

fn resolve_path(root: &Path, raw: Option<String>, fallback: &Path) -> PathBuf {
    let path = raw
        .map(PathBuf::from)
        .unwrap_or_else(|| fallback.to_path_buf());
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("missing_parent_for_path:{}", path.display()))?;
    fs::create_dir_all(parent).map_err(|e| format!("create_dir_all_failed:{e}"))
}

fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|e| format!("encode_json_failed:{e}"))?,
    )
    .map_err(|e| format!("write_json_failed:{e}"))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut existing = fs::read_to_string(path).unwrap_or_default();
    existing
        .push_str(&serde_json::to_string(value).map_err(|e| format!("encode_jsonl_failed:{e}"))?);
    existing.push('\n');
    fs::write(path, existing).map_err(|e| format!("write_jsonl_failed:{e}"))
}

fn array_from<'a>(object: &'a mut Map<String, Value>, key: &str) -> &'a mut Vec<Value> {
    let value = object
        .entry(key.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !value.is_array() {
        *value = Value::Array(Vec::new());
    }
    value.as_array_mut().expect("array")
}

fn value_string(value: Option<&Value>, fallback: &str) -> String {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn value_f64(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback)
}

fn stable_proposal_id(proposal: &Value) -> String {
    let title = proposal
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("proposal");
    let kind = proposal
        .get("type")
        .or_else(|| proposal.get("kind"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("generic");
    let seed = json!({
        "title": title,
        "proposal_type": kind,
        "payload": proposal
    });
    deterministic_receipt_hash(&seed)[..16].to_string()
}
