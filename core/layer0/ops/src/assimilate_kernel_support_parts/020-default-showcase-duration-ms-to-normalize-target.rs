
pub const DEFAULT_SHOWCASE_DURATION_MS: u64 = 10_000;
pub const DEFAULT_REALTIME_DURATION_MS: u64 = 0;
const DEFAULT_PREWARM_TTL_MS: i64 = 5 * 60 * 1000;
const BAR_WIDTH: usize = 64;
const FILLED_CHAR: char = '█';
const EMPTY_CHAR: char = '░';
const STATE_DIR_REL: &str = "local/state/tools/assimilate";
const PREWARM_STATE_REL: &str = "local/state/tools/assimilate/prewarm.json";
const METRICS_STATE_REL: &str = "local/state/tools/assimilate/metrics.json";
const RECON_MAX_FILES: usize = 2500;
const RECON_MAX_DEPTH: usize = 8;
const ASSIMILATION_PROTOCOL_VERSION: &str = "infring_assimilation_protocol_v1";

#[derive(Clone, Copy)]

pub struct Stage {
    pub percent: u32,
    pub label: &'static str,
    pub weight: f64,
}

pub const STAGES: [Stage; 5] = [
    Stage {
        percent: 20,
        label: "Spinning up swarm (5,000 agents)",
        weight: 0.2,
    },
    Stage {
        percent: 50,
        label: "Parallel analysis (manifest + docs)",
        weight: 0.3,
    },
    Stage {
        percent: 80,
        label: "Building bridges & adapters",
        weight: 0.3,
    },
    Stage {
        percent: 95,
        label: "Validating + signing receipts",
        weight: 0.15,
    },
    Stage {
        percent: 100,
        label: "Assimilation complete. Ready to use.",
        weight: 0.05,
    },
];

#[derive(Debug, Default)]
pub struct Options {
    pub target: String,
    pub duration_ms: Option<u64>,
    pub showcase: bool,
    pub scaffold_payload: bool,
    pub json: bool,
    pub prewarm: bool,
    pub allow_local_simulation: bool,
    pub plan_only: bool,
    pub strict: bool,
    pub hard_selector: String,
    pub selector_bypass: bool,
    pub core_domain: String,
    pub core_args_base64: String,
    pub help: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub domain: String,
    pub args: Vec<String>,
}

#[derive(Debug)]
pub struct RunResult {
    pub status: i32,
    pub latency_ms: u64,
    pub payload: Option<Value>,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetMetrics {
    pub count: u64,
    pub ok_count: u64,
    pub fail_count: u64,
    pub last_latency_ms: u64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub updated_at: String,
    #[serde(default)]
    pub latencies_ms: Vec<u64>,
}

impl Default for TargetMetrics {
    fn default() -> Self {
        Self {
            count: 0,
            ok_count: 0,
            fail_count: 0,
            last_latency_ms: 0,
            p50_ms: 0,
            p95_ms: 0,
            updated_at: now_iso(),
            latencies_ms: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetricsState {
    schema_version: String,
    #[serde(default)]
    targets: BTreeMap<String, TargetMetrics>,
}

impl Default for MetricsState {
    fn default() -> Self {
        Self {
            schema_version: "assimilate_metrics_v1".to_string(),
            targets: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrewarmState {
    ts_ms: i64,
    ts: String,
}

impl Default for PrewarmState {
    fn default() -> Self {
        Self {
            ts_ms: 0,
            ts: now_iso(),
        }
    }
}

pub fn usage() {
    println!("Usage: infring assimilate <target> [--payload-base64=...] [--strict=1] [--showcase=1] [--duration-ms=<n>] [--json=1] [--scaffold-payload=1] [--allow-local-simulation=1] [--plan-only=1] [--hard-selector=<selector>] [--selector-bypass=1]");
    println!();
    println!("Known targets route to governed core bridge lanes. Unknown targets fail as unadmitted unless --allow-local-simulation=1 is set.");
}

fn parse_bool_flag(raw: Option<&str>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw, fallback)
}

fn assimilation_denial_priority(code: &str) -> usize {
    match code {
        "assimilation_selector_bypass_rejected" => 0,
        "assimilation_hard_selector_closure_reject" => 1,
        "assimilation_candidate_closure_incomplete" => 2,
        "assimilation_manifest_surface_missing" => 3,
        "assimilation_structure_surface_empty" => 4,
        _ => 100,
    }
}

fn normalize_target(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if out.len() >= 120 {
            break;
        }
        if ch.is_control() {
            continue;
        }
        out.push(ch);
    }
    out.trim().to_string()
}
