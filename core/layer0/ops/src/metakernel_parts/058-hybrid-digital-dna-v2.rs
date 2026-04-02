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

fn mark_protected_gene(state: &mut HybridDnaState, instance_id: &str, gene_index: usize) {
    let entry = state
        .protected_gene_indexes
        .entry(instance_id.to_string())
        .or_default();
    if !entry.contains(&gene_index) {
        entry.push(gene_index);
        entry.sort_unstable();
        entry.dedup();
    }
}

fn add_hybrid_commit(root: &Path, commit: &HybridCommitRecord) {
    if let Ok(value) = serde_json::to_value(commit) {
        append_jsonl(&hybrid_dna_commits_path(root), &value);
        write_json(&hybrid_dna_latest_commit_path(root), &value);
    }
}

fn normalize_hybrid_region(raw: Option<&String>) -> String {
    let token = normalize_token(
        raw.map(String::as_str)
            .unwrap_or(HYBRID_REGION_ROOT_IDENTITY),
        HYBRID_REGION_ROOT_IDENTITY,
        96,
    );
    match token.as_str() {
        "root" | "root-identity" | "root_identity" => HYBRID_REGION_ROOT_IDENTITY.to_string(),
        "constitutional" | "constitutional-safety-rules" | "constitutional_safety_rules" => {
            HYBRID_REGION_CONSTITUTIONAL_SAFETY_RULES.to_string()
        }
        "lineage" | "lineage-parent-anchor" | "lineage_parent_anchor" => {
            HYBRID_REGION_LINEAGE_PARENT_ANCHOR.to_string()
        }
        "high-stakes" | "high-stakes-receipt" | "high_stakes_receipt" => {
            HYBRID_REGION_HIGH_STAKES_RECEIPT.to_string()
        }
        _ => token,
    }
}

fn validate_worm_region(region: &str) -> bool {
    matches!(
        region,
        HYBRID_REGION_ROOT_IDENTITY
            | HYBRID_REGION_CONSTITUTIONAL_SAFETY_RULES
            | HYBRID_REGION_LINEAGE_PARENT_ANCHOR
            | HYBRID_REGION_HIGH_STAKES_RECEIPT
    )
}

fn run_dna_hybrid_status(root: &Path) -> Value {
    let state = load_hybrid_dna_state(root);
    let commits = read_hybrid_commit_rows(root);
    json!({
        "ok": true,
        "type": "hybrid_dna_status",
        "schema_version": state.schema_version,
        "commit_count": commits.len(),
        "latest_commit_hash": state.latest_commit_hash,
        "worm_region_count": state.worm_regions.len(),
        "protected_gene_count": state.protected_gene_indexes.values().map(Vec::len).sum::<usize>(),
        "latest_receipt": read_json(&hybrid_dna_latest_receipt_path(root))
    })
}

