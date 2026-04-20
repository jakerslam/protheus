
pub fn run_guarded_cycle(request: &CycleRequest) -> Result<CycleReport, SeedError> {
    let root = blob_root();
    ensure_materialized(&root)?;
    let normalized_request = normalize_cycle_request(request.clone());

    let (previous_states, previous_manifest) = load_states(&root)?;

    let mut evolved_states = previous_states.iter().map(evolve_state).collect::<Vec<_>>();
    apply_drift_overrides(&mut evolved_states, &normalized_request.drift_overrides);

    let evolved_manifest = freeze_states(&evolved_states, &root)?;
    let (unfolded_states, unfolded_manifest) = load_states(&root)?;

    let previous_state_map = previous_states
        .iter()
        .map(|row| (row.loop_id.clone(), row.clone()))
        .collect::<HashMap<_, _>>();
    let evolved_state_map = evolved_states
        .iter()
        .map(|row| (row.loop_id.clone(), row.clone()))
        .collect::<HashMap<_, _>>();
    let unfolded_state_map = unfolded_states
        .iter()
        .map(|row| (row.loop_id.clone(), row.clone()))
        .collect::<HashMap<_, _>>();

    let previous_hash_map = previous_manifest
        .iter()
        .map(|row| (row.id.clone(), row.hash.clone()))
        .collect::<HashMap<_, _>>();
    let evolved_hash_map = evolved_manifest
        .iter()
        .map(|row| (row.id.clone(), row.hash.clone()))
        .collect::<HashMap<_, _>>();
    let unfolded_hash_map = unfolded_manifest
        .iter()
        .map(|row| (row.id.clone(), row.hash.clone()))
        .collect::<HashMap<_, _>>();

    let mut outcomes = Vec::new();
    let mut reasons = Vec::new();
    let mut max_drift_pct = 0.0_f64;

    for loop_id in LOOP_IDS {
        let prev = previous_state_map
            .get(loop_id)
            .ok_or_else(|| SeedError::UnknownBlob(loop_id.to_string()))?;
        let evolved = evolved_state_map
            .get(loop_id)
            .ok_or_else(|| SeedError::UnknownBlob(loop_id.to_string()))?;
        let unfolded = unfolded_state_map
            .get(loop_id)
            .ok_or_else(|| SeedError::UnknownBlob(loop_id.to_string()))?;

        let unfolded_match = unfolded.generation == evolved.generation
            && (unfolded.quality_score - evolved.quality_score).abs() < 0.0001
            && (unfolded.drift_pct - evolved.drift_pct).abs() < 0.0001;

        max_drift_pct = max_drift_pct.max(unfolded.drift_pct);
        if unfolded.drift_pct > DRIFT_FAIL_CLOSED_THRESHOLD_PCT {
            reasons.push(format!(
                "drift_threshold_exceeded:{}={:.3}%",
                loop_id, unfolded.drift_pct
            ));
        }
        if !unfolded_match {
            reasons.push(format!("unfold_mismatch:{loop_id}"));
        }

        outcomes.push(LoopCycleOutcome {
            loop_id: loop_id.to_string(),
            previous_generation: prev.generation,
            next_generation: unfolded.generation,
            drift_pct: round3(unfolded.drift_pct),
            frozen_hash: previous_hash_map
                .get(loop_id)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            evolved_hash: evolved_hash_map
                .get(loop_id)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            unfolded_hash: unfolded_hash_map
                .get(loop_id)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            unfolded_match,
        });
    }

    let fail_closed = max_drift_pct > DRIFT_FAIL_CLOSED_THRESHOLD_PCT;
    let avg_quality = unfolded_states
        .iter()
        .map(|row| row.quality_score)
        .sum::<f64>()
        / unfolded_states.len().max(1) as f64;
    let sovereignty_index = {
        let drift_penalty = max_drift_pct * 12.5;
        let fail_penalty = if fail_closed { 18.0 } else { 0.0 };
        round3((avg_quality - drift_penalty - fail_penalty).clamp(0.0, 100.0))
    };

    let cycle_digest = sha256_hex(
        serde_json::to_string(&outcomes)
            .unwrap_or_else(|_| "[]".to_string())
            .as_bytes(),
    );

    let status = if fail_closed {
        "fail_closed".to_string()
    } else {
        "stable".to_string()
    };

    Ok(CycleReport {
        ok: !fail_closed,
        fail_closed,
        max_drift_pct: round3(max_drift_pct),
        threshold_pct: DRIFT_FAIL_CLOSED_THRESHOLD_PCT,
        sovereignty_index,
        cycle_id: format!("ssc-{}", &cycle_digest[..16]),
        status,
        reasons,
        manifest_path: manifest_path(&root).display().to_string(),
        outcomes,
    })
}

pub fn run_guarded_cycle_json(request_json: &str) -> Result<String, SeedError> {
    let request: CycleRequest = if request_json.trim().is_empty() {
        CycleRequest::default()
    } else {
        serde_json::from_str(request_json)
            .map_err(|err| SeedError::InvalidRequest(format!("request_parse_failed:{err}")))?
    };
    let normalized_request = normalize_cycle_request_with_contract(request, true)?;

    let report = run_guarded_cycle(&normalized_request)?;
    serde_json::to_string(&report).map_err(|err| SeedError::SerializeFailed(err.to_string()))
}

pub fn show_seed_state_json() -> Result<String, SeedError> {
    let root = blob_root();
    ensure_materialized(&root)?;
    let (states, manifest) = load_states(&root)?;

    let payload = serde_json::json!({
      "ok": true,
      "blob_root": root.display().to_string(),
      "manifest_path": manifest_path(&root).display().to_string(),
      "manifest": manifest,
      "states": states
    });
    serde_json::to_string(&payload).map_err(|err| SeedError::SerializeFailed(err.to_string()))
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_guarded_cycle_wasm(request_json: &str) -> String {
    match run_guarded_cycle_json(request_json) {
        Ok(payload) => payload,
        Err(err) => serde_json::json!({ "ok": false, "error": err.to_string() }).to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn show_seed_state_wasm() -> String {
    match show_seed_state_json() {
        Ok(payload) => payload,
        Err(err) => serde_json::json!({ "ok": false, "error": err.to_string() }).to_string(),
    }
}
