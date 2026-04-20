// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// V6-ORGAN-001 — Stomach v1 kernel wrapper

use protheus_nexus_core_v1::{
    DefaultNexusPolicy, DeliveryAuthorizationInput, LeaseIssueRequest, MainNexusControlPlane,
    NexusFeatureFlags, TrustClass, VerityClass,
};
use protheus_stomach_core_v1::burn::{purge_artifact_path, transition_retention, RetentionEvent};
use protheus_stomach_core_v1::proposal::{TransformKind, TransformRequest};
use protheus_stomach_core_v1::state::{rollback_by_receipt, DigestState, DigestStatus};
use protheus_stomach_core_v1::{run_stomach_cycle, stable_hash, StomachConfig};
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{clean, deterministic_receipt_hash, now_iso};

fn usage() {
    println!("stomach-kernel commands:");
    println!("  protheus-ops stomach-kernel run --id=<digest_id> --source-root=<path> --origin=<https://...> [--commit=<hash>] [--refs=refs/heads/main] [--spdx=<MIT>] [--transform=namespace_fix|header_injection|path_remap|adapter_scaffold] [--targets=a.rs,b.rs] [--header=...]");
    println!("  protheus-ops stomach-kernel score --id=<digest_id> --source-root=<path> [--targets=a.rs,b.rs]");
    println!("  protheus-ops stomach-kernel status --id=<digest_id>");
    println!("  protheus-ops stomach-kernel rollback --id=<digest_id> --receipt=<receipt_id> [--reason=<text>]");
    println!("  protheus-ops stomach-kernel retention --id=<digest_id> --action=hold|release|eligible [--reason=<text>] [--retained-until=<epoch_secs>] [--approve-receipt=<receipt_id>]");
    println!("  protheus-ops stomach-kernel purge --id=<digest_id>");
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let prefix = format!("--{key}=");
    for token in argv {
        if let Some(rest) = token.strip_prefix(&prefix) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn csv_list(raw: Option<String>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(|row| row.trim().to_string())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
}

fn candidate_extension_allowed(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref(),
        Some("rs")
            | Some("ts")
            | Some("tsx")
            | Some("toml")
            | Some("json")
            | Some("yaml")
            | Some("yml")
            | Some("md")
            | Some("py")
    )
}

fn should_skip_candidate_path(path: &Path) -> bool {
    let normalized = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    normalized.contains("/.git/")
        || normalized.contains("/target/")
        || normalized.contains("/node_modules/")
        || normalized.contains("/dist/")
        || normalized.contains("/build/")
        || normalized.contains("/local/state/")
}

fn collect_candidate_paths_recursive(
    source_root: &Path,
    current: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<PathBuf>,
    max_files: usize,
) -> Result<(), String> {
    if depth > max_depth || out.len() >= max_files {
        return Ok(());
    }
    let entries = fs::read_dir(current)
        .map_err(|e| format!("stomach_candidate_scan_failed:{}:{e}", current.display()))?;
    for entry in entries {
        if out.len() >= max_files {
            break;
        }
        let entry = entry.map_err(|e| format!("stomach_candidate_entry_failed:{e}"))?;
        let path = entry.path();
        if should_skip_candidate_path(&path) {
            continue;
        }
        if path.is_dir() {
            collect_candidate_paths_recursive(
                source_root,
                &path,
                depth.saturating_add(1),
                max_depth,
                out,
                max_files,
            )?;
            continue;
        }
        if !path.is_file() || !candidate_extension_allowed(&path) {
            continue;
        }
        let rel = path
            .strip_prefix(source_root)
            .map(PathBuf::from)
            .unwrap_or(path.clone());
        out.push(rel);
    }
    Ok(())
}

fn score_authority_risk(path_rel: &str) -> u8 {
    let normalized = path_rel.to_ascii_lowercase();
    let mut score = 1u8;
    if normalized.contains("core/") || normalized.contains("/security/") || normalized.contains("/ops/") {
        score = 5;
    } else if normalized.contains("surface/") || normalized.contains("/autonomy/") {
        score = 4;
    } else if normalized.contains("client/runtime/") {
        score = 3;
    } else if normalized.contains("docs/") || normalized.ends_with(".md") {
        score = 2;
    }
    score.min(5)
}

fn score_migration_potential(path_rel: &str) -> u8 {
    let normalized = path_rel.to_ascii_lowercase();
    let mut score = 2u8;
    if normalized.ends_with(".rs") {
        score = 5;
    } else if normalized.ends_with(".ts") || normalized.ends_with(".tsx") {
        score = 4;
    } else if normalized.ends_with(".toml") || normalized.ends_with(".json") {
        score = 3;
    } else if normalized.ends_with(".md") {
        score = 2;
    }
    if normalized.contains("/tests/") || normalized.contains("/fixtures/") {
        score = score.saturating_sub(1).max(1);
    }
    score.min(5)
}

fn score_concept_opportunity(path_rel: &str) -> u8 {
    let normalized = path_rel.to_ascii_lowercase();
    let mut score = 2u8;
    if normalized.contains("planner")
        || normalized.contains("orchestration")
        || normalized.contains("memory")
        || normalized.contains("autonomy")
        || normalized.contains("tooling")
    {
        score = 5;
    } else if normalized.contains("gateway") || normalized.contains("conduit") || normalized.contains("receipt")
    {
        score = 4;
    } else if normalized.contains("ui/") || normalized.contains("docs/") {
        score = 3;
    }
    score.min(5)
}

fn concept_note_for(path_rel: &str) -> String {
    let normalized = path_rel.replace('\\', "/");
    let leaf = normalized.rsplit('/').next().unwrap_or(path_rel);
    format!("extract reusable concept from {}", clean(leaf, 120))
}

fn priority_score(authority: u8, migration: u8, concept: u8) -> f64 {
    let authority_w = (authority as f64) * 0.5;
    let migration_w = (migration as f64) * 0.3;
    let concept_w = (concept as f64) * 0.2;
    ((authority_w + migration_w + concept_w) * 100.0).round() / 100.0
}

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

fn authorize_stomach_command_with_nexus_inner(
    command: &str,
    force_block_pair: bool,
) -> Result<Value, String> {
    let mut policy = DefaultNexusPolicy::default();
    if force_block_pair {
        policy.block_pair("client_ingress", "stomach");
    }
    let mut nexus = MainNexusControlPlane::new(
        NexusFeatureFlags {
            hierarchical_nexus_enabled: true,
            coexist_with_flat_routing: true,
        },
        policy,
    );
    let _ = nexus.register_v1_adapters("stomach_kernel")?;
    let schema = format!("stomach.kernel.{}", clean(command, 64));
    let lease = nexus.issue_route_lease(
        "stomach_kernel",
        LeaseIssueRequest {
            source: "client_ingress".to_string(),
            target: "stomach".to_string(),
            schema_ids: vec![schema.clone()],
            verbs: vec!["invoke".to_string()],
            required_verity: VerityClass::Standard,
            trust_class: TrustClass::InterModuleData,
            requested_ttl_ms: 30_000,
            template_id: None,
            template_version: None,
        },
    )?;
    let delivery = nexus.authorize_direct_delivery(
        "stomach_kernel",
        DeliveryAuthorizationInput {
            lease_id: Some(lease.lease_id.clone()),
            source: "client_ingress".to_string(),
            target: "stomach".to_string(),
            schema_id: schema,
            verb: "invoke".to_string(),
            offered_verity: VerityClass::Standard,
            now_ms: None,
        },
    );
    if !delivery.allowed {
        return Err(format!(
            "stomach_nexus_delivery_denied:{}",
            clean(delivery.reason.as_str(), 200)
        ));
    }
    let receipt_ids = nexus
        .receipts()
        .iter()
        .map(|row| row.receipt_id.clone())
        .collect::<Vec<_>>();
    Ok(json!({
      "enabled": true,
      "route": {"source":"client_ingress","target":"stomach","verb":"invoke"},
      "lease_id": lease.lease_id,
      "delivery": delivery,
      "metrics": nexus.metrics(),
      "receipt_ids": receipt_ids
    }))
}

fn authorize_stomach_command_with_nexus(command: &str) -> Result<Value, String> {
    authorize_stomach_command_with_nexus_inner(command, nexus_force_block_pair_enabled())
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn receipt_envelope(kind: &str, ok: bool) -> Value {
    let ts = now_iso();
    json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string()
    })
}

