#[derive(Clone, Copy)]
struct ModuleSpec {
    name: &'static str,
    entries: &'static [(&'static str, &'static str)],
    task_keywords: &'static [&'static str],
    role_keywords: &'static [&'static str],
}

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

fn module_specs() -> Vec<ModuleSpec> {
    vec![
        ModuleSpec {
            name: "memory",
            entries: &[
                ("MCTX", "memory_context"),
                ("MCP", "memory_compaction"),
                ("RECALL", "recall"),
            ],
            task_keywords: &[
                "memory",
                "context",
                "history",
                "recall",
                "compaction",
                "summarize",
            ],
            role_keywords: &["memory", "dream", "librarian"],
        },
        ModuleSpec {
            name: "swarm",
            entries: &[
                ("ORCH", "orchestrator"),
                ("WRKR", "worker"),
                ("VRFY", "verifier"),
            ],
            task_keywords: &["swarm", "agents", "parallel", "orchestrate", "delegate"],
            role_keywords: &["coord", "orchestrator", "manager"],
        },
        ModuleSpec {
            name: "conduit",
            entries: &[
                ("CNRT", "conduit_route"),
                ("CNEN", "conduit_enforce"),
                ("CNFB", "conduit_fail_closed"),
            ],
            task_keywords: &["conduit", "route", "bridge", "pass", "policy"],
            role_keywords: &["router", "conduit"],
        },
        ModuleSpec {
            name: "receipts",
            entries: &[
                ("RCPT", "receipt_record"),
                ("MERK", "merkle"),
                ("PROV", "provenance"),
            ],
            task_keywords: &["receipt", "audit", "proof", "provenance", "hash"],
            role_keywords: &["auditor", "receipt", "verity"],
        },
        ModuleSpec {
            name: "inference",
            entries: &[("MDL", "model"), ("FALL", "fallback"), ("COST", "cost")],
            task_keywords: &["llm", "model", "inference", "routing", "provider", "tokens"],
            role_keywords: &["inference", "model-router"],
        },
        ModuleSpec {
            name: "scheduler",
            entries: &[
                ("PRIO", "priority"),
                ("TTL", "time_limit"),
                ("DEAD", "deadline"),
            ],
            task_keywords: &["schedule", "queue", "deadline", "priority", "cron"],
            role_keywords: &["scheduler", "planner"],
        },
        ModuleSpec {
            name: "safety",
            entries: &[
                ("JLOCK", "judicial_lock"),
                ("INV", "invariant"),
                ("HALT", "halt"),
            ],
            task_keywords: &["safety", "lock", "invariant", "fail-closed", "guardrail"],
            role_keywords: &["safety", "guardian"],
        },
        ModuleSpec {
            name: "red_team",
            entries: &[("ATTK", "attack"), ("PROBE", "probe"), ("BYP", "bypass")],
            task_keywords: &["attack", "adversarial", "break", "probe", "bypass"],
            role_keywords: &["red", "offense"],
        },
        ModuleSpec {
            name: "blue_team",
            entries: &[("DEF", "defense"), ("PATCH", "patch"), ("GUARD", "guard")],
            task_keywords: &["defend", "harden", "patch", "mitigate", "guard"],
            role_keywords: &["blue", "defense"],
        },
        ModuleSpec {
            name: "purple_team",
            entries: &[
                ("SYNC", "collaborate"),
                ("MIX", "hybrid_playbook"),
                ("PAIR", "paired_ops"),
            ],
            task_keywords: &[
                "purple",
                "collaborative security",
                "joint exercise",
                "tabletop",
            ],
            role_keywords: &["purple"],
        },
        ModuleSpec {
            name: "white_team",
            entries: &[("GOV", "governance"), ("POL", "policy"), ("AUD", "audit")],
            task_keywords: &["governance", "policy", "oversight", "board", "compliance"],
            role_keywords: &["white", "governance"],
        },
        ModuleSpec {
            name: "green_team",
            entries: &[("ECO", "energy"), ("EFF", "efficiency"), ("PRN", "prune")],
            task_keywords: &[
                "efficiency",
                "power",
                "energy",
                "sustainability",
                "optimize",
            ],
            role_keywords: &["green"],
        },
        ModuleSpec {
            name: "yellow_team",
            entries: &[("RISK", "risk"), ("HUNT", "hunt"), ("WARN", "warning")],
            task_keywords: &["risk", "hunting", "warning", "threat", "exposure"],
            role_keywords: &["yellow", "hunter"],
        },
        ModuleSpec {
            name: "observability",
            entries: &[
                ("MET", "metrics"),
                ("TRACE", "trace"),
                ("TELEM", "telemetry"),
            ],
            task_keywords: &["telemetry", "metrics", "trace", "monitor", "observability"],
            role_keywords: &["sre", "observer", "ops"],
        },
        ModuleSpec {
            name: "error",
            entries: &[
                ("EXC", "exception"),
                ("MIT", "mitigation"),
                ("RTRY", "retry"),
            ],
            task_keywords: &["error", "exception", "retry", "mitigation", "failure"],
            role_keywords: &["reliability"],
        },
        ModuleSpec {
            name: "logging",
            entries: &[
                ("LSTR", "structured_log"),
                ("LIDX", "log_index"),
                ("LQRY", "log_query"),
            ],
            task_keywords: &["log", "logging", "query logs", "index logs", "journal"],
            role_keywords: &["logger"],
        },
        ModuleSpec {
            name: "configuration",
            entries: &[("CFG", "config"), ("LOAD", "load"), ("VALID", "validate")],
            task_keywords: &["config", "configure", "settings", "validate config"],
            role_keywords: &["config"],
        },
        ModuleSpec {
            name: "backup",
            entries: &[
                ("SNAP", "snapshot"),
                ("REST", "restore"),
                ("VERS", "version"),
            ],
            task_keywords: &["backup", "snapshot", "restore", "version rollback"],
            role_keywords: &["backup", "recovery"],
        },
        ModuleSpec {
            name: "security",
            entries: &[
                ("AUTH", "auth"),
                ("ACL", "access_control"),
                ("HARD", "hardening"),
            ],
            task_keywords: &["security", "auth", "access", "hardening", "secrets"],
            role_keywords: &["security", "secops"],
        },
        ModuleSpec {
            name: "compliance",
            entries: &[
                ("CTRL", "control"),
                ("REG", "regulatory"),
                ("ATST", "attestation"),
            ],
            task_keywords: &["compliance", "regulatory", "soc2", "iso", "control"],
            role_keywords: &["compliance"],
        },
        ModuleSpec {
            name: "auditing",
            entries: &[
                ("ARVW", "audit_review"),
                ("AVRF", "audit_verify"),
                ("AFORE", "audit_forensics"),
            ],
            task_keywords: &["audit", "verify", "review", "forensics"],
            role_keywords: &["auditor"],
        },
        ModuleSpec {
            name: "incident_response",
            entries: &[("DET", "detect"), ("CONT", "contain"), ("REM", "remediate")],
            task_keywords: &["incident", "containment", "remediation", "outage", "breach"],
            role_keywords: &["incident", "ir"],
        },
        ModuleSpec {
            name: "disaster_recovery",
            entries: &[
                ("FAILOV", "failover"),
                ("RTO", "recovery_time_objective"),
                ("DRIL", "drill"),
            ],
            task_keywords: &["disaster", "failover", "rto", "business continuity"],
            role_keywords: &["dr", "recovery"],
        },
        ModuleSpec {
            name: "high_availability",
            entries: &[
                ("RED", "redundancy"),
                ("LBAL", "load_balance"),
                ("QUOR", "quorum"),
            ],
            task_keywords: &["ha", "availability", "redundancy", "quorum", "load balance"],
            role_keywords: &["ha", "sre"],
        },
        ModuleSpec {
            name: "performance",
            entries: &[("PROF", "profile"), ("TUNE", "tune"), ("HOT", "hot_path")],
            task_keywords: &[
                "performance",
                "profile",
                "optimize",
                "latency",
                "throughput",
            ],
            role_keywords: &["performance"],
        },
        ModuleSpec {
            name: "cost",
            entries: &[("BUD", "budget"), ("SPND", "spend"), ("ROI", "roi")],
            task_keywords: &["cost", "budget", "spend", "roi", "savings"],
            role_keywords: &["finance", "cost"],
        },
        ModuleSpec {
            name: "scalability",
            entries: &[
                ("HSCL", "horizontal_scale"),
                ("VSCL", "vertical_scale"),
                ("ELAS", "elasticity"),
            ],
            task_keywords: &["scale", "scaling", "elastic", "capacity", "grow"],
            role_keywords: &["scaling", "capacity"],
        },
        ModuleSpec {
            name: "resilience",
            entries: &[
                ("FTOL", "fault_tolerance"),
                ("SELF", "self_heal"),
                ("GRAC", "graceful_degrade"),
            ],
            task_keywords: &["resilience", "fault", "self-heal", "degrade gracefully"],
            role_keywords: &["resilience", "reliability"],
        },
        ModuleSpec {
            name: "alerting",
            entries: &[("PG", "page"), ("ESC", "escalate"), ("ACK", "ack")],
            task_keywords: &["alert", "page", "escalate", "acknowledge"],
            role_keywords: &["oncall"],
        },
        ModuleSpec {
            name: "triage",
            entries: &[("TRI", "triage"), ("RTE", "route"), ("CLS", "classify")],
            task_keywords: &["triage", "classify", "route", "priority queue"],
            role_keywords: &["triage"],
        },
        ModuleSpec {
            name: "forensics",
            entries: &[
                ("EVID", "evidence"),
                ("CHAIN", "chain_of_custody"),
                ("RCA", "root_cause"),
            ],
            task_keywords: &["forensics", "evidence", "chain of custody", "rca"],
            role_keywords: &["forensics"],
        },
        ModuleSpec {
            name: "threat_hunting",
            entries: &[
                ("HYP", "hypothesis"),
                ("IOC", "indicator"),
                ("SWEEP", "sweep"),
            ],
            task_keywords: &["threat hunt", "ioc", "hypothesis", "sweep"],
            role_keywords: &["hunter", "threat"],
        },
        ModuleSpec {
            name: "penetration_testing",
            entries: &[
                ("PTEST", "pentest"),
                ("SIM", "simulate_attack"),
                ("FIND", "finding"),
            ],
            task_keywords: &["pentest", "penetration", "simulate attack", "finding"],
            role_keywords: &["pentest", "red"],
        },
        ModuleSpec {
            name: "social_engineering",
            entries: &[
                ("SESIM", "social_simulation"),
                ("HFA", "human_factor"),
                ("TRAIN", "awareness"),
            ],
            task_keywords: &[
                "social engineering",
                "phishing",
                "human factor",
                "awareness",
            ],
            role_keywords: &["social"],
        },
        ModuleSpec {
            name: "physical_security",
            entries: &[
                ("FAC", "facility"),
                ("HW", "hardware"),
                ("ACCESS", "physical_access"),
            ],
            task_keywords: &[
                "physical security",
                "facility",
                "badge access",
                "hardware theft",
            ],
            role_keywords: &["physical"],
        },
    ]
}

