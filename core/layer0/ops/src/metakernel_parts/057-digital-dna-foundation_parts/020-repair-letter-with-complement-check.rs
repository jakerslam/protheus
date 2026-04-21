
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
    receipt["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&receipt));
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
    let digest = crate::deterministic_receipt_hash(&json!({
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