fn json_error(kind: &str, error: &str) -> Value {
    let mut out = receipt_envelope(kind, false);
    out["error"] = Value::String(error.to_string());
    out["fail_closed"] = Value::Bool(true);
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn json_receipt(kind: &str, payload: Value) -> Value {
    let mut out = receipt_envelope(kind, true);
    out["payload"] = payload;
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn stomach_state_root(root: &Path) -> PathBuf {
    root.join("local").join("state").join("stomach")
}

fn ensure_state_dirs(state_root: &Path) -> Result<(), String> {
    for rel in [
        "quarantine",
        "fetch",
        "snapshots",
        "provenance",
        "analysis",
        "proposals",
        "state",
    ] {
        fs::create_dir_all(state_root.join(rel))
            .map_err(|e| format!("stomach_state_dir_create_failed:{rel}:{e}"))?;
    }
    Ok(())
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("stomach_write_parent_create_failed:{e}"))?;
    }
    let encoded = serde_json::to_string_pretty(value)
        .map_err(|e| format!("stomach_write_encode_failed:{e}"))?;
    fs::write(path, format!("{encoded}\n")).map_err(|e| format!("stomach_write_failed:{e}"))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("stomach_read_failed:{e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("stomach_decode_failed:{e}"))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("stomach_jsonl_parent_create_failed:{e}"))?;
    }
    let line =
        serde_json::to_string(value).map_err(|e| format!("stomach_jsonl_encode_failed:{e}"))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("stomach_jsonl_open_failed:{e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("stomach_jsonl_write_failed:{e}"))
}

fn parse_transform(argv: &[String]) -> TransformRequest {
    let transform = parse_flag(argv, "transform").unwrap_or_else(|| "header_injection".to_string());
    let targets = csv_list(parse_flag(argv, "targets"));
    match transform.to_ascii_lowercase().as_str() {
        "namespace_fix" => TransformRequest {
            kind: TransformKind::NamespaceFix,
            target_paths: targets,
            namespace_from: parse_flag(argv, "namespace-from"),
            namespace_to: parse_flag(argv, "namespace-to"),
            header_text: None,
            path_prefix_from: None,
            path_prefix_to: None,
            adapter_name: None,
        },
        "path_remap" => TransformRequest {
            kind: TransformKind::PathRemap,
            target_paths: targets,
            namespace_from: None,
            namespace_to: None,
            header_text: None,
            path_prefix_from: parse_flag(argv, "path-from"),
            path_prefix_to: parse_flag(argv, "path-to"),
            adapter_name: None,
        },
        "adapter_scaffold" => TransformRequest {
            kind: TransformKind::AdapterScaffold,
            target_paths: targets,
            namespace_from: None,
            namespace_to: None,
            header_text: None,
            path_prefix_from: None,
            path_prefix_to: None,
            adapter_name: parse_flag(argv, "adapter-name"),
        },
        _ => TransformRequest::header_injection(
            targets,
            parse_flag(argv, "header").unwrap_or_else(|| "// staged by stomach".to_string()),
        ),
    }
}

fn run_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").unwrap_or_else(|| "stomach-default".to_string());
    let source_root = parse_flag(argv, "source-root")
        .map(PathBuf::from)
        .ok_or_else(|| "stomach_missing_source_root".to_string())?;
    let origin = parse_flag(argv, "origin")
        .unwrap_or_else(|| "https://github.com/protheuslabs/InfRing".to_string());
    let commit = parse_flag(argv, "commit").unwrap_or_else(|| "unknown".to_string());
    let refs = csv_list(parse_flag(argv, "refs"));
    let spdx = parse_flag(argv, "spdx");
    let transform = parse_transform(argv);
    let scoring_targets = if !transform.target_paths.is_empty() {
        transform.target_paths.clone()
    } else {
        Vec::<String>::new()
    };

    let state_root = stomach_state_root(root);
    ensure_state_dirs(&state_root)?;
    let mut scoring_rows = scored_candidate_rows(&source_root, scoring_targets.as_slice())?;
    let scoring_ledger_path =
        write_scoring_gate_ledger(root, &digest_id, &source_root, &scoring_rows, "preflight_scored")?;

    let out = run_stomach_cycle(
        &state_root,
        &digest_id,
        &source_root,
        &origin,
        &commit,
        &refs,
        spdx.as_deref(),
        &transform,
        &StomachConfig::default(),
    )?;

    write_json(
        &state_root.join("fetch").join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.fetch)
            .map_err(|e| format!("stomach_fetch_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root
            .join("snapshots")
            .join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.snapshot)
            .map_err(|e| format!("stomach_snapshot_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root
            .join("provenance")
            .join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.provenance)
            .map_err(|e| format!("stomach_provenance_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root
            .join("analysis")
            .join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.analysis)
            .map_err(|e| format!("stomach_analysis_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root
            .join("proposals")
            .join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.proposal)
            .map_err(|e| format!("stomach_proposal_encode_failed:{e}"))?,
    )?;
    write_json(
        &state_root.join("state").join(format!("{digest_id}.json")),
        &serde_json::to_value(&out.state)
            .map_err(|e| format!("stomach_state_encode_failed:{e}"))?,
    )?;
    let evidence_pointer = state_root
        .join("receipts.jsonl")
        .to_string_lossy()
        .to_string();
    advance_scoring_rows(&mut scoring_rows, &evidence_pointer, None);
    let _ = write_scoring_gate_ledger(
        root,
        &digest_id,
        &source_root,
        &scoring_rows,
        "completed",
    )?;
    let scoring_report_path =
        write_scoring_gate_markdown_report(root, &digest_id, &scoring_rows)?;

    let receipt_payload = json!({
      "digest_id": digest_id,
      "snapshot_id": out.snapshot.snapshot_id,
      "proposal_id": out.proposal.proposal_id,
      "execution_status": out.execution.status,
      "state_status": out.state.status,
      "cycle_hash": stable_hash(&out),
      "scoring_gate": {
        "mandatory": true,
        "ledger_path": scoring_ledger_path.to_string_lossy().to_string(),
        "report_path": scoring_report_path.to_string_lossy().to_string(),
        "row_count": scoring_rows.len()
      }
    });
    let receipt = json_receipt("stomach_kernel_run", receipt_payload);
    append_jsonl(&state_root.join("receipts.jsonl"), &receipt)?;
    Ok(receipt)
}

fn score_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").unwrap_or_else(|| "stomach-default".to_string());
    let source_root = parse_flag(argv, "source-root")
        .map(PathBuf::from)
        .ok_or_else(|| "stomach_missing_source_root".to_string())?;
    let targets = csv_list(parse_flag(argv, "targets"));
    let mut rows = scored_candidate_rows(&source_root, targets.as_slice())?;
    let evidence_pointer = stomach_state_root(root)
        .join("receipts.jsonl")
        .to_string_lossy()
        .to_string();
    advance_scoring_rows(&mut rows, &evidence_pointer, Some("score_only_mode"));
    let ledger_path = write_scoring_gate_ledger(root, &digest_id, &source_root, &rows, "score_only")?;
    let report_path = write_scoring_gate_markdown_report(root, &digest_id, &rows)?;
    Ok(json_receipt(
        "stomach_kernel_score",
        json!({
            "digest_id": digest_id,
            "mandatory_scoring_gate": true,
            "ledger_path": ledger_path.to_string_lossy().to_string(),
            "report_path": report_path.to_string_lossy().to_string(),
            "row_count": rows.len()
        }),
    ))
}

fn status_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").ok_or_else(|| "stomach_missing_id".to_string())?;
    let state_root = stomach_state_root(root);
    let state = read_json(&state_root.join("state").join(format!("{digest_id}.json")))?;
    let proposal = read_json(
        &state_root
            .join("proposals")
            .join(format!("{digest_id}.json")),
    )
    .ok();
    Ok(json_receipt(
        "stomach_kernel_status",
        json!({
          "digest_id": digest_id,
          "state": state,
          "proposal": proposal
        }),
    ))
}

fn rollback_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").ok_or_else(|| "stomach_missing_id".to_string())?;
    let receipt =
        parse_flag(argv, "receipt").ok_or_else(|| "stomach_missing_receipt".to_string())?;
    let reason = parse_flag(argv, "reason").unwrap_or_else(|| "manual_rollback".to_string());
    let state_root = stomach_state_root(root);
    let state_path = state_root.join("state").join(format!("{digest_id}.json"));
    let raw = read_json(&state_path)?;
    let mut state: DigestState =
        serde_json::from_value(raw).map_err(|e| format!("stomach_state_decode_failed:{e}"))?;
    let rollback = rollback_by_receipt(&mut state, &receipt, &reason)?;
    write_json(
        &state_path,
        &serde_json::to_value(&state).map_err(|e| format!("stomach_state_encode_failed:{e}"))?,
    )?;
    let out = json_receipt(
        "stomach_kernel_rollback",
        json!({
          "digest_id": digest_id,
          "rollback": rollback,
          "state_status": state.status
        }),
    );
    append_jsonl(&state_root.join("receipts.jsonl"), &out)?;
    Ok(out)
}

