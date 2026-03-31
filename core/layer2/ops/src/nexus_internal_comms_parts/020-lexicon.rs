fn core_lexicon_entries() -> Vec<(&'static str, &'static str)> {
    vec![
        ("BP", "backpressure"),
        ("Q", "queue_depth"),
        ("SB", "stale_blocks"),
        ("LAT", "latency_ms"),
        ("MEM", "memory_mb"),
        ("FRE", "freed_mb"),
        ("SIG", "conduit_signal"),
        ("OK", "success"),
        ("ERR", "error"),
        ("H", "high"),
        ("M", "medium"),
        ("L", "low"),
        ("URG", "urgent"),
        ("PEND", "pending"),
        ("DONE", "completed"),
        ("FAIL", "failed"),
        ("AG", "agent"),
        ("SW", "swarm"),
        ("COORD", "coordinator"),
        ("DS", "dream_sequencer"),
        ("RT", "red_team"),
        ("BT", "blue_team"),
        ("PT", "purple_team"),
        ("WT", "white_team"),
        ("GT", "green_team"),
        ("YT", "yellow_team"),
        ("CON", "conduit"),
        ("RC", "receipt"),
        ("SCH", "scheduler"),
        ("SP", "safety_plane"),
        ("L0", "layer0"),
        ("L1", "layer1"),
        ("L2", "layer2"),
        ("L3", "layer3"),
        ("L4", "layer4"),
        ("SPAWN", "spawn"),
        ("TERM", "terminate"),
        ("SCALE", "scale"),
        ("CMP", "compact"),
        ("SWP", "sweep"),
        ("ROUT", "route"),
        ("ENF", "enforce"),
        ("LOG", "log"),
        ("ALERT", "alert"),
        ("RECOV", "recover"),
    ]
}

fn module_catalog() -> Vec<&'static str> {
    vec![
        "memory",
        "swarm",
        "conduit",
        "receipts",
        "inference",
        "scheduler",
        "safety",
        "red_team",
        "blue_team",
        "purple_team",
        "white_team",
        "green_team",
        "yellow_team",
        "observability",
        "error",
        "logging",
        "configuration",
        "backup",
        "security",
        "compliance",
        "auditing",
        "incident_response",
        "disaster_recovery",
        "high_availability",
        "performance",
        "cost",
        "scalability",
        "resilience",
        "alerting",
        "triage",
        "forensics",
        "threat_hunting",
        "penetration_testing",
        "social_engineering",
        "physical_security",
    ]
}

