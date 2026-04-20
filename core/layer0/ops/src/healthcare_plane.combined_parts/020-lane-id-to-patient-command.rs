
const LANE_ID: &str = "healthcare_plane";
const ENV_KEY: &str = "PROTHEUS_HEALTHCARE_PLANE_STATE_ROOT";

fn usage() {
    println!("Usage:");
    println!(
        "  protheus-ops healthcare-plane patient --op=<register|status> --patient-id=<id> [--mrn=<id>] [--consent-json=<json>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops healthcare-plane phi-audit --op=<access|status> [--user=<id>] [--npi=<id>] [--patient-id=<id>] [--reason=<treatment|payment|operations|research>] [--break-glass=1|0] [--strict=1|0]"
    );
    println!(
        "  protheus-ops healthcare-plane cds --op=<evaluate|status> [--patient-id=<id>] [--meds=a,b] [--allergies=a,b] [--dose-mg=<n>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops healthcare-plane devices --op=<ingest|status> [--protocol=<hl7|fhir|dicom|ieee11073>] [--device-id=<id>] [--payload-json=<json>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops healthcare-plane documentation --op=<draft|status> [--soap-json=<json>] [--codes-json=<json>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops healthcare-plane alerts --op=<emit|ack|status> [--tier=<info|low|medium|high|critical>] [--key=<id>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops healthcare-plane coordination --op=<handoff|reconcile|status> [--sbar-json=<json>] [--meds-json=<json>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops healthcare-plane trials --op=<screen|consent|report-sae|status> [--patient-id=<id>] [--trial=<id>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops healthcare-plane imaging --op=<ingest|critical-route|status> [--study-id=<id>] [--finding=<text>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops healthcare-plane emergency --op=<break-glass|status> [--user=<id>] [--patient-id=<id>] [--justification=<text>] [--ttl-minutes=<n>] [--strict=1|0]"
    );
}

fn lane_root(root: &Path) -> PathBuf {
    scoped_state_root(root, ENV_KEY, LANE_ID)
}

fn lane_file(root: &Path, file_name: &str) -> PathBuf {
    lane_root(root).join(file_name)
}

fn patients_path(root: &Path) -> PathBuf {
    lane_file(root, "patients.json")
}

fn phi_log_path(root: &Path) -> PathBuf {
    lane_file(root, "phi_access_log.jsonl")
}

fn cds_path(root: &Path) -> PathBuf {
    lane_file(root, "cds_state.json")
}

fn devices_path(root: &Path) -> PathBuf {
    lane_file(root, "device_events.jsonl")
}

fn docs_path(root: &Path) -> PathBuf {
    lane_file(root, "clinical_docs.jsonl")
}

fn alerts_path(root: &Path) -> PathBuf {
    lane_file(root, "alerts.json")
}

fn coordination_path(root: &Path) -> PathBuf {
    lane_file(root, "coordination.jsonl")
}

fn trials_path(root: &Path) -> PathBuf {
    lane_file(root, "trials.json")
}

fn imaging_path(root: &Path) -> PathBuf {
    lane_file(root, "imaging.jsonl")
}

fn emergency_path(root: &Path) -> PathBuf {
    lane_file(root, "emergency.jsonl")
}

fn read_object(path: &Path) -> Map<String, Value> {
    read_json(path)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn csv_set(raw: Option<&String>) -> BTreeSet<String> {
    raw.map(|s| {
        s.split(',')
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty())
            .collect::<BTreeSet<_>>()
    })
    .unwrap_or_default()
}

fn emit(root: &Path, _command: &str, strict: bool, payload: Value, conduit: Option<&Value>) -> i32 {
    emit_attached_plane_receipt(root, ENV_KEY, LANE_ID, strict, payload, conduit)
}

fn patient_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        16,
    )
    .to_ascii_lowercase();
    let mut state = read_object(&patients_path(root));
    if op == "status" {
        return Ok(json!({
            "ok": true,
            "type": "healthcare_plane_patient",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "patients": state,
            "claim_evidence": [{
                "id": "V7-HEALTH-001.1",
                "claim": "patient_identity_plane_tracks_mpi_mrn_and_consent_scoped_context_without_raw_phi_leakage",
                "evidence": {"patient_count": state.len()}
            }]
        }));
    }
    if op != "register" {
        return Err("patient_op_invalid".to_string());
    }
    let patient_id = clean(
        parsed
            .flags
            .get("patient-id")
            .map(String::as_str)
            .unwrap_or("patient"),
        120,
    );
    let mrn = clean(
        parsed
            .flags
            .get("mrn")
            .map(String::as_str)
            .unwrap_or("MRN0000"),
        80,
    );
    let consent = parse_json_or_empty(parsed.flags.get("consent-json"));
    state.insert(
        patient_id.clone(),
        json!({
            "patient_id": patient_id,
            "mrn": mrn,
            "phi_hash": sha256_hex_str(&canonical_json_string(&consent)),
            "consent": consent,
            "updated_at": now_iso()
        }),
    );
    write_json(&patients_path(root), &Value::Object(state.clone()))?;
    Ok(json!({
        "ok": true,
        "type": "healthcare_plane_patient",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "record": state.get(&patient_id).cloned().unwrap_or_else(|| json!({})),
        "claim_evidence": [{
            "id": "V7-HEALTH-001.1",
            "claim": "patient_identity_plane_tracks_mpi_mrn_and_consent_scoped_context_without_raw_phi_leakage",
            "evidence": {"patient_id": patient_id}
        }]
    }))
}
