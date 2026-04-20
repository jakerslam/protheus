
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops llm-economy-organ run [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ enable <all|virtuals|bankrbot|nookplot|owocki|heurist|daydreams|fairscale|trade_router> [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ upgrade-trading-hand [--mode=analysis|paper|live] [--symbol=<pair>] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ debate-bullbear [--symbol=<pair>] [--bull-score=<0..1>] [--bear-score=<0..1>] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ alpaca-execute [--mode=analysis|paper|live] [--symbol=<pair>] [--side=buy|sell] [--qty=<n>] [--apply=1|0]");
        println!(
            "  protheus-ops llm-economy-organ virtuals-acp [--action=build|earn] [--apply=1|0]"
        );
        println!(
            "  protheus-ops llm-economy-organ bankrbot-defi [--strategy=<name>] [--apply=1|0]"
        );
        println!("  protheus-ops llm-economy-organ jobs-marketplace [--source=nookplot|owocki] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ skills-marketplace [--source=heurist|daydreams] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ fairscale-credit [--delta=<n>] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ mining-hand [--network=litcoin|minbot] [--hours=<n>] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ trade-router [--chain=solana] [--symbol=<pair>] [--side=buy|sell] [--qty=<n>] [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ model-support-refresh [--apply=1|0]");
        println!("  protheus-ops llm-economy-organ dashboard");
        println!("  protheus-ops llm-economy-organ status");
        return 0;
    }

    let latest = latest_path(root);
    let history = history_path(root);
    let contract = load_contract(root);
    let strict = parse_bool(parsed.flags.get("strict"), true);

    if !matches!(command.as_str(), "status" | "dashboard") {
        let conduit = conduit_enforcement(&parsed, strict, command.as_str());
        if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            let mut out = json!({
                "ok": false,
                "strict": strict,
                "type": "llm_economy_organ_conduit_gate",
                "lane": "core/layer0/ops",
                "command": command,
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            });
            persist_and_emit_with_receipt(&latest, &history, &mut out);
            return 1;
        }
    }

    if command == "status" {
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_organ_status",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "latest": read_json(&latest),
            "contract": {
                "path": ECONOMY_CONTRACT_PATH,
                "sha256": deterministic_receipt_hash(&contract)
            }
        });
        emit_with_receipt(&mut out);
        return 0;
    }

    if command == "dashboard" {
        let latest_payload = read_json(&latest);
        let enabled_map = current_enabled_map(latest_payload.as_ref());
        let enabled_count = enabled_map
            .values()
            .filter(|v| v.as_bool().unwrap_or(false))
            .count();
        let mut out = json!({
            "ok": true,
            "type": "llm_economy_organ_dashboard",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "enabled_count": enabled_count,
            "total_hands": ECONOMY_HANDS.len(),
            "enabled_hands": enabled_map,
            "contract": {
                "path": ECONOMY_CONTRACT_PATH,
                "sha256": deterministic_receipt_hash(&contract)
            },
            "claim_evidence": [
                {
                    "id": "V6-ECONOMY-001.8",
                    "claim": "economy_activation_and_dashboard_surface_enabled_hands_with_deterministic_receipts",
                    "evidence": {
                        "enabled_count": enabled_count,
                        "total_hands": ECONOMY_HANDS.len()
                    }
                }
            ]
        });
        persist_and_emit_with_receipt(&latest, &history, &mut out);
        return 0;
    }

