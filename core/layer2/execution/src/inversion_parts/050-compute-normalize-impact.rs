#[derive(Debug, Clone, Deserialize, Default)]
pub struct PersistInterfaceEnvelopeInput {
    #[serde(default)]
    pub latest_path: Option<String>,
    #[serde(default)]
    pub history_path: Option<String>,
    #[serde(default)]
    pub envelope: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PersistInterfaceEnvelopeOutput {
    pub ok: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TrimLibraryInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub max_entries: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TrimLibraryOutput {
    pub rows: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DetectImmutableAxiomViolationInput {
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub decision_input: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DetectImmutableAxiomViolationOutput {
    pub hits: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ComputeMaturityScoreInput {
    #[serde(default)]
    pub state: Option<Value>,
    #[serde(default)]
    pub policy: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ComputeMaturityScoreOutput {
    pub score: f64,
    pub band: String,
    pub pass_rate: f64,
    pub non_destructive_rate: f64,
    pub experience: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SelectLibraryCandidatesInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub query: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SelectLibraryCandidatesOutput {
    pub candidates: Vec<Value>,
}
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParseLaneDecisionInput {
    #[serde(default)]
    pub args: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ParseLaneDecisionOutput {
    pub selected_lane: String,
    pub source: String,
    pub route: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SweepExpiredSessionsInput {
    #[serde(default)]
    pub paths: Option<Value>,
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub date_str: Option<String>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SweepExpiredSessionsOutput {
    pub expired_count: i64,
    pub sessions: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoadImpossibilitySignalsInput {
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub date_str: Option<String>,
    #[serde(default)]
    pub root: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LoadImpossibilitySignalsOutput {
    pub signals: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EvaluateImpossibilityTriggerInput {
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub signals: Option<Value>,
    #[serde(default)]
    pub force: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EvaluateImpossibilityTriggerOutput {
    pub triggered: bool,
    pub forced: bool,
    pub enabled: bool,
    pub score: f64,
    pub threshold: f64,
    pub signal_count: i64,
    pub min_signal_count: i64,
    pub reasons: Vec<String>,
    pub components: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtractFirstPrincipleInput {
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub session: Option<Value>,
    #[serde(default)]
    pub args: Option<Value>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ExtractFirstPrincipleOutput {
    pub principle: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtractFailureClusterPrincipleInput {
    #[serde(default)]
    pub paths: Option<Value>,
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub session: Option<Value>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ExtractFailureClusterPrincipleOutput {
    pub principle: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PersistFirstPrincipleInput {
    #[serde(default)]
    pub paths: Option<Value>,
    #[serde(default)]
    pub session: Option<Value>,
    #[serde(default)]
    pub principle: Option<Value>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PersistFirstPrincipleOutput {
    pub principle: Value,
}

fn normalize_token(raw: &str, max_len: usize) -> String {
    let collapsed = raw
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_lowercase();
    collapsed.chars().take(max_len).collect::<String>()
}

pub fn compute_normalize_impact(input: &NormalizeImpactInput) -> NormalizeImpactOutput {
    let raw = normalize_token(input.value.as_deref().unwrap_or("medium"), 24);
    let value = match raw.as_str() {
        "low" | "medium" | "high" | "critical" => raw,
        _ => "medium".to_string(),
    };
    NormalizeImpactOutput { value }
}

pub fn compute_normalize_mode(input: &NormalizeModeInput) -> NormalizeModeOutput {
    let raw = normalize_token(input.value.as_deref().unwrap_or("live"), 16);
    let value = if raw == "test" {
        "test".to_string()
    } else {
        "live".to_string()
    };
    NormalizeModeOutput { value }
}

pub fn compute_normalize_target(input: &NormalizeTargetInput) -> NormalizeTargetOutput {
    let raw = normalize_token(input.value.as_deref().unwrap_or("tactical"), 24);
    let value = match raw.as_str() {
        "tactical" | "belief" | "identity" | "directive" | "constitution" => raw,
        _ => "tactical".to_string(),
    };
    NormalizeTargetOutput { value }
}

pub fn compute_normalize_result(input: &NormalizeResultInput) -> NormalizeResultOutput {
    let raw = normalize_token(input.value.as_deref().unwrap_or(""), 24);
    let value = match raw.as_str() {
        "success" | "neutral" | "fail" | "destructive" => raw,
        _ => String::new(),
    };
    NormalizeResultOutput { value }
}

fn is_valid_objective_id(raw: &str) -> bool {
    if raw.len() < 6 || raw.len() > 140 {
        return false;
    }
    let bytes = raw.as_bytes();
    let first = bytes[0] as char;
    let last = bytes[bytes.len() - 1] as char;
    if !first.is_ascii_alphanumeric() || !last.is_ascii_alphanumeric() {
        return false;
    }
    if bytes.len() < 2 {
        return false;
    }
    for ch in &bytes[1..(bytes.len() - 1)] {
        let c = *ch as char;
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == ':' || c == '-' {
            continue;
        }
        return false;
    }
    true
}

pub fn compute_objective_id_valid(input: &ObjectiveIdValidInput) -> ObjectiveIdValidOutput {
    let raw = input.value.as_deref().unwrap_or("").trim();
    ObjectiveIdValidOutput {
        valid: is_valid_objective_id(raw),
    }
}

fn normalize_trit_value(value: &Value) -> i32 {
    if let Some(n) = value.as_f64() {
        if n > 0.0 {
            return 1;
        }
        if n < 0.0 {
            return -1;
        }
        return 0;
    }
    if let Some(b) = value.as_bool() {
        return if b { 1 } else { 0 };
    }
    if let Some(text) = value.as_str() {
        let parsed = text.trim().parse::<f64>().ok().unwrap_or(0.0);
        if parsed > 0.0 {
            return 1;
        }
        if parsed < 0.0 {
            return -1;
        }
    }
    0
}

pub fn compute_trit_vector_from_input(
    input: &TritVectorFromInputInput,
) -> TritVectorFromInputOutput {
    if let Some(vec) = &input.trit_vector {
        let out = vec.iter().map(normalize_trit_value).collect::<Vec<_>>();
        return TritVectorFromInputOutput { vector: out };
    }
    let raw = input.trit_vector_csv.as_deref().unwrap_or("").trim();
    if raw.is_empty() {
        return TritVectorFromInputOutput { vector: Vec::new() };
    }
    let out = raw
        .split(',')
        .map(|token| {
            let parsed = token.trim().parse::<f64>().ok().unwrap_or(0.0);
            if parsed > 0.0 {
                1
            } else if parsed < 0.0 {
                -1
            } else {
                0
            }
        })
        .collect::<Vec<_>>();
    TritVectorFromInputOutput { vector: out }
}

pub fn compute_jaccard_similarity(input: &JaccardSimilarityInput) -> JaccardSimilarityOutput {
    let left = input
        .left_tokens
        .iter()
        .map(|token| token.trim())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect::<BTreeSet<_>>();
    let right = input
        .right_tokens
        .iter()
        .map(|token| token.trim())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect::<BTreeSet<_>>();
    if left.is_empty() && right.is_empty() {
        return JaccardSimilarityOutput { similarity: 1.0 };
    }
    if left.is_empty() || right.is_empty() {
        return JaccardSimilarityOutput { similarity: 0.0 };
    }
    let inter = left.intersection(&right).count() as f64;
    let union = left.union(&right).count() as f64;
    let similarity = if union > 0.0 { inter / union } else { 0.0 };
    JaccardSimilarityOutput { similarity }
}

fn majority_trit(values: &[Value]) -> i32 {
    if values.is_empty() {
        return 0;
    }
    let mut pain = 0;
    let mut unknown = 0;
    let mut ok = 0;
    for value in values {
        let trit = normalize_trit_value(value);
        if trit < 0 {
            pain += 1;
        } else if trit > 0 {
            ok += 1;
        } else {
            unknown += 1;
        }
    }
    if pain > ok && pain > unknown {
        -1
    } else if ok > pain && ok > unknown {
        1
    } else {
        0
    }
}

pub fn compute_trit_similarity(input: &TritSimilarityInput) -> TritSimilarityOutput {
    let trit = normalize_trit_value(input.entry_trit.as_ref().unwrap_or(&Value::Null));
    if input.query_vector.is_empty() {
        return TritSimilarityOutput {
            similarity: if trit == 0 { 1.0 } else { 0.5 },
        };
    }
    let majority = majority_trit(&input.query_vector);
    let similarity = if majority == trit {
        1.0
    } else if majority == 0 || trit == 0 {
        0.6
    } else {
        0.0
    };
    TritSimilarityOutput { similarity }
}

fn clamp_number(value: f64, lo: f64, hi: f64) -> f64 {
    value.max(lo).min(hi)
}

fn read_number_key(value: Option<&Value>, key: &str, fallback: f64) -> f64 {
    let Some(map) = value.and_then(|v| v.as_object()) else {
        return fallback;
    };
    map.get(key)
        .and_then(|v| v.as_f64())
        .map(|n| clamp_number(n, 0.0, 1.0))
        .unwrap_or(fallback)
}

pub fn compute_certainty_threshold(input: &CertaintyThresholdInput) -> CertaintyThresholdOutput {
    let thresholds = input.thresholds.as_ref().and_then(|v| v.as_object());
    let band = normalize_token(input.band.as_deref().unwrap_or("novice"), 24);
    let impact = normalize_token(input.impact.as_deref().unwrap_or("medium"), 24);
    let by_band = thresholds
        .and_then(|rows| rows.get(&band))
        .filter(|v| v.is_object())
        .or_else(|| thresholds.and_then(|rows| rows.get("novice")));
    let mut threshold = read_number_key(by_band, &impact, 1.0);
    if input.allow_zero_for_legendary_critical.unwrap_or(false)
        && band == "legendary"
        && impact == "critical"
    {
        threshold = 0.0;
    }
    CertaintyThresholdOutput { threshold }
}

fn read_rank_key(value: Option<&Value>, key: &str, fallback: i64) -> i64 {
    let Some(map) = value.and_then(|v| v.as_object()) else {
        return fallback;
    };
    map.get(key)
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|n| n.round() as i64)))
        .unwrap_or(fallback)
}

pub fn compute_max_target_rank(input: &MaxTargetRankInput) -> MaxTargetRankOutput {
    let band = normalize_token(input.maturity_band.as_deref().unwrap_or("novice"), 24);
    let impact = normalize_token(input.impact.as_deref().unwrap_or("medium"), 24);
    let maturity_rank = read_rank_key(input.maturity_max_target_rank_by_band.as_ref(), &band, 1);
    let impact_rank = read_rank_key(input.impact_max_target_rank.as_ref(), &impact, 1);
    let rank = maturity_rank.min(impact_rank).max(1);
    MaxTargetRankOutput { rank }
}

fn clean_text_runtime(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}
