
fn default_states() -> Vec<LoopState> {
    vec![
        LoopState {
            loop_id: AUTOGENESIS_LOOP_ID.to_string(),
            generation: 1,
            quality_score: 72.0,
            drift_pct: 1.1,
            last_mutation: "bootstrap_seed".to_string(),
            insights: vec![
                "spawn candidates ranked by quality receipts".to_string(),
                "promote only after deterministic replay".to_string(),
            ],
        },
        LoopState {
            loop_id: DUAL_BRAIN_LOOP_ID.to_string(),
            generation: 1,
            quality_score: 75.0,
            drift_pct: 0.9,
            last_mutation: "bootstrap_seed".to_string(),
            insights: vec![
                "human feedback weights loop reward model".to_string(),
                "symbiosis checkpoints protect intent fidelity".to_string(),
            ],
        },
        LoopState {
            loop_id: RED_LEGION_LOOP_ID.to_string(),
            generation: 1,
            quality_score: 70.0,
            drift_pct: 1.4,
            last_mutation: "bootstrap_seed".to_string(),
            insights: vec![
                "chaos inversion catches weak assumptions".to_string(),
                "2pct drift line triggers sovereignty scrutiny".to_string(),
            ],
        },
        LoopState {
            loop_id: BLOB_MORPHING_LOOP_ID.to_string(),
            generation: 1,
            quality_score: 73.5,
            drift_pct: 0.8,
            last_mutation: "bootstrap_seed".to_string(),
            insights: vec![
                "freeze evolve unfold parity enforced".to_string(),
                "manifest signatures gate blob mutation".to_string(),
            ],
        },
    ]
}

fn evolution_profile(loop_id: &str) -> (f64, f64) {
    match loop_id {
        AUTOGENESIS_LOOP_ID => (1.2, 0.65),
        DUAL_BRAIN_LOOP_ID => (0.9, 0.55),
        RED_LEGION_LOOP_ID => (1.4, 0.8),
        BLOB_MORPHING_LOOP_ID => (1.0, 0.6),
        _ => (0.5, 0.5),
    }
}

fn evolve_state(state: &LoopState) -> LoopState {
    let (quality_gain, drift_decay) = evolution_profile(&state.loop_id);
    let evolved_quality = (state.quality_score + quality_gain).clamp(0.0, 100.0);
    let evolved_drift = (state.drift_pct * drift_decay).clamp(0.0, 5.0);

    let mut insights = state.insights.clone();
    insights.push(format!(
        "generation_{}: quality+{:.2} drift->{:.3}",
        state.generation + 1,
        quality_gain,
        evolved_drift
    ));

    LoopState {
        loop_id: state.loop_id.clone(),
        generation: state.generation + 1,
        quality_score: round3(evolved_quality),
        drift_pct: round3(evolved_drift),
        last_mutation: "evolved_by_singularity_seed_orchestrator".to_string(),
        insights,
    }
}

fn apply_drift_overrides(states: &mut [LoopState], overrides: &[DriftOverride]) {
    for override_item in overrides {
        let loop_id = normalize_loop_id(&override_item.loop_id);
        if loop_id.is_empty() || !is_known_loop_id(&loop_id) {
            continue;
        }
        if let Some(state) = states
            .iter_mut()
            .find(|state| state.loop_id == loop_id)
        {
            state.drift_pct = round3(override_item.drift_pct.clamp(0.0, MAX_DRIFT_OVERRIDE_PCT));
            state.insights.push(format!(
                "override_applied: drift_pct={:.3}",
                state.drift_pct
            ));
        }
    }
}

fn ensure_blob_root(root: &Path) -> Result<(), SeedError> {
    std::fs::create_dir_all(root).map_err(|err| SeedError::IoFailed(err.to_string()))
}

fn freeze_states(states: &[LoopState], root: &Path) -> Result<Vec<BlobManifestEntry>, SeedError> {
    ensure_blob_root(root)?;

    let mut blob_storage: Vec<(String, Vec<u8>)> = Vec::new();
    for loop_id in LOOP_IDS {
        let state = states
            .iter()
            .find(|row| row.loop_id == loop_id)
            .ok_or_else(|| SeedError::UnknownBlob(loop_id.to_string()))?;
        let (blob_bytes, _) = fold_blob(state, loop_id)?;
        std::fs::write(loop_blob_path(root, loop_id), &blob_bytes)
            .map_err(|err| SeedError::IoFailed(err.to_string()))?;
        blob_storage.push((loop_id.to_string(), blob_bytes));
    }

    let manifest_refs = blob_storage
        .iter()
        .map(|(id, bytes)| (id.as_str(), bytes.as_slice()))
        .collect::<Vec<_>>();
    let manifest = generate_manifest(&manifest_refs);
    let manifest_bytes = encode_manifest(&manifest)?;
    std::fs::write(manifest_path(root), &manifest_bytes)
        .map_err(|err| SeedError::IoFailed(err.to_string()))?;

    Ok(manifest)
}

fn load_states(root: &Path) -> Result<(Vec<LoopState>, Vec<BlobManifestEntry>), SeedError> {
    let manifest_raw =
        std::fs::read(manifest_path(root)).map_err(|err| SeedError::IoFailed(err.to_string()))?;
    let manifest = decode_manifest(&manifest_raw)?;

    let mut loaded = Vec::new();
    for loop_id in LOOP_IDS {
        let entry = manifest
            .iter()
            .find(|row| row.id == loop_id)
            .ok_or_else(|| SeedError::MissingManifestEntry(loop_id.to_string()))?;

        let blob_bytes = std::fs::read(loop_blob_path(root, loop_id))
            .map_err(|err| SeedError::IoFailed(err.to_string()))?;

        let state: LoopState = unfold_blob_typed(loop_id, &entry.hash, &blob_bytes, &manifest)?;
        loaded.push(state);
    }

    Ok((loaded, manifest))
}

fn ensure_materialized(root: &Path) -> Result<(), SeedError> {
    let manifest_exists = manifest_path(root).exists();
    let all_blobs_exist = LOOP_IDS
        .iter()
        .all(|loop_id| loop_blob_path(root, loop_id).exists());

    if manifest_exists && all_blobs_exist {
        return Ok(());
    }

    let defaults = default_states();
    freeze_states(&defaults, root)?;
    Ok(())
}

pub fn freeze_seed() -> Result<CycleReport, SeedError> {
    let root = blob_root();
    ensure_blob_root(&root)?;
    let states = default_states();
    let manifest = freeze_states(&states, &root)?;

    let outcomes = manifest
        .iter()
        .map(|entry| LoopCycleOutcome {
            loop_id: entry.id.clone(),
            previous_generation: 0,
            next_generation: 1,
            drift_pct: states
                .iter()
                .find(|s| s.loop_id == entry.id)
                .map(|s| s.drift_pct)
                .unwrap_or(0.0),
            frozen_hash: entry.hash.clone(),
            evolved_hash: entry.hash.clone(),
            unfolded_hash: entry.hash.clone(),
            unfolded_match: true,
        })
        .collect::<Vec<_>>();

    Ok(CycleReport {
        ok: true,
        fail_closed: false,
        max_drift_pct: states
            .iter()
            .map(|row| row.drift_pct)
            .fold(0.0_f64, f64::max),
        threshold_pct: DRIFT_FAIL_CLOSED_THRESHOLD_PCT,
        sovereignty_index: 75.0,
        cycle_id: "seed_freeze_bootstrap".to_string(),
        status: "seeded".to_string(),
        reasons: vec![],
        manifest_path: manifest_path(&root).display().to_string(),
        outcomes,
    })
}