fn module_entries(module: &str) -> Vec<(&'static str, &'static str)> {
    match module {
        "memory" => vec![("MCTX", "memory_context"), ("MCP", "memory_compaction"), ("RECALL", "recall")],
        "swarm" => vec![("ORCH", "orchestrator"), ("WRKR", "worker"), ("VRFY", "verifier")],
        "conduit" => vec![("CNRT", "conduit_route"), ("CNEN", "conduit_enforce"), ("CNFB", "conduit_fail_closed")],
        "receipts" => vec![("RCPT", "receipt_record"), ("MERK", "merkle"), ("PROV", "provenance")],
        "inference" => vec![("MDL", "model"), ("FALL", "fallback"), ("COST", "cost")],
        "scheduler" => vec![("PRIO", "priority"), ("TTL", "time_limit"), ("DEAD", "deadline")],
        "safety" => vec![("JLOCK", "judicial_lock"), ("INV", "invariant"), ("HALT", "halt")],
        "red_team" => vec![("ATTK", "attack"), ("PROBE", "probe"), ("BYP", "bypass")],
        "blue_team" => vec![("DEF", "defense"), ("PATCH", "patch"), ("GUARD", "guard")],
        "purple_team" => vec![("SYNC", "collaborate"), ("MIX", "hybrid_playbook"), ("PAIR", "paired_ops")],
        "white_team" => vec![("GOV", "governance"), ("POL", "policy"), ("AUD", "audit")],
        "green_team" => vec![("ECO", "energy"), ("EFF", "efficiency"), ("PRN", "prune")],
        "yellow_team" => vec![("RISK", "risk"), ("HUNT", "hunt"), ("WARN", "warning")],
        "observability" => vec![("MET", "metrics"), ("TRACE", "trace"), ("TELEM", "telemetry")],
        "error" => vec![("EXC", "exception"), ("MIT", "mitigation"), ("RTRY", "retry")],
        "logging" => vec![("LSTR", "structured_log"), ("LIDX", "log_index"), ("LQRY", "log_query")],
        "configuration" => vec![("CFG", "config"), ("LOAD", "load"), ("VALID", "validate")],
        "backup" => vec![("SNAP", "snapshot"), ("REST", "restore"), ("VERS", "version")],
        "security" => vec![("AUTH", "auth"), ("ACL", "access_control"), ("HARD", "hardening")],
        "compliance" => vec![("CTRL", "control"), ("REG", "regulatory"), ("ATST", "attestation")],
        "auditing" => vec![("ARVW", "audit_review"), ("AVRF", "audit_verify"), ("AFORE", "audit_forensics")],
        "incident_response" => vec![("DET", "detect"), ("CONT", "contain"), ("REM", "remediate")],
        "disaster_recovery" => vec![("FAILOV", "failover"), ("RTO", "recovery_time_objective"), ("DRIL", "drill")],
        "high_availability" => vec![("RED", "redundancy"), ("LBAL", "load_balance"), ("QUOR", "quorum")],
        "performance" => vec![("PROF", "profile"), ("TUNE", "tune"), ("HOT", "hot_path")],
        "cost" => vec![("BUD", "budget"), ("SPND", "spend"), ("ROI", "roi")],
        "scalability" => vec![("HSCL", "horizontal_scale"), ("VSCL", "vertical_scale"), ("ELAS", "elasticity")],
        "resilience" => vec![("FTOL", "fault_tolerance"), ("SELF", "self_heal"), ("GRAC", "graceful_degrade")],
        "alerting" => vec![("PG", "page"), ("ESC", "escalate"), ("ACK", "ack")],
        "triage" => vec![("TRI", "triage"), ("RTE", "route"), ("CLS", "classify")],
        "forensics" => vec![("EVID", "evidence"), ("CHAIN", "chain_of_custody"), ("RCA", "root_cause")],
        "threat_hunting" => vec![("HYP", "hypothesis"), ("IOC", "indicator"), ("SWEEP", "sweep")],
        "penetration_testing" => vec![("PTEST", "pentest"), ("SIM", "simulate_attack"), ("FIND", "finding")],
        "social_engineering" => vec![("SESIM", "social_simulation"), ("HFA", "human_factor"), ("TRAIN", "awareness")],
        "physical_security" => vec![("FAC", "facility"), ("HW", "hardware"), ("ACCESS", "physical_access")],
        _ => Vec::new(),
    }
}

fn parse_modules(argv: &[String]) -> Result<Vec<String>, String> {
    let raw = parse_flag(argv, "modules")
        .or_else(|| parse_flag(argv, "module"))
        .unwrap_or_default();
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let modules = raw
        .split(',')
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if modules.len() > MAX_MODULES_PER_AGENT {
        return Err(format!(
            "module_limit_exceeded:max={MAX_MODULES_PER_AGENT}:got={}",
            modules.len()
        ));
    }
    let known = module_catalog().into_iter().collect::<BTreeSet<_>>();
    for module in &modules {
        if !known.contains(module.as_str()) {
            return Err(format!("unknown_module:{module}"));
        }
    }
    Ok(modules)
}

fn active_lexicon(modules: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::<String, String>::new();
    for (code, phrase) in core_lexicon_entries() {
        out.insert(code.to_string(), phrase.to_string());
    }
    let core_codes = out.keys().cloned().collect::<BTreeSet<_>>();
    for module in modules {
        for (code, phrase) in module_entries(module) {
            if core_codes.contains(code) {
                return Err(format!("module_redefines_core_symbol:{module}:{code}"));
            }
            if out.contains_key(code) {
                return Err(format!("module_symbol_collision:{module}:{code}"));
            }
            out.insert(code.to_string(), phrase.to_string());
        }
    }
    Ok(out)
}

fn reverse_lexicon(lexicon: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::<String, String>::new();
    for (code, phrase) in lexicon {
        out.insert(normalize_text_atom(phrase), code.clone());
    }
    out
}
