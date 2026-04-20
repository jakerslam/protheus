
fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("jsonl_parent_create_failed:{}:{err}", parent.display()))?;
    }
    let mut encoded = serde_json::to_string(value)
        .map_err(|err| format!("jsonl_encode_failed:{}", clean(err, 180)))?;
    encoded.push('\n');
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, encoded.as_bytes()))
        .map_err(|err| format!("jsonl_append_failed:{}:{err}", path.display()))
}

pub(super) fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops network-protocol status");
        println!("  protheus-ops network-protocol dashboard");
        println!("  protheus-ops network-protocol web-tooling-status [--provider=<id>] [--fetch-provider=<id>] [--mode=runtime|setup] [--cache=warm|cold] [--activate=1|0] [--strict=1|0]");
        println!("  protheus-ops network-protocol ignite-bitcoin [--seed=<text>] [--apply=1|0]");
        println!("  protheus-ops network-protocol stake [--action=stake|reward|slash] [--agent=<id>] [--amount=<n>] [--reason=<text>]");
        println!("  protheus-ops network-protocol oracle-query [--provider=polymarket] [--event=<text>] [--strict=1|0]");
        println!("  protheus-ops network-protocol truth-weight [--market=<id>] [--strict=1|0]");
        println!("  protheus-ops network-protocol contribution [--agent=<id>] [--contribution-type=<compute|memory|rl|breakthrough>] [--score=<0..1>] [--stake=<n>] [--reward=<n>] [--slash=<n>] [--strict=1|0]");
        println!("  protheus-ops network-protocol consensus [--op=<append|verify|status>] [--receipt-hash=<hex>] [--causality-hash=<hex>] [--strict=1|0]");
        println!("  protheus-ops network-protocol rsi-boundary [--stage=<sandbox|growth|expansion|mature>] [--action=<simulate|promote|merge>] [--oversight-approval=1|0] [--strict=1|0]");
        println!("  protheus-ops network-protocol join-hyperspace [--node=<id>] [--admission-token=<token>] [--stake=<n>] [--strict=1|0]");
        println!("  protheus-ops network-protocol governance-vote [--proposal=<id>] [--voter=<id>] [--vote=<approve|reject>] [--strict=1|0]");
        println!("  protheus-ops network-protocol merkle-root [--account=<id>] [--proof=1|0]");
        println!("  protheus-ops network-protocol emission [--height=<n>] [--halving-interval=<n>] [--initial-issuance=<n>]");
        println!("  protheus-ops network-protocol zk-claim [--claim-id=<id>] [--commitment=<hex>] [--challenge=<hex>] [--public-input=<text>] [--strict=1|0]");
        return 0;
    }

    if command == "web-tooling-status" {
        let strict = parse_bool(parsed.flags.get("strict"), false);
        let activate = parse_bool(parsed.flags.get("activate"), true);
        let mode = normalize_web_provider_token(
            parsed.flags.get("mode").map(String::as_str),
            "runtime",
        );
        let cache_state = normalize_web_provider_token(
            parsed.flags.get("cache").map(String::as_str),
            "warm",
        );

        let existing_runtime =
            read_json(&web_tooling_runtime_path(root)).unwrap_or_else(|| json!({}));
        let env_search_provider = std::env::var("WEB_SEARCH_PROVIDER").ok();
        let env_fetch_provider = std::env::var("WEB_FETCH_PROVIDER").ok();
        let search_provider = normalize_web_provider_token(
            parsed
                .flags
                .get("provider")
                .map(String::as_str)
                .or_else(|| parsed.flags.get("search-provider").map(String::as_str))
                .or_else(|| existing_runtime.get("search_provider").and_then(Value::as_str))
                .or(env_search_provider.as_deref()),
            "auto",
        );
        let fetch_provider = normalize_web_provider_token(
            parsed
                .flags
                .get("fetch-provider")
                .map(String::as_str)
                .or_else(|| existing_runtime.get("fetch_provider").and_then(Value::as_str))
                .or(env_fetch_provider.as_deref()),
            "auto",
        );

        let auth = detect_web_tooling_auth_presence();
        let search_auth_present = auth
            .get("search")
            .and_then(|row| row.get("present"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let fetch_auth_present = auth
            .get("fetch")
            .and_then(|row| row.get("present"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let auth_ready = search_auth_present || fetch_auth_present;

        let runtime = json!({
            "type": "network_web_tooling_runtime",
            "search_provider": search_provider,
            "fetch_provider": fetch_provider,
            "mode": if mode == "setup" { "setup" } else { "runtime" },
            "cache_state": if cache_state == "cold" { "cold" } else { "warm" },
            "auth": auth.clone(),
            "updated_at": now_iso()
        });
        if activate {
            let _ = write_json(&web_tooling_runtime_path(root), &runtime);
        }

        let auth_error = strict && !auth_ready;
        return emit(
            root,
            json!({
                "ok": !auth_error,
                "type": "network_protocol_web_tooling_status",
                "lane": "core/layer0/ops",
                "strict": strict,
                "activate": activate,
                "runtime": runtime,
                "runtime_path": web_tooling_runtime_path(root).display().to_string(),
                "errors": if auth_error {
                    Value::Array(vec![Value::String("network_web_tool_auth_missing".to_string())])
                } else {
                    Value::Array(Vec::new())
                },
                "claim_evidence": [
                    {
                        "id": "V8-NETWORK-004.1",
                        "claim": "network_protocol_exposes_receipted_web_tooling_auth_and_provider_runtime_state",
                        "evidence": {
                            "search_provider": search_provider,
                            "fetch_provider": fetch_provider,
                            "auth_ready": auth_ready
                        }
                    }
                ]
            }),
        );
    }

    if command == "status" {
        let ledger = load_ledger(root);
        let membership = read_json(&membership_path(root)).unwrap_or(Value::Null);
        let consensus_events = read_jsonl(&consensus_ledger_path(root));
        let votes = read_jsonl(&governance_votes_path(root));
        let oracle_latest =
            read_json(&state_root(root).join("oracle").join("latest.json")).unwrap_or(Value::Null);
        return emit(
            root,
            json!({
                "ok": true,
                "type": "network_protocol_status",
                "lane": "core/layer0/ops",
                "ledger": ledger,
                "latest": read_json(&latest_path(root)),
                "membership": membership,
                "consensus_events": consensus_events.len(),
                "governance_votes": votes.len(),
                "oracle_latest": oracle_latest,
                "web_tooling_runtime": read_json(&web_tooling_runtime_path(root)).unwrap_or(Value::Null),
                "web_tooling_auth": detect_web_tooling_auth_presence()
            }),
        );
    }

