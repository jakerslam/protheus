use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const DIGITAL_DNA_SCHEMA_VERSION: &str = "v1";
const GENE_START_MARKER: &str = "GENE_START";
const GENE_STOP_MARKER: &str = "GENE_STOP";
const DIGITAL_DNA_DEFAULT_PARENT_SIGNATURE: &str = "root";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Quark {
    value: i8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Baryon {
    q1: Quark,
    q2: Quark,
    q3: Quark,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Letter {
    core: Baryon,
    func: Baryon,
    #[serde(rename = "mod")]
    mod_: Baryon,
    verity: Baryon,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Codon {
    letters: [Letter; 4],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Gene {
    start_marker: String,
    stop_marker: String,
    repair_enabled: bool,
    codons: Vec<Codon>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct GenomeHeader {
    instance_id: String,
    parent_signature: String,
    version: String,
    birth_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct InstanceDna {
    header: GenomeHeader,
    genes: Vec<Gene>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct DigitalDnaState {
    schema_version: String,
    genomes: BTreeMap<String, InstanceDna>,
    last_receipt_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct QubitWrapper<T> {
    layer: String,
    superposed: Vec<T>,
    collapsed_index: Option<usize>,
    semantics: String,
}

impl Default for DigitalDnaState {
    fn default() -> Self {
        Self {
            schema_version: DIGITAL_DNA_SCHEMA_VERSION.to_string(),
            genomes: BTreeMap::new(),
            last_receipt_hash: None,
        }
    }
}

impl Quark {
    fn new(value: i8) -> Result<Self, String> {
        if matches!(value, -1 | 0 | 1) {
            Ok(Self { value })
        } else {
            Err("quark_value_out_of_range".to_string())
        }
    }

    fn complement(&self) -> Self {
        Self { value: -self.value }
    }
}

impl Baryon {
    fn from_trits(values: [i8; 3]) -> Result<Self, String> {
        Ok(Self {
            q1: Quark::new(values[0])?,
            q2: Quark::new(values[1])?,
            q3: Quark::new(values[2])?,
        })
    }

    fn values(&self) -> [i8; 3] {
        [self.q1.value, self.q2.value, self.q3.value]
    }

    fn complement(&self) -> Self {
        Self {
            q1: self.q1.complement(),
            q2: self.q2.complement(),
            q3: self.q3.complement(),
        }
    }
}

impl Letter {
    fn new(core: Baryon, func: Baryon, mod_: Baryon) -> Self {
        let verity = derive_verity(&core, &func, &mod_);
        Self {
            core,
            func,
            mod_,
            verity,
        }
    }

    fn is_valid(&self) -> bool {
        self.verity == derive_verity(&self.core, &self.func, &self.mod_)
    }
}

impl Codon {
    fn new(letters: [Letter; 4]) -> Result<Self, String> {
        if letters.iter().all(Letter::is_valid) {
            Ok(Self { letters })
        } else {
            Err("codon_contains_invalid_letter".to_string())
        }
    }

    fn is_valid(&self) -> bool {
        self.letters.iter().all(Letter::is_valid)
    }
}

fn trit_from_byte(byte: u8) -> i8 {
    match byte % 3 {
        0 => -1,
        1 => 0,
        _ => 1,
    }
}

// TODO-NORMATIVE: Standardize verity derivation across all runtime surfaces once Digital DNA v2 is approved.
fn derive_verity(core: &Baryon, func: &Baryon, mod_: &Baryon) -> Baryon {
    let mut hasher = Sha256::new();
    hasher.update(
        format!(
            "{:?}|{:?}|{:?}",
            core.values(),
            func.values(),
            mod_.values()
        )
        .as_bytes(),
    );
    let digest = hasher.finalize();
    Baryon::from_trits([
        trit_from_byte(digest[0]),
        trit_from_byte(digest[1]),
        trit_from_byte(digest[2]),
    ])
    .unwrap_or_else(|_| Baryon::from_trits([0, 0, 0]).expect("zero trits are valid"))
}

fn is_complement(a: &Baryon, b: &Baryon) -> bool {
    a.complement() == *b
}

fn seeded_baryon(seed: &str, scope: &str) -> Baryon {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    hasher.update(b"|");
    hasher.update(scope.as_bytes());
    let digest = hasher.finalize();
    Baryon::from_trits([
        trit_from_byte(digest[0]),
        trit_from_byte(digest[1]),
        trit_from_byte(digest[2]),
    ])
    .unwrap_or_else(|_| Baryon::from_trits([0, 0, 0]).expect("zero trits are valid"))
}

fn seeded_letter(seed: &str, scope: &str) -> Letter {
    let core = seeded_baryon(seed, &format!("{scope}:core"));
    let func = seeded_baryon(seed, &format!("{scope}:func"));
    let mod_ = seeded_baryon(seed, &format!("{scope}:mod"));
    Letter::new(core, func, mod_)
}

fn seeded_codon(seed: &str, index: usize) -> Codon {
    let letters = [
        seeded_letter(seed, &format!("codon_{index}_0")),
        seeded_letter(seed, &format!("codon_{index}_1")),
        seeded_letter(seed, &format!("codon_{index}_2")),
        seeded_letter(seed, &format!("codon_{index}_3")),
    ];
    Codon::new(letters).unwrap_or_else(|_| {
        Codon::new([
            Letter::new(
                Baryon::from_trits([0, 0, 0]).expect("zero trits valid"),
                Baryon::from_trits([0, 0, 0]).expect("zero trits valid"),
                Baryon::from_trits([0, 0, 0]).expect("zero trits valid"),
            ),
            Letter::new(
                Baryon::from_trits([0, 0, 0]).expect("zero trits valid"),
                Baryon::from_trits([0, 0, 0]).expect("zero trits valid"),
                Baryon::from_trits([0, 0, 0]).expect("zero trits valid"),
            ),
            Letter::new(
                Baryon::from_trits([0, 0, 0]).expect("zero trits valid"),
                Baryon::from_trits([0, 0, 0]).expect("zero trits valid"),
                Baryon::from_trits([0, 0, 0]).expect("zero trits valid"),
            ),
            Letter::new(
                Baryon::from_trits([0, 0, 0]).expect("zero trits valid"),
                Baryon::from_trits([0, 0, 0]).expect("zero trits valid"),
                Baryon::from_trits([0, 0, 0]).expect("zero trits valid"),
            ),
        ])
        .expect("fallback codon must remain valid")
    })
}

fn default_gene(seed: &str) -> Gene {
    Gene {
        start_marker: GENE_START_MARKER.to_string(),
        stop_marker: GENE_STOP_MARKER.to_string(),
        repair_enabled: true,
        codons: vec![seeded_codon(seed, 0)],
    }
}

fn validate_instance_dna(genome: &InstanceDna) -> Result<(), String> {
    if !is_token_id(&genome.header.instance_id) {
        return Err("instance_id_invalid".to_string());
    }
    if genome.header.version.trim().is_empty() {
        return Err("genome_header_version_required".to_string());
    }
    if genome.genes.is_empty() {
        return Err("genome_requires_at_least_one_gene".to_string());
    }
    for gene in &genome.genes {
        if gene.start_marker != GENE_START_MARKER {
            return Err("gene_start_marker_invalid".to_string());
        }
        if gene.stop_marker != GENE_STOP_MARKER {
            return Err("gene_stop_marker_invalid".to_string());
        }
        if gene.codons.is_empty() {
            return Err("gene_requires_at_least_one_codon".to_string());
        }
        if !gene.codons.iter().all(Codon::is_valid) {
            return Err("gene_contains_invalid_codon".to_string());
        }
    }
    Ok(())
}

fn repair_letter_with_complement_check(letter: &mut Letter) -> (bool, bool) {
    if letter.is_valid() {
        return (false, false);
    }
    let derived = derive_verity(&letter.core, &letter.func, &letter.mod_);
    let complement_match =
        is_complement(&letter.verity, &derived) || is_complement(&derived, &letter.verity);
    letter.verity = derived;
    (true, complement_match)
}

fn repair_instance_dna(genome: &mut InstanceDna) -> (usize, usize) {
    let mut repaired_letters = 0usize;
    let mut complement_matches = 0usize;
    for gene in &mut genome.genes {
        if !gene.repair_enabled {
            continue;
        }
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
    }
    (repaired_letters, complement_matches)
}

fn digital_dna_state_dir(root: &Path) -> PathBuf {
    state_root(root).join("digital_dna")
}

fn digital_dna_state_path(root: &Path) -> PathBuf {
    digital_dna_state_dir(root).join("state.json")
}

fn digital_dna_receipts_path(root: &Path) -> PathBuf {
    digital_dna_state_dir(root).join("receipts.jsonl")
}

fn digital_dna_latest_receipt_path(root: &Path) -> PathBuf {
    digital_dna_state_dir(root).join("latest_receipt.json")
}

fn load_digital_dna_state(root: &Path) -> DigitalDnaState {
    read_json(&digital_dna_state_path(root))
        .and_then(|value| serde_json::from_value::<DigitalDnaState>(value).ok())
        .unwrap_or_default()
}

fn save_digital_dna_state(root: &Path, state: &DigitalDnaState) {
    if let Ok(value) = serde_json::to_value(state) {
        write_json(&digital_dna_state_path(root), &value);
    }
}

fn write_digital_dna_receipt(
    root: &Path,
    action: &str,
    instance_dna_ref: &str,
    ok: bool,
    payload: &Value,
) -> Value {
    let mut receipt = json!({
        "ok": ok,
        "type": "digital_dna_receipt",
        "lane": "core/layer0/ops",
        "ts": now_iso(),
        "action": action,
        "instance_dna_ref": instance_dna_ref,
        "payload": payload,
        "layer_ref": {
            "layer0": "safety",
            "layer1": "policy_and_receipts"
        }
    });
    receipt["receipt_hash"] = Value::String(deterministic_receipt_hash(&receipt));
    append_jsonl(&digital_dna_receipts_path(root), &receipt);
    write_json(&digital_dna_latest_receipt_path(root), &receipt);
    receipt
}

fn parse_u64_clamped(raw: Option<&String>, fallback: u64, max: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .min(max)
}

fn normalize_schema_version(raw: Option<&String>) -> String {
    let candidate = clean(
        raw.map(String::as_str)
            .unwrap_or(DIGITAL_DNA_SCHEMA_VERSION)
            .to_string(),
        32,
    );
    if candidate.trim().is_empty() {
        DIGITAL_DNA_SCHEMA_VERSION.to_string()
    } else {
        candidate.trim().to_string()
    }
}

fn build_generated_instance_id(state: &DigitalDnaState) -> String {
    let digest = deterministic_receipt_hash(&json!({
        "ts": now_iso(),
        "count": state.genomes.len()
    }));
    format!("instance-{}", &digest[..12])
}

fn evaluate_subservience(
    root: &Path,
    instance_id: &str,
    expected_parent_signature_raw: Option<&String>,
    action: &str,
    strict: bool,
) -> Value {
    let action_is_critical = matches!(action, "invoke_agent" | "fork_instance");
    let expected_parent_signature = expected_parent_signature_raw
        .map(String::as_str)
        .map(|raw| normalize_token(raw, DIGITAL_DNA_DEFAULT_PARENT_SIGNATURE, 128))
        .unwrap_or_else(|| DIGITAL_DNA_DEFAULT_PARENT_SIGNATURE.to_string());

    if !action_is_critical || expected_parent_signature_raw.is_none() {
        return json!({
            "checked": false,
            "enforced": false,
            "ok": true,
            "reason": "subservience_not_enforced_for_this_action"
        });
    }

    let state = load_digital_dna_state(root);
    let parent_signature = state
        .genomes
        .get(instance_id)
        .map(|genome| genome.header.parent_signature.clone())
        .unwrap_or_default();
    let genome_exists = !parent_signature.is_empty();
    let signatures_match = genome_exists && parent_signature == expected_parent_signature;
    let ok = signatures_match;

    let judicial_lock_triggered = strict && !ok;
    if judicial_lock_triggered {
        let lock_path = judicial_lock_path(root);
        let lock_payload = json!({
            "type": "metakernel_judicial_lock",
            "active": true,
            "trigger": "digital_dna_subservience",
            "ts": now_iso(),
            "instance_dna": instance_id,
            "action": action,
            "expected_parent_signature": expected_parent_signature,
            "actual_parent_signature": parent_signature,
            "violation_codes": ["parent_signature_mismatch"]
        });
        write_json(&lock_path, &lock_payload);
    }

    json!({
        "checked": true,
        "enforced": true,
        "ok": ok,
        "reason": if ok { "subservience_verified" } else if !genome_exists { "instance_dna_not_found" } else { "parent_signature_mismatch" },
        "expected_parent_signature": expected_parent_signature,
        "actual_parent_signature": parent_signature,
        "judicial_lock_triggered": judicial_lock_triggered
    })
}

// TODO-NORMATIVE: Full source-bound subservience chain semantics and governance policy are DEFERRED-V1.
fn run_digital_dna_create(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    parent_signature_raw: Option<&String>,
    schema_version_raw: Option<&String>,
    generation_raw: Option<&String>,
    seed_raw: Option<&String>,
) -> Value {
    let mut state = load_digital_dna_state(root);
    let generated_id = build_generated_instance_id(&state);
    let instance_id = normalize_token(
        instance_id_raw.map(String::as_str).unwrap_or(&generated_id),
        &generated_id,
        96,
    );
    if state.genomes.contains_key(&instance_id) {
        return json!({
            "ok": false,
            "type": "digital_dna_create",
            "error": "instance_id_already_exists",
            "instance_dna_ref": instance_id
        });
    }
    let parent_signature = normalize_token(
        parent_signature_raw
            .map(String::as_str)
            .unwrap_or(DIGITAL_DNA_DEFAULT_PARENT_SIGNATURE),
        DIGITAL_DNA_DEFAULT_PARENT_SIGNATURE,
        128,
    );
    let schema_version = normalize_schema_version(schema_version_raw);
    let birth_generation = parse_u64_clamped(generation_raw, 0, 1_000_000_000);
    let seed = clean(
        seed_raw
            .map(String::as_str)
            .unwrap_or("digital_dna_seed")
            .to_string(),
        256,
    );

    let genome = InstanceDna {
        header: GenomeHeader {
            instance_id: instance_id.clone(),
            parent_signature,
            version: schema_version,
            birth_generation,
        },
        genes: vec![default_gene(&seed)],
    };
    if let Err(err) = validate_instance_dna(&genome) {
        let receipt = write_digital_dna_receipt(
            root,
            "create",
            &instance_id,
            false,
            &json!({"error": err, "strict": strict}),
        );
        return json!({
            "ok": false,
            "type": "digital_dna_create",
            "error": "genome_validation_failed",
            "detail": err,
            "instance_dna_ref": instance_id,
            "receipt": receipt
        });
    }

    state.genomes.insert(instance_id.clone(), genome.clone());
    save_digital_dna_state(root, &state);
    let receipt = write_digital_dna_receipt(
        root,
        "create",
        &instance_id,
        true,
        &json!({"strict": strict, "birth_generation": birth_generation}),
    );

    json!({
        "ok": true,
        "type": "digital_dna_create",
        "instance_dna_ref": instance_id,
        "genome": genome,
        "receipt": receipt
    })
}

fn run_digital_dna_mutate(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    mutation_raw: Option<&String>,
    seed_raw: Option<&String>,
) -> Value {
    let Some(instance_id_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "digital_dna_mutate",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_id_raw, "instance", 96);
    let mutation = normalize_token(
        mutation_raw.map(String::as_str).unwrap_or("repair"),
        "repair",
        64,
    );
    let mut state = load_digital_dna_state(root);
    let Some(genome) = state.genomes.get_mut(&instance_id) else {
        return json!({
            "ok": false,
            "type": "digital_dna_mutate",
            "error": "instance_id_not_found",
            "instance_dna_ref": instance_id
        });
    };

    let mut mutation_report = json!({
        "mutation": mutation,
        "strict": strict
    });

    match mutation.as_str() {
        "repair" => {
            let (repaired_letters, complement_matches) = repair_instance_dna(genome);
            mutation_report["repaired_letters"] = json!(repaired_letters);
            mutation_report["complement_matches"] = json!(complement_matches);
        }
        "append-codon" => {
            let seed = clean(
                seed_raw
                    .map(String::as_str)
                    .unwrap_or("digital_dna_mutation_seed")
                    .to_string(),
                256,
            );
            if genome.genes.is_empty() {
                genome.genes.push(default_gene(&seed));
            } else if let Some(first_gene) = genome.genes.first_mut() {
                let index = first_gene.codons.len();
                first_gene.codons.push(seeded_codon(&seed, index));
            }
            mutation_report["codon_appended"] = json!(true);
        }
        "bump-generation" => {
            genome.header.birth_generation = genome.header.birth_generation.saturating_add(1);
            mutation_report["birth_generation"] = json!(genome.header.birth_generation);
        }
        _ => {
            return json!({
                "ok": false,
                "type": "digital_dna_mutate",
                "error": "unknown_mutation",
                "mutation": mutation,
                "instance_dna_ref": instance_id
            });
        }
    }

    if let Err(err) = validate_instance_dna(genome) {
        let receipt = write_digital_dna_receipt(
            root,
            "mutate",
            &instance_id,
            false,
            &json!({"error": err, "mutation": mutation}),
        );
        return json!({
            "ok": false,
            "type": "digital_dna_mutate",
            "error": "genome_validation_failed",
            "detail": err,
            "instance_dna_ref": instance_id,
            "receipt": receipt
        });
    }

    let genome_snapshot = genome.clone();
    save_digital_dna_state(root, &state);
    let receipt = write_digital_dna_receipt(root, "mutate", &instance_id, true, &mutation_report);
    json!({
        "ok": true,
        "type": "digital_dna_mutate",
        "instance_dna_ref": instance_id,
        "mutation": mutation,
        "genome": genome_snapshot,
        "receipt": receipt
    })
}

fn run_digital_dna_subservience(
    root: &Path,
    strict: bool,
    instance_id_raw: Option<&String>,
    expected_parent_signature_raw: Option<&String>,
    action_raw: Option<&String>,
) -> Value {
    let Some(instance_id_raw) = instance_id_raw else {
        return json!({
            "ok": false,
            "type": "digital_dna_subservience",
            "error": "instance_id_required"
        });
    };
    let instance_id = normalize_token(instance_id_raw, "instance", 96);
    let action = normalize_token(
        action_raw.map(String::as_str).unwrap_or("invoke_agent"),
        "invoke_agent",
        96,
    );
    let check = evaluate_subservience(
        root,
        &instance_id,
        expected_parent_signature_raw,
        &action,
        strict,
    );
    let ok = check.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let receipt = write_digital_dna_receipt(
        root,
        "subservience_check",
        &instance_id,
        ok,
        &json!({"action": action, "check": check.clone()}),
    );
    json!({
        "ok": if strict { ok } else { true },
        "type": "digital_dna_subservience",
        "instance_dna_ref": instance_id,
        "action": action,
        "check": check,
        "receipt": receipt,
        "judicial_lock": {
            "triggered": !ok && strict
        }
    })
}

fn run_digital_dna_status(root: &Path) -> Value {
    let state = load_digital_dna_state(root);
    let latest_receipt = read_json(&digital_dna_latest_receipt_path(root));
    // TODO-NORMATIVE: Qubit wrapper semantics remain DEFERRED-V1; this structure is a placeholder for source-bound superposition metadata.
    let qubit_wrapper_example = QubitWrapper::<Value> {
        layer: "quark".to_string(),
        superposed: vec![
            json!({"value": -1}),
            json!({"value": 0}),
            json!({"value": 1}),
        ],
        collapsed_index: None,
        semantics: "DEFERRED-V1".to_string(),
    };
    json!({
        "ok": true,
        "type": "digital_dna_status",
        "schema_version": state.schema_version,
        "genome_count": state.genomes.len(),
        "instance_ids": state.genomes.keys().cloned().collect::<Vec<_>>(),
        "latest_receipt": latest_receipt,
        "deferred": {
            "qubit_wrapper": qubit_wrapper_example,
            "subservience_full_chain_rules": "DEFERRED-V1"
        }
    })
}

#[cfg(test)]
mod digital_dna_tests {
    use super::*;

    fn sample_letter() -> Letter {
        Letter::new(
            Baryon::from_trits([1, 0, -1]).expect("valid baryon"),
            Baryon::from_trits([0, 1, 1]).expect("valid baryon"),
            Baryon::from_trits([-1, 0, 1]).expect("valid baryon"),
        )
    }

    #[test]
    fn letter_validation_rejects_invalid_verity() {
        let mut letter = sample_letter();
        assert!(letter.is_valid());
        letter.verity = letter.verity.complement();
        assert!(!letter.is_valid());
    }

    #[test]
    fn codon_new_rejects_invalid_letter() {
        let mut invalid = sample_letter();
        invalid.verity = invalid.verity.complement();
        let valid = sample_letter();
        let out = Codon::new([invalid, valid.clone(), valid.clone(), valid.clone()]);
        assert!(out.is_err());
    }

    #[test]
    fn genome_create_emits_receipt_with_instance_reference() {
        let root = tempfile::tempdir().expect("tempdir");
        let instance_id = "instance-alpha".to_string();
        let parent = "root-parent".to_string();
        let schema = "v1".to_string();
        let generation = "3".to_string();
        let seed = "seed-alpha".to_string();
        let out = run_digital_dna_create(
            root.path(),
            true,
            Some(&instance_id),
            Some(&parent),
            Some(&schema),
            Some(&generation),
            Some(&seed),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("instance_dna_ref").and_then(Value::as_str),
            Some("instance-alpha")
        );
        let latest =
            read_json(&digital_dna_latest_receipt_path(root.path())).expect("latest receipt");
        assert_eq!(
            latest.get("instance_dna_ref").and_then(Value::as_str),
            Some("instance-alpha")
        );
    }

    #[test]
    fn subservience_mismatch_triggers_judicial_lock() {
        let root = tempfile::tempdir().expect("tempdir");
        let instance_id = "instance-beta".to_string();
        let parent = "parent-a".to_string();
        let schema = "v1".to_string();
        let generation = "0".to_string();
        let seed = "seed-beta".to_string();
        let _ = run_digital_dna_create(
            root.path(),
            true,
            Some(&instance_id),
            Some(&parent),
            Some(&schema),
            Some(&generation),
            Some(&seed),
        );

        let wrong_parent = "parent-b".to_string();
        let action = "invoke_agent".to_string();
        let out = run_digital_dna_subservience(
            root.path(),
            true,
            Some(&instance_id),
            Some(&wrong_parent),
            Some(&action),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.pointer("/judicial_lock/triggered")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
}