fn purge_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").ok_or_else(|| "stomach_missing_id".to_string())?;
    let state_root = stomach_state_root(root);
    let state_path = state_root.join("state").join(format!("{digest_id}.json"));
    let raw = read_json(&state_path)?;
    let mut state: DigestState =
        serde_json::from_value(raw).map_err(|e| format!("stomach_state_decode_failed:{e}"))?;
    if matches!(
        state.status,
        DigestStatus::Proposed | DigestStatus::Verified | DigestStatus::Assimilated
    ) && state.retention.explicit_purge_approval_receipt.is_none()
    {
        return Err(
            "stomach_purge_explicit_approval_required_for_proposed_or_assimilated".to_string(),
        );
    }
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let quarantine_dir = state_root.join("quarantine").join(&digest_id);
    purge_artifact_path(&quarantine_dir, &mut state.retention, now_secs)?;
    write_json(
        &state_path,
        &serde_json::to_value(&state).map_err(|e| format!("stomach_state_encode_failed:{e}"))?,
    )?;
    let out = json_receipt(
        "stomach_kernel_purge",
        json!({
          "digest_id": digest_id,
          "retention_state": state.retention_state()
        }),
    );
    append_jsonl(&state_root.join("receipts.jsonl"), &out)?;
    Ok(out)
}

