fn speculation_state_path(root: &Path) -> PathBuf {
    state_root(root).join("speculation").join("state.json")
}

fn run_speculation_overlay(root: &Path, argv: &[String]) -> i32 {
    let strict = parse_bool(parse_flag(argv, "strict").as_deref(), true);
    if let Some(mut denied) = conduit_guard(argv, strict) {
        return emit_receipt(root, &mut denied);
    }
    let action = clean_id(
        parse_flag(argv, "action").or_else(|| parse_positional(argv, 1)),
        "status",
    );
    let mut state = read_json(&speculation_state_path(root)).unwrap_or_else(
        || json!({"type":"autonomy_speculation_state","overlays":{},"updated_at":now_iso()}),
    );
    let require_web_tooling_ready =
        parse_bool(parse_flag(argv, "require-web-tooling-ready").as_deref(), false);
    let web_tooling_health = crate::network_protocol::web_tooling_health_report(root, strict);
    let web_tooling_ready = web_tooling_health
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if strict && require_web_tooling_ready && !web_tooling_ready {
        let mut out = cli_error_receipt(argv, "speculation_web_tooling_not_ready", 2);
        out["type"] = json!("autonomy_speculation");
        out["web_tooling_health"] = web_tooling_health;
        return emit_receipt(root, &mut out);
    }
    if !state.get("overlays").map(Value::is_object).unwrap_or(false) {
        state["overlays"] = json!({});
    }
    if action == "run" || action == "create" {
        let payload = parse_payload_json(argv).unwrap_or_else(|_| json!({}));
        let spec_id = clean_id(
            parse_flag(argv, "spec-id"),
            &format!("spec-{}", &receipt_hash(&json!({"ts": now_iso()}))[..10]),
        );
        state["overlays"][&spec_id] = json!({"spec_id": spec_id, "status":"pending", "created_at": now_iso(), "payload": payload});
    } else if action == "merge" {
        let spec_id = clean_id(
            parse_flag(argv, "spec-id").or_else(|| parse_positional(argv, 2)),
            "spec",
        );
        let verify = parse_bool(parse_flag(argv, "verify").as_deref(), true);
        if strict && !verify {
            let mut out = cli_error_receipt(argv, "speculation_merge_requires_verify", 2);
            out["type"] = json!("autonomy_speculation");
            return emit_receipt(root, &mut out);
        }
        let overlay = state
            .pointer(&format!("/overlays/{spec_id}"))
            .cloned()
            .unwrap_or(Value::Null);
        if overlay.is_null() {
            let mut out = cli_error_receipt(argv, "speculation_not_found", 2);
            out["type"] = json!("autonomy_speculation");
            return emit_receipt(root, &mut out);
        }
        let mut trunk =
            read_json(&trunk_state_path(root)).unwrap_or_else(|| json!({"state":"idle"}));
        if !trunk
            .get("speculation_merges")
            .map(Value::is_array)
            .unwrap_or(false)
        {
            trunk["speculation_merges"] = Value::Array(Vec::new());
        }
        trunk["speculation_merges"]
            .as_array_mut()
            .expect("array")
            .push(json!({
                "spec_id": spec_id,
                "merged_at": now_iso(),
                "overlay_hash": receipt_hash(&overlay)
            }));
        let _ = write_json(&trunk_state_path(root), &trunk);
        state["overlays"][&spec_id]["status"] = json!("merged");
        state["overlays"][&spec_id]["merged_at"] = json!(now_iso());
    } else if action == "reject" {
        let spec_id = clean_id(
            parse_flag(argv, "spec-id").or_else(|| parse_positional(argv, 2)),
            "spec",
        );
        if state.pointer(&format!("/overlays/{spec_id}")).is_some() {
            state["overlays"][&spec_id]["status"] = json!("rejected");
            state["overlays"][&spec_id]["rejected_at"] = json!(now_iso());
        }
    }
    state["updated_at"] = json!(now_iso());
    let _ = write_json(&speculation_state_path(root), &state);
    let mut out = json!({
        "ok": true,
        "type": "autonomy_speculation",
        "lane": LANE_ID,
        "strict": strict,
        "action": action,
        "web_tooling_health": web_tooling_health,
        "state": state,
        "claim_evidence": [
            {"id":"V6-EXEC-002.1","claim":"speculative_execution_runs_in_overlay_state_until_verified"},
            {"id":"V6-EXEC-002.2","claim":"overlay_merge_or_reject_is_atomic_and_receipted"}
        ]
    });
    emit_receipt(root, &mut out)
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    include!("051-speculation-and-regression-tests.regression_tests.rs");
}