fn run_dna_hybrid_commit(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    boundary_raw: Option<&String>,
    gene_index_raw: Option<&String>,
    critical_raw: Option<&String>,
) -> Value {
    let Some(instance_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_commit",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_raw, "instance", 96);
    let boundary = normalize_hybrid_boundary(boundary_raw);
    if !is_valid_hybrid_boundary(&boundary) {
        return json!({
            "ok": false,
            "type": "hybrid_dna_commit",
            "error": "boundary_invalid",
            "boundary": boundary
        });
    }
    let critical = parse_bool(critical_raw, false);

    let dna_state = load_digital_dna_state(root);
    let Some(genome) = dna_state.genomes.get(&instance_id) else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_commit",
            "error": "instance_dna_not_found",
            "instance_dna_ref": instance_id
        });
    };

    let gene_index = parse_hybrid_index(gene_index_raw, 0, 1_000_000);
    let boundary_key = if boundary == HYBRID_COMMIT_GENE_REVISION {
        format!("gene:{gene_index}")
    } else {
        boundary.clone()
    };

    let gene_merkle = if boundary == HYBRID_COMMIT_GENE_REVISION {
        genome.genes.get(gene_index).map(gene_merkle_root)
    } else {
        None
    };
    if boundary == HYBRID_COMMIT_GENE_REVISION && gene_merkle.is_none() {
        return json!({
            "ok": false,
            "type": "hybrid_dna_commit",
            "error": "gene_index_out_of_range",
            "instance_dna_ref": instance_id,
            "gene_index": gene_index
        });
    }
    let genome_merkle = genome_merkle_root(genome);

    let payload = json!({
        "instance_dna_ref": instance_id,
        "boundary": boundary,
        "boundary_key": boundary_key,
        "gene_index": gene_index,
        "gene_merkle_root": gene_merkle,
        "genome_merkle_root": genome_merkle,
        "critical": critical
    });
    let mut hybrid_state = load_hybrid_dna_state(root);
    let commit = build_commit_record(
        &instance_id,
        &boundary,
        &boundary_key,
        hybrid_state.latest_commit_hash.clone(),
        &payload,
        gene_merkle.clone(),
        genome_merkle.clone(),
        critical,
    );
    add_hybrid_commit(root, &commit);
    hybrid_state.latest_commit_hash = Some(commit.commit_hash.clone());
    if critical && boundary == HYBRID_COMMIT_GENE_REVISION {
        mark_protected_gene(&mut hybrid_state, &instance_id, gene_index);
    }
    save_hybrid_dna_state(root, &hybrid_state);

    let receipt = write_hybrid_receipt(
        root,
        "hybrid_commit",
        &instance_id,
        true,
        &payload,
        Some(&commit.commit_hash),
    );
    json!({
        "ok": true,
        "type": "hybrid_dna_commit",
        "instance_dna_ref": instance_id,
        "commit": commit,
        "receipt": receipt,
        "strict": strict
    })
}

fn run_dna_hybrid_verify(root: &Path, strict: bool, instance_id_raw: Option<&String>) -> Value {
    let requested_instance = instance_id_raw.map(|v| normalize_token(v, "instance", 96));
    let mut errors = Vec::new();
    let mut critical_errors = Vec::new();
    let commits = read_hybrid_commit_rows(root);
    let dna_state = load_digital_dna_state(root);

    for (idx, commit) in commits.iter().enumerate() {
        if let Some(instance_id) = requested_instance.as_ref() {
            if &commit.instance_dna_ref != instance_id {
                continue;
            }
        }
        let previous = if idx == 0 { None } else { commits.get(idx - 1) };
        if !validate_commit_link(commit, previous) {
            errors.push(json!({
                "type": "commit_chain_break",
                "commit_id": commit.commit_id,
                "instance_dna_ref": commit.instance_dna_ref,
                "critical": commit.critical
            }));
            if commit.critical {
                critical_errors.push(HybridCriticalEvent::CriticalCommitChainBreak);
            }
        }
        if commit.boundary == HYBRID_COMMIT_GENE_REVISION {
            let gene_idx = commit
                .boundary_key
                .strip_prefix("gene:")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0);
            if let Some(genome) = dna_state.genomes.get(&commit.instance_dna_ref) {
                if let Some(gene) = genome.genes.get(gene_idx) {
                    let actual = gene_merkle_root(gene);
                    if commit.gene_merkle_root.as_deref() != Some(actual.as_str()) {
                        errors.push(json!({
                            "type": "critical_merkle_mismatch",
                            "commit_id": commit.commit_id,
                            "instance_dna_ref": commit.instance_dna_ref,
                            "gene_index": gene_idx,
                            "expected": commit.gene_merkle_root,
                            "actual": actual
                        }));
                        if commit.critical {
                            critical_errors
                                .push(HybridCriticalEvent::CriticalMerkleMismatchOnProtectedData);
                        }
                    }
                }
            }
        }
    }

    let instance_ref = requested_instance.as_deref().unwrap_or("hybrid-dna-global");
    let lock_triggered = if let Some(event) = critical_errors.first().cloned() {
        lock_on_hybrid_critical_event(root, strict, event, instance_ref, json!({"errors": errors}))
    } else {
        false
    };

    let ok = errors.is_empty();
    let receipt = write_hybrid_receipt(
        root,
        "hybrid_verify",
        instance_ref,
        ok,
        &json!({
            "strict": strict,
            "error_count": errors.len(),
            "errors": errors
        }),
        None,
    );
    json!({
        "ok": if strict { ok } else { true },
        "type": "hybrid_dna_verify",
        "strict": strict,
        "errors": receipt.pointer("/payload/errors").cloned().unwrap_or_else(|| json!([])),
        "error_count": receipt.pointer("/payload/error_count").cloned().unwrap_or_else(|| json!(0)),
        "judicial_lock": { "triggered": lock_triggered },
        "receipt": receipt
    })
}

