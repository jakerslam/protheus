
const SCALE_IDS: [&str; 10] = [
    "V4-SCALE-001",
    "V4-SCALE-002",
    "V4-SCALE-003",
    "V4-SCALE-004",
    "V4-SCALE-005",
    "V4-SCALE-006",
    "V4-SCALE-007",
    "V4-SCALE-008",
    "V4-SCALE-009",
    "V4-SCALE-010",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramItem {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paths {
    pub state_path: PathBuf,
    pub latest_path: PathBuf,
    pub receipts_path: PathBuf,
    pub history_path: PathBuf,
    pub contract_dir: PathBuf,
    pub report_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budgets {
    pub max_cost_per_user_usd: f64,
    pub max_p95_latency_ms: i64,
    pub max_p99_latency_ms: i64,
    pub error_budget_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub version: String,
    pub enabled: bool,
    pub strict_default: bool,
    pub items: Vec<ProgramItem>,
    pub stage_gates: Vec<String>,
    pub paths: Paths,
    pub budgets: Budgets,
    pub policy_path: PathBuf,
}

fn normalize_id(v: &str) -> String {
    let out = clean(v.replace('`', ""), 80).to_ascii_uppercase();
    if out.len() == 12 && out.starts_with("V4-SCALE-") {
        out
    } else {
        String::new()
    }
}

fn to_bool(v: Option<&str>, fallback: bool) -> bool {
    lane_utils::parse_bool(v, fallback)
}

fn clamp_int(v: Option<i64>, lo: i64, hi: i64, fallback: i64) -> i64 {
    let Some(mut n) = v else {
        return fallback;
    };
    if n < lo {
        n = lo;
    }
    if n > hi {
        n = hi;
    }
    n
}

fn read_json(path: &Path) -> Value {
    lane_utils::read_json(path).unwrap_or(Value::Null)
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, row)
        .map_err(|e| format!("append_jsonl_failed:{}:{e}", path.display()))
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    lane_utils::write_json(path, value)
        .map_err(|e| format!("write_json_failed:{}:{e}", path.display()))
}

fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn resolve_path(root: &Path, raw: Option<&Value>, fallback_rel: &str) -> PathBuf {
    let fallback = root.join(fallback_rel);
    let Some(raw) = raw.and_then(Value::as_str) else {
        return fallback;
    };
    let clean_raw = clean(raw, 400);
    if clean_raw.is_empty() {
        return fallback;
    }
    let p = PathBuf::from(clean_raw);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

fn stable_hash(input: &str, len: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let hex = hex::encode(hasher.finalize());
    hex[..len.min(hex.len())].to_string()
}

pub fn default_policy(root: &Path) -> Policy {
    Policy {
        version: "1.0".to_string(),
        enabled: true,
        strict_default: true,
        items: SCALE_IDS
            .iter()
            .map(|id| ProgramItem {
                id: (*id).to_string(),
                title: (*id).to_string(),
            })
            .collect(),
        stage_gates: vec![
            "1k".to_string(),
            "10k".to_string(),
            "100k".to_string(),
            "1M".to_string(),
        ],
        paths: Paths {
            state_path: root.join("local/state/ops/scale_readiness_program/state.json"),
            latest_path: root.join("local/state/ops/scale_readiness_program/latest.json"),
            receipts_path: root.join("local/state/ops/scale_readiness_program/receipts.jsonl"),
            history_path: root.join("local/state/ops/scale_readiness_program/history.jsonl"),
            contract_dir: root.join("client/runtime/config/scale_readiness"),
            report_dir: root.join("local/state/ops/scale_readiness_program/reports"),
        },
        budgets: Budgets {
            max_cost_per_user_usd: 0.18,
            max_p95_latency_ms: 250,
            max_p99_latency_ms: 450,
            error_budget_pct: 0.01,
        },
        policy_path: root.join("client/runtime/config/scale_readiness_program_policy.json"),
    }
}