fn retention_cycle(root: &Path, argv: &[String]) -> Result<Value, String> {
    let digest_id = parse_flag(argv, "id").ok_or_else(|| "stomach_missing_id".to_string())?;
    let action = parse_flag(argv, "action")
        .unwrap_or_else(|| "hold".to_string())
        .to_ascii_lowercase();
    let reason = parse_flag(argv, "reason").unwrap_or_else(|| "manual_hold".to_string());
    let retained_until = parse_flag(argv, "retained-until").and_then(|raw| raw.parse::<u64>().ok());
    let approve_receipt = parse_flag(argv, "approve-receipt");

    let state_root = stomach_state_root(root);
    let state_path = state_root.join("state").join(format!("{digest_id}.json"));
    let raw = read_json(&state_path)?;
    let mut state: DigestState =
        serde_json::from_value(raw).map_err(|e| format!("stomach_state_decode_failed:{e}"))?;

    if let Some(epoch_secs) = retained_until {
        transition_retention(
            &mut state.retention,
            RetentionEvent::SetRetainedUntil { epoch_secs },
        )?;
    }
    if let Some(receipt_id) = approve_receipt {
        transition_retention(
            &mut state.retention,
            RetentionEvent::ApprovePurge { receipt_id },
        )?;
    }

    match action.as_str() {
        "hold" => transition_retention(
            &mut state.retention,
            RetentionEvent::PlaceHold {
                reason: reason.clone(),
            },
        )?,
        "release" => transition_retention(&mut state.retention, RetentionEvent::ReleaseHold)?,
        "eligible" => {
            transition_retention(&mut state.retention, RetentionEvent::MarkEligibleForPurge)?
        }
        _ => return Err("stomach_retention_unknown_action".to_string()),
    }

    write_json(
        &state_path,
        &serde_json::to_value(&state).map_err(|e| format!("stomach_state_encode_failed:{e}"))?,
    )?;
    let out = json_receipt(
        "stomach_kernel_retention",
        json!({
            "digest_id": digest_id,
            "action": action,
            "retention_state": state.retention_state(),
            "retained_until": state.retention.retained_until,
            "explicit_purge_approval_receipt": state.retention.explicit_purge_approval_receipt
        }),
    );
    append_jsonl(&state_root.join("receipts.jsonl"), &out)?;
    Ok(out)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let nexus_connection = if nexus_enabled(argv) {
        match authorize_stomach_command_with_nexus(command.as_str()) {
            Ok(meta) => Some(meta),
            Err(err) => {
                print_json_line(&json_error("stomach_kernel_nexus_error", &err));
                return 1;
            }
        }
    } else {
        Some(json!({
            "enabled": false,
            "reason": "nexus_disabled_by_flag_or_env"
        }))
    };
    let response = match command.as_str() {
        "score" => score_cycle(root, &argv[1..]),
        "run" => run_cycle(root, &argv[1..]),
        "status" => status_cycle(root, &argv[1..]),
        "rollback" => rollback_cycle(root, &argv[1..]),
        "retention" => retention_cycle(root, &argv[1..]),
        "purge" => purge_cycle(root, &argv[1..]),
        _ => Err("stomach_unknown_command".to_string()),
    };
    match response {
        Ok(mut value) => {
            if let Some(meta) = nexus_connection {
                value["nexus_connection"] = meta;
            }
            print_json_line(&value);
            0
        }
        Err(err) => {
            print_json_line(&json_error("stomach_kernel_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn stomach_run_and_status_roundtrip() {
        let root = tempdir().expect("tmp");
        let source = root.path().join("import");
        fs::create_dir_all(&source).expect("mkdir");
        fs::write(
            source.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .expect("write");
        fs::write(source.join("LICENSE"), "MIT").expect("license");
        let run_exit = run(
            root.path(),
            &[
                "run".to_string(),
                "--id=demo".to_string(),
                format!("--source-root={}", source.display()),
                "--origin=https://github.com/acme/repo".to_string(),
                "--commit=abc".to_string(),
                "--spdx=MIT".to_string(),
            ],
        );
        assert_eq!(run_exit, 0);
        let status_exit = run(
            root.path(),
            &["status".to_string(), "--id=demo".to_string()],
        );
        assert_eq!(status_exit, 0);
    }

    #[test]
    fn purge_requires_retention_eligibility_and_explicit_approval() {
        let root = tempdir().expect("tmp");
        let source = root.path().join("import");
        fs::create_dir_all(&source).expect("mkdir");
        fs::write(
            source.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .expect("write");
        fs::write(source.join("LICENSE"), "MIT").expect("license");

        let run_exit = run(
            root.path(),
            &[
                "run".to_string(),
                "--id=purge-demo".to_string(),
                format!("--source-root={}", source.display()),
                "--origin=https://github.com/acme/repo".to_string(),
                "--commit=abc".to_string(),
                "--spdx=MIT".to_string(),
            ],
        );
        assert_eq!(run_exit, 0);
        let purge_blocked = run(
            root.path(),
            &["purge".to_string(), "--id=purge-demo".to_string()],
        );
        assert_eq!(purge_blocked, 1);

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let retention_ok = run(
            root.path(),
            &[
                "retention".to_string(),
                "--id=purge-demo".to_string(),
                "--action=eligible".to_string(),
                format!("--retained-until={}", now_secs.saturating_sub(1)),
                "--approve-receipt=receipt:purge-demo:approve".to_string(),
            ],
        );
        assert_eq!(retention_ok, 0);
        let purge_ok = run(
            root.path(),
            &["purge".to_string(), "--id=purge-demo".to_string()],
        );
        assert_eq!(purge_ok, 0);
    }

    #[test]
    fn nexus_authorization_succeeds_for_stomach_route() {
        let out = authorize_stomach_command_with_nexus_inner("status", false).expect("nexus auth");
        assert_eq!(out.get("enabled").and_then(Value::as_bool), Some(true));
        assert!(out
            .get("lease_id")
            .and_then(Value::as_str)
            .map(|row| !row.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn nexus_authorization_fails_closed_when_blocked_pair_enabled() {
        let err = authorize_stomach_command_with_nexus_inner("status", true)
            .err()
            .unwrap_or_else(|| "missing_error".to_string());
        assert!(err.contains("lease_denied") || err.contains("delivery_denied"));
    }

    #[test]
    fn run_writes_mandatory_scoring_gate_ledger_and_report() {
        let root = tempdir().expect("tmp");
        let source = root.path().join("import");
        fs::create_dir_all(source.join("core")).expect("mkdir");
        fs::write(
            source.join("core").join("mod.rs"),
            "pub fn hello() -> &'static str { \"world\" }\n",
        )
        .expect("write source");
        fs::write(
            source.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .expect("write cargo");
        fs::write(source.join("LICENSE"), "MIT").expect("license");

        let run_exit = run(
            root.path(),
            &[
                "run".to_string(),
                "--id=score-demo".to_string(),
                format!("--source-root={}", source.display()),
                "--origin=https://github.com/acme/repo".to_string(),
                "--commit=abc".to_string(),
                "--spdx=MIT".to_string(),
            ],
        );
        assert_eq!(run_exit, 0);

        let ledger_path = root
            .path()
            .join("local/state/stomach/ledgers/score-demo_file_scores.json");
        assert!(ledger_path.exists(), "expected scoring ledger to exist");
        let ledger: Value =
            serde_json::from_str(&fs::read_to_string(&ledger_path).expect("read ledger"))
                .expect("decode ledger");
        assert_eq!(
            ledger
                .get("mandatory_scoring_gate")
                .and_then(Value::as_bool),
            Some(true)
        );
        let rows = ledger
            .get("rows")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!rows.is_empty(), "expected scored rows");
        for row in &rows {
            assert!(row.get("authority_risk_score").is_some());
            assert!(row.get("migration_potential_score").is_some());
            assert!(row.get("concept_opportunity_score").is_some());
            assert!(row.get("priority_score").is_some());
            assert_eq!(row.get("state").and_then(Value::as_str), Some("done"));
            let history = row
                .get("state_history")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let states = history
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();
            assert_eq!(states, vec!["queued", "in_progress", "done"]);
        }
        let report_glob_root = root.path().join("local/workspace/reports");
        assert!(report_glob_root.exists(), "expected report root to exist");
    }
}
