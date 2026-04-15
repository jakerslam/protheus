const HYBRID_DNA_V2_SCHEMA_VERSION: &str = "v2";
const HYBRID_COMMIT_GENE_REVISION: &str = "gene_revision_commit";
const HYBRID_COMMIT_GENOME_REVISION: &str = "genome_revision_commit";
const HYBRID_COMMIT_CRITICAL_RECEIPT: &str = "critical_receipt_commit";
const HYBRID_COMMIT_WORM_SUPERSESSION: &str = "worm_supersession_commit";
const HYBRID_REGION_ROOT_IDENTITY: &str = "root_identity";
const HYBRID_REGION_CONSTITUTIONAL_SAFETY_RULES: &str = "constitutional_safety_rules";
const HYBRID_REGION_LINEAGE_PARENT_ANCHOR: &str = "lineage_parent_anchor";
const HYBRID_REGION_HIGH_STAKES_RECEIPT: &str = "high_stakes_receipt";
const HYBRID_PROTECTED_REPAIR_FAILURE_LOCK_THRESHOLD: u32 = 3;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct HybridCommitRecord {
    commit_id: String,
    instance_dna_ref: String,
    boundary: String,
    boundary_key: String,
    previous_hash: Option<String>,
    payload_hash: String,
    commit_hash: String,
    gene_merkle_root: Option<String>,
    genome_merkle_root: Option<String>,
    critical: bool,
    ts: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct WormVersionRecord {
    version: u64,
    value_hash: String,
    previous_hash: Option<String>,
    supersession_commit_hash: String,
    ts: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct WormRegionState {
    region_type: String,
    region_key: String,
    current_hash: String,
    version: u64,
    history: Vec<WormVersionRecord>,
    failed_mutation_attempts: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct HybridDnaState {
    schema_version: String,
    latest_commit_hash: Option<String>,
    worm_regions: BTreeMap<String, WormRegionState>,
    protected_gene_indexes: BTreeMap<String, Vec<usize>>,
    protected_repair_failures: BTreeMap<String, u32>,
}

impl Default for HybridDnaState {
    fn default() -> Self {
        Self {
            schema_version: HYBRID_DNA_V2_SCHEMA_VERSION.to_string(),
            latest_commit_hash: None,
            worm_regions: BTreeMap::new(),
            protected_gene_indexes: BTreeMap::new(),
            protected_repair_failures: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HybridCriticalEvent {
    FailedLineageCheckOnCriticalAction,
    InvalidWormMutationAttempt,
    CriticalCommitChainBreak,
    CriticalMerkleMismatchOnProtectedData,
    RepeatedFailedRepairOnProtectedStructure,
}

fn hybrid_dna_state_path(root: &Path) -> PathBuf {
    digital_dna_state_dir(root).join("hybrid_state.json")
}

fn hybrid_dna_commits_path(root: &Path) -> PathBuf {
    digital_dna_state_dir(root).join("hybrid_commits.jsonl")
}

fn hybrid_dna_latest_commit_path(root: &Path) -> PathBuf {
    digital_dna_state_dir(root).join("hybrid_latest_commit.json")
}

fn hybrid_dna_receipts_path(root: &Path) -> PathBuf {
    digital_dna_state_dir(root).join("hybrid_receipts.jsonl")
}

fn hybrid_dna_latest_receipt_path(root: &Path) -> PathBuf {
    digital_dna_state_dir(root).join("hybrid_latest_receipt.json")
}

fn load_hybrid_dna_state(root: &Path) -> HybridDnaState {
    read_json(&hybrid_dna_state_path(root))
        .and_then(|value| serde_json::from_value::<HybridDnaState>(value).ok())
        .unwrap_or_default()
}

fn save_hybrid_dna_state(root: &Path, state: &HybridDnaState) {
    if let Ok(value) = serde_json::to_value(state) {
        write_json(&hybrid_dna_state_path(root), &value);
    }
}

fn read_hybrid_commit_rows(root: &Path) -> Vec<HybridCommitRecord> {
    let path = hybrid_dna_commits_path(root);
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            serde_json::from_str::<HybridCommitRecord>(trimmed).ok()
        })
        .collect()
}

fn hash_pair_hex(left: &str, right: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(left.as_bytes());
    hasher.update(b"|");
    hasher.update(right.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn hash_json_value(value: &Value) -> String {
    deterministic_receipt_hash(value)
}

// TODO-NORMATIVE: Freeze final Merkle canonicalization after Hybrid DNA v2 crypto profile is approved.
fn gene_merkle_root(gene: &Gene) -> String {
    let mut leaves = Vec::new();
    for codon in &gene.codons {
        let value =
            serde_json::to_value(codon).unwrap_or_else(|_| json!({"error": "encode_codon"}));
        leaves.push(hash_json_value(&value));
    }
    if leaves.is_empty() {
        leaves.push(hash_pair_hex("EMPTY", "GENE"));
    }
    let mut level = leaves;
    while level.len() > 1 {
        let mut next = Vec::new();
        let mut idx = 0usize;
        while idx < level.len() {
            let left = level[idx].clone();
            let right = if idx + 1 < level.len() {
                level[idx + 1].clone()
            } else {
                left.clone()
            };
            next.push(hash_pair_hex(&left, &right));
            idx += 2;
        }
        level = next;
    }
    level
        .first()
        .cloned()
        .unwrap_or_else(|| hash_pair_hex("EMPTY", "GENE"))
}

// TODO-NORMATIVE: Keep genome-level Merkle optional; callers decide whether to use it.
fn genome_merkle_root(genome: &InstanceDna) -> Option<String> {
    if genome.genes.is_empty() {
        return None;
    }
    let mut roots = genome
        .genes
        .iter()
        .map(gene_merkle_root)
        .collect::<Vec<_>>();
    while roots.len() > 1 {
        let mut next = Vec::new();
        let mut idx = 0usize;
        while idx < roots.len() {
            let left = roots[idx].clone();
            let right = if idx + 1 < roots.len() {
                roots[idx + 1].clone()
            } else {
                left.clone()
            };
            next.push(hash_pair_hex(&left, &right));
            idx += 2;
        }
        roots = next;
    }
    roots.first().cloned()
}

fn is_valid_hybrid_boundary(boundary: &str) -> bool {
    matches!(
        boundary,
        HYBRID_COMMIT_GENE_REVISION
            | HYBRID_COMMIT_GENOME_REVISION
            | HYBRID_COMMIT_CRITICAL_RECEIPT
            | HYBRID_COMMIT_WORM_SUPERSESSION
    )
}

fn normalize_hybrid_boundary(raw: Option<&String>) -> String {
    let token = normalize_token(
        raw.map(String::as_str)
            .unwrap_or(HYBRID_COMMIT_GENE_REVISION),
        HYBRID_COMMIT_GENE_REVISION,
        96,
    );
    match token.as_str() {
        "gene" | "gene-revision" | "gene_revision" | "gene-revision-commit" => {
            HYBRID_COMMIT_GENE_REVISION.to_string()
        }
        "genome" | "genome-revision" | "genome_revision" | "genome-revision-commit" => {
            HYBRID_COMMIT_GENOME_REVISION.to_string()
        }
        "critical" | "critical-receipt" | "critical_receipt" => {
            HYBRID_COMMIT_CRITICAL_RECEIPT.to_string()
        }
        "worm" | "worm-supersession" | "worm_supersession" => {
            HYBRID_COMMIT_WORM_SUPERSESSION.to_string()
        }
        _ => token,
    }
}

fn parse_hybrid_index(raw: Option<&String>, fallback: usize, max: usize) -> usize {
    raw.and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(fallback)
        .min(max)
}

fn hybrid_commit_hash(
    instance_dna_ref: &str,
    boundary: &str,
    boundary_key: &str,
    previous_hash: Option<&String>,
    payload_hash: &str,
    gene_merkle_root: Option<&String>,
    genome_merkle_root: Option<&String>,
    critical: bool,
    ts: &str,
) -> String {
    deterministic_receipt_hash(&json!({
        "instance_dna_ref": instance_dna_ref,
        "boundary": boundary,
        "boundary_key": boundary_key,
        "previous_hash": previous_hash.cloned(),
        "payload_hash": payload_hash,
        "gene_merkle_root": gene_merkle_root.cloned(),
        "genome_merkle_root": genome_merkle_root.cloned(),
        "critical": critical,
        "ts": ts
    }))
}

// TODO-NORMATIVE: finalize commit-id canonicalization once cryptographic profile is frozen.
fn build_commit_record(
    instance_dna_ref: &str,
    boundary: &str,
    boundary_key: &str,
    previous_hash: Option<String>,
    payload: &Value,
    gene_merkle_root: Option<String>,
    genome_merkle_root: Option<String>,
    critical: bool,
) -> HybridCommitRecord {
    let payload_hash = hash_json_value(payload);
    let ts = now_iso();
    let commit_hash = hybrid_commit_hash(
        instance_dna_ref,
        boundary,
        boundary_key,
        previous_hash.as_ref(),
        &payload_hash,
        gene_merkle_root.as_ref(),
        genome_merkle_root.as_ref(),
        critical,
        &ts,
    );
    let commit_id = format!("commit-{}", &commit_hash[..12]);
    HybridCommitRecord {
        commit_id,
        instance_dna_ref: instance_dna_ref.to_string(),
        boundary: boundary.to_string(),
        boundary_key: boundary_key.to_string(),
        previous_hash,
        payload_hash,
        commit_hash,
        gene_merkle_root,
        genome_merkle_root,
        critical,
        ts,
    }
}

fn validate_commit_link(
    current: &HybridCommitRecord,
    previous: Option<&HybridCommitRecord>,
) -> bool {
    if let Some(previous) = previous {
        if current.previous_hash.as_deref() != Some(previous.commit_hash.as_str()) {
            return false;
        }
    } else if current.previous_hash.is_some() {
        return false;
    }
    let recomputed = hybrid_commit_hash(
        &current.instance_dna_ref,
        &current.boundary,
        &current.boundary_key,
        current.previous_hash.as_ref(),
        &current.payload_hash,
        current.gene_merkle_root.as_ref(),
        current.genome_merkle_root.as_ref(),
        current.critical,
        &current.ts,
    );
    recomputed == current.commit_hash
}

fn should_trigger_judicial_lock(event: &HybridCriticalEvent) -> bool {
    matches!(
        event,
        HybridCriticalEvent::FailedLineageCheckOnCriticalAction
            | HybridCriticalEvent::InvalidWormMutationAttempt
            | HybridCriticalEvent::CriticalCommitChainBreak
            | HybridCriticalEvent::CriticalMerkleMismatchOnProtectedData
            | HybridCriticalEvent::RepeatedFailedRepairOnProtectedStructure
    )
}

fn lock_on_hybrid_critical_event(
    root: &Path,
    strict: bool,
    event: HybridCriticalEvent,
    instance_dna_ref: &str,
    detail: Value,
) -> bool {
    if !strict || !should_trigger_judicial_lock(&event) {
        return false;
    }
    let reason = match event {
        HybridCriticalEvent::FailedLineageCheckOnCriticalAction => {
            "failed_lineage_check_on_critical_action"
        }
        HybridCriticalEvent::InvalidWormMutationAttempt => "invalid_worm_mutation_attempt",
        HybridCriticalEvent::CriticalCommitChainBreak => "critical_commit_chain_break",
        HybridCriticalEvent::CriticalMerkleMismatchOnProtectedData => {
            "critical_merkle_mismatch_on_protected_data"
        }
        HybridCriticalEvent::RepeatedFailedRepairOnProtectedStructure => {
            "repeated_failed_repair_on_protected_structure"
        }
    };
    let lock_payload = json!({
        "type": "metakernel_judicial_lock",
        "active": true,
        "trigger": "hybrid_digital_dna_v2",
        "reason": reason,
        "instance_dna": instance_dna_ref,
        "ts": now_iso(),
        "violation_codes": [reason],
        "detail": detail
    });
    write_json(&judicial_lock_path(root), &lock_payload);
    true
}

fn write_hybrid_receipt(
    root: &Path,
    action: &str,
    instance_dna_ref: &str,
    ok: bool,
    payload: &Value,
    commit_hash: Option<&str>,
) -> Value {
    let mut receipt = json!({
        "ok": ok,
        "type": "hybrid_dna_receipt",
        "lane": "core/layer0/ops",
        "ts": now_iso(),
        "action": action,
        "instance_dna_ref": instance_dna_ref,
        "commit_hash": commit_hash,
        "payload": payload,
        "layer_ref": {
            "layer0": "safety_and_verity",
            "layer1": "policy_and_receipts"
        }
    });
    receipt["receipt_hash"] = Value::String(deterministic_receipt_hash(&receipt));
    append_jsonl(&hybrid_dna_receipts_path(root), &receipt);
    write_json(&hybrid_dna_latest_receipt_path(root), &receipt);
    receipt
}

fn protected_gene_key(instance_id: &str, gene_index: usize) -> String {
    format!("{instance_id}:{gene_index}")
}

fn is_protected_gene(state: &HybridDnaState, instance_id: &str, gene_index: usize) -> bool {
    state
        .protected_gene_indexes
        .get(instance_id)
        .map(|rows| rows.contains(&gene_index))
        .unwrap_or(false)
}
