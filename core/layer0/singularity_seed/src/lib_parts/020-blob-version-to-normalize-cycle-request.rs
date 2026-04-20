
const BLOB_VERSION: u32 = 1;
const MANIFEST_SIGNING_KEY: &str = "singularity-seed-manifest-signing-key-v1";
const DRIFT_FAIL_CLOSED_THRESHOLD_PCT: f64 = 2.0;
const MAX_DRIFT_OVERRIDES: usize = 64;
const MAX_DRIFT_OVERRIDE_PCT: f64 = 100.0;
const MAX_BLOB_ROOT_CHARS: usize = 512;

pub const AUTOGENESIS_LOOP_ID: &str = "autogenesis_loop";
pub const DUAL_BRAIN_LOOP_ID: &str = "dual_brain_loop";
pub const RED_LEGION_LOOP_ID: &str = "red_legion_loop";
pub const BLOB_MORPHING_LOOP_ID: &str = "blob_morphing_loop";

pub const LOOP_IDS: [&str; 4] = [
    AUTOGENESIS_LOOP_ID,
    DUAL_BRAIN_LOOP_ID,
    RED_LEGION_LOOP_ID,
    BLOB_MORPHING_LOOP_ID,
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlobManifestEntry {
    pub id: String,
    pub hash: String,
    pub version: u32,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FoldedBlob {
    id: String,
    version: u32,
    payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopState {
    pub loop_id: String,
    pub generation: u32,
    pub quality_score: f64,
    pub drift_pct: f64,
    pub last_mutation: String,
    pub insights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopCycleOutcome {
    pub loop_id: String,
    pub previous_generation: u32,
    pub next_generation: u32,
    pub drift_pct: f64,
    pub frozen_hash: String,
    pub evolved_hash: String,
    pub unfolded_hash: String,
    pub unfolded_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CycleReport {
    pub ok: bool,
    pub fail_closed: bool,
    pub max_drift_pct: f64,
    pub threshold_pct: f64,
    pub sovereignty_index: f64,
    pub cycle_id: String,
    pub status: String,
    pub reasons: Vec<String>,
    pub manifest_path: String,
    pub outcomes: Vec<LoopCycleOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriftOverride {
    pub loop_id: String,
    pub drift_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CycleRequest {
    #[serde(default)]
    pub drift_overrides: Vec<DriftOverride>,
}

#[derive(Debug, Clone)]
pub enum SeedError {
    InvalidBlobId,
    UnknownBlob(String),
    MissingManifestEntry(String),
    MissingSignature(String),
    SignatureMismatch {
        id: String,
        expected: String,
        actual: String,
    },
    HashMismatch {
        scope: &'static str,
        expected: String,
        actual: String,
    },
    IdMismatch {
        expected: String,
        actual: String,
    },
    UnsupportedVersion {
        id: String,
        version: u32,
    },
    SerializeFailed(String),
    DeserializeFailed(String),
    CompressFailed(String),
    DecompressFailed(String),
    ManifestEncodeFailed(String),
    ManifestDecodeFailed(String),
    IoFailed(String),
    InvalidRequest(String),
}

impl Display for SeedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SeedError::InvalidBlobId => write!(f, "blob_id_required"),
            SeedError::UnknownBlob(blob_id) => write!(f, "unknown_blob_id:{blob_id}"),
            SeedError::MissingManifestEntry(blob_id) => {
                write!(f, "manifest_missing_blob:{blob_id}")
            }
            SeedError::MissingSignature(blob_id) => {
                write!(f, "manifest_missing_signature:{blob_id}")
            }
            SeedError::SignatureMismatch {
                id,
                expected,
                actual,
            } => write!(
                f,
                "manifest_signature_mismatch id={id} expected={expected} actual={actual}"
            ),
            SeedError::HashMismatch {
                scope,
                expected,
                actual,
            } => write!(
                f,
                "blob_hash_mismatch scope={scope} expected={expected} actual={actual}"
            ),
            SeedError::IdMismatch { expected, actual } => {
                write!(f, "blob_id_mismatch expected={expected} actual={actual}")
            }
            SeedError::UnsupportedVersion { id, version } => {
                write!(f, "unsupported_blob_version id={id} version={version}")
            }
            SeedError::SerializeFailed(msg) => write!(f, "serialize_failed:{msg}"),
            SeedError::DeserializeFailed(msg) => write!(f, "deserialize_failed:{msg}"),
            SeedError::CompressFailed(msg) => write!(f, "compress_failed:{msg}"),
            SeedError::DecompressFailed(msg) => write!(f, "decompress_failed:{msg}"),
            SeedError::ManifestEncodeFailed(msg) => write!(f, "manifest_encode_failed:{msg}"),
            SeedError::ManifestDecodeFailed(msg) => write!(f, "manifest_decode_failed:{msg}"),
            SeedError::IoFailed(msg) => write!(f, "io_failed:{msg}"),
            SeedError::InvalidRequest(msg) => write!(f, "invalid_request:{msg}"),
        }
    }
}

impl std::error::Error for SeedError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn normalize_loop_id(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if matches!(ch, '_' | '-' | ' ') {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .chars()
        .take(96)
        .collect::<String>()
}

fn is_known_loop_id(loop_id: &str) -> bool {
    LOOP_IDS.contains(&loop_id)
}

fn normalize_cycle_request(request: CycleRequest) -> CycleRequest {
    let mut deduped: HashMap<String, f64> = HashMap::new();
    for override_item in request.drift_overrides.into_iter().take(MAX_DRIFT_OVERRIDES) {
        let loop_id = normalize_loop_id(&override_item.loop_id);
        if loop_id.is_empty() || !is_known_loop_id(&loop_id) || !override_item.drift_pct.is_finite() {
            continue;
        }
        deduped.insert(loop_id, override_item.drift_pct.clamp(0.0, MAX_DRIFT_OVERRIDE_PCT));
    }

    let mut drift_overrides = deduped
        .into_iter()
        .map(|(loop_id, drift_pct)| DriftOverride {
            loop_id,
            drift_pct: round3(drift_pct),
        })
        .collect::<Vec<_>>();
    drift_overrides.sort_by(|left, right| left.loop_id.cmp(&right.loop_id));
    CycleRequest { drift_overrides }
}
