
fn scored_candidate_rows(
    source_root: &Path,
    targets: &[String],
) -> Result<Vec<Value>, String> {
    let mut paths = Vec::<PathBuf>::new();
    if !targets.is_empty() {
        for target in targets {
            let joined = source_root.join(target);
            if joined.exists() && joined.is_file() && candidate_extension_allowed(&joined) {
                paths.push(PathBuf::from(target));
            }
        }
    } else {
        collect_candidate_paths_recursive(source_root, source_root, 0, 8, &mut paths, 256)?;
    }
    if paths.is_empty() {
        return Err("stomach_scoring_gate_no_candidates".to_string());
    }
    paths.sort();
    paths.dedup();

    let mut rows = paths
        .iter()
        .map(|rel| {
            let path_rel = rel.to_string_lossy().replace('\\', "/");
            let authority = score_authority_risk(path_rel.as_str());
            let migration = score_migration_potential(path_rel.as_str());
            let concept = score_concept_opportunity(path_rel.as_str());
            json!({
                "file_path": path_rel,
                "authority_risk_score": authority,
                "migration_potential_score": migration,
                "concept_opportunity_score": concept,
                "priority_score": priority_score(authority, migration, concept),
                "state": "queued",
                "state_history": ["queued"],
                "concept_note": concept_note_for(rel.to_string_lossy().as_ref()),
                "evidence_pointer": null
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        let ap = a
            .get("priority_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let bp = b
            .get("priority_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let aa = a
            .get("authority_risk_score")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let ba = b
            .get("authority_risk_score")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        bp.partial_cmp(&ap)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| ba.cmp(&aa))
            .then_with(|| {
                let al = a.get("file_path").and_then(Value::as_str).unwrap_or("");
                let bl = b.get("file_path").and_then(Value::as_str).unwrap_or("");
                al.cmp(bl)
            })
    });
    Ok(rows)
}

fn write_scoring_gate_ledger(
    root: &Path,
    digest_id: &str,
    source_root: &Path,
    rows: &[Value],
    stage: &str,
) -> Result<PathBuf, String> {
    let state_root = stomach_state_root(root);
    let ledger_path = state_root
        .join("ledgers")
        .join(format!("{digest_id}_file_scores.json"));
    let payload = json!({
        "schema_id": "stomach_file_scoring_ledger",
        "schema_version": "1.0",
        "digest_id": digest_id,
        "source_root": source_root.to_string_lossy().to_string(),
        "stage": clean(stage, 80),
        "mandatory_scoring_gate": true,
        "scored_at": now_iso(),
        "row_count": rows.len(),
        "rows": rows
    });
    write_json(&ledger_path, &payload)?;
    Ok(ledger_path)
}

fn write_scoring_gate_markdown_report(
    root: &Path,
    digest_id: &str,
    rows: &[Value],
) -> Result<PathBuf, String> {
    let today = now_iso();
    let date = today.split('T').next().unwrap_or("unknown-date");
    let reports_root = root
        .join("local")
        .join("workspace")
        .join("reports");
    fs::create_dir_all(&reports_root)
        .map_err(|e| format!("stomach_scoring_report_dir_create_failed:{e}"))?;
    let report_path = reports_root.join(format!("CODEX_FILE_LEDGER_{date}.md"));
    let mut out = String::new();
    out.push_str("# Stomach File Scoring Ledger\n\n");
    out.push_str(&format!("- digest_id: `{}`\n", clean(digest_id, 120)));
    out.push_str(&format!("- generated_at: `{}`\n", today));
    out.push_str("- scoring_gate: mandatory\n\n");
    out.push_str("| file | authority_risk_score | migration_potential_score | concept_opportunity_score | priority_score | state | evidence |\n");
    out.push_str("| --- | ---: | ---: | ---: | ---: | --- | --- |\n");
    for row in rows {
        let file_path = row.get("file_path").and_then(Value::as_str).unwrap_or("-");
        let authority = row
            .get("authority_risk_score")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let migration = row
            .get("migration_potential_score")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let concept = row
            .get("concept_opportunity_score")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let priority = row
            .get("priority_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let state = row.get("state").and_then(Value::as_str).unwrap_or("-");
        let evidence = row
            .get("evidence_pointer")
            .and_then(Value::as_str)
            .unwrap_or("-");
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {:.2} | `{}` | `{}` |\n",
            file_path, authority, migration, concept, priority, state, evidence
        ));
    }
    fs::write(&report_path, out).map_err(|e| format!("stomach_scoring_report_write_failed:{e}"))?;
    Ok(report_path)
}

fn advance_scoring_rows(
    rows: &mut [Value],
    evidence_pointer: &str,
    skipped_reason: Option<&str>,
) {
    for row in rows.iter_mut() {
        let state = if skipped_reason.is_some() {
            "skipped_with_reason"
        } else {
            "done"
        };
        let mut history = vec![Value::String("queued".to_string())];
        history.push(Value::String("in_progress".to_string()));
        history.push(Value::String(state.to_string()));
        row["state"] = Value::String(state.to_string());
        row["state_history"] = Value::Array(history);
        row["evidence_pointer"] = Value::String(clean(evidence_pointer, 300));
        if let Some(reason) = skipped_reason {
            row["skipped_reason"] = Value::String(clean(reason, 160));
        }
    }
}

fn bool_like(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
}

fn nexus_enabled(argv: &[String]) -> bool {
    if let Some(raw) = parse_flag(argv, "nexus") {
        return bool_like(raw.as_str());
    }
    std::env::var("PROTHEUS_HIERARCHICAL_NEXUS_V1")
        .ok()
        .map(|raw| bool_like(raw.as_str()))
        .unwrap_or(true)
}

fn nexus_force_block_pair_enabled() -> bool {
    std::env::var("PROTHEUS_HIERARCHICAL_NEXUS_BLOCK_STOMACH_ROUTE")
        .ok()
        .map(|raw| bool_like(raw.as_str()))
        .unwrap_or(false)
}