fn module_catalog() -> Vec<&'static str> {
    module_specs().into_iter().map(|spec| spec.name).collect()
}

fn module_entries(module: &str) -> Vec<(&'static str, &'static str)> {
    let normalized = normalize_module_name(module);
    module_specs()
        .into_iter()
        .find(|spec| spec.name == normalized.as_str())
        .map(|spec| spec.entries.to_vec())
        .unwrap_or_default()
}

fn module_catalog_manifest() -> Value {
    Value::Array(
        module_specs()
            .into_iter()
            .map(|spec| {
                json!({
                    "name": spec.name,
                    "symbol_count": spec.entries.len(),
                    "entries": spec.entries
                        .iter()
                        .map(|(code, phrase)| json!({"code": code, "phrase": phrase}))
                        .collect::<Vec<_>>(),
                    "task_keywords": spec.task_keywords,
                    "role_keywords": spec.role_keywords,
                })
            })
            .collect::<Vec<_>>(),
    )
}

fn normalize_module_name(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn parse_modules(argv: &[String]) -> Result<Vec<String>, String> {
    let raw = parse_flag(argv, "modules")
        .or_else(|| parse_flag(argv, "module"))
        .unwrap_or_default();
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let known = module_catalog().into_iter().collect::<BTreeSet<_>>();
    let mut modules = Vec::<String>::new();
    let mut seen = BTreeSet::<String>::new();
    for raw_item in raw.split(',') {
        let module = normalize_module_name(raw_item);
        if module.is_empty() {
            continue;
        }
        if !known.contains(module.as_str()) {
            return Err(format!("unknown_module:{module}"));
        }
        if seen.insert(module.clone()) {
            modules.push(module);
        }
    }
    if modules.len() > MAX_MODULES_PER_AGENT {
        return Err(format!(
            "module_limit_exceeded:max={MAX_MODULES_PER_AGENT}:got={}",
            modules.len()
        ));
    }
    Ok(modules)
}

fn module_context_scores(
    task: Option<&str>,
    role: Option<&str>,
    extra_text: Option<&str>,
) -> Vec<(String, u64)> {
    let task_norm = task.map(normalize_text_atom).unwrap_or_default();
    let role_norm = role.map(normalize_text_atom).unwrap_or_default();
    let extra_norm = extra_text.map(normalize_text_atom).unwrap_or_default();
    if task_norm.is_empty() && role_norm.is_empty() && extra_norm.is_empty() {
        return Vec::new();
    }
    let mut scored = Vec::<(String, u64)>::new();
    for spec in module_specs() {
        let mut score = 0u64;
        for raw_kw in spec.task_keywords {
            let kw = normalize_text_atom(raw_kw);
            if kw.is_empty() {
                continue;
            }
            if task_norm.contains(kw.as_str()) {
                score += 3;
            }
            if extra_norm.contains(kw.as_str()) {
                score += 1;
            }
        }
        for raw_kw in spec.role_keywords {
            let kw = normalize_text_atom(raw_kw);
            if !kw.is_empty() && role_norm.contains(kw.as_str()) {
                score += 4;
            }
        }
        if !task_norm.is_empty() && task_norm.contains(spec.name) {
            score += 2;
        }
        if !role_norm.is_empty() && role_norm.contains(spec.name) {
            score += 2;
        }
        if score > 0 {
            scored.push((spec.name.to_string(), score));
        }
    }
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    scored
}

fn infer_modules_for_task(
    task: Option<&str>,
    role: Option<&str>,
    extra_text: Option<&str>,
) -> Vec<String> {
    module_context_scores(task, role, extra_text)
        .into_iter()
        .map(|(module, _)| module)
        .take(MAX_MODULES_PER_AGENT)
        .collect()
}

fn resolve_modules_for_context(
    argv: &[String],
    seeded_modules: &[String],
    task: Option<&str>,
    role: Option<&str>,
    extra_text: Option<&str>,
) -> Result<Vec<String>, String> {
    let mut modules = parse_modules(argv)?;
    let known = module_catalog().into_iter().collect::<BTreeSet<_>>();
    let mut seen = modules.iter().cloned().collect::<BTreeSet<_>>();

    for raw_seed in seeded_modules {
        let seeded = normalize_module_name(raw_seed);
        if seeded.is_empty() {
            continue;
        }
        if !known.contains(seeded.as_str()) {
            return Err(format!("unknown_module:{seeded}"));
        }
        if seen.contains(&seeded) {
            continue;
        }
        if modules.len() >= MAX_MODULES_PER_AGENT {
            return Err(format!(
                "module_limit_exceeded:max={MAX_MODULES_PER_AGENT}:got={}",
                modules.len() + 1
            ));
        }
        seen.insert(seeded.clone());
        modules.push(seeded);
    }

    if modules.is_empty() {
        for inferred in infer_modules_for_task(task, role, extra_text) {
            if modules.len() >= MAX_MODULES_PER_AGENT {
                break;
            }
            if seen.insert(inferred.clone()) {
                modules.push(inferred);
            }
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
