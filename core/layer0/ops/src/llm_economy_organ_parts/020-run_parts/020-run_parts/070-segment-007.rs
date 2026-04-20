    let apply = parse_bool(parsed.flags.get("apply"), false);
    let mut out = json!({
        "ok": true,
        "type": "llm_economy_organ_run",
        "lane": "core/layer0/ops",
        "ts": now_iso(),
        "apply": apply,
        "model_routing": {
            "budget_band": if apply { "applied" } else { "dry_run" },
            "providers_ranked": [],
            "note": "core_authoritative_placeholder"
        },
        "receipts": {
            "strategy": clean(parsed.flags.get("strategy").cloned().unwrap_or_default(), 120),
            "capital": clean(parsed.flags.get("capital").cloned().unwrap_or_default(), 120)
        }
    });
    persist_and_emit_with_receipt(&latest, &history, &mut out);
    0
}