fn run_dna_hybrid_repair_gene(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    gene_index_raw: Option<&String>,
) -> Value {
    let Some(instance_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_repair_gene",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_raw, "instance", 96);
    let gene_index = parse_hybrid_index(gene_index_raw, 0, 1_000_000);

    let mut dna_state = load_digital_dna_state(root);
    let Some(genome) = dna_state.genomes.get_mut(&instance_id) else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_repair_gene",
            "error": "instance_dna_not_found",
            "instance_dna_ref": instance_id
        });
    };
    let Some(gene) = genome.genes.get_mut(gene_index) else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_repair_gene",
            "error": "gene_index_out_of_range",
            "instance_dna_ref": instance_id,
            "gene_index": gene_index
        });
    };

    let mut repaired_letters = 0usize;
    let mut complement_matches = 0usize;
    for codon in &mut gene.codons {
        for letter in &mut codon.letters {
            let (repaired, complement_match) = repair_letter_with_complement_check(letter);
            if repaired {
                repaired_letters += 1;
            }
            if complement_match {
                complement_matches += 1;
            }
        }
    }
    let gene_valid = gene.codons.iter().all(Codon::is_valid);
    let mut hybrid_state = load_hybrid_dna_state(root);
    let protected = is_protected_gene(&hybrid_state, &instance_id, gene_index);
    let failure_key = protected_gene_key(&instance_id, gene_index);
    let mut lock_triggered = false;

    if !gene_valid && protected {
        let failures = hybrid_state
            .protected_repair_failures
            .entry(failure_key.clone())
            .or_insert(0);
        *failures = failures.saturating_add(1);
        if *failures >= HYBRID_PROTECTED_REPAIR_FAILURE_LOCK_THRESHOLD {
            lock_triggered = lock_on_hybrid_critical_event(
                root,
                strict,
                HybridCriticalEvent::RepeatedFailedRepairOnProtectedStructure,
                &instance_id,
                json!({
                    "gene_index": gene_index,
                    "failures": *failures
                }),
            );
        }
    } else if gene_valid {
        hybrid_state.protected_repair_failures.remove(&failure_key);
    }

    save_hybrid_dna_state(root, &hybrid_state);
    save_digital_dna_state(root, &dna_state);

    let payload = json!({
        "gene_index": gene_index,
        "repaired_letters": repaired_letters,
        "complement_matches": complement_matches,
        "gene_valid": gene_valid,
        "protected": protected
    });
    let receipt = write_hybrid_receipt(
        root,
        "repair_mutable_gene",
        &instance_id,
        gene_valid,
        &payload,
        None,
    );
    json!({
        "ok": if strict { gene_valid } else { true },
        "type": "hybrid_dna_repair_gene",
        "instance_dna_ref": instance_id,
        "payload": payload,
        "judicial_lock": { "triggered": lock_triggered },
        "receipt": receipt
    })
}

