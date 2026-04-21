
fn parse_args(argv: &[String]) -> (String, bool) {
    let mut mode = "list".to_string();
    let mut json_out = false;
    let mut i = 0usize;
    while i < argv.len() {
        let token = argv[i].trim();
        if let Some(raw) = token.strip_prefix("--mode=") {
            let cleaned = raw.trim().to_ascii_lowercase();
            if !cleaned.is_empty() {
                mode = cleaned;
            }
        } else if token == "--mode" {
            if let Some(next) = argv.get(i + 1).map(|v| v.trim()).filter(|v| !v.is_empty()) {
                mode = next.to_ascii_lowercase();
                i += 1;
            }
        } else if token == "--json" || token == "--json=1" {
            json_out = true;
        } else if token == "help" || token == "--help" || token == "-h" {
            mode = "help".to_string();
        } else if token == "registry" {
            mode = "registry".to_string();
        }
        i += 1;
    }
    (mode, json_out)
}

fn commands_json() -> Value {
    Value::Array(
        COMMANDS
            .iter()
            .map(|row| {
                json!({
                    "synopsis": row.synopsis,
                    "command": command_token_from_synopsis(row.synopsis),
                    "desc": row.desc,
                    "tier": row.tier.as_str(),
                    "handler": row.handler.as_str(),
                    "availability": row.availability_flag(),
                    "script_rel": row.script_rel,
                    "read_only": row.read_only,
                    "unsafe_surface": row.unsafe_surface
                })
            })
            .collect::<Vec<_>>(),
    )
}

fn print_list() {
    println!("InfRing command list (tier1 first):");
    for row in COMMANDS {
        println!("  - {:<12} {}", row.tier.as_str(), row.synopsis);
    }
}

fn print_help() {
    println!("Usage: infring <command> [flags]");
    println!();
    println!("Tier1 (stable product surface):");
    for row in COMMANDS {
        if matches!(row.tier, CommandTier::Tier1) {
            println!("  {:<45} {}", row.synopsis, row.desc);
        }
    }
    println!();
    println!("Experimental:");
    for row in COMMANDS {
        if matches!(row.tier, CommandTier::Experimental) {
            println!("  {:<45} {}", row.synopsis, row.desc);
        }
    }
}

fn print_registry_summary() {
    let summary = command_registry_integrity();
    println!(
        "Command registry: total={}, tier1={}, experimental={}",
        summary.get("total").and_then(Value::as_u64).unwrap_or(0),
        summary.get("tier1").and_then(Value::as_u64).unwrap_or(0),
        summary
            .get("experimental")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    let duplicates = summary
        .get("duplicates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if duplicates.is_empty() {
        println!("Registry integrity: ok");
    } else {
        println!("Registry integrity: duplicate command descriptors found");
    }
}

pub fn run(_root: &std::path::Path, argv: &[String]) -> i32 {
    let (mode, json_out) = parse_args(argv);
    if json_out {
        println!(
            "{}",
            json!({
                "ok": true,
                "type": "infring_command_list",
                "mode": mode,
                "commands": commands_json(),
                "registry_integrity": command_registry_integrity(),
                "route_contracts": tier1_route_contracts()
                    .iter()
                    .map(|row| {
                        json!({
                            "cmd": row.cmd,
                            "rest": row.rest,
                            "expected_script": row.expected_script
                        })
                    })
                    .collect::<Vec<_>>(),
                "aliases": command_aliases()
                    .iter()
                    .map(|row| json!({"alias": row.alias, "canonical": row.canonical}))
                    .collect::<Vec<_>>(),
            })
        );
        return 0;
    }

    match mode.as_str() {
        "help" => print_help(),
        "registry" => print_registry_summary(),
        _ => print_list(),
    }
    0
}

#[cfg(test)]
include!("../command_list_kernel_tests.rs");