fn run_dna_hybrid_worm_supersede(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    region_raw: Option<&String>,
    region_key_raw: Option<&String>,
    value_raw: Option<&String>,
) -> Value {
    let Some(instance_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_worm_supersede",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_raw, "instance", 96);
    let region = normalize_hybrid_region(region_raw);
    if !validate_worm_region(&region) {
        return json!({
            "ok": false,
            "type": "hybrid_dna_worm_supersede",
            "error": "worm_region_invalid",
            "region": region
        });
    }
    let value = clean(
        value_raw
            .map(String::as_str)
            .unwrap_or("worm-value")
            .to_string(),
        2048,
    );
    if value.trim().is_empty() {
        return json!({
            "ok": false,
            "type": "hybrid_dna_worm_supersede",
            "error": "worm_value_required"
        });
    }
    let region_key = normalize_token(
        region_key_raw
            .map(String::as_str)
            .unwrap_or(instance_id.as_str()),
        instance_id.as_str(),
        128,
    );
    let worm_key = format!("{region}:{region_key}");
    let value_hash = hash_json_value(&json!({ "value": value }));

    let mut hybrid_state = load_hybrid_dna_state(root);
    let previous_hash = hybrid_state
        .worm_regions
        .get(&worm_key)
        .map(|row| row.current_hash.clone());
    let payload = json!({
        "region": region,
        "region_key": region_key,
        "value_hash": value_hash,
        "previous_hash": previous_hash
    });
    let commit = build_commit_record(
        &instance_id,
        HYBRID_COMMIT_WORM_SUPERSESSION,
        &worm_key,
        hybrid_state.latest_commit_hash.clone(),
        &payload,
        None,
        None,
        true,
    );
    add_hybrid_commit(root, &commit);
    hybrid_state.latest_commit_hash = Some(commit.commit_hash.clone());

    let region_state = hybrid_state
        .worm_regions
        .entry(worm_key.clone())
        .or_insert_with(|| WormRegionState {
            region_type: region.clone(),
            region_key: region_key.clone(),
            current_hash: value_hash.clone(),
            version: 0,
            history: Vec::new(),
            failed_mutation_attempts: 0,
        });
    let version = region_state.version.saturating_add(1);
    let row = WormVersionRecord {
        version,
        value_hash: value_hash.clone(),
        previous_hash: previous_hash.clone(),
        supersession_commit_hash: commit.commit_hash.clone(),
        ts: now_iso(),
    };
    region_state.version = version;
    region_state.current_hash = value_hash;
    region_state.history.push(row);
    save_hybrid_dna_state(root, &hybrid_state);

    let receipt = write_hybrid_receipt(
        root,
        "worm_supersession",
        &instance_id,
        true,
        &payload,
        Some(&commit.commit_hash),
    );
    json!({
        "ok": true,
        "type": "hybrid_dna_worm_supersede",
        "instance_dna_ref": instance_id,
        "worm_region": worm_key,
        "version": version,
        "strict": strict,
        "commit": commit,
        "receipt": receipt
    })
}

fn run_dna_hybrid_worm_mutate_attempt(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    region_raw: Option<&String>,
    region_key_raw: Option<&String>,
) -> Value {
    let Some(instance_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_worm_mutate",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_raw, "instance", 96);
    let region = normalize_hybrid_region(region_raw);
    let region_key = normalize_token(
        region_key_raw
            .map(String::as_str)
            .unwrap_or(instance_id.as_str()),
        instance_id.as_str(),
        128,
    );
    let worm_key = format!("{region}:{region_key}");
    let mut hybrid_state = load_hybrid_dna_state(root);
    let failed_mutation_attempts = {
        let region_state = hybrid_state
            .worm_regions
            .entry(worm_key.clone())
            .or_insert_with(|| WormRegionState {
                region_type: region.clone(),
                region_key: region_key.clone(),
                current_hash: String::new(),
                version: 0,
                history: Vec::new(),
                failed_mutation_attempts: 0,
            });
        region_state.failed_mutation_attempts =
            region_state.failed_mutation_attempts.saturating_add(1);
        region_state.failed_mutation_attempts
    };
    let repeated = failed_mutation_attempts >= HYBRID_PROTECTED_REPAIR_FAILURE_LOCK_THRESHOLD;
    save_hybrid_dna_state(root, &hybrid_state);

    let invalid_lock = lock_on_hybrid_critical_event(
        root,
        strict,
        HybridCriticalEvent::InvalidWormMutationAttempt,
        &instance_id,
        json!({
            "worm_region": worm_key,
            "failed_mutation_attempts": failed_mutation_attempts
        }),
    );
    let repeated_lock = if repeated {
        lock_on_hybrid_critical_event(
            root,
            strict,
            HybridCriticalEvent::RepeatedFailedRepairOnProtectedStructure,
            &instance_id,
            json!({
                "worm_region": worm_key,
                "failed_mutation_attempts": failed_mutation_attempts
            }),
        )
    } else {
        false
    };
    let receipt = write_hybrid_receipt(
        root,
        "worm_mutation_attempt",
        &instance_id,
        false,
        &json!({
            "worm_region": worm_key,
            "failed_mutation_attempts": failed_mutation_attempts
        }),
        None,
    );
    json!({
        "ok": if strict { false } else { true },
        "type": "hybrid_dna_worm_mutate",
        "instance_dna_ref": instance_id,
        "error": "worm_region_mutation_forbidden_use_supersession",
        "worm_region": worm_key,
        "failed_mutation_attempts": failed_mutation_attempts,
        "judicial_lock": { "triggered": invalid_lock || repeated_lock },
        "receipt": receipt
    })
}

fn run_dna_hybrid_protected_lineage_check(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    expected_parent_raw: Option<&String>,
    action_raw: Option<&String>,
) -> Value {
    let Some(instance_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "hybrid_dna_protected_lineage",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_raw, "instance", 96);
    let action = normalize_token(
        action_raw.map(String::as_str).unwrap_or("invoke_agent"),
        "invoke_agent",
        96,
    );
    let check = evaluate_subservience(root, &instance_id, expected_parent_raw, &action, strict);
    let ok = check.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let lock_triggered = if strict && !ok {
        lock_on_hybrid_critical_event(
            root,
            strict,
            HybridCriticalEvent::FailedLineageCheckOnCriticalAction,
            &instance_id,
            json!({
                "action": action,
                "check": check
            }),
        )
    } else {
        false
    };
    let receipt = write_hybrid_receipt(
        root,
        "protected_lineage_check",
        &instance_id,
        ok,
        &json!({
            "action": action,
            "check": check
        }),
        None,
    );
    json!({
        "ok": if strict { ok } else { true },
        "type": "hybrid_dna_protected_lineage",
        "instance_dna_ref": instance_id,
        "check": check,
        "judicial_lock": { "triggered": lock_triggered },
        "receipt": receipt
    })
}

#[cfg(test)]
mod hybrid_dna_v2_tests {
    use super::*;

    fn create_sample_instance(root: &Path, instance_id: &str, parent: &str) {
        let generation = "0".to_string();
        let schema = "v1".to_string();
        let seed = "hybrid-seed".to_string();
        let _ = run_digital_dna_create(
            root,
            true,
            Some(&instance_id.to_string()),
            Some(&parent.to_string()),
            Some(&schema),
            Some(&generation),
            Some(&seed),
        );
    }

    #[test]
    fn hybrid_gene_merkle_root_vector_exists() {
        let gene = default_gene("vector-seed");
        let root = gene_merkle_root(&gene);
        assert!(!root.is_empty());
    }

    #[test]
    fn hybrid_valid_commit_chain_example() {
        let root = tempfile::tempdir().expect("tempdir");
        create_sample_instance(root.path(), "instance-hybrid-1", "parent-a");
        let first = run_dna_hybrid_commit(
            root.path(),
            true,
            Some(&"instance-hybrid-1".to_string()),
            Some(&"gene_revision_commit".to_string()),
            Some(&"0".to_string()),
            Some(&"1".to_string()),
        );
        assert_eq!(first.get("ok").and_then(Value::as_bool), Some(true));
        let second = run_dna_hybrid_commit(
            root.path(),
            true,
            Some(&"instance-hybrid-1".to_string()),
            Some(&"genome_revision_commit".to_string()),
            None,
            Some(&"0".to_string()),
        );
        assert_eq!(second.get("ok").and_then(Value::as_bool), Some(true));
        let verify =
            run_dna_hybrid_verify(root.path(), true, Some(&"instance-hybrid-1".to_string()));
        assert_eq!(verify.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn hybrid_invalid_commit_chain_example() {
        let root = tempfile::tempdir().expect("tempdir");
        create_sample_instance(root.path(), "instance-hybrid-2", "parent-a");
        let _ = run_dna_hybrid_commit(
            root.path(),
            true,
            Some(&"instance-hybrid-2".to_string()),
            Some(&"gene_revision_commit".to_string()),
            Some(&"0".to_string()),
            Some(&"1".to_string()),
        );
        let mut rows = read_hybrid_commit_rows(root.path());
        assert_eq!(rows.len(), 1);
        rows[0].previous_hash = Some("broken-link".to_string());
        let text = rows
            .iter()
            .map(|row| serde_json::to_string(row).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(hybrid_dna_commits_path(root.path()), format!("{text}\n"))
            .expect("write tampered");
        let verify =
            run_dna_hybrid_verify(root.path(), true, Some(&"instance-hybrid-2".to_string()));
        assert_eq!(verify.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            verify
                .pointer("/judicial_lock/triggered")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn hybrid_mutable_region_repair_example() {
        let root = tempfile::tempdir().expect("tempdir");
        create_sample_instance(root.path(), "instance-hybrid-3", "parent-a");
        let mut dna_state = load_digital_dna_state(root.path());
        let genome = dna_state
            .genomes
            .get_mut("instance-hybrid-3")
            .expect("genome exists");
        genome.genes[0].codons[0].letters[0].verity =
            genome.genes[0].codons[0].letters[0].verity.complement();
        save_digital_dna_state(root.path(), &dna_state);

        let repair = run_dna_hybrid_repair_gene(
            root.path(),
            true,
            Some(&"instance-hybrid-3".to_string()),
            Some(&"0".to_string()),
        );
        assert_eq!(repair.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            repair
                .pointer("/payload/repaired_letters")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0,
            true
        );
    }

    #[test]
    fn hybrid_worm_supersession_example() {
        let root = tempfile::tempdir().expect("tempdir");
        create_sample_instance(root.path(), "instance-hybrid-4", "parent-a");
        let one = run_dna_hybrid_worm_supersede(
            root.path(),
            true,
            Some(&"instance-hybrid-4".to_string()),
            Some(&"lineage_parent_anchor".to_string()),
            Some(&"anchor-1".to_string()),
            Some(&"value-v1".to_string()),
        );
        assert_eq!(one.get("ok").and_then(Value::as_bool), Some(true));
        let two = run_dna_hybrid_worm_supersede(
            root.path(),
            true,
            Some(&"instance-hybrid-4".to_string()),
            Some(&"lineage_parent_anchor".to_string()),
            Some(&"anchor-1".to_string()),
            Some(&"value-v2".to_string()),
        );
        assert_eq!(two.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(two.get("version").and_then(Value::as_u64), Some(2));
    }

    #[test]
    fn hybrid_judicial_lock_invalid_worm_mutation_example() {
        let root = tempfile::tempdir().expect("tempdir");
        create_sample_instance(root.path(), "instance-hybrid-5", "parent-a");
        let out = run_dna_hybrid_worm_mutate_attempt(
            root.path(),
            true,
            Some(&"instance-hybrid-5".to_string()),
            Some(&"root_identity".to_string()),
            Some(&"identity-anchor".to_string()),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.pointer("/judicial_lock/triggered")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn hybrid_protected_lineage_example() {
        let root = tempfile::tempdir().expect("tempdir");
        create_sample_instance(root.path(), "instance-hybrid-6", "parent-a");
        let out = run_dna_hybrid_protected_lineage_check(
            root.path(),
            true,
            Some(&"instance-hybrid-6".to_string()),
            Some(&"parent-b".to_string()),
            Some(&"invoke_agent".to_string()),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.pointer("/judicial_lock/triggered")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
}
